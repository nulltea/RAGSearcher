//! MCP Server integration example
//!
//! Demonstrates how to:
//! 1. Create a RagClient with custom configuration
//! 2. Wrap it in an MCP server
//! 3. Start the MCP server over stdio
//!
//! Run with: cargo run --example mcp_client
//! (The server will wait for MCP protocol messages on stdin)

use project_rag::{Config, RagClient, mcp_server::RagMcpServer};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== RAG Searcher - MCP Server Example ===");
    println!();

    // Option 1: Default configuration
    println!("Creating MCP server with default configuration...");
    let _simple_server = RagMcpServer::new().await?;
    println!("  Server created successfully!");
    println!();

    // Option 2: Custom client
    println!("Creating MCP server with custom client...");
    let config = Config::new()?;
    println!("  Vector DB backend: {}", config.vector_db.backend);
    println!("  Min search score: {}", config.search.min_score);

    let client = RagClient::with_config(config).await?;
    let server = RagMcpServer::with_client(Arc::new(client))?;
    println!("  MCP server created!");
    println!();

    println!("Available MCP Tools:");
    println!("  1. search             - Semantic search across paper content");
    println!("  2. search_papers      - Search papers by title, authors, status");
    println!("  3. search_algorithms  - Search algorithms across papers by keyword/tags");
    println!("  4. get_statistics     - Get index statistics");
    println!();

    println!("Available Slash Commands:");
    println!("  /rag-searcher:search  /rag-searcher:papers  /rag-searcher:algorithms");
    println!();

    // Direct client usage
    let client = server.client();
    let stats = client.get_statistics().await?;
    println!("Index statistics:");
    println!("  Total chunks: {}", stats.total_chunks);
    for lang in stats.language_breakdown.iter().take(5) {
        println!("    - {}: {} files", lang.language, lang.file_count);
    }

    println!();
    println!("To run: cargo run --release");
    println!("Or:     ./target/release/rag-searcher");

    Ok(())
}
