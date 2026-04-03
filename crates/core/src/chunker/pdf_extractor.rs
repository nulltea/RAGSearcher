use anyhow::{Context, Result};
use std::path::Path;

/// Result of PDF extraction: markdown text and optional extracted title
pub struct PdfExtraction {
    pub text: String,
    pub title: Option<String>,
}

/// Extract text from a PDF file and convert to Markdown format.
/// Also attempts to extract the paper title from PDF metadata or content heuristic.
pub fn extract_pdf_to_markdown(path: &Path) -> Result<String> {
    let extraction = extract_pdf(path)?;
    Ok(extraction.text)
}

/// Extract text and title from a PDF file.
pub fn extract_pdf(path: &Path) -> Result<PdfExtraction> {
    let text = pdf_extract::extract_text(path).context("Failed to extract text from PDF")?;
    let markdown = format_as_markdown(&text);
    let title = extract_pdf_title(path, &text);
    Ok(PdfExtraction {
        text: markdown,
        title,
    })
}

/// Try to extract the paper title without AI.
/// Priority: PDF metadata /Title field → first-line heuristic from text.
fn extract_pdf_title(path: &Path, text: &str) -> Option<String> {
    // Try PDF metadata first
    if let Some(title) = extract_title_from_metadata(path) {
        return Some(title);
    }

    // Fall back to text heuristic
    extract_title_from_text(text)
}

/// Read the /Title field from the PDF's document info dictionary.
fn extract_title_from_metadata(path: &Path) -> Option<String> {
    let doc = lopdf::Document::load(path).ok()?;

    // Try the Info dictionary in the trailer
    let info_ref = doc.trailer.get(b"Info").ok()?;
    let info_ref = info_ref.as_reference().ok()?;
    let info = doc.get_dictionary(info_ref).ok()?;

    let title_obj = info.get(b"Title").ok()?;
    let title = match title_obj {
        lopdf::Object::String(bytes, _) => String::from_utf8_lossy(bytes).to_string(),
        _ => return None,
    };

    let trimmed = title.trim().to_string();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("untitled") {
        return None;
    }

    Some(trimmed)
}

/// Heuristic: the title is typically the first non-empty line(s) of the paper text,
/// before any all-caps heading, author line, or large blank gap.
fn extract_title_from_text(text: &str) -> Option<String> {
    let mut title_lines: Vec<&str> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Skip empty lines at the start
        if trimmed.is_empty() {
            if title_lines.is_empty() {
                continue;
            }
            // First blank line after title content — stop
            break;
        }

        // Stop if we hit a likely heading (all caps section like ABSTRACT, INTRODUCTION)
        if is_likely_heading(trimmed) && !title_lines.is_empty() {
            break;
        }

        // Stop if we hit author-like content (contains @, "university", "department", etc.)
        let lower = trimmed.to_lowercase();
        if !title_lines.is_empty()
            && (lower.contains('@')
                || lower.contains("university")
                || lower.contains("department")
                || lower.contains("institute")
                || lower.contains("abstract"))
        {
            break;
        }

        title_lines.push(trimmed);

        // Cap at 2 lines (titles are rarely longer)
        if title_lines.len() >= 2 {
            break;
        }
    }

    let title = title_lines.join(" ");
    let title = title.trim();

    if title.is_empty() || title.len() < 3 {
        return None;
    }

    // Cap at 200 chars
    if title.len() > 200 {
        return Some(title[..200].trim().to_string());
    }

    Some(title.to_string())
}

/// Format extracted PDF text as Markdown
/// This adds structure to the raw text extraction
fn format_as_markdown(text: &str) -> String {
    let lines = normalize_pdf_lines(text);
    let mut markdown = String::new();
    let mut in_table = false;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let prev_blank = index == 0 || lines[index - 1].trim().is_empty();
        let next_blank = index + 1 == lines.len() || lines[index + 1].trim().is_empty();

        // Skip empty lines
        if trimmed.is_empty() {
            if in_table {
                markdown.push_str("\n");
                in_table = false;
            }
            markdown.push_str("\n");
            continue;
        }

        // Detect potential table rows (lines with multiple tab-separated or spaced columns)
        if is_likely_table_row(trimmed) {
            if !in_table {
                // Start table - add header separator
                markdown.push_str(&format_table_row(trimmed));
                markdown.push('\n');
                markdown.push_str(&create_table_separator(trimmed));
                markdown.push('\n');
                in_table = true;
            } else {
                markdown.push_str(&format_table_row(trimmed));
                markdown.push('\n');
            }
        } else {
            if in_table {
                markdown.push('\n');
                in_table = false;
            }

            // Detect headings (ALL CAPS lines or lines ending with :)
            if should_emit_heading(trimmed, prev_blank, next_blank) {
                let level = if trimmed.len() < 30 { "##" } else { "###" };
                markdown.push_str(&format!("{} {}\n\n", level, trimmed.trim_end_matches(':')));
            } else {
                markdown.push_str(trimmed);
                markdown.push('\n');
            }
        }
    }

    markdown
}

fn normalize_pdf_lines(text: &str) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        let mut current = line.trim_end().to_string();

        while let Some(next) = lines.peek() {
            let next_trimmed = next.trim_start();
            if current.ends_with('-')
                && next_trimmed
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_lowercase())
            {
                current.pop();
                current.push_str(next_trimmed);
                lines.next();
            } else {
                break;
            }
        }

        normalized.push(current.trim_end().to_string());
    }

    normalized
}

