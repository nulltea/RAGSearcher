use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub id: String,
    pub quote: String,
    pub location: String,
    #[serde(rename = "type")]
    pub evidence_type: String,
    pub importance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceInventory {
    pub paper_title: String,
    pub evidence_items: Vec<EvidenceItem>,
    #[serde(default)]
    pub paper_type: String,
    #[serde(default, alias = "core_contribution_summary")]
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPattern {
    pub rank: usize,
    pub name: String,
    pub claim: Option<String>,
    pub evidence: Option<String>,
    pub context: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default = "default_confidence")]
    pub confidence: String,
}

fn default_confidence() -> String {
    "medium".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternExtractionOutput {
    pub patterns: Vec<ExtractedPattern>,
    #[serde(default)]
    pub total_evidence_used: usize,
    #[serde(default)]
    pub gaps_identified: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationIssue {
    pub pattern_rank: usize,
    pub field: String,
    pub issue: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyConcern {
    pub pattern_rank: usize,
    pub evidence_id: String,
    pub concern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verification_status: String,
    #[serde(default)]
    pub citation_issues: Vec<CitationIssue>,
    #[serde(default)]
    pub unused_evidence: Vec<String>,
    #[serde(default)]
    pub accuracy_concerns: Vec<AccuracyConcern>,
    #[serde(default)]
    pub overall_quality: String,
    #[serde(default)]
    pub improvement_suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub patterns: Vec<ExtractedPattern>,
    pub evidence: EvidenceInventory,
    pub verification: Option<VerificationResult>,
}
