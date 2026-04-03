use crate::metadata::models::{Algorithm, Paper, Pattern};
use crate::types::SearchResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct PaperUploadResponse {
    pub paper: Paper,
    pub chunk_count: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct PaperListResponse {
    pub papers: Vec<Paper>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub paper_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_min_score")]
    pub min_score: f32,
    #[serde(default = "default_hybrid")]
    pub hybrid: bool,
}

fn default_limit() -> usize {
    10
}
fn default_min_score() -> f32 {
    0.7
}
fn default_hybrid() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct StatisticsResponse {
    pub total_chunks: usize,
    pub total_vectors: usize,
    pub languages: Vec<(String, usize)>,
}

// --- Pattern types ---

#[derive(Debug, Serialize)]
pub struct ExtractResponse {
    pub paper_id: String,
    pub patterns: Vec<Pattern>,
    pub evidence_count: usize,
    pub verification_status: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct PatternListResponse {
    pub patterns: Vec<Pattern>,
}

#[derive(Debug, Deserialize)]
pub struct PatternReviewRequest {
    pub decisions: Vec<PatternDecision>,
}

#[derive(Debug, Deserialize)]
pub struct PatternDecision {
    pub pattern_id: String,
    pub approved: bool,
}

#[derive(Debug, Serialize)]
pub struct PatternReviewResponse {
    pub approved_count: usize,
    pub rejected_count: usize,
    pub patterns: Vec<Pattern>,
}

// --- Algorithm types ---

#[derive(Debug, Serialize)]
pub struct AlgorithmExtractResponse {
    pub paper_id: String,
    pub algorithms: Vec<Algorithm>,
    pub evidence_count: usize,
    pub verification_status: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct AlgorithmListResponse {
    pub algorithms: Vec<Algorithm>,
}

#[derive(Debug, Serialize)]
pub struct AlgorithmReviewResponse {
    pub approved_count: usize,
    pub rejected_count: usize,
    pub algorithms: Vec<Algorithm>,
}
