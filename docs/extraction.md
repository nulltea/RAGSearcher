# Extraction Pipeline

Extracts structured patterns and algorithms from uploaded papers using Claude CLI. Both pipelines use a 3-pass approach: inventory, extraction, verification.

Requires [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) installed and accessible as `claude` in PATH.

## Pattern Extraction

Extracts research patterns (claims, evidence, context) from paper text.

### Pipeline

| Pass | Model | Purpose |
|------|-------|---------|
| 1 | Haiku | **Evidence Inventory** — extract 12–25 direct quotes, findings, and definitions with IDs (E1, E2, ...) |
| 2 | Sonnet | **Pattern Extraction** — extract 3–8 patterns citing evidence via `[E#]` format. Each pattern has claim, evidence, context fields |
| 3 | Haiku | **Verification** — check citation validity, accuracy, evidence coverage. Non-fatal if it fails |

### Data Model

**EvidenceItem**: `id`, `quote`, `location`, `type` (finding/definition/mechanism/limitation/comparison/claim/methodology/result), `importance`

**ExtractedPattern**: `rank`, `name`, `claim`, `evidence`, `context`, `tags`, `evidence_ids`, `confidence`

**VerificationResult**: `verification_status` (pass/warn/fail), `citation_issues`, `unused_evidence`, `accuracy_concerns`, `overall_quality`

### API

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/papers/{id}/extract` | Trigger pattern extraction |
| GET | `/api/papers/{id}/patterns?status=` | List patterns (optional status filter) |
| POST | `/api/papers/{id}/patterns/review` | Submit approve/reject decisions |
| DELETE | `/api/papers/{id}/patterns` | Delete all patterns for a paper |

Review body:
```json
{
  "decisions": [
    { "pattern_id": "uuid", "approved": true },
    { "pattern_id": "uuid", "approved": false }
  ]
}
```

Approved patterns are embedded into LanceDB for semantic search. When all patterns and algorithms are reviewed, the paper status transitions to `active`.

## Algorithm Extraction

Extracts implementable algorithm definitions (steps, I/O, pseudocode) from paper text.

### Pipeline

| Pass | Model | Purpose |
|------|-------|---------|
| 1 | Haiku | **Algorithm Inventory** — identify algorithm candidates with descriptions and types |
| 2 | Sonnet | **Algorithm Extraction** — extract full definitions: numbered steps, inputs/outputs, preconditions, complexity, pseudocode |
| 3 | Haiku | **Verification** — check completeness, citation accuracy. Non-fatal if it fails |

If evidence from a prior pattern extraction is available, Pass 1 reuses it instead of re-extracting.

### Data Model

**ExtractedAlgorithm**: `rank`, `name`, `description`, `steps[]` (number/action/details/math), `inputs[]`, `outputs[]`, `preconditions`, `complexity`, `mathematical_notation`, `pseudocode`, `tags`, `evidence_ids`, `confidence`

**AlgorithmVerificationResult**: `verification_status`, `completeness_issues`, `citation_issues`, `overall_quality`

### API

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/papers/{id}/extract-algorithms` | Trigger algorithm extraction |
| GET | `/api/papers/{id}/algorithms?status=` | List algorithms (optional status filter) |
| POST | `/api/papers/{id}/algorithms/review` | Submit approve/reject decisions |
| DELETE | `/api/papers/{id}/algorithms` | Delete all algorithms for a paper |

Review body format is identical to pattern review. Approved algorithms are embedded into LanceDB.

## CLI Configuration

The Claude CLI path defaults to `claude`. Override by constructing extractors with a custom path:

```rust
PatternExtractor::with_path("/custom/path/to/claude".into());
AlgorithmExtractor::with_path("/custom/path/to/claude".into());
```

CLI flags used: `--print --model {model} --output-format json --max-turns 1 --no-session-persistence`. Prompts are piped via stdin to avoid OS argument length limits.

## Paper Status Flow

```
processing → ready_for_review → active
                ↑ (re-extract)
```

- `processing`: paper uploaded, text chunked and embedded
- `ready_for_review`: extraction complete, patterns/algorithms awaiting review
- `active`: all items reviewed, paper fully searchable (including pattern/algorithm embeddings)
