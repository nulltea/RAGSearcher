pub mod errors;
pub mod handlers;
pub mod models;

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use axum::http::Method;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::client::RagClient;
use crate::extraction::{AlgorithmExtractor, PatternExtractor};
use crate::metadata::MetadataStore;

pub struct AppState {
    pub client: Arc<RagClient>,
    pub metadata: Arc<MetadataStore>,
    pub upload_dir: PathBuf,
    pub extractor: Option<Arc<PatternExtractor>>,
    pub algorithm_extractor: Option<Arc<AlgorithmExtractor>>,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/api/papers", get(handlers::papers::list_papers)
            .post(handlers::papers::upload_paper))
        .route("/api/papers/{id}", get(handlers::papers::get_paper)
            .delete(handlers::papers::delete_paper))
        .route("/api/papers/{id}/extract", post(handlers::patterns::extract_patterns))
        .route("/api/papers/{id}/patterns", get(handlers::patterns::list_patterns)
            .delete(handlers::patterns::delete_patterns))
        .route("/api/papers/{id}/patterns/review", post(handlers::patterns::submit_review))
        .route("/api/papers/{id}/extract-algorithms", post(handlers::algorithms::extract_algorithms))
        .route("/api/papers/{id}/algorithms", get(handlers::algorithms::list_algorithms)
            .delete(handlers::algorithms::delete_algorithms))
        .route("/api/papers/{id}/algorithms/review", post(handlers::algorithms::submit_algorithm_review))
        .route("/api/search", post(handlers::search::search))
        .route("/api/statistics", get(handlers::search::statistics))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(DefaultBodyLimit::disable())
        .with_state(state)
}

pub async fn start_server(
    host: &str,
    port: u16,
    client: Arc<RagClient>,
    metadata: Arc<MetadataStore>,
    upload_dir: PathBuf,
    extractor: Option<Arc<PatternExtractor>>,
    algorithm_extractor: Option<Arc<AlgorithmExtractor>>,
) -> anyhow::Result<()> {
    // Ensure upload directory exists
    tokio::fs::create_dir_all(&upload_dir).await?;

    let state = Arc::new(AppState {
        client,
        metadata,
        upload_dir,
        extractor,
        algorithm_extractor,
    });

    let app = create_router(state);
    let addr = format!("{}:{}", host, port);

    tracing::info!("Starting web server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Start the backend on a random port, returning the port and a join handle.
/// Used by the Tauri desktop app to embed the backend in-process.
pub async fn start_server_background(
    client: Arc<RagClient>,
    metadata: Arc<MetadataStore>,
    upload_dir: PathBuf,
    extractor: Option<Arc<PatternExtractor>>,
    algorithm_extractor: Option<Arc<AlgorithmExtractor>>,
) -> anyhow::Result<(u16, tokio::task::JoinHandle<()>)> {
    tokio::fs::create_dir_all(&upload_dir).await?;

    let state = Arc::new(AppState {
        client,
        metadata,
        upload_dir,
        extractor,
        algorithm_extractor,
    });

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    tracing::info!("Starting background web server on 127.0.0.1:{}", port);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    Ok((port, handle))
}
