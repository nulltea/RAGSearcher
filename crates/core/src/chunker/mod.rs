//! Paper text chunking and PDF extraction
//!
//! Provides functionality to chunk paper text into units for embedding
//! and extract text from PDF files.

mod pdf_extractor;
mod strategies;

pub use pdf_extractor::{PdfExtraction, extract_pdf, extract_pdf_to_markdown};
pub use strategies::{ChunkStrategy, ContextAwareChunker, FixedChunker, HybridChunker, PdfChunkMeta};

use crate::types::ChunkMetadata;

/// Input for text chunking (papers, documents)
pub struct ChunkInput {
    pub relative_path: String,
    pub root_path: String,
    pub project: Option<String>,
    pub extension: Option<String>,
    pub language: Option<String>,
    pub content: String,
    pub hash: String,
}

/// Represents a text chunk ready for embedding
#[derive(Debug, Clone)]
pub struct CodeChunk {
    /// The actual text content of this chunk
    pub content: String,
    /// Metadata about this chunk (file path, line numbers, etc.)
    pub metadata: ChunkMetadata,
}
