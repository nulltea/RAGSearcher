//! Core library client for project-rag
//!
//! This module provides the main client interface for using project-rag
//! as a library for paper embedding and semantic search.

use crate::chunker::{ContextAwareChunker, FixedChunker};
use crate::config::Config;
use crate::embedding::{EmbeddingProvider, MistralRsEmbedder, format_retrieval_query};
use crate::types::*;
use crate::vector_db::VectorDatabase;

// Conditionally import the appropriate vector database backend
#[cfg(feature = "qdrant-backend")]
use crate::vector_db::QdrantVectorDB;

#[cfg(not(feature = "qdrant-backend"))]
use crate::vector_db::LanceVectorDB;

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Main client for interacting with the RAG system
///
/// This client provides a high-level API for embedding papers and performing
/// semantic searches. It contains all the core functionality and can be used
/// directly as a library or wrapped by the MCP server.
#[derive(Clone)]
pub struct RagClient {
    pub(crate) embedding_provider: Arc<MistralRsEmbedder>,
    #[cfg(feature = "qdrant-backend")]
    pub(crate) vector_db: Arc<QdrantVectorDB>,
    #[cfg(not(feature = "qdrant-backend"))]
    pub(crate) vector_db: Arc<LanceVectorDB>,
    pub(crate) chunker: Arc<FixedChunker>,
    pub(crate) pdf_chunker: Arc<ContextAwareChunker>,
    pub(crate) config: Arc<Config>,
}

impl RagClient {
    /// Get the embedding provider
    pub fn embedding_provider(&self) -> &Arc<MistralRsEmbedder> {
        &self.embedding_provider
    }

    /// Get the vector database
    #[cfg(feature = "qdrant-backend")]
    pub fn vector_db(&self) -> &Arc<QdrantVectorDB> {
        &self.vector_db
    }

    /// Get the vector database
    #[cfg(not(feature = "qdrant-backend"))]
    pub fn vector_db(&self) -> &Arc<LanceVectorDB> {
        &self.vector_db
    }

    /// Create a new RAG client with default configuration
    pub async fn new() -> Result<Self> {
        let config = Config::new().context("Failed to load configuration")?;
        Self::with_config(config).await
    }

    /// Create a new RAG client with custom configuration
    pub async fn with_config(config: Config) -> Result<Self> {
        tracing::info!("Initializing RAG client with configuration");
        tracing::debug!("Vector DB backend: {}", config.vector_db.backend);
        tracing::debug!("Embedding model: {}", config.embedding.model_name);
        tracing::debug!("Chunk size: {}", config.indexing.chunk_size);

        // Initialize embedding provider
        let embedding_provider = Arc::new(
            MistralRsEmbedder::new()
                .await
                .context("Failed to initialize embedding provider")?,
        );

        // Initialize the appropriate vector database backend
        #[cfg(feature = "qdrant-backend")]
        let vector_db = {
            tracing::info!(
                "Using Qdrant vector database backend at {}",
                config.vector_db.qdrant_url
            );
            Arc::new(
                QdrantVectorDB::with_url(&config.vector_db.qdrant_url)
                    .await
                    .context("Failed to initialize Qdrant vector database")?,
            )
        };

        #[cfg(not(feature = "qdrant-backend"))]
        let vector_db = {
            tracing::info!(
                "Using LanceDB vector database backend at {}",
                config.vector_db.lancedb_path.display()
            );
            Arc::new(
                LanceVectorDB::with_path(&config.vector_db.lancedb_path.to_string_lossy())
                    .await
                    .context("Failed to initialize LanceDB vector database")?,
            )
        };

        // Initialize the database with the embedding dimension
        vector_db
            .initialize(embedding_provider.dimension())
            .await
            .context("Failed to initialize vector database collections")?;

        // Create chunkers
        let chunker = Arc::new(FixedChunker::default_strategy());
        let pdf_chunker = Arc::new(ContextAwareChunker::new());

        Ok(Self {
            embedding_provider,
            vector_db,
            chunker,
            pdf_chunker,
            config: Arc::new(config),
        })
    }