/// Check if a line looks like a table row
fn is_likely_table_row(line: &str) -> bool {
    // Check for multiple whitespace-separated columns (3+ columns)
    let columns: Vec<&str> = line.split_whitespace().collect();
    if columns.len() >= 3 {
        // Check if columns have consistent spacing (indicating a table)
        let has_tabs = line.contains('\t');
        let has_multiple_spaces = line.contains("  ");
        return has_tabs || has_multiple_spaces;
    }
    false
}

/// Format a line as a markdown table row
fn format_table_row(line: &str) -> String {
    // Split by tabs or multiple spaces
    let columns: Vec<&str> = if line.contains('\t') {
        line.split('\t').map(|s| s.trim()).collect()
    } else {
        // Split by 2+ spaces
        line.split("  ")
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim())
            .collect()
    };

    format!("| {} |", columns.join(" | "))
}

/// Create a markdown table separator
fn create_table_separator(header: &str) -> String {
    let column_count = if header.contains('\t') {
        header.split('\t').count()
    } else {
        header.split("  ").filter(|s| !s.trim().is_empty()).count()
    };

    let separators = vec!["---"; column_count];
    format!("| {} |", separators.join(" | "))
}

/// Check if a line looks like a heading
fn is_likely_heading(line: &str) -> bool {
    if contains_math_symbols(line) {
        return false;
    }

    let trimmed = line.trim();
    if has_section_prefix(trimmed) {
        return true;
    }

    // Check for ALL CAPS (at least 80% uppercase)
    let uppercase_count = line.chars().filter(|c| c.is_uppercase()).count();
    let alpha_count = line.chars().filter(|c| c.is_alphabetic()).count();

    if alpha_count >= 5 {
        let uppercase_ratio = uppercase_count as f64 / alpha_count as f64;
        if uppercase_ratio > 0.8 && line.len() < 100 {
            return true;
        }
    }

    // Check for lines ending with colon (section headers)
    if alpha_count >= 4 && line.ends_with(':') && line.len() < 80 && !line.contains("://") {
        return true;
    }

    false
}

fn should_emit_heading(line: &str, prev_blank: bool, next_blank: bool) -> bool {
    is_likely_heading(line) && prev_blank && next_blank
}

fn has_section_prefix(line: &str) -> bool {
    let Some((prefix, rest)) = line.split_once(". ") else {
        return false;
    };

    let is_alpha_prefix = prefix.len() == 1
        && prefix
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase());
    let is_roman_prefix = !prefix.is_empty()
        && prefix
            .chars()
            .all(|ch| matches!(ch, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'));

    (is_alpha_prefix || is_roman_prefix)
        && rest.chars().filter(|ch| ch.is_alphabetic()).count() >= 3
}

fn contains_math_symbols(line: &str) -> bool {
    line.chars().any(|ch| {
        matches!(
            ch,
            '[' | ']' | '{' | '}' | '⊕' | '⊗' | '∑' | '∈' | '←' | '→' | '≤' | '≥' | 'ℓ'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_table_row() {
        assert!(is_likely_table_row("Column1  Column2  Column3"));
        assert!(is_likely_table_row("Name\tAge\tCity"));
        assert!(!is_likely_table_row("This is a normal sentence"));
        assert!(!is_likely_table_row("Only two"));
    }

    #[test]
    fn test_format_table_row() {
        let row = "Name  Age  City";
        assert_eq!(format_table_row(row), "| Name | Age | City |");

        let tab_row = "Name\tAge\tCity";
        assert_eq!(format_table_row(tab_row), "| Name | Age | City |");
    }

    #[test]
    fn test_create_table_separator() {
        let header = "Name  Age  City";
        assert_eq!(create_table_separator(header), "| --- | --- | --- |");
    }

    #[test]
    fn test_is_likely_heading() {
        assert!(is_likely_heading("INTRODUCTION"));
        assert!(is_likely_heading("Chapter 1:"));
        assert!(is_likely_heading("Section Title:"));
        assert!(!is_likely_heading("B"));
        assert!(!is_likely_heading("JxKA"));
        assert!(!is_likely_heading("This is a normal sentence"));
        assert!(!is_likely_heading("https://example.com"));
    }

    #[test]
    fn test_format_as_markdown_simple() {
        let text = "INTRODUCTION\n\nThis is some text.\n\nSection 1:\nMore text here.";
        let markdown = format_as_markdown(text);
        assert!(markdown.contains("## INTRODUCTION"));
        assert!(markdown.contains("## Section 1"));
    }

    #[test]
    fn test_format_as_markdown_with_table() {
        let text = "Name  Age  City\nJohn  30  NYC\nJane  25  LA";
        let markdown = format_as_markdown(text);
        assert!(markdown.contains("| Name | Age | City |"));
        assert!(markdown.contains("| --- | --- | --- |"));
        assert!(markdown.contains("| John | 30 | NYC |"));
    }

    #[test]
    fn test_heading_requires_blank_lines() {
        assert!(should_emit_heading("A. Related work", true, true));
        assert!(!should_emit_heading("A. Related work", false, true));
    }

    #[test]
    fn test_format_as_markdown_does_not_promote_math_fragments() {
        let text = "A. Related work\n\n[b]\nB\n\n0 ⊕ [b]\nB\n1 = b\n";
        let markdown = format_as_markdown(text);
        assert!(markdown.contains("## A. Related work"));
        assert!(!markdown.contains("## B"));
        assert!(!markdown.contains("## [b]"));
    }
}
