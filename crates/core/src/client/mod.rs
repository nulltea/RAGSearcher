//! Core library client for project-rag
//!
//! This module provides the main client interface for using project-rag
//! as a library for paper embedding and semantic search.

use crate::chunker::{ContextAwareChunker, FixedChunker};
use crate::config::Config;
use crate::embedding::{EmbeddingProvider, MistralRsEmbedder, format_retrieval_query};
use crate::types::*;
use crate::vector_db::VectorDatabase;

// Conditionally import the appropriate vector database backend
#[cfg(feature = "qdrant-backend")]
use crate::vector_db::QdrantVectorDB;

#[cfg(not(feature = "qdrant-backend"))]
use crate::vector_db::LanceVectorDB;

use anyhow::{Context, Result};
use rust_stemmers::{Algorithm, Stemmer};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

const EXACT_MATCH_SCORE: f32 = 0.95;
const KEYWORD_MATCH_SCORE: f32 = 0.85;
const SEMANTIC_MIN_SCORE: f32 = 0.75;
const REFERENCE_PENALTY: f32 = 0.85;
const GLOBAL_PAPER_CAP: usize = 3;
const MIN_CANDIDATE_LIMIT: usize = 50;
const MAX_CANDIDATE_LIMIT: usize = 200;
const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "in", "into", "is", "of", "on",
    "or", "that", "the", "to", "with", "within",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryIntent {
    Exact,
    Keyword,
    Semantic,
}

#[derive(Debug, Clone)]
struct QueryProfile {
    intent: QueryIntent,
    exact_literal: Option<String>,
    significant_terms: Vec<String>,
}

/// Main client for interacting with the RAG system
///
/// This client provides a high-level API for embedding papers and performing
/// semantic searches. It contains all the core functionality and can be used
/// directly as a library or wrapped by the MCP server.
#[derive(Clone)]
pub struct RagClient {
    pub(crate) embedding_provider: Arc<MistralRsEmbedder>,
    #[cfg(feature = "qdrant-backend")]
    pub(crate) vector_db: Arc<QdrantVectorDB>,
    #[cfg(not(feature = "qdrant-backend"))]
    pub(crate) vector_db: Arc<LanceVectorDB>,
    pub(crate) chunker: Arc<FixedChunker>,
    pub(crate) pdf_chunker: Arc<ContextAwareChunker>,
    pub(crate) config: Arc<Config>,
}

impl RagClient {
    /// Get the embedding provider
    pub fn embedding_provider(&self) -> &Arc<MistralRsEmbedder> {
        &self.embedding_provider
    }

    /// Get the vector database
    #[cfg(feature = "qdrant-backend")]
    pub fn vector_db(&self) -> &Arc<QdrantVectorDB> {
        &self.vector_db
    }

    /// Get the vector database
    #[cfg(not(feature = "qdrant-backend"))]
    pub fn vector_db(&self) -> &Arc<LanceVectorDB> {
        &self.vector_db
    }

    /// Create a new RAG client with default configuration
    pub async fn new() -> Result<Self> {
        let config = Config::new().context("Failed to load configuration")?;
        Self::with_config(config).await
    }

    /// Create a new RAG client with custom configuration
    pub async fn with_config(config: Config) -> Result<Self> {
        tracing::info!("Initializing RAG client with configuration");
        tracing::debug!("Vector DB backend: {}", config.vector_db.backend);
        tracing::debug!("Embedding model: {}", config.embedding.model_name);
        tracing::debug!("Chunk size: {}", config.indexing.chunk_size);

        // Initialize embedding provider
        let embedding_provider = Arc::new(
            MistralRsEmbedder::new()
                .await
                .context("Failed to initialize embedding provider")?,
        );

        // Initialize the appropriate vector database backend
        #[cfg(feature = "qdrant-backend")]
        let vector_db = {
            tracing::info!(
                "Using Qdrant vector database backend at {}",
                config.vector_db.qdrant_url
            );
            Arc::new(
                QdrantVectorDB::with_url(&config.vector_db.qdrant_url)
                    .await
                    .context("Failed to initialize Qdrant vector database")?,
            )
        };

        #[cfg(not(feature = "qdrant-backend"))]
        let vector_db = {
            tracing::info!(
                "Using LanceDB vector database backend at {}",
                config.vector_db.lancedb_path.display()
            );
            Arc::new(
                LanceVectorDB::with_path(&config.vector_db.lancedb_path.to_string_lossy())
                    .await
                    .context("Failed to initialize LanceDB vector database")?,
            )
        };

        // Initialize the database with the embedding dimension
        vector_db
            .initialize(embedding_provider.dimension())
            .await
            .context("Failed to initialize vector database collections")?;

        // Create chunkers
        let chunker = Arc::new(FixedChunker::default_strategy());
        let pdf_chunker = Arc::new(ContextAwareChunker::new());

        Ok(Self {
            embedding_provider,
            vector_db,
            chunker,
            pdf_chunker,
            config: Arc::new(config),
        })
    }

