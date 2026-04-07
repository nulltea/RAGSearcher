use super::EmbeddingProvider;
use anyhow::{Context, Result};
use mistralrs::{EmbeddingModelBuilder, EmbeddingRequest, Model};

/// Token-length bucket boundaries for batching chunks with similar lengths.
/// Reduces padding waste by grouping similarly-sized inputs together.
const TOKEN_BUCKETS: [usize; 4] = [128, 256, 384, 512];

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

impl MistralRsEmbedder {
    /// Assign a text to the smallest bucket that fits its approximate token count.
    /// Uses char_count / 4 as a rough token estimate (avoids tokenizer overhead).
    fn bucket_index(text: &str) -> usize {
        let approx_tokens = text.len() / 4;
        TOKEN_BUCKETS
            .iter()
            .position(|&b| approx_tokens <= b)
            .unwrap_or(TOKEN_BUCKETS.len() - 1)
    }
}

impl EmbeddingProvider for MistralRsEmbedder {
    fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let total = texts.len();
        tracing::info!(total_texts = total, "Generating embeddings");

        // Sort texts into token-length buckets to reduce padding waste.
        // Track original indices so we can reassemble results in order.
        let mut indexed: Vec<(usize, &String)> = texts.iter().enumerate().collect();
        indexed.sort_by_key(|(_, t)| Self::bucket_index(t));

        let handle = tokio::runtime::Handle::current();
        let mut all_embeddings = vec![Vec::new(); total];

        // Process each bucket as a separate request
        let mut bucket_start = 0;
        while bucket_start < indexed.len() {
            let current_bucket = Self::bucket_index(indexed[bucket_start].1);
            let bucket_end = indexed[bucket_start..]
                .iter()
                .position(|(_, t)| Self::bucket_index(t) != current_bucket)
                .map(|p| bucket_start + p)
                .unwrap_or(indexed.len());

            let bucket_texts: Vec<String> = indexed[bucket_start..bucket_end]
                .iter()
                .map(|(_, t)| (*t).clone())
                .collect();
            let bucket_indices: Vec<usize> = indexed[bucket_start..bucket_end]
                .iter()
                .map(|(i, _)| *i)
                .collect();

            tracing::info!(
                bucket = current_bucket,
                bucket_size = bucket_texts.len(),
                max_tokens = TOKEN_BUCKETS[current_bucket],
                "Embedding bucket"
            );

            let embeddings = handle.block_on(async {
                let request = EmbeddingRequest::builder().add_prompts(bucket_texts);
                self.model
                    .generate_embeddings(request)
                    .await
                    .map_err(|e| anyhow::anyhow!("Embedding generation failed: {}", e))
            })?;

            for (emb, &orig_idx) in embeddings.into_iter().zip(&bucket_indices) {
                all_embeddings[orig_idx] = emb;
            }

            bucket_start = bucket_end;
        }

        tracing::info!(total_texts = total, "Embedding complete");
        Ok(all_embeddings)
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
        let embeddings =
            tokio::task::spawn_blocking(move || m.embed_batch(vec!["Hello world".to_string()]))
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
