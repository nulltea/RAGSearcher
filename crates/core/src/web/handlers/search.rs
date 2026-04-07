use axum::Json;
use axum::extract::State;
use std::sync::Arc;

use crate::types::QueryRequest;
use crate::web::AppState;
use crate::web::errors::ApiError;
use crate::web::models::{SearchRequest, SearchResponse, StatisticsResponse};

pub async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, ApiError> {
    let query_req = QueryRequest {
        query: req.query,
        path: None,
        project: req.paper_id,
        limit: req.limit,
        min_score: req.min_score,
        hybrid: req.hybrid,
    };

    let response = state
        .client
        .query(query_req)
        .await
        .map_err(|e| ApiError::Internal(format!("Search failed: {:#}", e)))?;

    Ok(Json(SearchResponse {
        results: response.results,
        duration_ms: response.duration_ms,
        threshold_used: response.threshold_used,
        threshold_lowered: response.threshold_lowered,
    }))
}

pub async fn statistics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatisticsResponse>, ApiError> {
    let stats = state
        .client
        .vector_db
        .get_statistics()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get statistics: {:#}", e)))?;

    Ok(Json(StatisticsResponse {
        total_chunks: stats.total_points,
        total_vectors: stats.total_vectors,
        languages: stats.language_breakdown,
    }))
}
