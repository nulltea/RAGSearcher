# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Key Technology Stack:**
- Rust 2024 edition with async/await (Tokio)
- MCP protocol via `rmcp` crate (v0.8) with macros
- FastEmbed (all-MiniLM-L6-v2 model, 384 dimensions)
- LanceDB vector database (default, embedded) or Qdrant (optional, external server)
- Tantivy BM25 keyword search with Reciprocal Rank Fusion (RRF) for hybrid search
- PDF extraction and text chunking for paper processing
- Pattern and algorithm extraction via Claude CLI
- Tauri 2 desktop app with Next.js frontend

## Essential Commands

### Building and Running
```bash
# Build debug version
cargo build

# Build optimized release version
cargo build --release

# Quick compile check without building
cargo check

# Run the MCP server over stdio
cargo run

# Run the web server for paper management
cargo run -- web --port 3001

# Run the Tauri desktop app
cd crates/tauri-app && cargo tauri dev
```

### Testing
```bash
# Run all unit tests
cargo test --lib

# Run tests for specific module
cargo test --lib types::tests
cargo test --lib chunker::tests
cargo test --lib bm25_search::tests

# Run with verbose output
cargo test --lib -- --nocapture

# Run tests with debug logging
RUST_LOG=debug cargo test --lib -- --nocapture
```

### Code Quality
```bash
# Format code (required before commits)
cargo fmt

# Check lints with clippy
cargo clippy

# Auto-fix clippy suggestions
cargo clippy --fix
```

### Debugging
```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with trace-level logging
RUST_LOG=trace cargo run
```

## Architecture

### Project Structure
- `crates/core` — Library + CLI binary (MCP server, web server, embedding, search)
- `crates/tauri-app` — Tauri 2 desktop app wrapper
- `frontend/` — Next.js static export (paper library UI)

### Core Design Principles

1. **Papers-Only Focus**: This project is a paper library with semantic search. It supports uploading PDFs, extracting text, chunking, embedding, and searching papers.

2. **Modular Trait-Based Design**: Each major component is defined by a trait (EmbeddingProvider, VectorDatabase) with concrete implementations, enabling easy swapping of backends.

3. **MCP Protocol Integration**: Uses `rmcp` macros (`#[tool]`, `#[prompt]`, `#[tool_router]`, `#[prompt_router]`) to define 4 MCP tools and 4 prompts. The server communicates over stdio following MCP spec.

4. **Hybrid Search**: Combines vector similarity (semantic understanding) with BM25 keyword matching using Reciprocal Rank Fusion (RRF) for optimal search results. Tantivy provides full-text search capabilities with IDF scoring.

5. **Web Server**: Axum HTTP server for paper upload, search, pattern extraction, and algorithm extraction. Used by both the Tauri desktop app and standalone deployment.

### Critical Implementation Details

**1. MCP Server Pattern (mcp_server.rs)**
- Uses `#[tool_router]` and `#[prompt_router]` macros to generate routers
- Tools: `search`, `search_papers`, `get_statistics`
- Prompts: `search`, `papers`
- Tools return `Result<String, String>` (JSON-serialized responses)
- Server implements `ServerHandler` trait with `#[tool_handler]` and `#[prompt_handler]`

**2. Paper Upload Pipeline (web/handlers/papers.rs)**
- PDF upload → text extraction → chunking → embedding → vector storage
- Metadata stored in SQLite via `MetadataStore`
- Pattern and algorithm extraction via Claude CLI (3-pass pipeline)

**3. Hybrid Search (bm25_search.rs)**
- Combines vector similarity with BM25 keyword matching
- Uses Tantivy inverted index for full-text search with IDF scoring
- Reciprocal Rank Fusion (RRF) merges both rankings using 1/(k+rank) formula (k=60)
- Both indexes queried in parallel for fast results

## Development Guidelines

### Source File Size Constraint
**CRITICAL**: All source files must stay under 600 lines. This is enforced in the project. If adding features, break large files into submodules.

### Error Handling
- Use `anyhow::Result` for functions that can fail
- Add context with `.context("Descriptive error message")`
- Return formatted errors in MCP tools: `.map_err(|e| format!("{:#}", e))`
- Use alternate display (`{:#}`) to show full error chain

### Testing Requirements
- Add unit tests for all new functionality
- Tests in same file using `#[cfg(test)]` module
- Test serialization/deserialization for all request/response types

### Async Patterns
- Use `tokio::spawn_blocking` for CPU-intensive or blocking I/O operations
- Prefer `Arc<T>` over `Arc<RwLock<T>>` when possible (immutable shared state)
- Batch operations to reduce async overhead (8 chunks per embedding batch)

### MCP Tool Development
When adding new tools:
1. Define request/response types in `types.rs` with `#[derive(JsonSchema)]`
2. Add tool method in `#[tool_router]` impl block with `#[tool]` attribute
3. Add corresponding prompt in `#[prompt_router]` impl block with `#[prompt]` attribute
4. Return `Result<String, String>` from tools (serialize response to JSON)
5. Return `Vec<PromptMessage>` from prompts (user messages for slash commands)
