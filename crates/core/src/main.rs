use anyhow::Result;
use clap::{Parser, Subcommand};
use project_rag::extraction::{AlgorithmExtractor, PatternExtractor};
use project_rag::mcp_server::RagMcpServer;
use project_rag::metadata::MetadataStore;
use project_rag::RagClient;
use std::panic;
use std::sync::Arc;

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

    /// Show version and system information
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli = Cli::parse();

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

            if let Err(e) =
                project_rag::web::start_server(&host, port, client, metadata, upload_dir, Some(extractor), Some(algorithm_extractor)).await
            {
                tracing::error!("Fatal error in web server: {:#}", e);
                eprintln!("Fatal error: {:#}", e);
                std::process::exit(1);
            }
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
