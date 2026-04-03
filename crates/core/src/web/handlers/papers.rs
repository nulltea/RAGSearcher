use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::chunker::{ChunkInput, PdfChunkMeta, extract_pdf};
use crate::embedding::{EmbeddingProvider, format_retrieval_document};
use crate::metadata::models::{PaperCreate, PaperListParams, PaperStatus};
use crate::types::ChunkMetadata;
use crate::vector_db::VectorDatabase;
use crate::web::AppState;
use crate::web::errors::ApiError;
use crate::web::models::{PaperListResponse, PaperUploadResponse};

pub async fn upload_paper(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<PaperUploadResponse>, ApiError> {
    let start = Instant::now();
    let paper_id = uuid::Uuid::new_v4().to_string();

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut text_content: Option<String> = None;
    let mut url: Option<String> = None;
    let mut title: Option<String> = None;
    let mut authors: Vec<String> = Vec::new();
    let mut source: Option<String> = None;
    let mut published_date: Option<String> = None;
    let mut paper_type = "research_paper".to_string();
    let mut original_filename: Option<String> = None;

    // Parse multipart fields
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                original_filename = field.file_name().map(|s| s.to_string());
                let mut data = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(|e| {
                    ApiError::BadRequest(format!("Failed to read file chunk: {}", e))
                })? {
                    data.extend_from_slice(&chunk);
                }
                file_bytes = Some(data);
            }
            "text" => {
                text_content =
                    Some(field.text().await.map_err(|e| {
                        ApiError::BadRequest(format!("Failed to read text: {}", e))
                    })?);
            }
            "url" => {
                url = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read URL: {}", e)))?,
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
                authors = val
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
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

    // Download PDF from URL if provided
    if let Some(ref pdf_url) = url {
        if file_bytes.is_none() && text_content.is_none() {
            let response = reqwest::get(pdf_url)
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to download URL: {}", e)))?;

            if !response.status().is_success() {
                return Err(ApiError::BadRequest(format!(
                    "URL returned status {}",
                    response.status()
                )));
            }

            let bytes = response
                .bytes()
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to read response body: {}", e)))?
                .to_vec();

            // Derive filename from URL
            let url_filename = pdf_url
                .rsplit('/')
                .next()
                .unwrap_or("download.pdf")
                .split('?')
                .next()
                .unwrap_or("download.pdf");
            original_filename = Some(url_filename.to_string());

            // Use URL as source if not explicitly provided
            if source.is_none() {
                source = Some(pdf_url.clone());
            }

            file_bytes = Some(bytes);
        }
    }

    // Extract text content from file or use provided text
    let mut stored_file_path: Option<String> = None;
    let mut pdf_title: Option<String> = None;
    let mut _saved_pdf_path: Option<std::path::PathBuf> = None;
    let file_ext = original_filename
        .as_deref()
        .and_then(|f| f.rsplit('.').next())
        .unwrap_or("pdf")
        .to_string();

    let content = if let Some(bytes) = file_bytes {
        // Save file to disk
        let ext = file_ext.as_str();
        let file_path = state.upload_dir.join(format!("{}.{}", paper_id, ext));
        tokio::fs::write(&file_path, &bytes)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to save file: {}", e)))?;

        stored_file_path = file_path
            .canonicalize()
            .ok()
            .map(|p| p.to_string_lossy().to_string());
        _saved_pdf_path = Some(file_path.clone());

        if ext.eq_ignore_ascii_case("pdf") {
            let path = file_path.clone();
            let extraction = tokio::task::spawn_blocking(move || extract_pdf(&path))
                .await
                .map_err(|e| ApiError::Internal(format!("Task join error: {}", e)))?
                .map_err(|e| ApiError::Internal(format!("PDF extraction failed: {:#}", e)))?;
            pdf_title = extraction.title;
            extraction.text
        } else {
            String::from_utf8(bytes)
                .map_err(|_| ApiError::BadRequest("File is not valid UTF-8 text".to_string()))?
        }
    } else if let Some(text) = text_content {
        text
    } else {
        return Err(ApiError::BadRequest(
            "Either 'file', 'text', or 'url' field is required".to_string(),
        ));
    };

    // Title priority: user-provided > PDF metadata/heuristic > filename without extension > "Untitled Paper"
    let title = title.or(pdf_title).unwrap_or_else(|| {
        original_filename
            .as_deref()
            .map(|f| {
                f.rsplit('.')
                    .nth(1)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| f.to_string())
            })
            .unwrap_or_else(|| "Untitled Paper".to_string())
    });

    // Create paper record
    let create = PaperCreate {
        title: title.clone(),
        authors: authors.clone(),
        source: source.clone(),
        published_date: published_date.clone(),
        paper_type: paper_type.clone(),
        original_filename: original_filename.clone(),
        file_path: stored_file_path,
    };

    let paper = state
        .metadata
        .create_paper(&paper_id, create)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create paper record: {:#}", e)))?;

    // Save extracted text for later pattern extraction
    let text_path = state.upload_dir.join(format!("{}.txt", paper_id));
    tokio::fs::write(&text_path, &content)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to save text content: {}", e)))?;

    // Chunk the content
    let content_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let chunks = if file_ext.eq_ignore_ascii_case("pdf") {
        let pdf_meta = PdfChunkMeta {
            relative_path: format!("papers/{}", paper_id),
            root_path: "papers".to_string(),
            project: Some(paper_id.clone()),
            hash: content_hash.clone(),
        };
        let chunker = state.client.pdf_chunker.clone();
        let text = content.clone();
        tokio::task::spawn_blocking(move || chunker.chunk_text(&text, &pdf_meta))
            .await
            .map_err(|e| ApiError::Internal(format!("Chunking task error: {}", e)))?
            .map_err(|e| ApiError::Internal(format!("PDF chunking failed: {:#}", e)))?
    } else {
        let chunk_input = ChunkInput {
            relative_path: format!("papers/{}", paper_id),
            root_path: "papers".to_string(),
            project: Some(paper_id.clone()),
            extension: Some("md".to_string()),
            language: Some("Markdown".to_string()),
            content: content.clone(),
            hash: content_hash,
        };
        state.client.chunker.chunk_file(&chunk_input)
    };

    if chunks.is_empty() {
        state
            .metadata
            .update_paper_status(&paper_id, PaperStatus::Active, 0)
            .await
            .map_err(|e| ApiError::Internal(format!("{:#}", e)))?;

        let mut paper = paper;
        paper.status = PaperStatus::Active;
        paper.pattern_count = 0;
        paper.algorithm_count = 0;
        return Ok(Json(PaperUploadResponse {
            paper,
            chunk_count: 0,
            duration_ms: start.elapsed().as_millis() as u64,
        }));
    }

    // Generate embeddings
    let texts: Vec<String> = chunks
        .iter()
        .map(|c| format_retrieval_document(Some(&title), &c.content))
        .collect();
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
