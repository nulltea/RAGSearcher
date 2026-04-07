use crate::types::{QueryRequest, SearchResult};
use crate::vector_db::VectorDatabase;
use anyhow::Result;
use std::sync::Arc;

const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "in", "into", "is", "of",
    "on", "or", "that", "the", "to", "with", "within",
];

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum FusionMethod {
    RRF,
    Weighted,
    CombSum,
    MaxScore,
}

#[derive(Debug, Clone)]
pub struct HybridConfig {
    pub fusion_method: FusionMethod,
    pub max_candidates: usize,
    pub heading_match_boost: f32,
    pub figure_table_boost: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            fusion_method: FusionMethod::RRF,
            max_candidates: 100,
            heading_match_boost: 0.03,
            figure_table_boost: 0.04,
        }
    }
}

/// Local retrieval wrapper over the repo's dense+sparse backend path.
pub struct HybridSearchEngine {
    vector_db: Arc<dyn VectorDatabase>,
    config: HybridConfig,
}

impl HybridSearchEngine {
    pub fn new(vector_db: Arc<dyn VectorDatabase>) -> Self {
        Self {
            vector_db,
            config: HybridConfig::default(),
        }
    }

    pub fn config(&self) -> &HybridConfig {
        &self.config
    }

    pub fn fusion_method(&self) -> FusionMethod {
        self.config.fusion_method.clone()
    }

    pub async fn search(
        &self,
        query_vector: Vec<f32>,
        request: &QueryRequest,
    ) -> Result<Vec<SearchResult>> {
        let mut results = self
            .vector_db
            .search(
                query_vector,
                &request.query,
                request.limit.clamp(50, self.config.max_candidates.max(50)),
                0.0,
                request.project.clone(),
                request.path.clone(),
                request.hybrid,
            )
            .await?;

        self.apply_metadata_boosts(&request.query, &mut results);
        Ok(results)
    }

    fn apply_metadata_boosts(&self, query: &str, results: &mut [SearchResult]) {
        let query_terms = normalize_query_terms(query);
        if query_terms.is_empty() {
            return;
        }

        for result in results.iter_mut() {
            let mut boost = 0.0f32;

            if let Some(heading_context) = &result.heading_context {
                let heading_terms = normalize_query_terms(heading_context);
                let overlap = query_terms
                    .iter()
                    .filter(|term| heading_terms.iter().any(|heading| heading == *term))
                    .count();
                if overlap > 0 {
                    boost += self.config.heading_match_boost * overlap as f32;
                }
            }

            let query_lower = query.to_lowercase();
            if query_lower.contains("table")
                && result
                    .content
                    .lines()
                    .next()
                    .is_some_and(|line| line.to_lowercase().contains("table"))
            {
                boost += self.config.figure_table_boost;
            }

            if query_lower.contains("figure")
                && result
                    .content
                    .lines()
                    .next()
                    .is_some_and(|line| line.to_lowercase().contains("figure"))
            {
                boost += self.config.figure_table_boost;
            }

            if boost > 0.0 {
                result.combined_score = Some(
                    result
                        .combined_score
                        .unwrap_or(result.vector_score)
                        .mul_add(1.0, boost),
                );
            }
        }

        results.sort_by(|a, b| {
            b.combined_score
                .unwrap_or(b.vector_score)
                .partial_cmp(&a.combined_score.unwrap_or(a.vector_score))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

fn normalize_query_terms(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::trim)
        .filter(|term| term.len() >= 2)
        .map(|term| term.to_lowercase())
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_terms_drop_stop_words() {
        let terms = normalize_query_terms("the arithmetic share conversion");
        assert_eq!(terms, vec!["arithmetic", "share", "conversion"]);
    }
}
