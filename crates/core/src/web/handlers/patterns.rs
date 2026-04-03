use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use crate::embedding::{EmbeddingProvider, format_retrieval_document};
use crate::metadata::models::{PaperStatus, PatternStatus};
use crate::types::ChunkMetadata;
use crate::vector_db::VectorDatabase;
use crate::web::AppState;
use crate::web::errors::ApiError;
use crate::web::models::{
    ExtractResponse, PatternListResponse, PatternReviewRequest, PatternReviewResponse,
};

/// POST /api/papers/{id}/extract — trigger pattern extraction via Claude CLI
pub async fn extract_patterns(
    State(state): State<Arc<AppState>>,
    Path(paper_id): Path<String>,
) -> Result<Json<ExtractResponse>, ApiError> {
    let start = Instant::now();

    // Verify paper exists
    let paper = state
        .metadata
        .get_paper(&paper_id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Paper '{}' not found", paper_id)))?;

    // Verify text file exists
    let text_path = state.upload_dir.join(format!("{}.txt", paper_id));
    if !text_path.exists() {
        return Err(ApiError::Internal(format!(
            "Paper text not found at {}. Was the paper uploaded correctly?",
            text_path.display(),
        )));
    }
    let text_path_str = text_path.to_string_lossy().to_string();

    let extractor = state
        .extractor
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Pattern extractor not configured".to_string()))?;

    // Run 3-pass extraction
    let result = extractor
        .extract_patterns(&text_path_str)
        .await
        .map_err(|e| ApiError::Internal(format!("Extraction failed: {:#}", e)))?;

    // Delete any existing patterns for this paper (re-extraction)
    state
        .metadata
        .delete_patterns_by_paper(&paper_id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    // Save patterns to SQLite
    let mut patterns = Vec::new();
    for p in &result.patterns {
        let pattern = state
            .metadata
            .create_pattern(
                &paper_id,
                &p.name,
                p.claim.as_deref(),
                p.evidence.as_deref(),
                p.context.as_deref(),
                &p.tags,
                &p.confidence,
            )
            .await
            .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;
        patterns.push(pattern);
    }

    // Update paper status to ready_for_review
    state
        .metadata
        .update_paper_status(&paper_id, PaperStatus::ReadyForReview, paper.chunk_count)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    let verification_status = result
        .verification
        .as_ref()
        .map(|v| v.verification_status.clone());

    Ok(Json(ExtractResponse {
        paper_id,
        patterns,
        evidence_count: result.evidence.evidence_items.len(),
        verification_status,
        duration_ms: start.elapsed().as_millis() as u64,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PatternListParams {
    pub status: Option<String>,
}

/// GET /api/papers/{id}/patterns — list patterns for a paper
pub async fn list_patterns(
    State(state): State<Arc<AppState>>,
    Path(paper_id): Path<String>,
    Query(params): Query<PatternListParams>,
) -> Result<Json<PatternListResponse>, ApiError> {
    let patterns = state
        .metadata
        .list_patterns(&paper_id, params.status.as_deref())
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    Ok(Json(PatternListResponse { patterns }))
}

/// POST /api/papers/{id}/patterns/review — submit review decisions
pub async fn submit_review(
    State(state): State<Arc<AppState>>,
    Path(paper_id): Path<String>,
    Json(request): Json<PatternReviewRequest>,
) -> Result<Json<PatternReviewResponse>, ApiError> {
    if request.decisions.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one decision is required".to_string(),
        ));
    }

    let mut approved_count = 0usize;
    let mut rejected_count = 0usize;

    for decision in &request.decisions {
        let status = if decision.approved {
            approved_count += 1;
            PatternStatus::Approved
        } else {
            rejected_count += 1;
            PatternStatus::Rejected
        };

        state
            .metadata
            .update_pattern_status(&decision.pattern_id, status)
            .await
            .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;
    }

    // Embed approved patterns into LanceDB
    if approved_count > 0 {
        let approved = state
            .metadata
            .list_patterns(&paper_id, Some("approved"))
            .await
            .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

        let texts: Vec<String> = approved
            .iter()
            .map(|p| {
                let mut parts = vec![p.name.clone()];
                if let Some(ref c) = p.claim {
                    parts.push(c.clone());
                }
                if let Some(ref e) = p.evidence {
                    parts.push(e.clone());
                }
                if let Some(ref ctx) = p.context {
                    parts.push(ctx.clone());
                }
                parts.join(" | ")
            })
            .collect();

        let metadata: Vec<ChunkMetadata> = approved
            .iter()
            .map(|p| ChunkMetadata {
                chunk_id: None,
                file_path: format!("patterns/{}", p.paper_id),
                root_path: Some("patterns".to_string()),
                start_line: 0,
                end_line: 0,
                language: Some("Pattern".to_string()),
                extension: Some("pattern".to_string()),
                file_hash: p.id.clone(),
                indexed_at: chrono::Utc::now().timestamp(),
                project: Some(format!("pattern:{}", p.paper_id)),
                page_numbers: None,
                heading_context: None,
                element_types: None,
            })
            .collect();

        let contents: Vec<String> = texts.clone();
        let texts: Vec<String> = texts
            .into_iter()
            .map(|text| format_retrieval_document(None, &text))
            .collect();

        let provider = state.client.embedding_provider.clone();
        let embeddings = tokio::task::spawn_blocking(move || provider.embed_batch(texts))
            .await
            .map_err(|e| ApiError::Internal(format!("Embedding task error: {}", e)))?
            .map_err(|e| ApiError::Internal(format!("Embedding failed: {:#}", e)))?;

        state
            .client
            .vector_db
            .store_embeddings(embeddings, metadata, contents, "patterns")
            .await
            .map_err(|e| {
                ApiError::Internal(format!("Failed to store pattern embeddings: {:#}", e))
            })?;
    }

    // Check if all items (patterns + algorithms) are reviewed; if so, update paper to active
    let (pending_patterns, _, _) = state
        .metadata
        .count_patterns_by_status(&paper_id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;
    let (pending_algorithms, _, _) = state
        .metadata
        .count_algorithms_by_status(&paper_id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    if pending_patterns == 0 && pending_algorithms == 0 {
        let paper = state
            .metadata
            .get_paper(&paper_id)
            .await
            .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;
        if let Some(p) = paper {
            state
                .metadata
                .update_paper_status(&paper_id, PaperStatus::Active, p.chunk_count)
                .await
                .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;
        }
    }

    let patterns = state
        .metadata
        .list_patterns(&paper_id, None)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    Ok(Json(PatternReviewResponse {
        approved_count,
        rejected_count,
        patterns,
    }))
}

/// DELETE /api/papers/{id}/patterns — delete all patterns for a paper
pub async fn delete_patterns(
    State(state): State<Arc<AppState>>,
    Path(paper_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .metadata
        .delete_patterns_by_paper(&paper_id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    // Also delete pattern embeddings from vector DB
    let project = format!("pattern:{}", paper_id);
    state
        .client
        .vector_db
        .delete_by_project(&project)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}
