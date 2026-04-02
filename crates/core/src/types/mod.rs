use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request to query the indexed content
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryRequest {
    /// The question or search query
    pub query: String,
    /// Optional path to filter by specific indexed content
    #[serde(default)]
    pub path: Option<String>,
    /// Optional project name to filter by
    #[serde(default)]
    pub project: Option<String>,
    /// Number of results to return (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Minimum similarity score (0.0 to 1.0, default: 0.7)
    #[serde(default = "default_min_score")]
    pub min_score: f32,
    /// Enable hybrid search (vector + keyword) - default: true
    #[serde(default = "default_hybrid")]
    pub hybrid: bool,
}

fn default_hybrid() -> bool {
    true
}

fn default_limit() -> usize {
    10
}

fn default_min_score() -> f32 {
    0.7
}

/// A single search result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    /// File path relative to the indexed root
    pub file_path: String,
    /// Absolute path to the indexed root directory
    #[serde(default)]
    pub root_path: Option<String>,
    /// The content chunk
    pub content: String,
    /// Combined similarity score (0.0 to 1.0)
    pub score: f32,
    /// Vector similarity score (0.0 to 1.0)
    pub vector_score: f32,
    /// Keyword match score (0.0 to 1.0) - only present in hybrid search
    pub keyword_score: Option<f32>,
    /// Starting line number in the file
    pub start_line: usize,
    /// Ending line number in the file
    pub end_line: usize,
    /// Language or content type
    pub language: String,
    /// Optional project name for multi-project support
    pub project: Option<String>,
    /// Page numbers this chunk spans (for PDF context-aware chunking)
    #[serde(default)]
    pub page_numbers: Option<Vec<u32>>,
    /// Heading context from the document structure
    #[serde(default)]
    pub heading_context: Option<String>,
}

/// Response from query operation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryResponse {
    /// List of search results, ordered by relevance
    pub results: Vec<SearchResult>,
    /// Time taken in milliseconds
    pub duration_ms: u64,
    /// The actual threshold used (may be lower than requested if adaptive search kicked in)
    #[serde(default)]
    pub threshold_used: f32,
    /// Whether the threshold was automatically lowered to find results
    #[serde(default)]
    pub threshold_lowered: bool,
}

/// Request to get statistics about the index
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StatisticsRequest {}

/// Statistics about the indexed content
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StatisticsResponse {
    /// Total number of indexed files
    pub total_files: usize,
    /// Total number of chunks
    pub total_chunks: usize,
    /// Total number of embeddings
    pub total_embeddings: usize,
    /// Size of the vector database in bytes
    pub database_size_bytes: u64,
    /// Breakdown by language/type
    pub language_breakdown: Vec<LanguageStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LanguageStats {
    pub language: String,
    pub file_count: usize,
    pub chunk_count: usize,
}

/// Request to clear the index
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClearRequest {}

/// Response from clear operation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClearResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Optional message
    pub message: String,
}

/// Request to search papers by keyword and filters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchPapersRequest {
    /// Keyword to search in paper title and authors (optional)
    #[serde(default)]
    pub query: Option<String>,
    /// Filter by status: "processing", "ready_for_review", "active", "archived"
    #[serde(default)]
    pub status: Option<String>,
    /// Filter by paper type (e.g. "research_paper")
    #[serde(default)]
    pub paper_type: Option<String>,
    /// Number of results to return (default: 20)
    #[serde(default = "default_papers_limit")]
    pub limit: usize,
    /// Pagination offset (default: 0)
    #[serde(default)]
    pub offset: usize,
}

fn default_papers_limit() -> usize {
    20
}

/// A single paper result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PaperResult {
    /// Unique paper ID
    pub id: String,
    /// Paper title
    pub title: String,
    /// List of authors
    pub authors: Vec<String>,
    /// Source URL or reference
    pub source: Option<String>,
    /// Publication date
    pub published_date: Option<String>,
    /// Paper type (e.g. "research_paper")
    pub paper_type: String,
    /// Current status: "processing", "ready_for_review", "active", "archived"
    pub status: String,
    /// Number of indexed chunks
    pub chunk_count: usize,
    /// Absolute path to the stored paper file (if available)
    pub file_path: Option<String>,
    /// Creation timestamp (RFC3339)
    pub created_at: String,
}

/// Response from search_papers
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchPapersResponse {
    /// Matching papers
    pub papers: Vec<PaperResult>,
    /// Total matching papers (before pagination)
    pub total: usize,
    /// Limit used
    pub limit: usize,
    /// Offset used
    pub offset: usize,
    /// Time taken in milliseconds
    pub duration_ms: u64,
}

