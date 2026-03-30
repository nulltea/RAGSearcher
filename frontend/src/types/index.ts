/**
 * TypeScript types matching project-rag Rust backend.
 */

// ============================================================================
// Enums
// ============================================================================

export type PaperType =
  | "research_paper"
  | "blog_post"
  | "article"
  | "technical_report"
  | "book_chapter";

export type PaperStatus = "processing" | "ready_for_review" | "active" | "archived";

export type Confidence = "high" | "medium" | "low";

export type PatternStatus = "pending" | "approved" | "rejected";

// ============================================================================
// Paper Types
// ============================================================================

export interface PaperResponse {
  id: string;
  title: string;
  authors: string[];
  source: string | null;
  published_date: string | null;
  paper_type: string;
  status: PaperStatus;
  original_filename: string | null;
  chunk_count: number;
  created_at: string;
  updated_at: string;
}

export interface PaperListResponse {
  papers: PaperResponse[];
  total: number;
  limit: number;
  offset: number;
}

export interface PaperUploadResponse {
  paper: PaperResponse;
  chunk_count: number;
  duration_ms: number;
}

export interface PaperListParams {
  status?: PaperStatus;
  paper_type?: string;
  limit?: number;
  offset?: number;
}

// ============================================================================
// Search Types
// ============================================================================

export interface SearchResult {
  file_path: string;
  content: string;
  score: number;
  vector_score: number;
  keyword_score: number | null;
  combined_score: number | null;
  start_line: number;
  end_line: number;
  language: string | null;
  project: string | null;
}

export interface SearchResponse {
  results: SearchResult[];
  duration_ms: number;
}

export interface SearchRequest {
  query: string;
  paper_id?: string;
  limit?: number;
  min_score?: number;
  hybrid?: boolean;
}

// ============================================================================
// Pattern Types
// ============================================================================

export interface PatternResponse {
  id: string;
  paper_id: string;
  name: string;
  claim: string | null;
  evidence: string | null;
  context: string | null;
  tags: string[];
  confidence: string;
  status: PatternStatus;
  created_at: string;
  updated_at: string;
}

export interface ExtractedPatternResponse {
  temp_id: string;
  name: string;
  claim: string | null;
  evidence: string | null;
  context: string | null;
  tags: string[];
  confidence: Confidence;
}

export interface PatternDecision {
  pattern_id: string;
  approved: boolean;
}

export interface PatternReviewResponse {
  approved_count: number;
  rejected_count: number;
  patterns: PatternResponse[];
}

export interface ExtractResponse {
  paper_id: string;
  patterns: PatternResponse[];
  evidence_count: number;
  verification_status: string | null;
  duration_ms: number;
}

export interface PatternListResponse {
  patterns: PatternResponse[];
}

// ============================================================================
// Statistics Types
// ============================================================================

export interface StatisticsResponse {
  total_chunks: number;
  total_vectors: number;
  languages: [string, number][];
}

// ============================================================================
// Health Types
// ============================================================================

export interface HealthResponse {
  status: string;
  version: string;
}

// ============================================================================
// Error Types
// ============================================================================

export interface ApiError {
  code: string;
  message: string;
}

export class ApiRequestError extends Error {
  code: string;
  status: number;

  constructor(code: string, message: string, status: number) {
    super(message);
    this.name = "ApiRequestError";
    this.code = code;
    this.status = status;
  }
}
