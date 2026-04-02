use crate::chunker::CodeChunk;
use crate::types::ChunkMetadata;
use anyhow::{Context, Result};
use oxidize_pdf::parser::PdfDocument;
use oxidize_pdf::pipeline::{HybridChunkConfig, MergePolicy};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Metadata needed to construct CodeChunks from PDF parsing results
pub struct PdfChunkMeta {
    pub relative_path: String,
    pub root_path: String,
    pub project: Option<String>,
    pub hash: String,
}

/// Context-aware PDF chunker using oxidize-pdf's structure-aware parsing.
///
/// Extracts document structure (headings, paragraphs, tables) directly from PDF
/// and produces chunks that respect semantic boundaries. Each chunk carries heading
/// context for better embedding quality.
pub struct ContextAwareChunker {
    config: HybridChunkConfig,
}

impl ContextAwareChunker {
    /// Create with defaults tuned for jina-embeddings-v2-base-en (8192 token limit).
    ///
    /// - `max_tokens: 512` — good balance for retrieval granularity with long-context model
    /// - `overlap_tokens: 50` — cross-chunk retrieval continuity
    /// - `propagate_headings: true` — each chunk gets its section heading
    /// - `merge_policy: AnyInlineContent` — paragraphs+lists merge within sections
    pub fn new() -> Self {
        Self {
            config: HybridChunkConfig {
                max_tokens: 512,
                overlap_tokens: 50,
                merge_adjacent: true,
                propagate_headings: true,
                merge_policy: MergePolicy::AnyInlineContent,
            },
        }
    }

    pub fn with_config(config: HybridChunkConfig) -> Self {
        Self { config }
    }

    /// Chunk a PDF file directly using structure-aware parsing.
    pub fn chunk_pdf(&self, path: &Path, meta: &PdfChunkMeta) -> Result<Vec<CodeChunk>> {
        let doc = PdfDocument::open(path)
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Failed to open PDF for context-aware chunking")?;

        let rag_chunks = doc
            .rag_chunks_with(self.config.clone())
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Failed to extract RAG chunks from PDF")?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let chunks = rag_chunks
            .into_iter()
            .filter(|rc| !rc.full_text.trim().is_empty())
            .map(|rc| CodeChunk {
                content: rc.full_text,
                metadata: ChunkMetadata {
                    file_path: meta.relative_path.clone(),
                    root_path: Some(meta.root_path.clone()),
                    project: meta.project.clone(),
                    start_line: 0,
                    end_line: 0,
                    language: Some("PDF".to_string()),
                    extension: Some("pdf".to_string()),
                    file_hash: meta.hash.clone(),
                    indexed_at: timestamp,
                    page_numbers: Some(rc.page_numbers),
                    heading_context: rc.heading_context,
                    element_types: Some(rc.element_types),
                },
            })
            .collect();

        Ok(chunks)
    }
}

impl Default for ContextAwareChunker {
    fn default() -> Self {
        Self::new()
    }
}
