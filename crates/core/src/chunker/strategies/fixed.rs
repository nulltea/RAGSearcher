use crate::chunker::{ChunkInput, CodeChunk};
use crate::types::ChunkMetadata;
use std::time::{SystemTime, UNIX_EPOCH};

/// Strategy for chunking text
pub enum ChunkStrategy {
    /// Fixed number of lines per chunk
    FixedLines(usize),
    /// Sliding window with overlap
    SlidingWindow { size: usize, overlap: usize },
}

pub struct FixedChunker {
    strategy: ChunkStrategy,
}

impl FixedChunker {
    pub fn new(strategy: ChunkStrategy) -> Self {
        Self { strategy }
    }

    /// Create a chunker with default strategy (50 lines per chunk)
    pub fn default_strategy() -> Self {
        Self::new(ChunkStrategy::FixedLines(50))
    }

    /// Chunk text into multiple chunks
    pub fn chunk_file(&self, input: &ChunkInput) -> Vec<CodeChunk> {
        match &self.strategy {
            ChunkStrategy::FixedLines(lines_per_chunk) => {
                self.chunk_fixed_lines(input, *lines_per_chunk)
            }
            ChunkStrategy::SlidingWindow { size, overlap } => {
                self.chunk_sliding_window(input, *size, *overlap)
            }
        }
    }

    fn chunk_fixed_lines(&self, input: &ChunkInput, lines_per_chunk: usize) -> Vec<CodeChunk> {
        let lines: Vec<&str> = input.content.lines().collect();
        let mut chunks = Vec::new();

        if lines.is_empty() {
            return chunks;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for (chunk_idx, chunk_lines) in lines.chunks(lines_per_chunk).enumerate() {
            let start_line = chunk_idx * lines_per_chunk + 1;
            let end_line = start_line + chunk_lines.len() - 1;
            let content = chunk_lines.join("\n");

            if content.trim().is_empty() {
                continue;
            }

            let metadata = ChunkMetadata {
                chunk_id: None,
                file_path: input.relative_path.clone(),
                root_path: Some(input.root_path.clone()),
                project: input.project.clone(),
                start_line,
                end_line,
                language: input.language.clone(),
                extension: input.extension.clone(),
                file_hash: input.hash.clone(),
                indexed_at: timestamp,
                page_numbers: None,
                heading_context: None,
                element_types: None,
            };

            chunks.push(CodeChunk { content, metadata });
        }

        chunks
    }

    fn chunk_sliding_window(
        &self,
        input: &ChunkInput,
        size: usize,
        overlap: usize,
    ) -> Vec<CodeChunk> {
        let lines: Vec<&str> = input.content.lines().collect();
        let mut chunks = Vec::new();

        if lines.is_empty() {
            return chunks;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let step = if overlap < size { size - overlap } else { 1 };
        let mut start_idx = 0;

        while start_idx < lines.len() {
            let end_idx = (start_idx + size).min(lines.len());
            let chunk_lines = &lines[start_idx..end_idx];
            let content = chunk_lines.join("\n");

            if content.trim().is_empty() {
                start_idx += step;
                continue;
            }

            let start_line = start_idx + 1;
            let end_line = end_idx;

            let metadata = ChunkMetadata {
                chunk_id: None,
                file_path: input.relative_path.clone(),
                root_path: Some(input.root_path.clone()),
                project: input.project.clone(),
                start_line,
                end_line,
                language: input.language.clone(),
                extension: input.extension.clone(),
                file_hash: input.hash.clone(),
                indexed_at: timestamp,
                page_numbers: None,
                heading_context: None,
                element_types: None,
            };

            chunks.push(CodeChunk { content, metadata });

            if end_idx >= lines.len() {
                break;
            }

            start_idx += step;
        }

        chunks
    }
}

impl Default for FixedChunker {
    fn default() -> Self {
        Self::default_strategy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_input(content: &str) -> ChunkInput {
        ChunkInput {
            relative_path: "test.md".to_string(),
            root_path: "papers".to_string(),
            project: None,
            extension: Some("md".to_string()),
            language: Some("Markdown".to_string()),
            content: content.to_string(),
            hash: "test_hash".to_string(),
        }
    }

    #[test]
    fn test_fixed_lines_chunking() {
        let content = (1..=100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let input = create_test_input(&content);

        let chunker = FixedChunker::new(ChunkStrategy::FixedLines(10));
        let chunks = chunker.chunk_file(&input);

        assert_eq!(chunks.len(), 10);
        assert_eq!(chunks[0].metadata.start_line, 1);
        assert_eq!(chunks[0].metadata.end_line, 10);
        assert_eq!(chunks[9].metadata.start_line, 91);
        assert_eq!(chunks[9].metadata.end_line, 100);
    }

    #[test]
    fn test_sliding_window_chunking() {
        let content = (1..=20)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let input = create_test_input(&content);

        let chunker = FixedChunker::new(ChunkStrategy::SlidingWindow {
            size: 10,
            overlap: 5,
        });
        let chunks = chunker.chunk_file(&input);
        assert!(chunks.len() >= 3);
        assert_eq!(chunks[0].metadata.start_line, 1);
    }

    #[test]
    fn test_default_strategy() {
        let chunker = FixedChunker::default_strategy();
        assert!(matches!(chunker.strategy, ChunkStrategy::FixedLines(50)));
    }

    #[test]
    fn test_empty_content() {
        let input = create_test_input("");
        let chunker = FixedChunker::new(ChunkStrategy::FixedLines(10));
        let chunks = chunker.chunk_file(&input);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_whitespace_only() {
        let input = create_test_input("   \n\t\n   ");
        let chunker = FixedChunker::new(ChunkStrategy::FixedLines(10));
        let chunks = chunker.chunk_file(&input);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_single_line() {
        let input = create_test_input("Hello world");
        let chunker = FixedChunker::new(ChunkStrategy::FixedLines(10));
        let chunks = chunker.chunk_file(&input);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].metadata.start_line, 1);
        assert_eq!(chunks[0].metadata.end_line, 1);
    }

    #[test]
    fn test_metadata_fields() {
        let mut input = create_test_input("Hello world");
        input.project = Some("test-project".to_string());
        input.hash = "abc123".to_string();

        let chunker = FixedChunker::new(ChunkStrategy::FixedLines(10));
        let chunks = chunker.chunk_file(&input);

        assert_eq!(chunks.len(), 1);
        let chunk = &chunks[0];
        assert_eq!(chunk.metadata.file_path, "test.md");
        assert_eq!(chunk.metadata.project, Some("test-project".to_string()));
        assert_eq!(chunk.metadata.file_hash, "abc123");
        assert!(chunk.metadata.indexed_at > 0);
    }
}
