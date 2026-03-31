# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

**Key Technology Stack:**
- Rust 2024 edition with async/await (Tokio)
- MCP protocol via `rmcp` crate (v0.8) with macros
- FastEmbed (all-MiniLM-L6-v2 model, 384 dimensions)
- LanceDB vector database (default, embedded) or Qdrant (optional, external server)
- Tantivy BM25 keyword search with Reciprocal Rank Fusion (RRF) for hybrid search
- Tree-sitter AST-based chunking for 12 languages
- Persistent hash cache for incremental updates across restarts
- File walking with .gitignore support via `ignore` crate

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
# Or directly:
./target/release/project-rag
```

### Testing
```bash
# Run all unit tests (386 tests across all modules)
cargo test --lib

# Run tests for specific module
cargo test --lib types::tests
cargo test --lib chunker::tests
cargo test --lib cache::tests
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

### Vector Database Management

**Default (LanceDB - Embedded)**
```bash
# No external setup required - LanceDB is embedded
# Data stored in ./.lancedb directory by default
```

## Architecture

### Core Design Principles

1. **Modular Trait-Based Design**: Each major component is defined by a trait (EmbeddingProvider, VectorDatabase) with concrete implementations, enabling easy swapping of backends.

2. **MCP Protocol Integration**: Uses `rmcp` macros (`#[tool]`, `#[prompt]`, `#[tool_router]`, `#[prompt_router]`) to define 9 MCP tools and 9 slash commands. The server communicates over stdio following MCP spec.

3. **Async-First Architecture**: Built on Tokio runtime with async traits. File walking runs on blocking threads via `tokio::task::spawn_blocking` to avoid blocking the async runtime.

4. **Smart Indexing with Auto-Detection**: The `index_codebase` tool automatically detects whether to perform full indexing (new codebase) or incremental updates (previously indexed). Tracks file hashes (SHA256) in persistent cache (`.cache/project-rag/hash_cache.json`) to detect changes across server restarts.

5. **Hybrid Search**: Combines vector similarity (semantic understanding) with BM25 keyword matching using Reciprocal Rank Fusion (RRF) for optimal search results. Tantivy provides full-text search capabilities with IDF scoring.

### Critical Implementation Details

**1. MCP Server Pattern (mcp_server.rs)**
- Uses `#[tool_router]` and `#[prompt_router]` macros to generate routers
- Tools return `Result<String, String>` (JSON-serialized responses)
- Prompts return `Vec<PromptMessage>` for slash command expansion
- Server implements `ServerHandler` trait with `#[tool_handler]` and `#[prompt_handler]`

**2. Smart Indexing and Incremental Updates**
- Deprecated: Legacy `incremental_update` tool (removed in favor of smart indexing)
- `index_codebase` now auto-detects: full indexing for new codebases, incremental for previously indexed
- Persistent hash cache stored in `.cache/project-rag/hash_cache.json`
- Normalizes paths to canonical absolute form for consistent cache lookups
- Skips `.git` directories automatically to avoid indexing repository metadata
- Detects: new files (no old hash), modified files (hash changed), deleted files (in cache but not on disk)
- Deletes old embeddings before re-indexing modified files
- Updates cache after successful indexing and persists to disk

**3. Hybrid Search (bm25_search.rs)**
- Combines vector similarity with BM25 keyword matching
- Uses Tantivy inverted index for full-text search with IDF scoring
- Reciprocal Rank Fusion (RRF) merges both rankings using 1/(k+rank) formula (k=60)
- Both indexes queried in parallel for fast results
- Returns combined scores from both semantic and keyword matching

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
- Mock file system for file walker tests (use `create_test_file_info`)
- Current test coverage: 386 tests across 12 modules (types, chunker, cache, BM25, embedding, AST parser, file walker, relations types, symbol extractor, reference finder)

### Async Patterns
- Use `tokio::spawn_blocking` for CPU-intensive or blocking I/O operations
- Prefer `Arc<T>` over `Arc<RwLock<T>>` when possible (immutable shared state)
- Use `Arc<RwLock<T>>` for mutable shared state (e.g., indexed_roots cache)
- Batch operations to reduce async overhead (32 chunks per embedding batch)

### MCP Tool Development
When adding new tools:
1. Define request/response types in `types.rs` with `#[derive(JsonSchema)]`
2. Add tool method in `#[tool_router]` impl block with `#[tool]` attribute
3. Add corresponding prompt in `#[prompt_router]` impl block with `#[prompt]` attribute
4. Return `Result<String, String>` from tools (serialize response to JSON)
5. Return `Vec<PromptMessage>` from prompts (user messages for slash commands)
6. Update server count in comments (e.g., "6 tools" instead of "5 tools")
