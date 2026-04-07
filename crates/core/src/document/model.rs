#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    Paper,
    Book,
    Code,
    Markdown,
    PlainText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Paragraph,
    List,
    Table,
    Code,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::List => "list",
            Self::Table => "table",
            Self::Code => "code",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructuralNode {
    pub kind: NodeKind,
    pub text: String,
    pub heading_path: Vec<String>,
    pub caption: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub page_numbers: Option<Vec<u32>>,
}

#[derive(Debug, Clone)]
pub struct NormalizedDocument {
    pub source_kind: SourceKind,
    pub title: Option<String>,
    pub nodes: Vec<StructuralNode>,
}
