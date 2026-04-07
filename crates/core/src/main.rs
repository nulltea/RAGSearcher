use anyhow::Result;
use clap::{Parser, Subcommand};
use project_rag::RagClient;
use project_rag::embedding::{EmbeddingProvider, format_retrieval_document};
use project_rag::extraction::{AlgorithmExtractor, PatternExtractor};
use project_rag::mcp_server::RagMcpServer;
use project_rag::metadata::MetadataStore;
use project_rag::metadata::models::{PaperStatus, Pattern, PatternStatus};
use project_rag::paths::PlatformPaths;
use project_rag::types::ChunkMetadata;
use std::panic;
use std::sync::Arc;
use std::time::Instant;

/// Project-RAG: Paper library with semantic search
#[derive(Parser)]
#[command(name = "rag-searcher")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Paper library with semantic search and MCP server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server over stdio (default mode)
    Serve,

    /// Start the HTTP web server for paper upload and search
    Web {
        /// Port to listen on
        #[arg(short, long, default_value = "3001")]
        port: u16,
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },

    /// Extract patterns from an indexed paper (3-pass AI pipeline, auto-approves)
    ExtractPatterns {
        /// Paper ID to extract patterns from
        paper_id: String,
    },

    /// List patterns for a paper (JSON output)
    ListPatterns {
        /// Paper ID to list patterns for
        paper_id: String,
        /// Filter by status: pending, approved, rejected
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Check if a paper exists by ID (JSON output: { "exists": bool, "paper": ... })
    CheckPaper {
        /// Paper ID to check
        paper_id: String,
    },

    /// Show version and system information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize tracing — MCP serve mode MUST use stderr (stdout is JSON-RPC channel)
    let use_stderr = matches!(
        cli.command,
        Some(Commands::Serve)
            | Some(Commands::ExtractPatterns { .. })
            | Some(Commands::ListPatterns { .. })
            | Some(Commands::CheckPaper { .. })
            | None
    );
    if use_stderr {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt::init();
    }

    // Handle commands
    match cli.command {
        Some(Commands::Version) => {
            show_version_info();
            return Ok(());
        }
        Some(Commands::Web { port, host }) => {
            setup_panic_handler();

            let client = Arc::new(
                RagClient::new()
                    .await
                    .expect("Failed to initialize RAG client"),
            );

            let data_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("project-rag");
            let db_path = data_dir.join("papers.db");
            let upload_dir = data_dir.join("uploads");

            let metadata = Arc::new(
                MetadataStore::new(&db_path).expect("Failed to initialize metadata store"),
            );

            let extractor = Arc::new(PatternExtractor::new());
            let algorithm_extractor = Arc::new(AlgorithmExtractor::new());

            if let Err(e) = project_rag::web::start_server(
                &host,
                port,
                client,
                metadata,
                upload_dir,
                Some(extractor),
                Some(algorithm_extractor),
            )
            .await
            {
                tracing::error!("Fatal error in web server: {:#}", e);
                eprintln!("Fatal error: {:#}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::ExtractPatterns { paper_id }) => {
            setup_panic_handler();
            extract_patterns_cli(&paper_id).await?;
        }
        Some(Commands::ListPatterns { paper_id, status }) => {
            list_patterns_cli(&paper_id, status.as_deref()).await?;
        }
        Some(Commands::CheckPaper { paper_id }) => {
            check_paper_cli(&paper_id).await?;
        }
        Some(Commands::Serve) | None => {
            // Set up global panic handler
            setup_panic_handler();

            // Start the RAG MCP server over stdio with error handling
            if let Err(e) = RagMcpServer::serve_stdio().await {
                tracing::error!("Fatal error in MCP server: {:#}", e);
                eprintln!("Fatal error: {:#}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

/// Display comprehensive version and system information
fn show_version_info() {
    println!("rag-searcher v{}", env!("CARGO_PKG_VERSION"));
    println!();

    println!("System Information:");
    println!("  Build Date:      {}", env!("BUILD_TIMESTAMP"));
    println!("  Git Commit:      {}", env!("GIT_COMMIT_HASH"));
    println!("  Rust Version:    {}", env!("CARGO_PKG_RUST_VERSION"));
    println!();

    println!("Vector Database:");
    let backend = env!("VECTOR_DB_BACKEND");
    println!("  Backend:         {}", backend);

    #[cfg(not(feature = "qdrant-backend"))]
    {
        use project_rag::vector_db::lance_client::LanceVectorDB;
        let default_path = LanceVectorDB::default_lancedb_path();
        println!("  Default Path:    {}", default_path);
        println!("  Type:            Embedded (no external server required)");
    }

    #[cfg(feature = "qdrant-backend")]
    {
        use project_rag::vector_db::qdrant_client::QdrantVectorDB;
        let default_url = QdrantVectorDB::default_url();
        println!("  Default URL:     {}", default_url);
        println!("  Type:            External server (requires Qdrant running)");
    }
    println!();

    println!("Embedding Model:");
    println!("  Model:           all-MiniLM-L6-v2");
    println!("  Dimensions:      384");
    println!("  Provider:        FastEmbed (local, no API calls)");
    println!();

    println!("Configuration:");
    use project_rag::paths::PlatformPaths;
    let config_path = PlatformPaths::default_config_path();
    println!("  Config File:     {}", config_path.display());
    println!("  Config Priority: CLI args > Env vars > Config file > Defaults");
    println!("  Env Prefix:      PROJECT_RAG_*");
    println!();

    println!("Features:");
    println!("  Hybrid Search:   Enabled (Vector + BM25 keyword search)");
    println!("  Paper Library:   Upload, extract, and search papers");
    println!("  Extraction:      Pattern and algorithm extraction via Claude CLI");
}

// --- CLI command implementations ---

#[derive(serde::Serialize)]
struct ExtractPatternsResponse {
    paper_id: String,
    pattern_count: usize,
    evidence_count: usize,
    verification_status: Option<String>,
    duration_ms: u64,
}

#[derive(serde::Serialize)]
struct ListPatternsResponse {
    patterns: Vec<Pattern>,
    count: usize,
}

async fn extract_patterns_cli(paper_id: &str) -> Result<()> {
    let start = Instant::now();

    let data_dir = PlatformPaths::project_data_dir();
    let db_path = data_dir.join("papers.db");
    let upload_dir = data_dir.join("uploads");

    let client = Arc::new(RagClient::new().await?);
    let metadata = Arc::new(MetadataStore::new(&db_path)?);
    let extractor = PatternExtractor::new();

    // Verify paper exists
    let paper = metadata
        .get_paper(paper_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Paper '{}' not found", paper_id))?;

    // Verify text file exists
    let text_path = upload_dir.join(format!("{}.txt", paper_id));
    if !text_path.exists() {
        anyhow::bail!("Paper text not found at {}", text_path.display());
    }
    let text_path_str = text_path.to_string_lossy().to_string();

    // Run 3-pass extraction
    let result = extractor.extract_patterns(&text_path_str).await?;

    // Delete existing patterns (re-extraction)
    metadata.delete_patterns_by_paper(paper_id).await?;

    // Save patterns to SQLite
    for p in &result.patterns {
        metadata
            .create_pattern(
                paper_id,
                &p.name,
                p.claim.as_deref(),
                p.evidence.as_deref(),
                p.context.as_deref(),
                &p.tags,
                &p.confidence,
            )
            .await?;
    }

    // Auto-approve all extracted patterns
    let pending = metadata.list_patterns(paper_id, Some("pending")).await?;
    for p in &pending {
        metadata
            .update_pattern_status(&p.id, PatternStatus::Approved)
            .await?;
    }

    // Embed approved patterns into LanceDB
    if !pending.is_empty() {
        let texts: Vec<String> = pending
            .iter()
            .map(|p| {
                let mut parts = vec![p.name.clone()];
                if let Some(ref c) = p.claim {
                    parts.push(c.clone());
                }
                if let Some(ref e) = p.evidence {
                    parts.push(e.clone());
                }
                if let Some(ref ctx) = p.context {
                    parts.push(ctx.clone());
                }
                parts.join(" | ")
            })
            .collect();

        let chunk_metadata: Vec<ChunkMetadata> = pending
            .iter()
            .map(|p| ChunkMetadata {
                chunk_id: None,
                file_path: format!("patterns/{}", p.paper_id),
                root_path: Some("patterns".to_string()),
                start_line: 0,
                end_line: 0,
                language: Some("Pattern".to_string()),
                extension: Some("pattern".to_string()),
                file_hash: p.id.clone(),
                indexed_at: chrono::Utc::now().timestamp(),
                project: Some(format!("pattern:{}", p.paper_id)),
                page_numbers: None,
                heading_context: None,
                element_types: None,
            })
            .collect();

        let contents: Vec<String> = texts.clone();
        let texts: Vec<String> = texts
            .into_iter()
            .map(|text| format_retrieval_document(None, &text))
            .collect();
        let provider = client.embedding_provider().clone();
        let embeddings = tokio::task::spawn_blocking(move || provider.embed_batch(texts)).await??;

        client
            .vector_db()
            .store_embeddings(embeddings, chunk_metadata, contents, "patterns")
            .await?;
    }

    // Update paper status to Active
    metadata
        .update_paper_status(paper_id, PaperStatus::Active, paper.chunk_count)
        .await?;

    let verification_status = result
        .verification
        .as_ref()
        .map(|v| v.verification_status.clone());

    let response = ExtractPatternsResponse {
        paper_id: paper_id.to_string(),
        pattern_count: result.patterns.len(),
        evidence_count: result.evidence.evidence_items.len(),
        verification_status,
        duration_ms: start.elapsed().as_millis() as u64,
    };

    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

async fn list_patterns_cli(paper_id: &str, status: Option<&str>) -> Result<()> {
    let data_dir = PlatformPaths::project_data_dir();
    let db_path = data_dir.join("papers.db");
    let metadata = MetadataStore::new(&db_path)?;

    let patterns = metadata.list_patterns(paper_id, status).await?;
    let count = patterns.len();

    let response = ListPatternsResponse { patterns, count };
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

async fn check_paper_cli(paper_id: &str) -> Result<()> {
    let data_dir = PlatformPaths::project_data_dir();
    let db_path = data_dir.join("papers.db");
    let metadata = MetadataStore::new(&db_path)?;

    let paper = metadata.get_paper(paper_id).await?;
    let response = serde_json::json!({
        "exists": paper.is_some(),
        "paper": paper,
    });
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

/// Set up a global panic handler that logs panic information
fn setup_panic_handler() {
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::capture();

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic message".to_string()
        };

        tracing::error!(
            "PANIC at {}: {}\nBacktrace:\n{:?}",
            location,
            message,
            backtrace
        );

        eprintln!("\n!!! PANIC !!!");
        eprintln!("Location: {}", location);
        eprintln!("Message: {}", message);
        eprintln!("Backtrace:\n{:?}", backtrace);
        eprintln!("!!! END PANIC !!!\n");
    }));

    tracing::info!("Global panic handler initialized");
}
