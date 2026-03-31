# RAGSearcher — Paper Library with Semantic Search

- **Desktop app** (Tauri) for managing a research paper library — upload PDFs, extract patterns and algorithms, search semantically.
- **MCP server** that gives your coding agent token-efficient access to paper content and algorithm definitions via RAG. Hybrid search (FastEmbed vectors + BM25 keywords) over LanceDB.


## Acknowledgments

This project is based on two separate projects:
- Local indexing with FastEmbed + LanceDB, semantic searches with hybrid vector + BM25. https://github.com/Brainwires/project-rag
- Parts of frontend and claim/evidence/context extraction pipeline. https://github.com/aakashsharan/research-vault

## How It Works

1. Upload a PDF (or paste a URL)
2. Text is extracted, chunked, and embedded locally via FastEmbed (all-MiniLM-L6-v2, 384 dims)
3. Chunks are stored in LanceDB (embedded vector DB) with BM25 keyword index (Tantivy)
4. Search combines vector similarity + BM25 via Reciprocal Rank Fusion
5. Optionally extract patterns and algorithms from papers via Claude CLI

No API keys needed for core functionality. All embedding runs locally.

## Prerequisites

- Rust 1.88+ (2024 edition)
- Node.js (for desktop app frontend)

## MCP Server

### Install

```bash
git clone https://github.com/nulltead/RAGSearcher.git && cd RAGSearcher
cargo install --path crates/core
```

This builds and installs the `rag-searcher` binary to `~/.cargo/bin/`.

### Add to Claude Code

```bash
claude mcp add rag-searcher rag-searcher
```

Or manually in `~/.claude.json`:
```json
{
  "mcpServers": {
    "rag-searcher": {
      "command": "rag-searcher",
      "args": []
    }
  }
}
```

### Add to Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "rag-searcher": {
      "command": "rag-searcher"
    }
  }
}
```

### MCP Tools

| Tool | Description |
|------|-------------|
| `search` | Semantic search across paper content |
| `search_papers` | Search papers by title, authors, status, type |
| `search_algorithms` | Search algorithms across papers by keyword, tags, status |

Slash commands: `/rag-searcher:search`, `/rag-searcher:papers`, `/rag-searcher:algorithms`

### Usage Examples

Once the MCP server is added, you can reference your paper library directly in Claude Code:

```
# Find papers on a topic
> /rag-searcher:papers attention mechanisms

# Search paper content semantically
> /rag-searcher:search how does the transformer handle positional encoding

# Use papers as development context
> Search my papers for the HNSW algorithm, then implement it in Rust based on what the paper describes

> Look up the loss function from "Focal Loss for Dense Object Detection" in my papers and add it to src/losses.rs

> Find all papers about batch normalization, summarize the key trade-offs, and recommend which variant to use for my model

# Implement directly from a paper
> Implement the B2A algorithm from "Efficient Three-party Boolean-to-Arithmetic Share" paper
```

Claude Code will automatically call `search` and `search_papers` to retrieve relevant paper chunks, then use them as context for answering questions or writing code.

## Desktop App

### Build

Make sure `tauri cli` is installed

```bash
cargo install tauri-cli --version "^2.0.0" --locked
```

```bash
cd frontend && npm install && cd ..
cargo tauri build --bundles app
```

### Install (macOS)

```bash
cp -r ./target/release/bundle/macos/RAGSearcher.app /Applications/
```

Launch from Applications or Spotlight.

## Configuration

Copy `config.example.toml` to your platform config directory:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/RAGSearcher/config.toml` |
| Linux | `~/.config/project-rag/config.toml` |
| Windows | `%APPDATA%\project-rag\config.toml` |

Key settings:

```toml
[vector_db]
backend = "lancedb"                    # or "qdrant"
# lancedb_path = "/custom/path"       # default: platform data dir

[embedding]
model_name = "all-MiniLM-L6-v2"       # 384 dims, fast
batch_size = 8
# fastembed_cache_path = "/custom"     # or set FASTEMBED_CACHE_DIR

[search]
min_score = 0.7                        # similarity threshold
hybrid = true                          # vector + BM25

[storage]
# papers_path = "/custom/path"        # default: platform data dir + /uploads
```

Environment overrides: `PROJECT_RAG_DB_BACKEND`, `PROJECT_RAG_LANCEDB_PATH`, `PROJECT_RAG_MODEL`, `PROJECT_RAG_BATCH_SIZE`, `PROJECT_RAG_MIN_SCORE`, `FASTEMBED_CACHE_DIR`

## Project Structure

```
crates/core/       Rust library + CLI (MCP server, web server, embedding, search)
crates/tauri-app/  Tauri 2 desktop app wrapper
frontend/          Next.js static export (paper library UI)
```

## Development

### MCP
```bash
cargo check              # compile check
cargo test --lib         # run tests (77 tests)
cargo fmt                # format
cargo clippy             # lint
RUST_LOG=debug cargo run # debug logging
```

### App
```bash
cargo tauri dev
```

## Troubleshooting

**BM25 lock error** — Another process is indexing. Wait for it to finish, or remove stale locks:
```bash
rm ~/.local/share/project-rag/lancedb/lancedb_bm25/.tantivy-*.lock
```

**Model download fails** — Set `FASTEMBED_CACHE_DIR` to a writable directory, or set `HF_ENDPOINT=https://hf-mirror.com` for mirrors.

**Qdrant connection fails** — Check `curl http://localhost:6334/health` and verify the container is running.

## License

MIT