fn default_algorithms_limit() -> usize {
    20
}

fn default_algorithm_status() -> Option<String> {
    Some("approved".to_string())
}

/// Request to search algorithms across all papers
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchAlgorithmsRequest {
    /// Keyword to search in algorithm name and description
    #[serde(default)]
    pub query: Option<String>,
    /// Filter by status: "pending", "approved", "rejected" (default: "approved")
    #[serde(default = "default_algorithm_status")]
    pub status: Option<String>,
    /// Filter by paper ID (omit to search across all papers)
    #[serde(default)]
    pub paper_id: Option<String>,
    /// Filter by tags (algorithms must have ALL specified tags)
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Number of results to return (default: 20)
    #[serde(default = "default_algorithms_limit")]
    pub limit: usize,
    /// Pagination offset (default: 0)
    #[serde(default)]
    pub offset: usize,
}

/// A single algorithm result with paper context
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AlgorithmResult {
    /// Unique algorithm ID
    pub id: String,
    /// Paper this algorithm was extracted from
    pub paper_id: String,
    /// Title of the source paper
    pub paper_title: String,
    /// Algorithm name
    pub name: String,
    /// Algorithm description
    pub description: Option<String>,
    /// Ordered steps
    pub steps: Vec<serde_json::Value>,
    /// Algorithm inputs
    pub inputs: Vec<serde_json::Value>,
    /// Algorithm outputs
    pub outputs: Vec<serde_json::Value>,
    /// Required preconditions
    pub preconditions: Vec<String>,
    /// Time/space complexity
    pub complexity: Option<String>,
    /// LaTeX notation
    pub mathematical_notation: Option<String>,
    /// Pseudocode
    pub pseudocode: Option<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Confidence level
    pub confidence: String,
    /// Review status
    pub status: String,
    /// Creation timestamp
    pub created_at: String,
}

/// Response from search_algorithms
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchAlgorithmsResponse {
    /// Matching algorithms
    pub algorithms: Vec<AlgorithmResult>,
    /// Total matching algorithms (before pagination)
    pub total: usize,
    /// Limit used
    pub limit: usize,
    /// Offset used
    pub offset: usize,
    /// Time taken in milliseconds
    pub duration_ms: u64,
}

fn default_paper_type() -> String {
    "research_paper".to_string()
}

/// Request to index a paper via MCP (from file path or URL)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IndexPaperRequest {
    /// Absolute path to a local PDF or text file
    #[serde(default)]
    pub file_path: Option<String>,
    /// URL to download a PDF from
    #[serde(default)]
    pub url: Option<String>,
    /// Paper title (auto-detected from PDF metadata if omitted)
    #[serde(default)]
    pub title: Option<String>,
    /// Comma-separated author names
    #[serde(default)]
    pub authors: Option<String>,
    /// Source reference
    #[serde(default)]
    pub source: Option<String>,
    /// Paper type (default: "research_paper")
    #[serde(default = "default_paper_type")]
    pub paper_type: String,
}

/// Response from index_paper
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IndexPaperResponse {
    /// Unique paper ID
    pub paper_id: String,
    /// Paper title
    pub title: String,
    /// Number of chunks indexed
    pub chunk_count: usize,
    /// Paper status after indexing
    pub status: String,
    /// Time taken in milliseconds
    pub duration_ms: u64,
}

/// Request to extract algorithms from an indexed paper
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractAlgorithmsRequest {
    /// ID of the paper to extract algorithms from (must be indexed first via index_paper)
    pub paper_id: String,
}

/// Response from extract_algorithms
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractAlgorithmsResponse {
    /// Paper ID
    pub paper_id: String,
    /// Number of algorithms extracted
    pub algorithm_count: usize,
    /// Number of evidence items found
    pub evidence_count: usize,
    /// Verification status from quality check pass ("pass", "warn", "fail", or null)
    pub verification_status: Option<String>,
    /// Time taken in milliseconds
    pub duration_ms: u64,
}

/// Metadata stored with each chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// File path relative to indexed root
    pub file_path: String,
    /// Absolute path to the indexed root directory
    #[serde(default)]
    pub root_path: Option<String>,
    /// Project name (for multi-project support)
    pub project: Option<String>,
    /// Starting line number
    pub start_line: usize,
    /// Ending line number
    pub end_line: usize,
    /// Language or content type
    pub language: Option<String>,
    /// File extension
    pub extension: Option<String>,
    /// SHA256 hash of the file content
    pub file_hash: String,
    /// Timestamp when indexed
    pub indexed_at: i64,
    /// Page numbers this chunk spans (for PDF context-aware chunking)
    #[serde(default)]
    pub page_numbers: Option<Vec<u32>>,
    /// Heading context from the document structure
    #[serde(default)]
    pub heading_context: Option<String>,
    /// Element types in this chunk (e.g., "Paragraph", "Table")
    #[serde(default)]
    pub element_types: Option<Vec<String>>,
}

