use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize any JSON value as a String.
/// Objects/arrays get serialized to their JSON representation.
pub fn string_or_json<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Null => Ok(String::new()),
        other => Ok(other.to_string()),
    }
}

/// Same as string_or_json but returns empty string for missing/null fields.
pub fn string_or_json_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Option::<serde_json::Value>::deserialize(deserializer)?;
    match val {
        None | Some(serde_json::Value::Null) => Ok(String::new()),
        Some(serde_json::Value::String(s)) => Ok(s),
        Some(other) => Ok(other.to_string()),
    }
}

/// Deserialize an Option<String> that might come as a JSON object.
pub fn opt_string_or_json<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Option::<serde_json::Value>::deserialize(deserializer)?;
    match val {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(other) => Ok(Some(other.to_string())),
    }
}

/// Deserialize a usize that might come as a string.
pub fn usize_or_string<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::Number(n) => n
            .as_u64()
            .map(|v| v as usize)
            .ok_or_else(|| serde::de::Error::custom("expected unsigned integer")),
        serde_json::Value::String(s) => s
            .parse::<usize>()
            .map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom("expected number or string")),
    }
}

/// Deserialize evidence items from either a JSON array or an object containing an array.
fn vec_or_wrapped<'de, D>(deserializer: D) -> Result<Vec<EvidenceItem>, D::Error>
where
    D: Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::Array(_) => {
            serde_json::from_value(val).map_err(serde::de::Error::custom)
        }
        serde_json::Value::Object(ref map) => {
            for v in map.values() {
                if v.is_array() {
                    return serde_json::from_value::<Vec<EvidenceItem>>(v.clone())
                        .map_err(serde::de::Error::custom);
                }
            }
            Err(serde::de::Error::custom(
                "evidence_items is an object but contains no array",
            ))
        }
        _ => Err(serde::de::Error::custom(
            "evidence_items must be an array or object containing an array",
        )),
    }
}

/// Deserialize a Vec<String> that might contain non-string elements.
pub fn flexible_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::Array(arr) => Ok(arr
            .into_iter()
            .map(|v| match v {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            })
            .collect()),
        serde_json::Value::Null => Ok(Vec::new()),
        _ => Ok(vec![val.to_string()]),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    #[serde(deserialize_with = "string_or_json")]
    pub id: String,
    #[serde(deserialize_with = "string_or_json")]
    pub quote: String,
    #[serde(default, deserialize_with = "string_or_json_default")]
    pub location: String,
    #[serde(rename = "type", deserialize_with = "string_or_json")]
    pub evidence_type: String,
    #[serde(default, deserialize_with = "string_or_json_default")]
    pub importance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceInventory {
    #[serde(deserialize_with = "string_or_json")]
    pub paper_title: String,
    #[serde(deserialize_with = "vec_or_wrapped")]
    pub evidence_items: Vec<EvidenceItem>,
    #[serde(default, deserialize_with = "string_or_json_default")]
    pub paper_type: String,
    #[serde(default, alias = "core_contribution_summary", deserialize_with = "string_or_json_default")]
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPattern {
    #[serde(default, deserialize_with = "usize_or_string")]
    pub rank: usize,
    #[serde(deserialize_with = "string_or_json")]
    pub name: String,
    #[serde(default, deserialize_with = "opt_string_or_json")]
    pub claim: Option<String>,
    #[serde(default, deserialize_with = "opt_string_or_json")]
    pub evidence: Option<String>,
    #[serde(default, deserialize_with = "opt_string_or_json")]
    pub context: Option<String>,
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
pub struct PatternExtractionOutput {
    pub patterns: Vec<ExtractedPattern>,
    #[serde(default)]
    pub total_evidence_used: usize,
    #[serde(default, deserialize_with = "flexible_string_vec")]
    pub gaps_identified: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationIssue {
    #[serde(default)]
    pub pattern_rank: usize,
    #[serde(default)]
    pub field: String,
    #[serde(default)]
    pub issue: String,
    #[serde(default)]
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyConcern {
    #[serde(default)]
    pub pattern_rank: usize,
    #[serde(default)]
    pub evidence_id: String,
    #[serde(default)]
    pub concern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    #[serde(default)]
    pub verification_status: String,
    #[serde(default)]
    pub citation_issues: Vec<CitationIssue>,
    #[serde(default, deserialize_with = "flexible_string_vec")]
    pub unused_evidence: Vec<String>,
    #[serde(default)]
    pub accuracy_concerns: Vec<AccuracyConcern>,
    #[serde(default)]
    pub overall_quality: String,
    #[serde(default, deserialize_with = "flexible_string_vec")]
    pub improvement_suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub patterns: Vec<ExtractedPattern>,
    pub evidence: EvidenceInventory,
    pub verification: Option<VerificationResult>,
}
