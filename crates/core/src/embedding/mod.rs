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

fn sanitize_prompt_field(value: &str) -> String {
    value
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format a retrieval query for EmbeddingGemma.
pub fn format_retrieval_query(query: &str) -> String {
    format!(
        "task: search result | query: {}",
        sanitize_prompt_field(query)
    )
}

/// Format a retrieval document for EmbeddingGemma.
pub fn format_retrieval_document(title: Option<&str>, text: &str) -> String {
    let title = title
        .map(sanitize_prompt_field)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "none".to_string());

    format!("title: {} | text: {}", title, text.trim())
}

/// Format a retrieval document with chunk-level structure context.
pub fn format_retrieval_chunk(
    title: Option<&str>,
    heading_context: Option<&str>,
    element_types: Option<&[String]>,
    text: &str,
) -> String {
    let title = title
        .map(sanitize_prompt_field)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "none".to_string());
    let heading = heading_context
        .map(sanitize_prompt_field)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "none".to_string());
    let element_types = element_types
        .filter(|values| !values.is_empty())
        .map(|values| values.join(","))
        .unwrap_or_else(|| "none".to_string());

    format!(
        "title: {} | heading: {} | types: {} | text: {}",
        title,
        heading,
        element_types,
        text.trim()
    )
}