impl QueryRequest {
    /// Validate the query request
    pub fn validate(&self) -> Result<(), String> {
        if self.query.trim().is_empty() {
            return Err("query cannot be empty".to_string());
        }

        const MAX_QUERY_LENGTH: usize = 10_240;
        if self.query.len() > MAX_QUERY_LENGTH {
            return Err(format!(
                "query too long: {} bytes (max: {} bytes)",
                self.query.len(),
                MAX_QUERY_LENGTH
            ));
        }

        if !(0.0..=1.0).contains(&self.min_score) {
            return Err(format!(
                "min_score must be between 0.0 and 1.0, got: {}",
                self.min_score
            ));
        }

        const MAX_LIMIT: usize = 1000;
        if self.limit > MAX_LIMIT {
            return Err(format!(
                "limit too large: {} (max: {})",
                self.limit, MAX_LIMIT
            ));
        }

        if let Some(ref project) = self.project {
            if project.is_empty() {
                return Err("project name cannot be empty".to_string());
            }
            if project.len() > 256 {
                return Err("project name too long (max 256 characters)".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_request_defaults() {
        let req: QueryRequest = serde_json::from_str(r#"{"query": "test"}"#).unwrap();
        assert_eq!(req.limit, 10);
        assert!((req.min_score - 0.7).abs() < f32::EPSILON);
        assert!(req.hybrid);
    }

    #[test]
    fn test_query_request_validate_empty_query() {
        let req = QueryRequest {
            query: "".to_string(),
            path: None,
            project: None,
            limit: 10,
            min_score: 0.7,
            hybrid: true,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_query_request_validate_valid() {
        let req = QueryRequest {
            query: "test query".to_string(),
            path: None,
            project: None,
            limit: 10,
            min_score: 0.7,
            hybrid: true,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_query_request_validate_min_score_out_of_range() {
        let req = QueryRequest {
            query: "test".to_string(),
            path: None,
            project: None,
            limit: 10,
            min_score: 1.5,
            hybrid: true,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_query_request_validate_limit_too_large() {
        let req = QueryRequest {
            query: "test".to_string(),
            path: None,
            project: None,
            limit: 5000,
            min_score: 0.7,
            hybrid: true,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_search_result_creation() {
        let result = SearchResult {
            file_path: "test.md".to_string(),
            root_path: Some("papers".to_string()),
            content: "test content".to_string(),
            score: 0.95,
            vector_score: 0.90,
            keyword_score: Some(0.85),
            start_line: 1,
            end_line: 10,
            language: "Markdown".to_string(),
            project: Some("test-project".to_string()),
            page_numbers: None,
            heading_context: None,
        };
        assert_eq!(result.file_path, "test.md");
    }

    #[test]
    fn test_chunk_metadata_creation() {
        let meta = ChunkMetadata {
            file_path: "test.md".to_string(),
            root_path: Some("papers".to_string()),
            project: Some("test-project".to_string()),
            start_line: 1,
            end_line: 50,
            language: Some("Markdown".to_string()),
            extension: Some("md".to_string()),
            file_hash: "abc123".to_string(),
            indexed_at: 1234567890,
            page_numbers: None,
            heading_context: None,
            element_types: None,
        };
        assert_eq!(meta.file_path, "test.md");
    }

    #[test]
    fn test_clear_response() {
        let response = ClearResponse {
            success: true,
            message: "Cleared".to_string(),
        };
        assert!(response.success);
    }

    #[test]
    fn test_statistics_response() {
        let response = StatisticsResponse {
            total_files: 10,
            total_chunks: 100,
            total_embeddings: 100,
            database_size_bytes: 1024,
            language_breakdown: vec![LanguageStats {
                language: "Markdown".to_string(),
                file_count: 10,
                chunk_count: 100,
            }],
        };
        assert_eq!(response.total_files, 10);
    }

    #[test]
    fn test_query_response_serialization() {
        let response = QueryResponse {
            results: vec![],
            duration_ms: 42,
            threshold_used: 0.7,
            threshold_lowered: false,
        };
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: QueryResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.duration_ms, 42);
    }

    #[test]
    fn test_papers_request_serialization() {
        let req = SearchPapersRequest {
            query: Some("neural".to_string()),
            status: Some("active".to_string()),
            paper_type: None,
            limit: 20,
            offset: 0,
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: SearchPapersRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.query, Some("neural".to_string()));
    }
}