    /// Create a new client with custom database path (for testing)
    #[cfg(test)]
    pub async fn new_with_db_path(db_path: &str, _cache_path: PathBuf) -> Result<Self> {
        let mut config = Config::default();
        config.vector_db.lancedb_path = PathBuf::from(db_path);
        Self::with_config(config).await
    }

    /// Normalize a path to a canonical absolute form
    pub fn normalize_path(path: &str) -> Result<String> {
        let path_buf = PathBuf::from(path);
        let canonical = std::fs::canonicalize(&path_buf)
            .with_context(|| format!("Failed to canonicalize path: {}", path))?;
        Ok(canonical.to_string_lossy().to_string())
    }

    fn strip_wrapping_quotes(query: &str) -> Option<&str> {
        let trimmed = query.trim();
        let quoted = [('"', '"'), ('\'', '\''), ('“', '”')];

        for (start, end) in quoted {
            if trimmed.starts_with(start) && trimmed.ends_with(end) && trimmed.len() > 1 {
                return Some(trimmed[start.len_utf8()..trimmed.len() - end.len_utf8()].trim());
            }
        }

        None
    }

    fn normalize_query_terms(text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .map(str::trim)
            .filter(|term| term.len() >= 2)
            .map(|term| term.to_lowercase())
            .filter(|term| !STOP_WORDS.contains(&term.as_str()))
            .collect()
    }

    fn stem_term(term: &str) -> String {
        Stemmer::create(Algorithm::English).stem(term).to_string()
    }

    fn content_term_sets(content: &str) -> (HashSet<String>, HashSet<String>) {
        let literal_terms: HashSet<String> = content
            .split(|c: char| !c.is_alphanumeric())
            .map(str::trim)
            .filter(|term| term.len() >= 2)
            .map(|term| term.to_lowercase())
            .collect();
        let stemmed_terms = literal_terms
            .iter()
            .map(|term| Self::stem_term(term))
            .collect::<HashSet<_>>();

        (literal_terms, stemmed_terms)
    }

    fn has_exact_token_signal(query: &str, significant_terms: &[String]) -> bool {
        if significant_terms.len() != 1 {
            return false;
        }

        let trimmed = query.trim();
        let has_identifier_marker = !trimmed.contains(char::is_whitespace)
            && (trimmed.contains('_')
                || trimmed.contains("::")
                || trimmed.contains('/')
                || trimmed.contains('.'));
        let has_digit = trimmed.chars().any(|c| c.is_ascii_digit());
        let has_uppercase_word = trimmed.chars().any(|c| c.is_ascii_alphabetic())
            && trimmed
                .chars()
                .all(|c| !c.is_ascii_alphabetic() || c.is_ascii_uppercase());

        has_identifier_marker || has_digit || has_uppercase_word
    }

    fn build_query_profile(query: &str) -> QueryProfile {
        if let Some(quoted) = Self::strip_wrapping_quotes(query)
            && !quoted.is_empty()
        {
            return QueryProfile {
                intent: QueryIntent::Exact,
                exact_literal: Some(quoted.to_lowercase()),
                significant_terms: Self::normalize_query_terms(quoted),
            };
        }

        let significant_terms = Self::normalize_query_terms(query);
        if Self::has_exact_token_signal(query, &significant_terms) {
            return QueryProfile {
                intent: QueryIntent::Exact,
                exact_literal: Some(query.trim().to_lowercase()),
                significant_terms,
            };
        }

        let intent = if significant_terms.len() <= 2 {
            QueryIntent::Keyword
        } else {
            QueryIntent::Semantic
        };

        QueryProfile {
            intent,
            exact_literal: None,
            significant_terms,
        }
    }

