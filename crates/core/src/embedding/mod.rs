mod mistralrs_manager;

pub use mistralrs_manager::MistralRsEmbedder;

use anyhow::Result;

/// Trait for embedding generation
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for a batch of text
    fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    /// Get the dimension of the embeddings
    fn dimension(&self) -> usize;

    /// Get the model name
    fn model_name(&self) -> &str;
}
