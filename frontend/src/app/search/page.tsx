"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { getPaper, searchDocuments } from "@/lib/api";
import type { SearchResponse, SearchResult } from "@/types";
import { AlertCircle, Clock, FileCode, Loader2, Search } from "lucide-react";
import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useSearchParams } from "next/navigation";

function parseSearchPath(
  filePath: string
): { kind: string; paperId: string } | null {
  const match = /^(papers|patterns|algorithms)\/(.+)$/.exec(filePath);
  if (!match) return null;
  return { kind: match[1], paperId: match[2] };
}

function buildPreview(content: string, query: string): string {
  const normalized = content.trim();
  if (!normalized) return "";

  const trimmedQuery = query.trim().toLowerCase();
  if (!trimmedQuery) {
    return normalized.length > 300
      ? normalized.slice(0, 300).trimEnd() + "…"
      : normalized;
  }

  const lowerContent = normalized.toLowerCase();
  const significantTerms = query
    .toLowerCase()
    .split(/[^a-z0-9]+/i)
    .map((term) => term.trim())
    .filter((term) => term.length >= 2);
  const snippetTargets = [trimmedQuery, ...significantTerms];
  const matchIndex =
    snippetTargets
      .map((target) => lowerContent.indexOf(target))
      .find((index) => index >= 0) ?? -1;
  if (matchIndex === -1) {
    return normalized.length > 300
      ? normalized.slice(0, 300).trimEnd() + "…"
      : normalized;
  }

  const contextRadius = 160;
  const start = Math.max(0, matchIndex - contextRadius);
  const end = Math.min(
    normalized.length,
    matchIndex + trimmedQuery.length + contextRadius
  );
  const prefix = start > 0 ? "…" : "";
  const suffix = end < normalized.length ? "…" : "";
  return `${prefix}${normalized.slice(start, end).trim()}${suffix}`;
}

function formatLocation(result: SearchResult): string {
  if (result.page_numbers?.length) {
    const uniquePages = Array.from(new Set(result.page_numbers)).sort(
      (a, b) => a - b
    );
    const first = uniquePages[0];
    const last = uniquePages[uniquePages.length - 1];
    return first === last ? `page ${first}` : `pages ${first}-${last}`;
  }

  if (result.heading_context) {
    return `section ${result.heading_context}`;
  }

  if (result.start_line > 0 || result.end_line > 0) {
    return `lines ${result.start_line}-${result.end_line}`;
  }

  return result.language === "PDF" ? "PDF chunk" : "chunk";
}

// ============================================================================
// Score bar
// ============================================================================

