/**
 * API client for Project RAG backend.
 */

import type {
  ApiError,
  HealthResponse,
  PaperListParams,
  PaperListResponse,
  PaperResponse,
  PaperUploadResponse,
  SearchRequest,
  SearchResponse,
  StatisticsResponse,
} from "@/types";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";

// ============================================================================
// Error Handling
// ============================================================================

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

async function parseError(response: Response): Promise<ApiRequestError> {
  try {
    const data = await response.json();
    const error = data.detail as ApiError | undefined;
    if (error && typeof error === "object" && "code" in error) {
      return new ApiRequestError(error.code, error.message, response.status);
    }
    return new ApiRequestError(
      "UNKNOWN_ERROR",
      typeof data.detail === "string" ? data.detail : "Request failed",
      response.status,
    );
  } catch {
    return new ApiRequestError(
      "UNKNOWN_ERROR",
      `Request failed with status ${response.status}`,
      response.status,
    );
  }
}

async function request<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const response = await fetch(`${API_URL}${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...options.headers,
    },
  });

  if (!response.ok) {
    throw await parseError(response);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

async function requestFormData<T>(
  path: string,
  formData: FormData,
): Promise<T> {
  const response = await fetch(`${API_URL}${path}`, {
    method: "POST",
    body: formData,
  });

  if (!response.ok) {
    throw await parseError(response);
  }

  return response.json();
}

// ============================================================================
// Health
// ============================================================================

export async function getHealth(): Promise<HealthResponse> {
  return request<HealthResponse>("/health");
}

// ============================================================================
// Papers
// ============================================================================

export async function uploadPaper(
  formData: FormData,
): Promise<PaperUploadResponse> {
  return requestFormData<PaperUploadResponse>("/api/papers", formData);
}

export async function listPapers(
  params?: PaperListParams,
): Promise<PaperListResponse> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set("status", params.status);
  if (params?.paper_type) searchParams.set("paper_type", params.paper_type);
  if (params?.limit) searchParams.set("limit", params.limit.toString());
  if (params?.offset) searchParams.set("offset", params.offset.toString());

  const query = searchParams.toString();
  const path = query ? `/api/papers?${query}` : "/api/papers";
  return request<PaperListResponse>(path);
}

export async function getPaper(paperId: string): Promise<PaperResponse> {
  return request<PaperResponse>(`/api/papers/${paperId}`);
}

export async function deletePaper(paperId: string): Promise<void> {
  return request<void>(`/api/papers/${paperId}`, { method: "DELETE" });
}

// ============================================================================
// Search
// ============================================================================

export async function searchDocuments(
  req: SearchRequest,
): Promise<SearchResponse> {
  return request<SearchResponse>("/api/search", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

// ============================================================================
// Statistics
// ============================================================================

export async function getStatistics(): Promise<StatisticsResponse> {
  return request<StatisticsResponse>("/api/statistics");
}

// ============================================================================
// Convenience Export
// ============================================================================

export const api = {
  health: getHealth,
  papers: {
    upload: uploadPaper,
    list: listPapers,
    get: getPaper,
    delete: deletePaper,
  },
  search: searchDocuments,
  statistics: getStatistics,
};
