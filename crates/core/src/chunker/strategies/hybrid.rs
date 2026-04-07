use crate::chunker::{ChunkInput, CodeChunk};
use crate::document::{NodeKind, NormalizedDocument, SourceKind, StructuralNode, normalize_chunk_input, normalize_pdf_markdown};
use crate::tokenization::count_embedding_tokens;
use crate::types::ChunkMetadata;
use anyhow::{Context, Result};
use std::time::{SystemTime, UNIX_EPOCH};
use text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter};

use super::context_aware::PdfChunkMeta;

#[derive(Debug, Clone)]
struct ChunkSeed {
    text: String,
    heading_context: Option<String>,
    element_types: Vec<String>,
    page_numbers: Option<Vec<u32>>,
    start_line: usize,
    end_line: usize,
    token_count: usize,
}

/// Docling-inspired chunker that keeps structure first and uses tokenizer-aware splitting
/// for oversized seeds.
pub struct HybridChunker {
    min_tokens: usize,
    target_tokens: usize,
    max_tokens: usize,
    overlap_tokens: usize,
}

impl HybridChunker {
    pub fn new() -> Self {
        Self {
            min_tokens: 120,
            target_tokens: 384,
            max_tokens: 448,
            overlap_tokens: 48,
        }
    }

    pub fn chunk_file(&self, input: &ChunkInput) -> Vec<CodeChunk> {
        let doc = normalize_chunk_input(input);
        self.chunk_document(
            &doc,
            &input.relative_path,
            &input.root_path,
            input.project.clone(),
            input.language.clone(),
            input.extension.clone(),
            &input.hash,
        )
        .unwrap_or_default()
    }

    pub fn chunk_text(&self, text: &str, meta: &PdfChunkMeta) -> Result<Vec<CodeChunk>> {
        let doc = normalize_pdf_markdown(text, meta);
        self.chunk_document(
            &doc,
            &meta.relative_path,
            &meta.root_path,
            meta.project.clone(),
            Some("PDF".to_string()),
            Some("pdf".to_string()),
            &meta.hash,
        )
    }

