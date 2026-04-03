use crate::chunker::CodeChunk;
use crate::types::ChunkMetadata;
use anyhow::{Context, Result};
use std::time::{SystemTime, UNIX_EPOCH};
use text_splitter::{ChunkConfig, MarkdownSplitter};
use tokenizers::Tokenizer;

/// Metadata needed to construct CodeChunks from PDF text
pub struct PdfChunkMeta {
    pub relative_path: String,
    pub root_path: String,
    pub project: Option<String>,
    pub hash: String,
}

/// Content-aware PDF chunker using MarkdownSplitter with Hugging Face tokenizer.
///
/// Splits extracted PDF markdown at heading/section boundaries while respecting
/// token limits of the embedding model. Uses the actual model tokenizer for
/// accurate token counting.
pub struct ContextAwareChunker {
    min_tokens: usize,
    max_tokens: usize,
    overlap_tokens: usize,
}

impl ContextAwareChunker {
    /// Create with defaults tuned for EmbeddingGemma-300M (2048 context).
    pub fn new() -> Self {
        Self {
            min_tokens: 256,
            max_tokens: 448,
            overlap_tokens: 48,
        }
    }

    /// Chunk pre-extracted PDF markdown using heading-aware splitting.
    #[tracing::instrument(skip(self, text, meta), fields(text_len = text.len()))]
    pub fn chunk_text(&self, text: &str, meta: &PdfChunkMeta) -> Result<Vec<CodeChunk>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(text_chars = text.len(), "Loading tokenizer for chunking");
        let tokenizer = Tokenizer::from_pretrained("google/embeddinggemma-300m", None)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        let config = ChunkConfig::new(self.min_tokens..self.max_tokens)
            .with_sizer(tokenizer)
            .with_overlap(self.overlap_tokens)
            .context("Invalid chunk config (overlap >= capacity)")?
            .with_trim(true);

        let splitter = MarkdownSplitter::new(config);
        tracing::info!(
            min_tokens = self.min_tokens,
            max_tokens = self.max_tokens,
            overlap = self.overlap_tokens,
            "Splitting markdown"
        );
        let text_chunks: Vec<&str> = splitter.chunks(text).collect();
        tracing::info!(
            chunk_count = text_chunks.len(),
            "Markdown splitting complete"
        );

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let chunks = text_chunks
            .into_iter()
            .filter(|c| c.trim().len() >= 50)
            .map(|c| CodeChunk {
                content: c.to_string(),
                metadata: ChunkMetadata {
                    chunk_id: None,
                    file_path: meta.relative_path.clone(),
                    root_path: Some(meta.root_path.clone()),
                    project: meta.project.clone(),
                    start_line: 0,
                    end_line: 0,
                    language: Some("PDF".to_string()),
                    extension: Some("pdf".to_string()),
                    file_hash: meta.hash.clone(),
                    indexed_at: timestamp,
                    page_numbers: None,
                    heading_context: Self::extract_heading_context(c),
                    element_types: None,
                },
            })
            .collect();

        Ok(chunks)
    }

    fn extract_heading_context(chunk: &str) -> Option<String> {
        chunk.lines().find_map(|line| {
            let trimmed = line.trim();
            let heading = trimmed.strip_prefix('#')?.trim_start_matches('#').trim();
            if heading.is_empty() {
                None
            } else {
                Some(heading.to_string())
            }
        })
    }
}

impl Default for ContextAwareChunker {
    fn default() -> Self {
        Self::new()
    }
}
