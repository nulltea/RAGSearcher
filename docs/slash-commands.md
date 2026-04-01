# Slash Commands

MCP slash commands exposed by the `rag-searcher` server. Use as `/rag-searcher:<command>` in Claude Code or Claude Desktop.

## `/rag-searcher:search`

Semantic search across paper content using hybrid vector + BM25 matching.

**Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | string | required | Search query |
| `path` | string | — | Filter by indexed content path |
| `project` | string | — | Filter by project name |
| `limit` | int | 10 | Max results (up to 1000) |
| `min_score` | float | 0.7 | Similarity threshold (0.0–1.0) |
| `hybrid` | bool | true | Combine vector + BM25 search |

**Returns:** Ranked results with content chunks, scores (vector + keyword), file paths, and line ranges.

## `/rag-searcher:papers`

Search papers by title, authors, status, or type. Returns paper metadata.

**Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | string | — | Keyword search in title/authors |
| `status` | string | — | Filter: `processing`, `ready_for_review`, `active`, `archived` |
| `paper_type` | string | — | Filter by type (e.g. `research_paper`) |
| `limit` | int | 20 | Max results |
| `offset` | int | 0 | Pagination offset |

**Returns:** Papers with id, title, authors, source, status, chunk count, file path, creation date.

## `/rag-searcher:algorithms`

Search algorithms extracted from papers by keyword, status, tags, or paper.

**Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `query` | string | — | Keyword search in algorithm name/description |
| `status` | string | `approved` | Filter: `pending`, `approved`, `rejected` |
| `paper_id` | string | — | Scope to a specific paper |
| `tags` | string[] | — | Filter by tags (must match all) |
| `limit` | int | 20 | Max results |
| `offset` | int | 0 | Pagination offset |

**Returns:** Algorithms with id, paper_id, paper_title, name, description, steps, inputs, outputs, preconditions, complexity, mathematical_notation, pseudocode, tags, confidence, status.

## `/rag-searcher:index`

Upload and index a paper from a local file path or URL. Extracts text from PDF, chunks it, generates embeddings, and stores in the vector database.

**Parameters:**

| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `file_path` | string | — | Absolute path to a local PDF or text file |
| `url` | string | — | URL to download a PDF from |

Either `file_path` or `url` must be provided. Title and authors are automatically extracted from PDF metadata.

**Returns:** Paper ID, title, chunk count, status, and duration.
