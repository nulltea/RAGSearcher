#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

struct BackendPort {
    port: u16,
}

#[tauri::command]
fn get_backend_port(state: tauri::State<'_, BackendPort>) -> u16 {
    state.port
}

fn main() {
    // Ensure fastembed uses the platform cache directory instead of CWD/.fastembed_cache.
    // This must be set before any fastembed initialization.
    let cache_dir = project_rag::paths::PlatformPaths::cache_dir().join("fastembed");
    // SAFETY: Called at the very start of main() before any threads are spawned.
    unsafe { std::env::set_var("FASTEMBED_CACHE_DIR", &cache_dir) };

    tauri::Builder::default()
        .setup(|app| {
            let (tx, rx) = std::sync::mpsc::channel();

            // Spawn a dedicated thread with its own tokio runtime for the backend.
            // This avoids conflicts with Tauri's async runtime.
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime");

                rt.block_on(async {
                    tracing_subscriber::fmt::init();

                    let data_dir = dirs::data_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join("project-rag");
                    let db_path = data_dir.join("papers.db");
                    let upload_dir = data_dir.join("uploads");

                    let client = Arc::new(
                        project_rag::RagClient::new()
                            .await
                            .expect("Failed to initialize RAG client"),
                    );

                    let metadata = Arc::new(
                        project_rag::metadata::MetadataStore::new(&db_path)
                            .expect("Failed to initialize metadata store"),
                    );

                    let extractor = Arc::new(
                        project_rag::extraction::PatternExtractor::new(),
                    );
                    let algorithm_extractor = Arc::new(
                        project_rag::extraction::AlgorithmExtractor::new(),
                    );

                    let (port, _handle) = project_rag::web::start_server_background(
                        client,
                        metadata,
                        upload_dir,
                        Some(extractor),
                        Some(algorithm_extractor),
                    )
                    .await
                    .expect("Failed to start backend server");

                    tracing::info!("Backend started on port {}", port);
                    tx.send(port).expect("Failed to send port");

                    // Keep the runtime alive for the lifetime of the app
                    _handle.await.ok();
                });
            });

            let port = rx.recv().expect("Failed to receive backend port");
            app.manage(BackendPort { port });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_backend_port])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
