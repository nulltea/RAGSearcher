use crate::client::RagClient;
use crate::embedding::EmbeddingProvider;
use crate::extraction::AlgorithmExtractor;
use crate::indexer::{ChunkInput, extract_pdf};
use crate::metadata::MetadataStore;
use crate::metadata::models::{AlgorithmIORow, AlgorithmStepRow, PaperCreate, PaperStatus, PatternStatus};
use crate::paths::PlatformPaths;
use crate::types::*;
use crate::vector_db::VectorDatabase;

use anyhow::{Context, Result};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
    handler::server::{router::prompt::PromptRouter, tool::ToolRouter, wrapper::Parameters},
    model::*,
    prompt, prompt_handler, prompt_router,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct RagMcpServer {
    client: Arc<RagClient>,
    metadata: Arc<MetadataStore>,
    upload_dir: PathBuf,
    algorithm_extractor: Arc<AlgorithmExtractor>,
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
}

impl RagMcpServer {
    /// Create a new RAG MCP server with default configuration
    pub async fn new() -> Result<Self> {
        let client = RagClient::new().await?;
        let data_dir = PlatformPaths::project_data_dir();
        let db_path = data_dir.join("papers.db");
        let upload_dir = data_dir.join("uploads");
        let metadata = MetadataStore::new(&db_path)?;
        Ok(Self {
            client: Arc::new(client),
            metadata: Arc::new(metadata),
            upload_dir,
            algorithm_extractor: Arc::new(AlgorithmExtractor::new()),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        })
    }

    /// Create a new RAG MCP server with an existing client
    pub fn with_client(client: Arc<RagClient>) -> Result<Self> {
        let data_dir = PlatformPaths::project_data_dir();
        let db_path = data_dir.join("papers.db");
        let upload_dir = data_dir.join("uploads");
        let metadata = MetadataStore::new(&db_path)?;
        Ok(Self {
            client,
            metadata: Arc::new(metadata),
            upload_dir,
            algorithm_extractor: Arc::new(AlgorithmExtractor::new()),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        })
    }

    /// Get the underlying client
    pub fn client(&self) -> &RagClient {
        &self.client
    }

    /// Create a new RAG MCP server with custom configuration
    pub async fn with_config(config: crate::config::Config) -> Result<Self> {
        let client = RagClient::with_config(config).await?;
        Self::with_client(Arc::new(client))
    }

    /// Create a new RAG MCP server with an existing client and metadata store
    pub fn with_client_and_metadata(client: Arc<RagClient>, metadata: Arc<MetadataStore>, upload_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            client,
            metadata,
            upload_dir,
            algorithm_extractor: Arc::new(AlgorithmExtractor::new()),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        })
    }

    /// Normalize a path to a canonical absolute form
    pub fn normalize_path(path: &str) -> Result<String> {
        RagClient::normalize_path(path)
    }
}

#[tool_router(router = tool_router)]
impl RagMcpServer {
    #[tool(description = "Search indexed papers using semantic search")]
    async fn search(
        &self,
        Parameters(req): Parameters<QueryRequest>,
    ) -> Result<String, String> {
        req.validate()?;

        let response = self
            .client
            .query_codebase(req)
            .await
            .map_err(|e| format!("{:#}", e))?;

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {}", e))
    }

    #[tool(description = "Get statistics about the indexed content")]
    async fn get_statistics(
        &self,
        Parameters(_req): Parameters<StatisticsRequest>,
    ) -> Result<String, String> {
        let response = self
            .client
            .get_statistics()
            .await
            .map_err(|e| format!("{:#}", e))?;

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {}", e))
    }

