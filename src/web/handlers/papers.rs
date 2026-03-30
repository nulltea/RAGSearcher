use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use crate::embedding::EmbeddingProvider;
use crate::indexer::{FileInfo, extract_pdf_to_markdown};
use crate::metadata::models::{PaperCreate, PaperListParams, PaperStatus};
use crate::types::ChunkMetadata;
use crate::vector_db::VectorDatabase;
use crate::web::errors::ApiError;
use crate::web::models::{PaperListResponse, PaperUploadResponse};
use crate::web::AppState;

pub async fn upload_paper(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<PaperUploadResponse>, ApiError> {
    let start = Instant::now();
    let paper_id = uuid::Uuid::new_v4().to_string();

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut text_content: Option<String> = None;
    let mut title: Option<String> = None;
    let mut authors: Vec<String> = Vec::new();
    let mut source: Option<String> = None;
    let mut published_date: Option<String> = None;
    let mut paper_type = "research_paper".to_string();
    let mut original_filename: Option<String> = None;

    // Parse multipart fields
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                original_filename = field.file_name().map(|s| s.to_string());
                file_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?
                        .to_vec(),
                );
            }
            "text" => {
                text_content = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read text: {}", e)))?,
                );
            }
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(e.to_string()))?,
                );
            }
            "authors" => {
                let val = field
                    .text()
                    .await
                    .map_err(|e| ApiError::BadRequest(e.to_string()))?;
                authors = val.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            "source" => {
                source = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(e.to_string()))?,
                );
            }
            "published_date" => {
                published_date = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(e.to_string()))?,
                );
            }
            "paper_type" => {
                paper_type = field
                    .text()
                    .await
                    .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            }
            _ => {}
        }
    }

    // Extract text content from file or use provided text
    let content = if let Some(bytes) = file_bytes {
        // Save PDF to disk
        let ext = original_filename
            .as_deref()
            .and_then(|f| f.rsplit('.').next())
            .unwrap_or("pdf");
        let file_path = state.upload_dir.join(format!("{}.{}", paper_id, ext));
        tokio::fs::write(&file_path, &bytes)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to save file: {}", e)))?;

        if ext.eq_ignore_ascii_case("pdf") {
            let path = file_path.clone();
            tokio::task::spawn_blocking(move || extract_pdf_to_markdown(&path))
                .await
                .map_err(|e| ApiError::Internal(format!("Task join error: {}", e)))?
                .map_err(|e| ApiError::Internal(format!("PDF extraction failed: {:#}", e)))?
        } else {
            // Plain text file
            String::from_utf8(bytes)
                .map_err(|_| ApiError::BadRequest("File is not valid UTF-8 text".to_string()))?
        }
    } else if let Some(text) = text_content {
        text
    } else {
        return Err(ApiError::BadRequest(
            "Either 'file' or 'text' field is required".to_string(),
        ));
    };

    let title = title.unwrap_or_else(|| {
        original_filename
            .as_deref()
            .unwrap_or("Untitled Paper")
            .to_string()
    });

    // Create paper record
    let create = PaperCreate {
        title: title.clone(),
        authors: authors.clone(),
        source: source.clone(),
        published_date: published_date.clone(),
        paper_type: paper_type.clone(),
        original_filename: original_filename.clone(),
    };

    let paper = state
        .metadata
        .create_paper(&paper_id, create)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create paper record: {:#}", e)))?;

    // Chunk the content
    let file_info = FileInfo {
        path: PathBuf::from(format!("papers/{}", paper_id)),
        relative_path: format!("papers/{}", paper_id),
        root_path: "papers".to_string(),
        project: Some(paper_id.clone()),
        extension: Some("md".to_string()),
        language: Some("Markdown".to_string()),
        content: content.clone(),
        hash: {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            format!("{:x}", hasher.finalize())
        },
    };

    let chunks = state.client.chunker.chunk_file(&file_info);

    if chunks.is_empty() {
        state
            .metadata
            .update_paper_status(&paper_id, PaperStatus::Active, 0)
            .await
            .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

        let mut paper = paper;
        paper.status = PaperStatus::Active;
        return Ok(Json(PaperUploadResponse {
            paper,
            chunk_count: 0,
            duration_ms: start.elapsed().as_millis() as u64,
        }));
    }

    // Generate embeddings
    let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
    let metadata: Vec<ChunkMetadata> = chunks.iter().map(|c| c.metadata.clone()).collect();
    let contents: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();

    let provider = state.client.embedding_provider.clone();
    let embeddings = tokio::task::spawn_blocking(move || provider.embed_batch(texts))
        .await
        .map_err(|e| ApiError::Internal(format!("Embedding task error: {}", e)))?
        .map_err(|e| ApiError::Internal(format!("Embedding generation failed: {:#}", e)))?;

    // Store in vector DB
    let chunk_count = state
        .client
        .vector_db
        .store_embeddings(embeddings, metadata, contents, "papers")
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to store embeddings: {:#}", e)))?;

    // Update paper status
    state
        .metadata
        .update_paper_status(&paper_id, PaperStatus::Active, chunk_count)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    let mut paper = paper;
    paper.status = PaperStatus::Active;
    paper.chunk_count = chunk_count;

    Ok(Json(PaperUploadResponse {
        paper,
        chunk_count,
        duration_ms: start.elapsed().as_millis() as u64,
    }))
}

pub async fn list_papers(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaperListParams>,
) -> Result<Json<PaperListResponse>, ApiError> {
    let limit = params.limit.unwrap_or(20);
    let offset = params.offset.unwrap_or(0);

    let (papers, total) = state
        .metadata
        .list_papers(params)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    Ok(Json(PaperListResponse {
        papers,
        total,
        limit,
        offset,
    }))
}

pub async fn get_paper(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let paper = state
        .metadata
        .get_paper(&id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    match paper {
        Some(p) => Ok(Json(p)),
        None => Err(ApiError::NotFound(format!("Paper '{}' not found", id))),
    }
}

pub async fn delete_paper(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Delete from metadata DB
    let deleted = state
        .metadata
        .delete_paper(&id)
        .await
        .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

    if !deleted {
        return Err(ApiError::NotFound(format!("Paper '{}' not found", id)));
    }

    // Delete embeddings from vector DB
    state
        .client
        .vector_db
        .delete_by_project(&id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to delete embeddings: {:#}", e)))?;

    // Remove uploaded file if exists
    let _ = tokio::fs::remove_file(state.upload_dir.join(format!("{}.pdf", id))).await;
    let _ = tokio::fs::remove_file(state.upload_dir.join(format!("{}.txt", id))).await;

    Ok(StatusCode::NO_CONTENT)
}