    /// Create a new client with custom database path (for testing)
    #[cfg(test)]
    pub async fn new_with_db_path(db_path: &str, cache_path: PathBuf) -> Result<Self> {
        let mut config = Config::default();
        config.vector_db.lancedb_path = PathBuf::from(db_path);
        Self::with_config(config).await
    }

    /// Normalize a path to a canonical absolute form
    pub fn normalize_path(path: &str) -> Result<String> {
        let path_buf = PathBuf::from(path);
        let canonical = std::fs::canonicalize(&path_buf)
            .with_context(|| format!("Failed to canonicalize path: {}", path))?;
        Ok(canonical.to_string_lossy().to_string())
    }

    /// Query the indexed content using semantic search
    pub async fn query_codebase(&self, request: QueryRequest) -> Result<QueryResponse> {
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        let start = Instant::now();

        let provider = self.embedding_provider.clone();
        let query_text = format_retrieval_query(&request.query);
        let query_embedding =
            tokio::task::spawn_blocking(move || provider.embed_batch(vec![query_text]))
                .await?
                .context("Failed to generate query embedding")?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No embedding generated"))?;

        let original_threshold = request.min_score;
        let mut threshold_used = original_threshold;
        let mut threshold_lowered = false;

        let mut results = self
            .vector_db
            .search(
                query_embedding.clone(),
                &request.query,
                request.limit,
                threshold_used,
                request.project.clone(),
                request.path.clone(),
                request.hybrid,
            )
            .await
            .context("Failed to search")?;

        if results.is_empty() && original_threshold > 0.3 {
            let fallback_thresholds = [0.6, 0.5, 0.4, 0.3];

            for &threshold in &fallback_thresholds {
                if threshold >= original_threshold {
                    continue;
                }

                results = self
                    .vector_db
                    .search(
                        query_embedding.clone(),
                        &request.query,
                        request.limit,
                        threshold,
                        request.project.clone(),
                        request.path.clone(),
                        request.hybrid,
                    )
                    .await
                    .context("Failed to search")?;

                if !results.is_empty() {
                    threshold_used = threshold;
                    threshold_lowered = true;
                    break;
                }
            }
        }

        Ok(QueryResponse {
            results,
            duration_ms: start.elapsed().as_millis() as u64,
            threshold_used,
            threshold_lowered,
        })
    }

    /// Get statistics about the indexed content
    pub async fn get_statistics(&self) -> Result<StatisticsResponse> {
        let stats = self
            .vector_db
            .get_statistics()
            .await
            .context("Failed to get statistics")?;

        let language_breakdown = stats
            .language_breakdown
            .into_iter()
            .map(|(language, count)| LanguageStats {
                language,
                file_count: count,
                chunk_count: count,
            })
            .collect();

        Ok(StatisticsResponse {
            total_files: stats.total_points,
            total_chunks: stats.total_vectors,
            total_embeddings: stats.total_vectors,
            database_size_bytes: 0,
            language_breakdown,
        })
    }

    /// Clear all indexed data from the vector database
    pub async fn clear_index(&self) -> Result<ClearResponse> {
        match self.vector_db.clear().await {
            Ok(_) => {
                if let Err(e) = self
                    .vector_db
                    .initialize(self.embedding_provider.dimension())
                    .await
                {
                    Ok(ClearResponse {
                        success: false,
                        message: format!("Cleared but failed to reinitialize: {}", e),
                    })
                } else {
                    Ok(ClearResponse {
                        success: true,
                        message: "Successfully cleared all indexed data".to_string(),
                    })
                }
            }
            Err(e) => Ok(ClearResponse {
                success: false,
                message: format!("Failed to clear index: {}", e),
            }),
        }
    }

    /// Get the configuration used by this client
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the embedding dimension used by this client
    pub fn embedding_dimension(&self) -> usize {
        self.embedding_provider.dimension()
    }
}
