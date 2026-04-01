//! Paper text chunking and PDF extraction
//!
//! Provides functionality to chunk paper text into units for embedding
//! and extract text from PDF files.

mod chunker;
mod pdf_extractor;

pub use chunker::{ChunkInput, ChunkStrategy, CodeChunker};
pub use pdf_extractor::{PdfExtraction, extract_pdf, extract_pdf_to_markdown};

use crate::types::ChunkMetadata;

/// Represents a text chunk ready for embedding
#[derive(Debug, Clone)]
pub struct CodeChunk {
    /// The actual text content of this chunk
    pub content: String,
    /// Metadata about this chunk (file path, line numbers, etc.)
    pub metadata: ChunkMetadata,
}