    fn is_reference_chunk(result: &SearchResult) -> bool {
        if let Some(heading_context) = &result.heading_context {
            let heading = heading_context.to_lowercase();
            if heading.contains("references") || heading.contains("bibliography") {
                return true;
            }
        }

        let content = result.content.to_lowercase();
        let lines: Vec<&str> = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        if lines.is_empty() {
            return false;
        }

        let citation_lines = lines
            .iter()
            .filter(|line| {
                line.starts_with('[')
                    && line
                        .chars()
                        .nth(1)
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
            })
            .count();

        let year_like_tokens = content
            .split_whitespace()
            .filter(|token| {
                let token = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
                token.len() == 4
                    && token.chars().all(|c| c.is_ascii_digit())
                    && matches!(token.as_bytes()[0], b'1' | b'2')
            })
            .count();

        citation_lines >= 2
            || (citation_lines >= 1 && year_like_tokens >= 2)
            || ((content.contains("http://") || content.contains("https://"))
                && (year_like_tokens >= 2 || content.contains("pp.")))
    }

    fn chunk_contains_all_terms(content: &str, terms: &[String]) -> bool {
        if terms.is_empty() {
            return false;
        }

        let (literal_terms, stemmed_terms) = Self::content_term_sets(content);
        terms.iter().all(|term| {
            literal_terms.contains(term) || stemmed_terms.contains(&Self::stem_term(term))
        })
    }

    fn result_group_key(result: &SearchResult) -> String {
        if let Some(project) = &result.project {
            return format!("project:{project}");
        }

        let mut parts = result.file_path.split('/');
        match (parts.next(), parts.next()) {
            (Some("papers" | "patterns" | "algorithms"), Some(paper_id)) => {
                format!("paper:{paper_id}")
            }
            _ => result
                .chunk_id
                .clone()
                .unwrap_or_else(|| format!("file:{}", result.file_path)),
        }
    }

    fn diversify_results(results: Vec<SearchResult>, limit: usize) -> Vec<SearchResult> {
        let mut per_group_counts: HashMap<String, usize> = HashMap::new();
        let mut diversified = Vec::new();

        for result in results {
            let group_key = Self::result_group_key(&result);
            let count = per_group_counts.entry(group_key).or_default();
            if *count >= GLOBAL_PAPER_CAP {
                continue;
            }

            *count += 1;
            diversified.push(result);

            if diversified.len() >= limit {
                break;
            }
        }

        diversified
    }

    fn calibrate_result(
        query_profile: &QueryProfile,
        mut result: SearchResult,
        min_score: f32,
    ) -> Option<SearchResult> {
        let match_type = match query_profile.intent {
            QueryIntent::Exact => {
                let exact_literal = query_profile.exact_literal.as_ref()?;
                if result.content.to_lowercase().contains(exact_literal) {
                    Some(SearchMatchType::Exact)
                } else {
                    None
                }
            }
            QueryIntent::Keyword => {
                if Self::chunk_contains_all_terms(&result.content, &query_profile.significant_terms)
                {
                    Some(SearchMatchType::Keyword)
                } else {
                    None
                }
            }
            QueryIntent::Semantic => {
                if Self::chunk_contains_all_terms(&result.content, &query_profile.significant_terms)
                {
                    Some(SearchMatchType::Keyword)
                } else if result.vector_score >= SEMANTIC_MIN_SCORE {
                    Some(SearchMatchType::Semantic)
                } else {
                    None
                }
            }
        }?;

        let mut score = match match_type {
            SearchMatchType::Exact => EXACT_MATCH_SCORE,
            SearchMatchType::Keyword => KEYWORD_MATCH_SCORE,
            SearchMatchType::Semantic => result.vector_score,
        };

        if Self::is_reference_chunk(&result) {
            score *= REFERENCE_PENALTY;
        }

        if score < min_score {
            return None;
        }

        result.score = score.clamp(0.0, 1.0);
        result.match_type = Some(match_type);
        Some(result)
    }

