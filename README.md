# Project RAG - MCP Server for Code Understanding

[![Tests](https://img.shields.io/badge/tests-413%20passing-brightgreen)](https://github.com/Brainwires/project-rag)
[![Coverage](https://img.shields.io/badge/coverage-94%25-brightgreen)](https://github.com/Brainwires/project-rag)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/project-rag)](https://crates.io/crates/project-rag)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust-based Model Context Protocol (MCP) server that provides AI assistants with powerful RAG (Retrieval-Augmented Generation)
[[TODO]]

## Overview

[[TODO]]

This MCP server enables AI assistants to efficiently search and understand large projects by:
- Creating semantic embeddings of code files
- Storing them in a local vector database
- Providing fast semantic search capabilities
- Supporting incremental updates for efficiency

## Features

- **Local-First**: All processing happens locally using fastembed-rs (no API keys required)
- **Hybrid Search**: Combines vector similarity with BM25 keyword matching using Reciprocal Rank Fusion (RRF) for optimal results
- **AST-Based Chunking**: Uses Tree-sitter to extract semantic units (functions, classes, methods) for 12 languages
- **Comprehensive File Support**: Indexes 40+ file types including code, documentation (with PDF→Markdown conversion), and configuration files
- [[TODO]]
- **Slash Commands**: 9 convenient slash commands via MCP Prompts

## MCP Slash Commands

The server provides 9 slash commands for quick access in Claude Code:

1. **`/project:index`** - Index a codebase directory (automatically performs full or incremental)
2. **`/project:query`** - Search the indexed codebase
3. **`/project:stats`** - Get index statistics
4. **`/project:clear`** - Clear all indexed data
5. **`/project:search`** - Advanced search with filters
6. **`/project:git-search`** - Search git commit history with on-demand indexing
7. **`/project:definition`** - Find where a symbol is defined (LSP-like)
8. **`/project:references`** - Find all references to a symbol
9. **`/project:callgraph`** - Get call graph for a function (callers/callees)


## MCP Tools

The server provides 9 tools that can be used directly:

[[TODO]]

2. **query** - Hybrid semantic + keyword search across the indexed code
   - Combines vector similarity with BM25 keyword matching (enabled by default)
   - Returns relevant code chunks with both vector and keyword scores
   - Configurable result limit and score threshold
   - Optional project filtering for multi-project setups

3. **get_statistics** - Get statistics about the indexed codebase
   - File counts, chunk counts, embedding counts
   - Language breakdown

4. **clear_index** - Clear all indexed data
   - Deletes the entire vector database collection
   - Prepares for fresh indexing

5. **search_by_filters** - Advanced hybrid search with filters
   - Always uses hybrid search for best results
   - Filter by file extensions (e.g., ["rs", "toml"])
   - Filter by programming languages
   - Filter by path patterns
   - Optional project filtering

6. **search_git_history** - Search git commit history using semantic search
   - Automatically indexes commits on-demand (default: 10 commits, configurable)
   - Searches commit messages, diffs, author info, and changed files
   - Smart caching: only indexes new commits as needed
   - Regex filtering by author name/email and file paths
   - Date range filtering (ISO 8601 or Unix timestamp)
   - Branch selection support

7. **find_definition** - Find where a symbol is defined (LSP-like)
   - Specify file path, line number, and column
   - Returns definition location with symbol metadata
   - Uses hybrid approach: high-precision stack-graphs (Python, TypeScript, Java, Ruby) or AST-based RepoMap fallback
   - Reports precision level of results

8. **find_references** - Find all references to a symbol
   - Specify file path, line number, and column
   - Returns all locations where the symbol is used
   - Categorizes reference types: Call, Read, Write, Import, TypeReference, Inheritance, Instantiation
   - Optional: include definition site in results

9. **get_call_graph** - Get call graph for a function
   - Specify file path, line number, and column for a function
   - Returns callers (what calls this function) and callees (what this function calls)
   - Configurable traversal depth (default: 1 level)
   - Useful for understanding code flow and impact analysis

## Prerequisites

- **Rust**: 1.88+ with Rust 2024 edition support

### Vector Database Options

**LanceDB (Default - Embedded, Stable)**

No additional setup needed! LanceDB is an embedded vector database that runs directly in the application. It stores data in `./.lancedb` directory by default.

**Why LanceDB is the default:**
- **Embedded** - No external dependencies or servers required
- **Stable** - Production-proven with ACID transactions
- **Feature-rich** - Full SQL-like filtering capabilities
- **Hybrid search built-in** - Tantivy BM25 + LanceDB vector with Reciprocal Rank Fusion
- **Columnar storage** - Efficient for large datasets with Apache Arrow
- **Zero-copy** - Memory-mapped files for fast queries

**Qdrant (Optional - Server-Based)**

To use Qdrant instead of LanceDB, build with the `qdrant-backend` feature:

```bash
cargo build --release --no-default-features --features qdrant-backend
```

Then start a Qdrant instance:

**Using Docker (Recommended):**
```bash
docker run -p 6333:6333 -p 6334:6334 \
    -v $(pwd)/qdrant_data:/qdrant/storage \
    qdrant/qdrant
```

**Using Docker Compose:**
```yaml
version: '3.8'
services:
  qdrant:
    image: qdrant/qdrant
    ports:
      - "6333:6333"
      - "6334:6334"
    volumes:
      - ./qdrant_data:/qdrant/storage
```

**Or download standalone:** https://qdrant.tech/documentation/guides/installation/

## Installation

[[TODO]]

```bash
# Navigate to the project
cd project-rag

# Install protobuf compiler (Ubuntu/Debian)
sudo apt-get install protobuf-compiler

# Build the release binary (with default LanceDB backend - stable and embedded!)
cargo build --release

# Or build with Qdrant backend (requires external server)
cargo build --release --no-default-features --features qdrant-backend

# The binary will be at target/release/project-rag
```

## Usage

[[TODO]]

### Running as MCP Server

The server communicates over stdio following the MCP protocol:

```bash
./target/release/project-rag
```

### Configuring in Claude Code

Add the MCP server to Claude Code using the CLI:

```bash
# Navigate to the project directory first
cd /path/to/project-rag

# Add the MCP server to Claude Code
claude mcp add project --command "$(pwd)/target/release/project-rag"

# Or with logging enabled
claude mcp add project --command "$(pwd)/target/release/project-rag" --env RUST_LOG=info
```

After adding, restart Claude Code to load the server. The slash commands (`/project:index`, `/project:query`, etc.) will be available immediately.

### Configuring in Claude Desktop

Add to your Claude Desktop config:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Linux**: `~/.config/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "project-rag": {
      "command": "/absolute/path/to/project-rag/target/release/project-rag",
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Development

[[TODO]]

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Check without building
cargo check
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint with clippy
cargo clippy

# Fix clippy warnings
cargo clippy --fix
```

### Debugging

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with trace logging
RUST_LOG=trace cargo run
```

## Performance

### Benchmarks (Typical Hardware)

- **Indexing Speed**: ~1000 files/minute
  - Depends on file size and complexity
  - Includes file I/O, hashing, chunking, embedding generation

- **Search Latency**: 20-30ms per query
  - ~95% recall with HNSW index
  - Sub-50ms for most queries

- **Memory Usage**:
  - Base: ~100MB
  - Embedding model: ~50MB
  - Per 10k chunks: ~40MB (embeddings + metadata)

- **Storage**:
  - Embeddings: ~1.5KB per chunk (384 floats)
  - Typical project (1000 files): ~75MB in Qdrant

### Optimization Tips

1. **Adjust chunk size**: Smaller chunks = more precise but slower indexing
2. **Use filters**: Pre-filter by language/extension for faster searches
3. **Batch processing**: Default 32 chunks per batch is optimal for most systems
4. **Incremental updates**: Use after initial index to save time

## Troubleshooting

### Index Lock Errors

**Error: "BM25 index is currently being used by another process"**

This means another agent or process is actively indexing. This is expected behavior to prevent index corruption.

**Solutions:**
1. **Wait**: Let the current indexing operation complete (typically seconds to minutes)
2. **Check processes**: Verify no other Claude Code/Desktop instances are running indexing operations
3. **Force cleanup** (last resort): If you're certain no other process is running, manually remove stale locks:
   ```bash
   rm ~/.local/share/project-rag/lancedb/lancedb_bm25/.tantivy-*.lock
   ```

**Note:** The system automatically detects and cleans up stale locks (>5 minutes old) from crashed processes. You should rarely need manual intervention.

### Qdrant Connection Fails
```bash
# Check if Qdrant is running
curl http://localhost:6334/health

# View Qdrant logs
docker logs <container-id>
```

### Model Download Fails
```bash
# Pre-download model
python -c "from fastembed import TextEmbedding; TextEmbedding()"

# Or set HuggingFace mirror
export HF_ENDPOINT=https://hf-mirror.com
```

### Out of Memory
```bash
# Reduce batch size (edit source)
# Or index in smaller chunks
# Or use smaller embedding model
```

### Slow Indexing
```bash
# Check disk I/O
# Reduce max_file_size
# Use exclude_patterns to skip unnecessary files
```

## License

MIT License - see LICENSE file for details
