use crate::chunker::{ChunkInput, PdfChunkMeta};
use crate::document::{NodeKind, NormalizedDocument, SourceKind, StructuralNode};
use regex::Regex;
use std::sync::OnceLock;

pub fn normalize_chunk_input(input: &ChunkInput) -> NormalizedDocument {
    let source_kind = source_kind_from_extension(input.extension.as_deref(), input.language.as_deref());
    normalize_text(&input.content, source_kind)
}

pub fn normalize_pdf_markdown(text: &str, _meta: &PdfChunkMeta) -> NormalizedDocument {
    normalize_text(text, SourceKind::Paper)
}

fn source_kind_from_extension(extension: Option<&str>, language: Option<&str>) -> SourceKind {
    let ext = extension.unwrap_or_default().to_ascii_lowercase();
    let lang = language.unwrap_or_default().to_ascii_lowercase();

    if matches!(
        ext.as_str(),
        "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "java" | "go" | "cpp" | "c" | "h"
    ) || lang.contains("rust")
        || lang.contains("python")
        || lang.contains("javascript")
        || lang.contains("typescript")
    {
        SourceKind::Code
    } else if matches!(ext.as_str(), "md" | "markdown") {
        SourceKind::Markdown
    } else {
        SourceKind::PlainText
    }
}

fn normalize_text(text: &str, source_kind: SourceKind) -> NormalizedDocument {
    match source_kind {
        SourceKind::Code => normalize_code(text),
        other => normalize_prose(text, other),
    }
}

fn normalize_code(text: &str) -> NormalizedDocument {
    let mut nodes = Vec::new();
    let mut block = Vec::new();
    let mut block_start = 1;

    for (index, raw_line) in text.lines().enumerate() {
        let line_no = index + 1;
        if raw_line.trim().is_empty() {
            flush_block(
                &mut nodes,
                NodeKind::Code,
                &mut block,
                &[],
                None,
                block_start,
                line_no.saturating_sub(1),
            );
            block_start = line_no + 1;
            continue;
        }

        if block.is_empty() {
            block_start = line_no;
        }
        block.push(raw_line.to_string());
    }

    flush_block(
        &mut nodes,
        NodeKind::Code,
        &mut block,
        &[],
        None,
        block_start,
        text.lines().count(),
    );

    NormalizedDocument {
        source_kind: SourceKind::Code,
        title: None,
        nodes,
    }
}

fn normalize_prose(text: &str, source_kind: SourceKind) -> NormalizedDocument {
    let mut nodes = Vec::new();
    let mut headings = Vec::new();
    let mut pending_caption: Option<String> = None;
    let mut block = Vec::new();
    let mut block_kind = NodeKind::Paragraph;
    let mut block_start = 1;
    let mut fenced_code = false;
    let mut title = None;

    for (index, raw_line) in text.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = raw_line.trim();

        if fenced_code {
            if block.is_empty() {
                block_start = line_no;
            }
            block.push(raw_line.to_string());
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                flush_block(
                    &mut nodes,
                    NodeKind::Code,
                    &mut block,
                    &headings,
                    None,
                    block_start,
                    line_no,
                );
                fenced_code = false;
            }
            continue;
        }

        if trimmed.is_empty() {
            flush_block(
                &mut nodes,
                block_kind.clone(),
                &mut block,
                &headings,
                pending_caption.take(),
                block_start,
                line_no.saturating_sub(1),
            );
            block_kind = NodeKind::Paragraph;
            block_start = line_no + 1;
            continue;
        }

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush_block(
                &mut nodes,
                block_kind.clone(),
                &mut block,
                &headings,
                pending_caption.take(),
                block_start,
                line_no.saturating_sub(1),
            );
            block_kind = NodeKind::Code;
            block_start = line_no;
            block.push(raw_line.to_string());
            fenced_code = true;
            continue;
        }

        if let Some((level, heading)) = parse_heading(trimmed) {
            flush_block(
                &mut nodes,
                block_kind.clone(),
                &mut block,
                &headings,
                pending_caption.take(),
                block_start,
                line_no.saturating_sub(1),
            );
            if title.is_none() {
                title = Some(heading.clone());
            }
            apply_heading(&mut headings, level, &heading);
            block_kind = NodeKind::Paragraph;
            block_start = line_no + 1;
            continue;
        }

        if is_caption(trimmed) {
            flush_block(
                &mut nodes,
                block_kind.clone(),
                &mut block,
                &headings,
                pending_caption.take(),
                block_start,
                line_no.saturating_sub(1),
            );
            pending_caption = Some(trimmed.to_string());
            block_kind = NodeKind::Paragraph;
            block_start = line_no + 1;
            continue;
        }

        let next_kind = if is_table_row(trimmed) {
            NodeKind::Table
        } else if is_list_item(trimmed) {
            NodeKind::List
        } else {
            NodeKind::Paragraph
        };

        if block.is_empty() {
            block_start = line_no;
            block_kind = next_kind.clone();
        } else if block_kind != next_kind {
            flush_block(
                &mut nodes,
                block_kind.clone(),
                &mut block,
                &headings,
                pending_caption.take(),
                block_start,
                line_no.saturating_sub(1),
            );
            block_start = line_no;
            block_kind = next_kind.clone();
        }

        block.push(raw_line.to_string());
    }

    flush_block(
        &mut nodes,
        block_kind,
        &mut block,
        &headings,
        pending_caption,
        block_start,
        text.lines().count(),
    );

    NormalizedDocument {
        source_kind,
        title,
        nodes,
    }
}