    /// Query the indexed content using semantic search
    pub async fn query_codebase(&self, request: QueryRequest) -> Result<QueryResponse> {
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        let start = Instant::now();

        let provider = self.embedding_provider.clone();
        let query_text = format_retrieval_query(&request.query);
        let query_embedding =
            tokio::task::spawn_blocking(move || provider.embed_batch(vec![query_text]))
                .await?
                .context("Failed to generate query embedding")?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No embedding generated"))?;
        let query_profile = Self::build_query_profile(&request.query);
        let candidate_limit = request
            .limit
            .saturating_mul(10)
            .clamp(MIN_CANDIDATE_LIMIT, MAX_CANDIDATE_LIMIT);

        let raw_results = self
            .vector_db
            .search(
                query_embedding.clone(),
                &request.query,
                candidate_limit,
                0.0,
                request.project.clone(),
                request.path.clone(),
                request.hybrid,
            )
            .await
            .context("Failed to search")?;
        let mut results: Vec<SearchResult> = raw_results
            .into_iter()
            .filter_map(|result| Self::calibrate_result(&query_profile, result, request.min_score))
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.combined_score
                        .unwrap_or(b.vector_score)
                        .partial_cmp(&a.combined_score.unwrap_or(a.vector_score))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        let results = if request.project.is_none() {
            Self::diversify_results(results, request.limit)
        } else {
            let mut results = results;
            results.truncate(request.limit);
            results
        };

        Ok(QueryResponse {
            results,
            duration_ms: start.elapsed().as_millis() as u64,
            threshold_used: request.min_score,
            threshold_lowered: false,
        })
    }

    /// Get statistics about the indexed content
    pub async fn get_statistics(&self) -> Result<StatisticsResponse> {
        let stats = self
            .vector_db
            .get_statistics()
            .await
            .context("Failed to get statistics")?;

        let language_breakdown = stats
            .language_breakdown
            .into_iter()
            .map(|(language, count)| LanguageStats {
                language,
                file_count: count,
                chunk_count: count,
            })
            .collect();

        Ok(StatisticsResponse {
            total_files: stats.total_points,
            total_chunks: stats.total_vectors,
            total_embeddings: stats.total_vectors,
            database_size_bytes: 0,
            language_breakdown,
        })
    }

    /// Clear all indexed data from the vector database
    pub async fn clear_index(&self) -> Result<ClearResponse> {
        match self.vector_db.clear().await {
            Ok(_) => {
                if let Err(e) = self
                    .vector_db
                    .initialize(self.embedding_provider.dimension())
                    .await
                {
                    Ok(ClearResponse {
                        success: false,
                        message: format!("Cleared but failed to reinitialize: {}", e),
                    })
                } else {
                    Ok(ClearResponse {
                        success: true,
                        message: "Successfully cleared all indexed data".to_string(),
                    })
                }
            }
            Err(e) => Ok(ClearResponse {
                success: false,
                message: format!("Failed to clear index: {}", e),
            }),
        }
    }

