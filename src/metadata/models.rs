use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperStatus {
    Processing,
    ReadyForReview,
    Active,
    Archived,
}

impl fmt::Display for PaperStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaperStatus::Processing => write!(f, "processing"),
            PaperStatus::ReadyForReview => write!(f, "ready_for_review"),
            PaperStatus::Active => write!(f, "active"),
            PaperStatus::Archived => write!(f, "archived"),
        }
    }
}

impl PaperStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "ready_for_review" => PaperStatus::ReadyForReview,
            "active" => PaperStatus::Active,
            "archived" => PaperStatus::Archived,
            _ => PaperStatus::Processing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternStatus {
    Pending,
    Approved,
    Rejected,
}

impl fmt::Display for PatternStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternStatus::Pending => write!(f, "pending"),
            PatternStatus::Approved => write!(f, "approved"),
            PatternStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl PatternStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "approved" => PatternStatus::Approved,
            "rejected" => PatternStatus::Rejected,
            _ => PatternStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub paper_id: String,
    pub name: String,
    pub claim: Option<String>,
    pub evidence: Option<String>,
    pub context: Option<String>,
    pub tags: Vec<String>,
    pub confidence: String,
    pub status: PatternStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub source: Option<String>,
    pub published_date: Option<String>,
    pub paper_type: String,
    pub status: PaperStatus,
    pub original_filename: Option<String>,
    pub file_path: Option<String>,
    pub chunk_count: usize,
    pub pattern_count: usize,
    pub algorithm_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaperCreate {
    pub title: String,
    #[serde(default)]
    pub authors: Vec<String>,
    pub source: Option<String>,
    pub published_date: Option<String>,
    #[serde(default = "default_paper_type")]
    pub paper_type: String,
    pub original_filename: Option<String>,
    pub file_path: Option<String>,
}

fn default_paper_type() -> String {
    "research_paper".to_string()
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PaperListParams {
    pub status: Option<String>,
    pub paper_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Algorithm {
    pub id: String,
    pub paper_id: String,
    pub name: String,
    pub description: Option<String>,
    pub steps: Vec<AlgorithmStepRow>,
    pub inputs: Vec<AlgorithmIORow>,
    pub outputs: Vec<AlgorithmIORow>,
    pub preconditions: Vec<String>,
    pub complexity: Option<String>,
    pub mathematical_notation: Option<String>,
    pub pseudocode: Option<String>,
    pub tags: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub confidence: String,
    pub status: PatternStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmStepRow {
    pub number: usize,
    pub action: String,
    pub details: String,
    pub math: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmIORow {
    pub name: String,
    #[serde(rename = "type")]
    pub io_type: String,
    pub description: String,
}
