use super::EmbeddingProvider;
use anyhow::{Context, Result};
use mistralrs::{EmbeddingModelBuilder, EmbeddingRequest, Model};

/// MistralRS-based embedding provider using EmbeddingGemma-300M with Metal acceleration.
pub struct MistralRsEmbedder {
    model: Model,
    dimension: usize,
}

impl MistralRsEmbedder {
    /// Create a new embedder with google/embeddinggemma-300m (768 dimensions, Metal-accelerated).
    pub async fn new() -> Result<Self> {
        tracing::info!("Initializing MistralRS EmbeddingGemma-300M model");

        let model = EmbeddingModelBuilder::new("google/embeddinggemma-300m")
            .with_logging()
            .build()
            .await
            .context("Failed to initialize MistralRS embedding model")?;

        tracing::info!("MistralRS embedding model ready (768 dims, Metal)");

        Ok(Self {
            model,
            dimension: 768,
        })
    }
}

impl EmbeddingProvider for MistralRsEmbedder {
    fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let total = texts.len();
        tracing::info!(total_texts = total, "Generating embeddings");

        // Use tokio handle to bridge sync trait → async mistralrs API.
        // Callers wrap this in spawn_blocking, so Handle::current() is available.
        let handle = tokio::runtime::Handle::current();
        let embeddings = handle.block_on(async {
            let request = EmbeddingRequest::builder().add_prompts(texts);
            self.model
                .generate_embeddings(request)
                .await
                .map_err(|e| anyhow::anyhow!("Embedding generation failed: {}", e))
        })?;

        tracing::info!(total_texts = total, "Embedding complete");
        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        "google/embeddinggemma-300m"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedding_generation() {
        let manager = std::sync::Arc::new(MistralRsEmbedder::new().await.unwrap());
        let m = manager.clone();
        let embeddings = tokio::task::spawn_blocking(move || {
            let texts = vec![
                "fn main() { println!(\"Hello, world!\"); }".to_string(),
                "pub struct Vector { x: f32, y: f32 }".to_string(),
            ];
            m.embed_batch(texts)
        })
        .await
        .unwrap()
        .unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 768);
        assert_eq!(embeddings[1].len(), 768);
    }

    #[tokio::test]
    async fn test_empty_batch() {
        let manager = MistralRsEmbedder::new().await.unwrap();
        let embeddings = manager.embed_batch(vec![]).unwrap();
        assert_eq!(embeddings.len(), 0);
    }

    #[tokio::test]
    async fn test_dimension() {
        let manager = MistralRsEmbedder::new().await.unwrap();
        assert_eq!(manager.dimension(), 768);
    }

    #[tokio::test]
    async fn test_model_name() {
        let manager = MistralRsEmbedder::new().await.unwrap();
        assert_eq!(manager.model_name(), "google/embeddinggemma-300m");
    }

    #[tokio::test]
    async fn test_single_text() {
        let manager = std::sync::Arc::new(MistralRsEmbedder::new().await.unwrap());
        let m = manager.clone();
        let embeddings = tokio::task::spawn_blocking(move || {
            m.embed_batch(vec!["Hello world".to_string()])
        })
        .await
        .unwrap()
        .unwrap();
        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].len(), 768);
    }

    #[tokio::test]
    async fn test_large_batch() {
        let manager = std::sync::Arc::new(MistralRsEmbedder::new().await.unwrap());
        let m = manager.clone();
        let embeddings = tokio::task::spawn_blocking(move || {
            let texts: Vec<String> = (0..10).map(|i| format!("Test text {}", i)).collect();
            m.embed_batch(texts)
        })
        .await
        .unwrap()
        .unwrap();
        assert_eq!(embeddings.len(), 10);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 768);
        }
    }
}