    #[tool(description = "Search papers by title, authors, status, or type. Returns paper metadata including chunk count and status.")]
    async fn search_papers(
        &self,
        Parameters(req): Parameters<SearchPapersRequest>,
    ) -> Result<String, String> {
        let start = std::time::Instant::now();

        let (papers, total) = self
            .metadata
            .search_papers(
                req.query.as_deref(),
                req.status.as_deref(),
                req.paper_type.as_deref(),
                req.limit,
                req.offset,
            )
            .await
            .map_err(|e| format!("{:#}", e))?;

        let results: Vec<PaperResult> = papers
            .into_iter()
            .map(|p| PaperResult {
                id: p.id,
                title: p.title,
                authors: p.authors,
                source: p.source,
                published_date: p.published_date,
                paper_type: p.paper_type,
                status: p.status.to_string(),
                chunk_count: p.chunk_count,
                file_path: p.file_path,
                created_at: p.created_at,
            })
            .collect();

        let response = SearchPapersResponse {
            total,
            limit: req.limit,
            offset: req.offset,
            duration_ms: start.elapsed().as_millis() as u64,
            papers: results,
        };

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {}", e))
    }

    #[tool(description = "Search algorithms across all papers by keyword, status, tags, or paper. Returns structured algorithm data including steps, I/O, and pseudocode.")]
    async fn search_algorithms(
        &self,
        Parameters(req): Parameters<SearchAlgorithmsRequest>,
    ) -> Result<String, String> {
        let start = std::time::Instant::now();

        let (algorithms, total) = self
            .metadata
            .search_algorithms(
                req.query.as_deref(),
                req.status.as_deref(),
                req.paper_id.as_deref(),
                req.tags.as_deref(),
                req.limit,
                req.offset,
            )
            .await
            .map_err(|e| format!("{:#}", e))?;

        let results: Vec<AlgorithmResult> = algorithms
            .into_iter()
            .map(|(alg, paper_title)| AlgorithmResult {
                id: alg.id,
                paper_id: alg.paper_id,
                paper_title,
                name: alg.name,
                description: alg.description,
                steps: alg.steps.into_iter().map(|s| serde_json::to_value(s).unwrap_or_default()).collect(),
                inputs: alg.inputs.into_iter().map(|i| serde_json::to_value(i).unwrap_or_default()).collect(),
                outputs: alg.outputs.into_iter().map(|o| serde_json::to_value(o).unwrap_or_default()).collect(),
                preconditions: alg.preconditions,
                complexity: alg.complexity,
                mathematical_notation: alg.mathematical_notation,
                pseudocode: alg.pseudocode,
                tags: alg.tags,
                confidence: alg.confidence,
                status: alg.status.to_string(),
                created_at: alg.created_at,
            })
            .collect();

        let response = SearchAlgorithmsResponse {
            total,
            limit: req.limit,
            offset: req.offset,
            duration_ms: start.elapsed().as_millis() as u64,
            algorithms: results,
        };

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {}", e))
    }

