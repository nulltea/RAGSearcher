pub mod errors;
pub mod handlers;
pub mod models;

use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::client::RagClient;
use crate::metadata::MetadataStore;

pub struct AppState {
    pub client: Arc<RagClient>,
    pub metadata: Arc<MetadataStore>,
    pub upload_dir: PathBuf,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(handlers::health::health))
        .route("/api/papers", post(handlers::papers::upload_paper))
        .route("/api/papers", get(handlers::papers::list_papers))
        .route("/api/papers/{id}", get(handlers::papers::get_paper))
        .route("/api/papers/{id}", delete(handlers::papers::delete_paper))
        .route("/api/search", post(handlers::search::search))
        .route("/api/statistics", get(handlers::search::statistics))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn start_server(
    host: &str,
    port: u16,
    client: Arc<RagClient>,
    metadata: Arc<MetadataStore>,
    upload_dir: PathBuf,
) -> anyhow::Result<()> {
    // Ensure upload directory exists
    tokio::fs::create_dir_all(&upload_dir).await?;

    let state = Arc::new(AppState {
        client,
        metadata,
        upload_dir,
    });

    let app = create_router(state);
    let addr = format!("{}:{}", host, port);

    tracing::info!("Starting web server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
