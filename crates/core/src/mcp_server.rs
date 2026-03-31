use crate::client::RagClient;
use crate::metadata::MetadataStore;
use crate::paths::PlatformPaths;
use crate::types::*;

use anyhow::{Context, Result};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
    handler::server::{router::prompt::PromptRouter, tool::ToolRouter, wrapper::Parameters},
    model::*,
    prompt, prompt_handler, prompt_router,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct RagMcpServer {
    client: Arc<RagClient>,
    metadata: Arc<MetadataStore>,
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
}

impl RagMcpServer {
    /// Create a new RAG MCP server with default configuration
    pub async fn new() -> Result<Self> {
        let client = RagClient::new().await?;
        let db_path = PlatformPaths::project_data_dir().join("papers.db");
        let metadata = MetadataStore::new(&db_path)?;
        Ok(Self {
            client: Arc::new(client),
            metadata: Arc::new(metadata),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        })
    }

    /// Create a new RAG MCP server with an existing client
    pub fn with_client(client: Arc<RagClient>) -> Result<Self> {
        let db_path = PlatformPaths::project_data_dir().join("papers.db");
        let metadata = MetadataStore::new(&db_path)?;
        Ok(Self {
            client,
            metadata: Arc::new(metadata),
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
    pub fn with_client_and_metadata(client: Arc<RagClient>, metadata: Arc<MetadataStore>) -> Result<Self> {
        Ok(Self {
            client,
            metadata,
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
                title: Some("Project RAG - Paper Library with Semantic Search".into()),
                version: env!("CARGO_PKG_VERSION").into(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "RAG-based paper library with semantic search. \
                Use search to search paper content semantically, \
                search_papers to find papers by title/authors/status."
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
