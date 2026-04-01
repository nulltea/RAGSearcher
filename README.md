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
| `index_paper` | Upload and index a paper from a local file path or URL |
| `extract_algorithms` | Run 3-pass AI extraction pipeline on an indexed paper |

Slash commands: `/rag-searcher:search`, `/rag-searcher:papers`, `/rag-searcher:algorithms`, `/rag-searcher:index`

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

## Zotero 7 Plugin

Upload papers and extract algorithms directly from your Zotero library. The plugin communicates with `rag-searcher` via MCP over stdio — no web server needed.

### Prerequisites

- Zotero 7+
- `rag-searcher` binary installed (see [MCP Server > Install](#install) above)
- Node.js (to build the plugin)

### Build

```bash
cd zotero-plugin
npm install
npm run build
```

This produces `build/zotero-rag-library-<version>.xpi`.

### Install in Zotero

1. Open Zotero 7
2. Go to **Tools > Plugins** (or **Add-ons**)
3. Click the gear icon > **Install Add-on From File...**
4. Select the `.xpi` file from `zotero-plugin/build/`

### Usage

Right-click any item in your Zotero library:

- **Upload to RAG Library** — Reads the PDF attachment, extracts text, chunks and embeds it locally. Stores a `RAG-ID` in the item's Extra field for future operations.
- **Extract Algorithms** — Runs the 3-pass AI pipeline (evidence inventory, algorithm definitions, verification) on the uploaded paper. Opens a review dialog showing extracted algorithms with collapsible steps, I/O, pseudocode, and approve/reject buttons. Requires Claude CLI installed.
- **View Algorithms** — Re-opens the review dialog for previously extracted algorithms.

### Configuration

The plugin auto-detects `rag-searcher` from your PATH. To use a custom binary path, set it in **Zotero > Settings > RAG Library > Binary Path**.

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
zotero-plugin/     Zotero 7 Bootstrap extension (TypeScript, MCP over stdio)
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
