/** MCP JSON-RPC protocol types */

export interface JsonRpcRequest {
  jsonrpc: "2.0";
  id: number;
  method: string;
  params?: Record<string, unknown>;
}

export interface JsonRpcResponse {
  jsonrpc: "2.0";
  id: number;
  result?: unknown;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export interface McpInitializeResult {
  protocolVersion: string;
  capabilities: Record<string, unknown>;
  serverInfo: {
    name: string;
    version: string;
  };
}

export interface McpToolCallResult {
  content: Array<{
    type: "text";
    text: string;
  }>;
  isError?: boolean;
}

// Tool argument types matching Rust backend
export interface IndexPaperArgs {
  file_path?: string;
  url?: string;
  title?: string;
  authors?: string;
  source?: string;
  paper_type?: string;
}

export interface ExtractAlgorithmsArgs {
  paper_id: string;
}

export interface SearchAlgorithmsArgs {
  query?: string;
  status?: string;
  paper_id?: string;
  tags?: string[];
  limit?: number;
  offset?: number;
}

export interface SearchPapersArgs {
  query?: string;
  status?: string;
  paper_type?: string;
  limit?: number;
  offset?: number;
}

export interface QueryRequest {
  query: string;
  path?: string;
  project?: string;
  limit?: number;
  min_score?: number;
  hybrid?: boolean;
}

// Response types from tool JSON output
export interface IndexPaperResult {
  paper_id: string;
  title: string;
  chunk_count: number;
  status: string;
  duration_ms: number;
}

export interface ExtractAlgorithmsResult {
  paper_id: string;
  algorithm_count: number;
  evidence_count: number;
  verification_status: string | null;
  duration_ms: number;
}

export interface AlgorithmResult {
  id: string;
  paper_id: string;
  paper_title: string;
  name: string;
  description: string | null;
  steps: Array<{
    number: number;
    action: string;
    details: string;
    math: string | null;
  }>;
  inputs: Array<{
    name: string;
    type: string;
    description: string;
  }>;
  outputs: Array<{
    name: string;
    type: string;
    description: string;
  }>;
  preconditions: string[];
  complexity: string | null;
  mathematical_notation: string | null;
  pseudocode: string | null;
  tags: string[];
  confidence: string;
  status: string;
  created_at: string;
}

export interface SearchAlgorithmsResult {
  algorithms: AlgorithmResult[];
  total: number;
  limit: number;
  offset: number;
  duration_ms: number;
}

// Pattern types (from CLI JSON output, not MCP)
export interface PatternResult {
  id: string;
  paper_id: string;
  name: string;
  claim: string | null;
  evidence: string | null;
  context: string | null;
  tags: string[];
  confidence: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface ExtractPatternsCliResult {
  paper_id: string;
  pattern_count: number;
  evidence_count: number;
  verification_status: string | null;
  duration_ms: number;
}

export interface ListPatternsCliResult {
  patterns: PatternResult[];
  count: number;
}

export interface SearchPapersResult {
  papers: Array<{
    id: string;
    title: string;
    authors: string[];
    source: string | null;
    published_date: string | null;
    paper_type: string;
    status: string;
    chunk_count: number;
    file_path: string | null;
    created_at: string;
  }>;
  total: number;
  limit: number;
  offset: number;
  duration_ms: number;
}

export interface SemanticSearchResult {
  file_path: string;
  root_path?: string | null;
  content: string;
  score: number;
  vector_score: number;
  keyword_score?: number | null;
  start_line: number;
  end_line: number;
  language: string;
  project?: string | null;
}

export interface QueryResponse {
  results: SemanticSearchResult[];
  duration_ms: number;
  threshold_used: number;
  threshold_lowered: boolean;
}
