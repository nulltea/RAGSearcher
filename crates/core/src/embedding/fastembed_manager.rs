use super::EmbeddingProvider;
use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::RwLock;

/// FastEmbed-based embedding provider with CoreML acceleration on Apple Silicon.
///
/// Uses RwLock for safe interior mutability since fastembed's embed() requires &mut self.
pub struct FastEmbedManager {
    model: RwLock<TextEmbedding>,
    dimension: usize,
    model_name_str: &'static str,
}

impl FastEmbedManager {
    /// Create a new FastEmbedManager with the default model (jina-embeddings-v2-base-en)
    pub fn new() -> Result<Self> {
        Self::with_model(EmbeddingModel::JinaEmbeddingsV2BaseEN)
    }

    /// Create a new FastEmbedManager from a model name string
    pub fn from_model_name(model_name: &str) -> Result<Self> {
        let model = match model_name {
            "jinaai/jina-embeddings-v2-base-en" => EmbeddingModel::JinaEmbeddingsV2BaseEN,
            "all-MiniLM-L6-v2" => EmbeddingModel::AllMiniLML6V2,
            "all-MiniLM-L12-v2" => EmbeddingModel::AllMiniLML12V2,
            "BAAI/bge-base-en-v1.5" => EmbeddingModel::BGEBaseENV15,
            "BAAI/bge-small-en-v1.5" => EmbeddingModel::BGESmallENV15,
            _ => {
                tracing::warn!(
                    "Unknown model '{}', falling back to jinaai/jina-embeddings-v2-base-en",
                    model_name
                );
                EmbeddingModel::JinaEmbeddingsV2BaseEN
            }
        };
        Self::with_model(model)
    }

    /// Create a new FastEmbedManager with a specific model
    pub fn with_model(model: EmbeddingModel) -> Result<Self> {
        tracing::info!("Initializing FastEmbed model: {:?}", model);

        let (dimension, model_name_str) = match model {
            EmbeddingModel::JinaEmbeddingsV2BaseEN => (768, "jinaai/jina-embeddings-v2-base-en"),
            EmbeddingModel::AllMiniLML6V2 => (384, "all-MiniLM-L6-v2"),
            EmbeddingModel::AllMiniLML12V2 => (384, "all-MiniLM-L12-v2"),
            EmbeddingModel::BGEBaseENV15 => (768, "BAAI/bge-base-en-v1.5"),
            EmbeddingModel::BGESmallENV15 => (384, "BAAI/bge-small-en-v1.5"),
            _ => (384, "unknown"),
        };

        let cache_dir = crate::paths::PlatformPaths::cache_dir().join("fastembed");
        tracing::info!("FastEmbed cache dir: {}", cache_dir.display());

        // Configure CoreML execution provider for Metal acceleration on Apple Silicon
        let execution_providers = vec![ort::ep::CoreML::default().build()];
        tracing::info!("Configured CoreML execution provider for Metal acceleration");

        let mut options = InitOptions::default();
        options.model_name = model;
        options.show_download_progress = true;
        options.cache_dir = cache_dir;
        options.execution_providers = execution_providers;

        let embedding_model =
            TextEmbedding::try_new(options).context("Failed to initialize FastEmbed model")?;

        Ok(Self {
            model: RwLock::new(embedding_model),
            dimension,
            model_name_str,
        })
    }
}

impl EmbeddingProvider for FastEmbedManager {
    fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        tracing::debug!("Generating embeddings for {} texts", texts.len());

        // Acquire write lock safely. If the lock is poisoned (due to a panic while holding
        // the lock), we recover by taking ownership of the inner value.
        let mut model = self.model.write().unwrap_or_else(|poisoned| {
            tracing::warn!("FastEmbed model lock was poisoned, recovering...");
            poisoned.into_inner()
        });

        // Generate embeddings using the mutable reference
        // Note: For timeout protection, wrap calls to this method in tokio::time::timeout
        // at the async call site (e.g., in mcp_server/indexing.rs)
        let embeddings = model
            .embed(texts, None)
            .context("Failed to generate embeddings")?;

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        self.model_name_str
    }
}

impl Default for FastEmbedManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize default FastEmbed model")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_generation() {
        let manager = FastEmbedManager::new().unwrap();
        let texts = vec![
            "fn main() { println!(\"Hello, world!\"); }".to_string(),
            "pub struct Vector { x: f32, y: f32 }".to_string(),
        ];

        let embeddings = manager.embed_batch(texts).unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 768);
        assert_eq!(embeddings[1].len(), 768);
    }

    #[test]
    fn test_empty_batch() {
        let manager = FastEmbedManager::new().unwrap();
        let embeddings = manager.embed_batch(vec![]).unwrap();
        assert_eq!(embeddings.len(), 0);
    }

    #[test]
    fn test_dimension() {
        let manager = FastEmbedManager::new().unwrap();
        assert_eq!(manager.dimension(), 768);
    }

    #[test]
    fn test_model_name() {
        let manager = FastEmbedManager::new().unwrap();
        assert_eq!(manager.model_name(), "jinaai/jina-embeddings-v2-base-en");
    }

    #[test]
    fn test_default() {
        let manager = FastEmbedManager::default();
        assert_eq!(manager.dimension(), 768);
        assert_eq!(manager.model_name(), "jinaai/jina-embeddings-v2-base-en");
    }

    #[test]
    fn test_single_text() {
        let manager = FastEmbedManager::new().unwrap();
        let texts = vec!["Hello world".to_string()];
        let embeddings = manager.embed_batch(texts).unwrap();
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].len(), 768);
    }

    #[test]
    fn test_large_batch() {
        let manager = FastEmbedManager::new().unwrap();
        let texts: Vec<String> = (0..10).map(|i| format!("Test text {}", i)).collect();
        let embeddings = manager.embed_batch(texts).unwrap();
        assert_eq!(embeddings.len(), 10);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 768);
        }
    }

    #[test]
    fn test_with_model_allminilm_l12() {
        let manager = FastEmbedManager::with_model(EmbeddingModel::AllMiniLML12V2).unwrap();
        assert_eq!(manager.dimension(), 384);
    }

    #[test]
    fn test_with_model_bge_base() {
        let manager = FastEmbedManager::with_model(EmbeddingModel::BGEBaseENV15).unwrap();
        assert_eq!(manager.dimension(), 768);
    }

    #[test]
    fn test_with_model_bge_small() {
        let manager = FastEmbedManager::with_model(EmbeddingModel::BGESmallENV15).unwrap();
        assert_eq!(manager.dimension(), 384);
    }
}
