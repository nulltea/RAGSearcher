use serde::{Deserialize, Serialize};

use super::types::{
    CitationIssue, flexible_string_vec, opt_string_or_json, string_or_json,
    string_or_json_default, usize_or_string,
};

// --- Pass 1: Algorithm Inventory ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmCandidate {
    #[serde(deserialize_with = "string_or_json")]
    pub id: String,
    #[serde(deserialize_with = "string_or_json")]
    pub name: String,
    #[serde(default, deserialize_with = "string_or_json_default")]
    pub description: String,
    #[serde(default, deserialize_with = "string_or_json_default")]
    pub location: String,
    #[serde(default, rename = "type", deserialize_with = "string_or_json_default")]
    pub algorithm_type: String,
    #[serde(default, deserialize_with = "flexible_string_vec")]
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmInventory {
    #[serde(deserialize_with = "string_or_json")]
    pub paper_title: String,
    pub algorithms: Vec<AlgorithmCandidate>,
    #[serde(default, deserialize_with = "string_or_json_default")]
    pub paper_type: String,
}

// --- Pass 2: Algorithm Extraction ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmStep {
    #[serde(default, deserialize_with = "usize_or_string")]
    pub number: usize,
    #[serde(deserialize_with = "string_or_json")]
    pub action: String,
    #[serde(default, deserialize_with = "string_or_json_default")]
    pub details: String,
    #[serde(default, deserialize_with = "opt_string_or_json")]
    pub math: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmIO {
    #[serde(deserialize_with = "string_or_json")]
    pub name: String,
    #[serde(default, rename = "type", deserialize_with = "string_or_json_default")]
    pub io_type: String,
    #[serde(default, deserialize_with = "string_or_json_default")]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedAlgorithm {
    #[serde(default, deserialize_with = "usize_or_string")]
    pub rank: usize,
    #[serde(deserialize_with = "string_or_json")]
    pub name: String,
    #[serde(default, deserialize_with = "string_or_json_default")]
    pub description: String,
    pub steps: Vec<AlgorithmStep>,
    #[serde(default)]
    pub inputs: Vec<AlgorithmIO>,
    #[serde(default)]
    pub outputs: Vec<AlgorithmIO>,
    #[serde(default, deserialize_with = "flexible_string_vec")]
    pub preconditions: Vec<String>,
    #[serde(default, deserialize_with = "opt_string_or_json")]
    pub complexity: Option<String>,
    #[serde(default, deserialize_with = "opt_string_or_json")]
    pub mathematical_notation: Option<String>,
    #[serde(default, deserialize_with = "opt_string_or_json")]
    pub pseudocode: Option<String>,
    #[serde(default, deserialize_with = "flexible_string_vec")]
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "flexible_string_vec")]
    pub evidence_ids: Vec<String>,
    #[serde(default = "default_confidence", deserialize_with = "string_or_json")]
    pub confidence: String,
}

fn default_confidence() -> String {
    "medium".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmExtractionOutput {
    pub algorithms: Vec<ExtractedAlgorithm>,
    #[serde(default)]
    pub total_evidence_used: usize,
}

// --- Pass 3: Verification ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletenessIssue {
    #[serde(default)]
    pub algorithm_rank: usize,
    #[serde(default)]
    pub issue: String,
    #[serde(default)]
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmVerificationResult {
    #[serde(default)]
    pub verification_status: String,
    #[serde(default)]
    pub completeness_issues: Vec<CompletenessIssue>,
    #[serde(default)]
    pub citation_issues: Vec<CitationIssue>,
    #[serde(default)]
    pub overall_quality: String,
    #[serde(default, deserialize_with = "flexible_string_vec")]
    pub improvement_suggestions: Vec<String>,
}

// --- Combined result ---

#[derive(Debug, Clone)]
pub struct AlgorithmExtractionResult {
    pub algorithms: Vec<ExtractedAlgorithm>,
    pub verification: Option<AlgorithmVerificationResult>,
}
