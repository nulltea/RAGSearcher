use crate::metadata::models::Paper;
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
    0.5
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
