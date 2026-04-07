use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use crate::embedding::{EmbeddingProvider, format_retrieval_document};
use crate::metadata::models::{AlgorithmIORow, AlgorithmStepRow, PatternStatus};
use crate::types::ChunkMetadata;
use crate::web::AppState;
use crate::web::errors::ApiError;
use crate::web::models::{
    AlgorithmExtractResponse, AlgorithmListResponse, AlgorithmReviewResponse, PatternReviewRequest,
};

/// POST /api/papers/{id}/extract-algorithms
pub async fn extract_algorithms(
    State(state): State<Arc<AppState>>,
    Path(paper_id): Path<String>,
) -> Result<Json<AlgorithmExtractResponse>, ApiError> {
    let start = Instant::now();

    let paper = state
        .metadata
        .get_paper(&paper_id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Paper '{}' not found", paper_id)))?;

    let text_path = state.upload_dir.join(format!("{}.txt", paper_id));
    if !text_path.exists() {
        return Err(ApiError::Internal(format!(
            "Paper text not found at {}. Was the paper uploaded correctly?",
            text_path.display(),
        )));
    }
    let text_path_str = text_path.to_string_lossy().to_string();

    let extractor = state
        .algorithm_extractor
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Algorithm extractor not configured".to_string()))?;

    // TODO: could load existing evidence from prior pattern extraction
    let result = extractor
        .extract_algorithms(&text_path_str)
        .await
        .map_err(|e| ApiError::Internal(format!("Algorithm extraction failed: {:#}", e)))?;

    // Delete any existing algorithms for this paper (re-extraction)
    state
        .metadata
        .delete_algorithms_by_paper(&paper_id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    // Save algorithms to SQLite
    let mut algorithms = Vec::new();
    for a in &result.algorithms {
        let steps: Vec<AlgorithmStepRow> = a
            .steps
            .iter()
            .map(|s| AlgorithmStepRow {
                number: s.number,
                action: s.action.clone(),
                details: s.details.clone(),
                math: s.math.clone(),
            })
            .collect();

        let inputs: Vec<AlgorithmIORow> = a
            .inputs
            .iter()
            .map(|io| AlgorithmIORow {
                name: io.name.clone(),
                io_type: io.io_type.clone(),
                description: io.description.clone(),
            })
            .collect();

        let outputs: Vec<AlgorithmIORow> = a
            .outputs
            .iter()
            .map(|io| AlgorithmIORow {
                name: io.name.clone(),
                io_type: io.io_type.clone(),
                description: io.description.clone(),
            })
            .collect();

        let algorithm = state
            .metadata
            .create_algorithm(
                &paper_id,
                &a.name,
                Some(&a.description),
                &steps,
                &inputs,
                &outputs,
                &a.preconditions,
                a.complexity.as_deref(),
                a.mathematical_notation.as_deref(),
                a.pseudocode.as_deref(),
                &a.tags,
                &a.evidence_ids,
                &a.confidence,
            )
            .await
            .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;
        algorithms.push(algorithm);
    }

    // Update paper status to ready_for_review
    state
        .metadata
        .update_paper_status(
            &paper_id,
            crate::metadata::models::PaperStatus::ReadyForReview,
            paper.chunk_count,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    let verification_status = result
        .verification
        .as_ref()
        .map(|v| v.verification_status.clone());

    Ok(Json(AlgorithmExtractResponse {
        paper_id,
        algorithms,
        evidence_count: 0,
        verification_status,
        duration_ms: start.elapsed().as_millis() as u64,
    }))
}

#[derive(Debug, Deserialize)]
pub struct AlgorithmListParams {
    pub status: Option<String>,
}

/// GET /api/papers/{id}/algorithms
pub async fn list_algorithms(
    State(state): State<Arc<AppState>>,
    Path(paper_id): Path<String>,
    Query(params): Query<AlgorithmListParams>,
) -> Result<Json<AlgorithmListResponse>, ApiError> {
    let algorithms = state
        .metadata
        .list_algorithms(&paper_id, params.status.as_deref())
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    Ok(Json(AlgorithmListResponse { algorithms }))
}

/// POST /api/papers/{id}/algorithms/review — submit review decisions
pub async fn submit_algorithm_review(
    State(state): State<Arc<AppState>>,
    Path(paper_id): Path<String>,
    Json(request): Json<PatternReviewRequest>,
) -> Result<Json<AlgorithmReviewResponse>, ApiError> {
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
            .update_algorithm_status(&decision.pattern_id, status)
            .await
            .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;
    }

    // Embed approved algorithms into LanceDB
    if approved_count > 0 {
        let approved = state
            .metadata
            .list_algorithms(&paper_id, Some("approved"))
            .await
            .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

        let texts: Vec<String> = approved
            .iter()
            .map(|a| {
                let mut parts = vec![a.name.clone()];
                if let Some(ref d) = a.description {
                    parts.push(d.clone());
                }
                for step in &a.steps {
                    parts.push(format!("{}. {}", step.number, step.action));
                }
                parts.join(" | ")
            })
            .collect();

        let metadata: Vec<ChunkMetadata> = approved
            .iter()
            .map(|a| ChunkMetadata {
                chunk_id: None,
                file_path: format!("algorithms/{}", a.paper_id),
                root_path: Some("algorithms".to_string()),
                start_line: 0,
                end_line: 0,
                language: Some("Algorithm".to_string()),
                extension: Some("algorithm".to_string()),
                file_hash: a.id.clone(),
                indexed_at: chrono::Utc::now().timestamp(),
                project: Some(format!("algorithm:{}", a.paper_id)),
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
            .store_embeddings(embeddings, metadata, contents, "algorithms")
            .await
            .map_err(|e| {
                ApiError::Internal(format!("Failed to store algorithm embeddings: {:#}", e))
            })?;
    }

    // Check if all items (patterns + algorithms) are reviewed; if so, mark paper active
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
                .update_paper_status(
                    &paper_id,
                    crate::metadata::models::PaperStatus::Active,
                    p.chunk_count,
                )
                .await
                .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;
        }
    }

    let algorithms = state
        .metadata
        .list_algorithms(&paper_id, None)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    Ok(Json(AlgorithmReviewResponse {
        approved_count,
        rejected_count,
        algorithms,
    }))
}

/// DELETE /api/papers/{id}/algorithms
pub async fn delete_algorithms(
    State(state): State<Arc<AppState>>,
    Path(paper_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .metadata
        .delete_algorithms_by_paper(&paper_id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    let project = format!("algorithm:{}", paper_id);
    state
        .client
        .vector_db
        .delete_by_project(&project)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}