    fn chunk_document(
        &self,
        doc: &NormalizedDocument,
        relative_path: &str,
        root_path: &str,
        project: Option<String>,
        language: Option<String>,
        extension: Option<String>,
        hash: &str,
    ) -> Result<Vec<CodeChunk>> {
        if doc.nodes.is_empty() {
            return Ok(Vec::new());
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let seeds = self.build_chunk_seeds(&doc.nodes)?;
        let merged = self.merge_small_seeds(seeds)?;

        let mut chunks = Vec::new();
        for seed in merged {
            let mut element_types = seed.element_types;
            element_types.push(source_kind_label(&doc.source_kind).to_string());
            element_types.sort();
            element_types.dedup();

            chunks.push(CodeChunk {
                content: seed.text,
                metadata: ChunkMetadata {
                    chunk_id: None,
                    file_path: relative_path.to_string(),
                    root_path: Some(root_path.to_string()),
                    project: project.clone(),
                    start_line: seed.start_line,
                    end_line: seed.end_line,
                    language: language.clone(),
                    extension: extension.clone(),
                    file_hash: hash.to_string(),
                    indexed_at: timestamp,
                    page_numbers: seed.page_numbers,
                    heading_context: seed.heading_context,
                    element_types: Some(element_types),
                },
            });
        }

        Ok(chunks)
    }

    fn build_chunk_seeds(&self, nodes: &[StructuralNode]) -> Result<Vec<ChunkSeed>> {
        let mut seeds = Vec::new();
        let mut pending: Vec<&StructuralNode> = Vec::new();

        for node in nodes {
            let node_text = serialize_node(node);
            let node_tokens = count_embedding_tokens(&node_text)?;

            if node_tokens > self.max_tokens {
                if !pending.is_empty() {
                    seeds.push(self.seed_from_nodes(&pending)?);
                    pending.clear();
                }

                let heading_context = heading_context(&node.heading_path);
                for split in self.split_oversized_node(node, &node_text)? {
                    let token_count = count_embedding_tokens(&split)?;
                    seeds.push(ChunkSeed {
                        text: split,
                        heading_context: heading_context.clone(),
                        element_types: vec![node.kind.as_str().to_string()],
                        page_numbers: node.page_numbers.clone(),
                        start_line: node.start_line,
                        end_line: node.end_line,
                        token_count,
                    });
                }
                continue;
            }

            pending.push(node);
            let current = self.seed_from_nodes(&pending)?;
            if current.token_count >= self.target_tokens {
                seeds.push(current);
                pending.clear();
            }
        }

        if !pending.is_empty() {
            seeds.push(self.seed_from_nodes(&pending)?);
        }

        Ok(seeds)
    }

    fn split_oversized_node(&self, node: &StructuralNode, text: &str) -> Result<Vec<String>> {
        let config = ChunkConfig::new(self.min_tokens..self.max_tokens)
            .with_sizer(crate::tokenization::embedding_tokenizer()?.clone())
            .with_overlap(self.overlap_tokens)
            .context("invalid chunk config")?
            .with_trim(true);

        let splits: Vec<String> = match node.kind {
            NodeKind::Paragraph | NodeKind::List => MarkdownSplitter::new(config)
                .chunks(text)
                .map(ToString::to_string)
                .collect(),
            NodeKind::Table | NodeKind::Code => TextSplitter::new(config)
                .chunks(text)
                .map(ToString::to_string)
                .collect(),
        };

        Ok(splits.into_iter().filter(|chunk| !chunk.trim().is_empty()).collect())
    }

    fn seed_from_nodes(&self, nodes: &[&StructuralNode]) -> Result<ChunkSeed> {
        let mut text_parts = Vec::new();
        let mut element_types = Vec::new();
        let mut page_numbers = Vec::new();

        for node in nodes {
            text_parts.push(serialize_node(node));
            element_types.push(node.kind.as_str().to_string());
            if let Some(pages) = &node.page_numbers {
                page_numbers.extend(pages.iter().copied());
            }
        }

        let text = text_parts.join("\n\n");
        let token_count = count_embedding_tokens(&text)?;
        page_numbers.sort_unstable();
        page_numbers.dedup();

        Ok(ChunkSeed {
            text,
            heading_context: heading_context(&nodes[0].heading_path),
            element_types,
            page_numbers: (!page_numbers.is_empty()).then_some(page_numbers),
            start_line: nodes[0].start_line,
            end_line: nodes[nodes.len() - 1].end_line,
            token_count,
        })
    }

    fn merge_small_seeds(&self, seeds: Vec<ChunkSeed>) -> Result<Vec<ChunkSeed>> {
        let mut merged = Vec::new();
        let mut index = 0;

        while index < seeds.len() {
            let mut current = seeds[index].clone();

            while current.token_count < self.min_tokens && index + 1 < seeds.len() {
                let next = &seeds[index + 1];
                if current.heading_context != next.heading_context {
                    break;
                }

                let combined = format!("{}\n\n{}", current.text, next.text);
                let combined_tokens = count_embedding_tokens(&combined)?;
                if combined_tokens > self.max_tokens {
                    break;
                }

                current.text = combined;
                current.end_line = next.end_line;
                current.token_count = combined_tokens;
                current.element_types.extend(next.element_types.iter().cloned());
                current.element_types.sort();
                current.element_types.dedup();
                if current.page_numbers.is_none() {
                    current.page_numbers = next.page_numbers.clone();
                }
                index += 1;
            }

            merged.push(current);
            index += 1;
        }

        Ok(merged)
    }
}

impl Default for HybridChunker {
    fn default() -> Self {
        Self::new()
    }
}

fn heading_context(path: &[String]) -> Option<String> {
    (!path.is_empty()).then(|| path.join(" > "))
}

fn serialize_node(node: &StructuralNode) -> String {
    match &node.caption {
        Some(caption) => format!("{caption}\n{}", node.text.trim()),
        None => node.text.trim().to_string(),
    }
}

fn source_kind_label(kind: &SourceKind) -> &'static str {
    match kind {
        SourceKind::Paper => "paper",
        SourceKind::Book => "book",
        SourceKind::Code => "code",
        SourceKind::Markdown => "markdown",
        SourceKind::PlainText => "plain_text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(content: &str) -> ChunkInput {
        ChunkInput {
            relative_path: "papers/test".to_string(),
            root_path: "papers".to_string(),
            project: Some("paper-1".to_string()),
            extension: Some("md".to_string()),
            language: Some("Markdown".to_string()),
            content: content.to_string(),
            hash: "hash".to_string(),
        }
    }

    #[test]
    fn preserves_heading_context_and_table_type() {
        let chunker = HybridChunker::new();
        let chunks = chunker.chunk_file(&input(
            "# Title\n\n## Results\n\nTable 1: Quality\nMetric  Score\nMRR     0.91\n",
        ));

        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0].metadata.heading_context.as_deref(),
            Some("Title > Results")
        );
        assert!(
            chunks[0]
                .metadata
                .element_types
                .as_ref()
                .unwrap()
                .iter()
                .any(|kind| kind == "table")
        );
    }
}