function ScoreBar({ score }: { score: number }) {
  const pct = Math.round(Math.min(1, Math.max(0, score)) * 100);
  const color =
    pct >= 70
      ? "bg-green-500"
      : pct >= 45
        ? "bg-yellow-500"
        : "bg-muted-foreground/40";

  return (
    <div className="flex items-center gap-2">
      <div className="h-1.5 w-24 overflow-hidden rounded-full bg-muted">
        <div
          className={`h-full rounded-full transition-all ${color}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="text-xs tabular-nums text-muted-foreground">{pct}%</span>
    </div>
  );
}

// ============================================================================
// Result card
// ============================================================================

function ResultCard({
  result,
  paperTitle,
  query,
}: {
  result: SearchResult;
  paperTitle?: string;
  query: string;
}) {
  const displayScore = result.score;
  const preview = buildPreview(result.content, query);
  const parsed = parseSearchPath(result.file_path);
  const title = paperTitle ?? parsed?.paperId ?? result.file_path;
  const location = formatLocation(result);
  const matchType = result.match_type
    ? result.match_type[0].toUpperCase() + result.match_type.slice(1)
    : null;
  const subtitle = parsed
    ? `${parsed.kind} • ${result.file_path} • ${location}`
    : `${result.file_path} • ${location}`;

  return (
    <Card>
      <CardContent className="p-4">
        {/* Top row: file path + language badge */}
        <div className="mb-2 flex items-start justify-between gap-2">
          <div className="min-w-0">
            <div className="mb-0.5 truncate text-lg font-semibold text-foreground">
              {title}
            </div>
            <div className="flex min-w-0 items-center gap-1.5 text-sm text-muted-foreground">
              <FileCode className="h-3.5 w-3.5 shrink-0" />
              <span className="truncate font-mono text-xs">{subtitle}</span>
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {matchType && (
              <Badge variant="outline" className="text-xs">
                {matchType}
              </Badge>
            )}
            {result.language && (
              <Badge variant="secondary" className="text-xs">
                {result.language}
              </Badge>
            )}
          </div>
        </div>

        {/* Content preview */}
        <pre className="mb-3 overflow-x-auto whitespace-pre-wrap break-words rounded-md bg-muted p-3 font-mono text-xs leading-relaxed text-foreground">
          {preview}
        </pre>

        {/* Score bar */}
        <ScoreBar score={displayScore} />
      </CardContent>
    </Card>
  );
}

// ============================================================================
// Inner page (uses useSearchParams — must be inside Suspense)
// ============================================================================

function SearchPageInner() {
  const searchParams = useSearchParams();
  const paperId = searchParams.get("paper_id") ?? undefined;

  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [response, setResponse] = useState<SearchResponse | null>(null);
  const [paperTitle, setPaperTitle] = useState<string | null>(null);
  const [titleMap, setTitleMap] = useState<Record<string, string>>({});
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (paperId) {
      getPaper(paperId)
        .then((p) => setPaperTitle(p.title))
        .catch(() => setPaperTitle(null));
    }
  }, [paperId]);

  useEffect(() => {
    if (!response?.results.length) return;

    const idsToFetch = Array.from(
      new Set(
        response.results
          .map((result) => parseSearchPath(result.file_path)?.paperId)
          .filter((id): id is string => Boolean(id) && !titleMap[id as string])
      )
    );

    if (!idsToFetch.length) return;

    let cancelled = false;
    void Promise.all(
      idsToFetch.map(async (id) => {
        try {
          const paper = await getPaper(id);
          return [id, paper.title] as const;
        } catch {
          return null;
        }
      })
    ).then((entries) => {
      if (cancelled) return;
      const updates = Object.fromEntries(
        entries.filter((entry): entry is readonly [string, string] =>
          Boolean(entry)
        )
      );
      if (Object.keys(updates).length) {
        setTitleMap((prev) => ({ ...prev, ...updates }));
      }
    });

    return () => {
      cancelled = true;
    };
  }, [response, titleMap]);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      const trimmed = query.trim();
      if (!trimmed || loading) return;

      setLoading(true);
      setError(null);

      try {
        const result = await searchDocuments({
          query: trimmed,
          paper_id: paperId,
          limit: 10,
          min_score: 0.7,
          hybrid: true,
        });
        setResponse(result);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Search failed");
        setResponse(null);
      } finally {
        setLoading(false);
      }
    },
    [query, loading, paperId]
  );

  const hasResults = response !== null;
  const resultCount = response?.results.length ?? 0;

  return (
    <div className="mx-auto max-w-3xl px-4 py-8">
      {/* Page header */}
      <div className="mb-6">
        <h1 className="text-2xl font-semibold text-foreground">Search</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          {paperId
            ? `Searching within: ${paperTitle ?? paperId}`
            : "Semantic search across indexed documents"}
        </p>
      </div>

      {/* Search form */}
      <form onSubmit={handleSubmit} className="mb-6 flex gap-2">
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search code, functions, concepts…"
            className="h-10 w-full rounded-md border border-input bg-background pl-9 pr-3 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            autoFocus
          />
        </div>
        <Button type="submit" disabled={loading || !query.trim()}>
          {loading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Search className="h-4 w-4" />
          )}
          <span className="ml-1.5">{loading ? "Searching…" : "Search"}</span>
        </Button>
      </form>

      {/* Error */}
      {error && (
        <div className="mb-4 flex items-center gap-2 rounded-lg border border-destructive bg-destructive/10 p-4 text-sm text-destructive">
          <AlertCircle className="h-4 w-4 shrink-0" />
          {error}
        </div>
      )}

      {/* Duration */}
      {hasResults && (
        <div className="mb-4 flex items-center gap-1.5 text-xs text-muted-foreground">
          <Clock className="h-3.5 w-3.5" />
          <span>
            {resultCount} result{resultCount !== 1 ? "s" : ""} in{" "}
            {response!.duration_ms}ms
          </span>
          {response!.threshold_lowered && (
            <span>
              threshold lowered to {Math.round(response!.threshold_used * 100)}%
            </span>
          )}
        </div>
      )}

      {/* Results */}
      {hasResults && resultCount === 0 && (
        <p className="py-12 text-center text-sm text-muted-foreground">
          No strong results found. Try a more specific query.
        </p>
      )}

      {hasResults && resultCount > 0 && (
        <div className="flex flex-col gap-3">
          {response!.results.map((result, i) => (
            <ResultCard
              key={
                result.chunk_id ??
                `${result.file_path}-${result.start_line}-${i}`
              }
              result={result}
              query={query}
              paperTitle={(() => {
                const parsed = parseSearchPath(result.file_path);
                return parsed ? titleMap[parsed.paperId] : undefined;
              })()}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Page export — wraps inner component in Suspense for useSearchParams
// ============================================================================

export default function SearchPage() {
  return (
    <Suspense
      fallback={
        <div className="flex h-64 items-center justify-center">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <SearchPageInner />
    </Suspense>
  );
}