    #[tool(description = "Index a paper from a local file path or URL. Extracts text, chunks it, generates embeddings, and stores in the vector database for semantic search.")]
    async fn index_paper(
        &self,
        Parameters(req): Parameters<IndexPaperRequest>,
    ) -> Result<String, String> {
        let start = std::time::Instant::now();
        let paper_id = uuid::Uuid::new_v4().to_string();

        // Ensure upload dir exists
        tokio::fs::create_dir_all(&self.upload_dir)
            .await
            .map_err(|e| format!("Failed to create upload directory: {}", e))?;

        // Read file content
        let (bytes, original_filename, source) = if let Some(ref path) = req.file_path {
            let path = std::path::Path::new(path);
            if !path.exists() {
                return Err(format!("File not found: {}", path.display()));
            }
            let bytes = tokio::fs::read(path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;
            let filename = path.file_name().map(|f| f.to_string_lossy().to_string());
            (bytes, filename, req.source.clone())
        } else if let Some(ref url) = req.url {
            let response = reqwest::get(url)
                .await
                .map_err(|e| format!("Failed to download URL: {}", e))?;
            if !response.status().is_success() {
                return Err(format!("URL returned status {}", response.status()));
            }
            let bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Failed to read response body: {}", e))?
                .to_vec();
            let filename = url.rsplit('/').next()
                .unwrap_or("download.pdf")
                .split('?').next()
                .unwrap_or("download.pdf")
                .to_string();
            (bytes, Some(filename), req.source.clone().or_else(|| Some(url.clone())))
        } else {
            return Err("Either 'file_path' or 'url' is required".to_string());
        };

        // Determine extension and extract text
        let ext = original_filename.as_deref()
            .and_then(|f| f.rsplit('.').next())
            .unwrap_or("pdf");

        // Save file to upload dir
        let saved_path = self.upload_dir.join(format!("{}.{}", paper_id, ext));
        tokio::fs::write(&saved_path, &bytes)
            .await
            .map_err(|e| format!("Failed to save file: {}", e))?;

        let stored_file_path = saved_path.canonicalize().ok().map(|p| p.to_string_lossy().to_string());

        let (content, pdf_title) = if ext.eq_ignore_ascii_case("pdf") {
            let path = saved_path.clone();
            let extraction = tokio::task::spawn_blocking(move || extract_pdf(&path))
                .await
                .map_err(|e| format!("Task join error: {}", e))?
                .map_err(|e| format!("PDF extraction failed: {:#}", e))?;
            (extraction.text, extraction.title)
        } else {
            let text = String::from_utf8(bytes)
                .map_err(|_| "File is not valid UTF-8 text".to_string())?;
            (text, None)
        };

        // Resolve title
        let title = req.title.or(pdf_title).unwrap_or_else(|| {
            original_filename.as_deref()
                .and_then(|f| f.rsplit('.').nth(1).map(|s| s.to_string()))
                .unwrap_or_else(|| "Untitled Paper".to_string())
        });

        let authors: Vec<String> = req.authors
            .map(|a| a.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();

        // Create paper metadata record
        let create = PaperCreate {
            title: title.clone(),
            authors,
            source,
            published_date: None,
            paper_type: req.paper_type.clone(),
            original_filename,
            file_path: stored_file_path,
        };

        self.metadata
            .create_paper(&paper_id, create)
            .await
            .map_err(|e| format!("Failed to create paper record: {:#}", e))?;

        // Save extracted text for later pattern/algorithm extraction
        let text_path = self.upload_dir.join(format!("{}.txt", paper_id));
        tokio::fs::write(&text_path, &content)
            .await
            .map_err(|e| format!("Failed to save text content: {}", e))?;

        // Chunk the content
        let chunk_input = ChunkInput {
            relative_path: format!("papers/{}", paper_id),
            root_path: "papers".to_string(),
            project: Some(paper_id.clone()),
            extension: Some("md".to_string()),
            language: Some("Markdown".to_string()),
            content: content.clone(),
            hash: {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                format!("{:x}", hasher.finalize())
            },
        };

        let chunks = self.client.chunker.chunk_file(&chunk_input);
        let chunk_count = if chunks.is_empty() {
            self.metadata
                .update_paper_status(&paper_id, PaperStatus::Active, 0)
                .await
                .map_err(|e| format!("{:#}", e))?;
            0
        } else {
            let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
            let metadata: Vec<ChunkMetadata> = chunks.iter().map(|c| c.metadata.clone()).collect();
            let contents: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();

            let provider = self.client.embedding_provider.clone();
            let embeddings = tokio::task::spawn_blocking(move || provider.embed_batch(texts))
                .await
                .map_err(|e| format!("Embedding task error: {}", e))?
                .map_err(|e| format!("Embedding generation failed: {:#}", e))?;

            let count = self.client.vector_db
                .store_embeddings(embeddings, metadata, contents, "papers")
                .await
                .map_err(|e| format!("Failed to store embeddings: {:#}", e))?;

            self.metadata
                .update_paper_status(&paper_id, PaperStatus::Active, count)
                .await
                .map_err(|e| format!("{:#}", e))?;
            count
        };

        let response = IndexPaperResponse {
            paper_id,
            title,
            chunk_count,
            status: PaperStatus::Active.to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        };

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {}", e))
    }

    #[tool(description = "Extract algorithms from an indexed paper using a 3-pass AI pipeline (evidence inventory → algorithm definitions → verification). The paper must be indexed first via index_paper. Takes 30-120 seconds depending on paper length.")]
    async fn extract_algorithms(
        &self,
        Parameters(req): Parameters<ExtractAlgorithmsRequest>,
    ) -> Result<String, String> {
        let start = std::time::Instant::now();

        // Verify paper exists
        let paper = self
            .metadata
            .get_paper(&req.paper_id)
            .await
            .map_err(|e| format!("{:#}", e))?
            .ok_or_else(|| format!("Paper '{}' not found", req.paper_id))?;

        // Load extracted text
        let text_path = self.upload_dir.join(format!("{}.txt", req.paper_id));
        let text = tokio::fs::read_to_string(&text_path).await.map_err(|e| {
            format!(
                "Paper text not found at {}. Was the paper uploaded correctly? Error: {}",
                text_path.display(),
                e
            )
        })?;

        // Run 3-pass extraction pipeline
        let result = self
            .algorithm_extractor
            .extract_algorithms(&text, None)
            .await
            .map_err(|e| format!("Algorithm extraction failed: {:#}", e))?;

        // Delete existing algorithms for this paper (re-extraction)
        self.metadata
            .delete_algorithms_by_paper(&req.paper_id)
            .await
            .map_err(|e| format!("{:#}", e))?;

        // Save extracted algorithms to SQLite
        for a in &result.algorithms {
            let steps: Vec<AlgorithmStepRow> = a
                .steps
                .iter()
                .map(|s| AlgorithmStepRow {
                    number: s.number,
                    action: s.action.clone(),
                    details: s.details.clone(),
                    math: s.math.clone(),
                })
                .collect();

            let inputs: Vec<AlgorithmIORow> = a
                .inputs
                .iter()
                .map(|io| AlgorithmIORow {
                    name: io.name.clone(),
                    io_type: io.io_type.clone(),
                    description: io.description.clone(),
                })
                .collect();

            let outputs: Vec<AlgorithmIORow> = a
                .outputs
                .iter()
                .map(|io| AlgorithmIORow {
                    name: io.name.clone(),
                    io_type: io.io_type.clone(),
                    description: io.description.clone(),
                })
                .collect();

            self.metadata
                .create_algorithm(
                    &req.paper_id,
                    &a.name,
                    Some(&a.description),
                    &steps,
                    &inputs,
                    &outputs,
                    &a.preconditions,
                    a.complexity.as_deref(),
                    a.mathematical_notation.as_deref(),
                    a.pseudocode.as_deref(),
                    &a.tags,
                    &a.evidence_ids,
                    &a.confidence,
                )
                .await
                .map_err(|e| format!("{:#}", e))?;
        }

        // Auto-approve all extracted algorithms and embed into LanceDB
        let approved = self
            .metadata
            .list_algorithms(&req.paper_id, Some("pending"))
            .await
            .map_err(|e| format!("{:#}", e))?;

        for alg in &approved {
            self.metadata
                .update_algorithm_status(&alg.id, PatternStatus::Approved)
                .await
                .map_err(|e| format!("{:#}", e))?;
        }

        if !approved.is_empty() {
            let texts: Vec<String> = approved
                .iter()
                .map(|a| {
                    let mut parts = vec![a.name.clone()];
                    if let Some(ref d) = a.description {
                        parts.push(d.clone());
                    }
                    for step in &a.steps {
                        parts.push(format!("{}. {}", step.number, step.action));
                    }
                    parts.join(" | ")
                })
                .collect();

            let metadata: Vec<ChunkMetadata> = approved
                .iter()
                .map(|a| ChunkMetadata {
                    file_path: format!("algorithms/{}", a.paper_id),
                    root_path: Some("algorithms".to_string()),
                    start_line: 0,
                    end_line: 0,
                    language: Some("Algorithm".to_string()),
                    extension: Some("algorithm".to_string()),
                    file_hash: a.id.clone(),
                    indexed_at: chrono::Utc::now().timestamp(),
                    project: Some(format!("algorithm:{}", a.paper_id)),
                })
                .collect();

            let contents: Vec<String> = texts.clone();
            let provider = self.client.embedding_provider.clone();
            let embeddings = tokio::task::spawn_blocking(move || provider.embed_batch(texts))
                .await
                .map_err(|e| format!("Embedding task error: {}", e))?
                .map_err(|e| format!("Embedding failed: {:#}", e))?;

            self.client
                .vector_db
                .store_embeddings(embeddings, metadata, contents, "algorithms")
                .await
                .map_err(|e| format!("Failed to store algorithm embeddings: {:#}", e))?;
        }

        // Update paper status to active (algorithms auto-approved)
        self.metadata
            .update_paper_status(&req.paper_id, PaperStatus::Active, paper.chunk_count)
            .await
            .map_err(|e| format!("{:#}", e))?;

        let verification_status = result
            .verification
            .as_ref()
            .map(|v| v.verification_status.clone());

        let response = ExtractAlgorithmsResponse {
            paper_id: req.paper_id.clone(),
            algorithm_count: result.algorithms.len(),
            evidence_count: result.evidence.evidence_items.len(),
            verification_status,
            duration_ms: start.elapsed().as_millis() as u64,
        };

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {}", e))
    }
}

// Prompts for slash commands
#[prompt_router]
impl RagMcpServer {
    #[prompt(
        name = "search",
        description = "Search indexed papers using semantic search"
    )]
    async fn search_prompt(
        &self,
        Parameters(args): Parameters<serde_json::Value>,
    ) -> Result<Vec<PromptMessage>, McpError> {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

        Ok(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            format!("Please search the papers for: {}", query),
        )])
    }

    #[prompt(
        name = "papers",
        description = "Search papers in the knowledge base by title, authors, status, or type"
    )]
    async fn papers_prompt(
        &self,
        Parameters(args): Parameters<serde_json::Value>,
    ) -> Result<Vec<PromptMessage>, McpError> {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

        Ok(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            if query.is_empty() {
                "Please list all available papers in the knowledge base.".to_string()
            } else {
                format!("Please search for papers matching: {}", query)
            },
        )])
    }

    #[prompt(
        name = "index",
        description = "Index a paper from a local file path or URL"
    )]
    async fn index_prompt(
        &self,
        Parameters(args): Parameters<serde_json::Value>,
    ) -> Result<Vec<PromptMessage>, McpError> {
        let path_or_url = args
            .get("path_or_url")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            if path_or_url.is_empty() {
                "Please index a paper. Provide a file path or URL.".to_string()
            } else {
                format!("Please index this paper: {}", path_or_url)
            },
        )])
    }

    #[prompt(
        name = "algorithms",
        description = "Search algorithms extracted from papers by keyword, tags, or paper"
    )]
    async fn algorithms_prompt(
        &self,
        Parameters(args): Parameters<serde_json::Value>,
    ) -> Result<Vec<PromptMessage>, McpError> {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

        Ok(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            if query.is_empty() {
                "Please list approved algorithms from the paper library.".to_string()
            } else {
                format!("Please search for algorithms matching: {}", query)
            },
        )])
    }
}

#[tool_handler(router = self.tool_router)]
#[prompt_handler]
impl ServerHandler for RagMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            server_info: Implementation {
                name: "rag-searcher".into(),
                title: Some("RAGSearcher - Paper Library with Semantic Search".into()),
                version: env!("CARGO_PKG_VERSION").into(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "RAG-based paper library with semantic search. \
                Use search to search paper content semantically, \
                search_papers to find papers by title/authors/status, \
                search_algorithms to find algorithms across papers by keyword/tags, \
                index_paper to upload and index a paper from a file path or URL, \
                extract_algorithms to run AI-powered algorithm extraction on an indexed paper."
                    .into(),
            ),
        }
    }
}

impl RagMcpServer {
    pub async fn serve_stdio() -> Result<()> {
        tracing::info!("Starting RAG MCP server");

        let server = Self::new().await.context("Failed to create MCP server")?;

        let transport = rmcp::transport::io::stdio();

        server.serve(transport).await?.waiting().await?;

        Ok(())
    }
}