fn flush_block(
    nodes: &mut Vec<StructuralNode>,
    kind: NodeKind,
    block: &mut Vec<String>,
    headings: &[String],
    caption: Option<String>,
    start_line: usize,
    end_line: usize,
) {
    if block.is_empty() {
        return;
    }

    let text = match kind {
        NodeKind::Paragraph => join_paragraph_lines(block),
        _ => block.join("\n"),
    };
    block.clear();

    if text.trim().is_empty() {
        return;
    }

    nodes.push(StructuralNode {
        kind,
        text,
        heading_path: headings.to_vec(),
        caption,
        start_line,
        end_line: end_line.max(start_line),
        page_numbers: None,
    });
}

fn join_paragraph_lines(block: &[String]) -> String {
    let mut joined = String::new();
    for (index, line) in block.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if index > 0 && !joined.ends_with('-') {
            joined.push(' ');
        }
        if joined.ends_with('-') && trimmed.chars().next().is_some_and(|ch| ch.is_lowercase()) {
            joined.pop();
        } else if index > 0 && joined.ends_with('-') {
            joined.push(' ');
        }
        joined.push_str(trimmed);
    }
    joined
}

fn parse_heading(line: &str) -> Option<(usize, String)> {
    if line.starts_with('#') {
        let level = line.chars().take_while(|ch| *ch == '#').count().max(1);
        let heading = line[level..].trim();
        return (!heading.is_empty()).then(|| (level, heading.to_string()));
    }

    let numbered = numbered_heading_regex();
    if numbered.is_match(line) {
        return Some((
            parse_heading_level(line),
            line.trim_end_matches(':').to_string(),
        ));
    }

    let alpha_count = line.chars().filter(|ch| ch.is_alphabetic()).count();
    let upper_count = line.chars().filter(|ch| ch.is_uppercase()).count();
    if alpha_count >= 4 && upper_count * 5 >= alpha_count * 4 && line.len() <= 96 {
        return Some((1, line.trim_end_matches(':').to_string()));
    }

    None
}

fn apply_heading(headings: &mut Vec<String>, level: usize, heading: &str) {
    if headings.len() >= level {
        headings.truncate(level.saturating_sub(1));
    }
    headings.push(heading.to_string());
}

fn parse_heading_level(heading: &str) -> usize {
    let prefix = heading
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_end_matches('.');
    let level = prefix.split('.').filter(|part| !part.is_empty()).count();
    level.max(1)
}

fn is_caption(line: &str) -> bool {
    caption_regex().is_match(line)
}

fn is_table_row(line: &str) -> bool {
    let pipe_columns = line.matches('|').count() >= 2;
    let spaced_columns = line.contains("  ") && line.split_whitespace().count() >= 2;
    pipe_columns || spaced_columns
}

fn is_list_item(line: &str) -> bool {
    line.starts_with("- ")
        || line.starts_with("* ")
        || line.starts_with("+ ")
        || line
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit() && line.contains(". "))
}

fn caption_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^(Table|Figure|Fig\.)\s+\d+([:.].*)?$").unwrap())
}

fn numbered_heading_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^\d+(\.\d+)*\.?\s+\S").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk_input(content: &str) -> ChunkInput {
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
    fn extracts_heading_hierarchy_and_caption() {
        let doc = normalize_chunk_input(&chunk_input(
            "# Title\n\n## Methods\n\nTable 1: Results\nA  B  C\n1  2  3\n",
        ));

        assert_eq!(doc.title.as_deref(), Some("Title"));
        assert_eq!(doc.nodes.len(), 1);
        assert_eq!(doc.nodes[0].heading_path, vec!["Title".to_string(), "Methods".to_string()]);
        assert_eq!(doc.nodes[0].caption.as_deref(), Some("Table 1: Results"));
    }
}