    /// Get the configuration used by this client
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the embedding dimension used by this client
    pub fn embedding_dimension(&self) -> usize {
        self.embedding_provider.dimension()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result(content: &str) -> SearchResult {
        SearchResult {
            chunk_id: Some("chunk-1".to_string()),
            file_path: "papers/test".to_string(),
            root_path: Some("papers".to_string()),
            content: content.to_string(),
            score: 0.2,
            match_type: None,
            combined_score: Some(0.03),
            vector_score: 0.8,
            keyword_score: Some(12.0),
            start_line: 0,
            end_line: 0,
            language: "PDF".to_string(),
            project: Some("paper-1".to_string()),
            page_numbers: None,
            heading_context: None,
        }
    }

    #[test]
    fn test_build_query_profile_classifies_exact_acronym() {
        let profile = RagClient::build_query_profile("SPDZ");
        assert_eq!(profile.intent, QueryIntent::Exact);
        assert_eq!(profile.exact_literal.as_deref(), Some("spdz"));
    }

    #[test]
    fn test_build_query_profile_classifies_keyword_and_semantic() {
        let keyword = RagClient::build_query_profile("share conversion");
        assert_eq!(keyword.intent, QueryIntent::Keyword);

        let semantic = RagClient::build_query_profile("boolean to arithmetic share conversion");
        assert_eq!(semantic.intent, QueryIntent::Semantic);
    }

    #[test]
    fn test_reference_detection_uses_heading_and_citation_density() {
        let mut by_heading = sample_result("Actively secure setup for SPDZ.");
        by_heading.heading_context = Some("References".to_string());
        assert!(RagClient::is_reference_chunk(&by_heading));

        let by_content = sample_result(
            "[9] A. Aly, E. Orsini, D. Rotaru, and T. Wood, 2019, https://eprint.iacr.org/2019/974.\n[10] D. Rotaru, N. P. Smart, T. Tanguy, 2020, pp. 227-249.",
        );
        assert!(RagClient::is_reference_chunk(&by_content));

        let body = sample_result("This protocol converts boolean shares into arithmetic shares.");
        assert!(!RagClient::is_reference_chunk(&body));
    }

    #[test]
    fn test_calibrate_result_filters_unverified_keyword_hits() {
        let profile = RagClient::build_query_profile("birthday");
        let result =
            RagClient::calibrate_result(&profile, sample_result("Secret sharing is useful."), 0.7);
        assert!(result.is_none());
    }

    #[test]
    fn test_calibrate_result_keeps_exact_reference_hit_with_penalty() {
        let profile = RagClient::build_query_profile("SPDZ");
        let mut result = sample_result("Actively secure setup for SPDZ.");
        result.heading_context = Some("References".to_string());

        let calibrated = RagClient::calibrate_result(&profile, result, 0.7).unwrap();
        assert_eq!(calibrated.match_type, Some(SearchMatchType::Exact));
        assert!((calibrated.score - (EXACT_MATCH_SCORE * REFERENCE_PENALTY)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calibrate_result_requires_strong_semantic_score() {
        let profile = RagClient::build_query_profile("boolean to arithmetic share conversion");
        let mut weak = sample_result("This chunk is conceptually related.");
        weak.vector_score = 0.74;
        assert!(RagClient::calibrate_result(&profile, weak, 0.7).is_none());

        let mut strong = sample_result("This chunk is conceptually related.");
        strong.vector_score = 0.82;
        let calibrated = RagClient::calibrate_result(&profile, strong, 0.7).unwrap();
        assert_eq!(calibrated.match_type, Some(SearchMatchType::Semantic));
        assert!((calibrated.score - 0.82).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calibrate_result_promotes_semantic_query_with_all_terms_present() {
        let profile = RagClient::build_query_profile("Replicated Secret Sharing");
        assert_eq!(profile.intent, QueryIntent::Semantic);

        let mut result = sample_result(
            "We utilize the 2out-of-3 Replicated Secret Sharing (RSS) construction in this protocol.",
        );
        result.vector_score = 0.41;

        let calibrated = RagClient::calibrate_result(&profile, result, 0.7).unwrap();
        assert_eq!(calibrated.match_type, Some(SearchMatchType::Keyword));
        assert!((calibrated.score - KEYWORD_MATCH_SCORE).abs() < f32::EPSILON);
    }

    #[test]
    fn test_chunk_contains_all_terms_supports_stemming() {
        assert!(RagClient::chunk_contains_all_terms(
            "This system searches documents efficiently.",
            &["search".to_string()]
        ));
    }

    #[test]
    fn test_diversify_results_caps_per_paper() {
        let mut results = Vec::new();
        for idx in 0..5 {
            let mut result = sample_result(&format!("paper one chunk {idx}"));
            result.project = Some("paper-1".to_string());
            result.file_path = format!("papers/paper-1/chunk-{idx}");
            result.score = 0.95 - (idx as f32 * 0.01);
            results.push(result);
        }

        let mut secondary = sample_result("paper two match");
        secondary.project = Some("paper-2".to_string());
        secondary.file_path = "papers/paper-2/chunk-1".to_string();
        secondary.score = 0.80;
        results.push(secondary);

        let diversified = RagClient::diversify_results(results, 10);
        let paper_one_count = diversified
            .iter()
            .filter(|result| result.project.as_deref() == Some("paper-1"))
            .count();
        assert_eq!(paper_one_count, GLOBAL_PAPER_CAP);
        assert!(
            diversified
                .iter()
                .any(|result| result.project.as_deref() == Some("paper-2"))
        );
    }
}
