"use client";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  FilterBar,
  PaperCard,
  type FilterState,
} from "@/components/library";
import { deletePaper, extractAlgorithms, extractPatterns, listPapers } from "@/lib/api";
import type { PaperResponse } from "@/types";
import { AlertCircle, Lightbulb, Code2, Loader2, Plus } from "lucide-react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useCallback, useEffect, useState } from "react";

const ITEMS_PER_PAGE = 20;

function LibraryContent() {
  const router = useRouter();
  const searchParams = useSearchParams();

  // Initialize filters from URL
  const [filters, setFilters] = useState<FilterState>({
    search: searchParams.get("search") || "",
    status: (searchParams.get("status") as FilterState["status"]) || "",
    type: (searchParams.get("type") as FilterState["type"]) || "",
  });

  // Data state
  const [papers, setPapers] = useState<PaperResponse[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Pagination
  const [offset, setOffset] = useState(0);
  const [loadingMore, setLoadingMore] = useState(false);

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<PaperResponse | null>(null);
  const [deleting, setDeleting] = useState(false);

  // Extract dialog
  const [extractTarget, setExtractTarget] = useState<PaperResponse | null>(null);
  const [extractingPatterns, setExtractingPatterns] = useState(false);
  const [extractingAlgorithms, setExtractingAlgorithms] = useState(false);
  const [extractError, setExtractError] = useState<string | null>(null);

  // Load papers
  const loadPapers = useCallback(
    async (resetOffset = false) => {
      const currentOffset = resetOffset ? 0 : offset;

      if (resetOffset) {
        setLoading(true);
        setOffset(0);
      } else {
        setLoadingMore(true);
      }

      setError(null);

      try {
        const response = await listPapers({
          status: filters.status || undefined,
          paper_type: filters.type || undefined,
          limit: ITEMS_PER_PAGE,
          offset: currentOffset,
        });

        // Client-side search filtering (API doesn't support search)
        let filteredPapers = response.papers;
        if (filters.search) {
          const searchLower = filters.search.toLowerCase();
          filteredPapers = response.papers.filter(
            (p) =>
              p.title.toLowerCase().includes(searchLower) ||
              p.authors.some((a) => a.toLowerCase().includes(searchLower))
          );
        }

        if (resetOffset) {
          setPapers(filteredPapers);
        } else {
          setPapers((prev) => [...prev, ...filteredPapers]);
        }
        setTotal(response.total);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load papers");
      } finally {
        setLoading(false);
        setLoadingMore(false);
      }
    },
    [filters, offset]
  );

  // Initial load and filter changes
  useEffect(() => {
    loadPapers(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters.status, filters.type]);

  // Debounced search
  useEffect(() => {
    const timer = setTimeout(() => {
      loadPapers(true);
    }, 300);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters.search]);

  // Update URL when filters change
  useEffect(() => {
    const params = new URLSearchParams();
    if (filters.search) params.set("search", filters.search);
    if (filters.status) params.set("status", filters.status);
    if (filters.type) params.set("type", filters.type);
    const query = params.toString();
    router.replace(query ? `/library?${query}` : "/library", { scroll: false });
  }, [filters, router]);

  // Handle load more
  const handleLoadMore = useCallback(() => {
    setOffset((prev) => prev + ITEMS_PER_PAGE);
  }, []);

  // Load more when offset changes
  useEffect(() => {
    if (offset > 0) {
      loadPapers(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [offset]);

  // Handle delete
  const handleDelete = useCallback(async () => {
    if (!deleteTarget) return;

    setDeleting(true);
    try {
      await deletePaper(deleteTarget.id);
      setPapers((prev) => prev.filter((p) => p.id !== deleteTarget.id));
      setTotal((prev) => prev - 1);
      setDeleteTarget(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete paper");
    } finally {
      setDeleting(false);
    }
  }, [deleteTarget]);

  // Handle view paper
  const handleView = useCallback((paper: PaperResponse) => {
    if (paper.status === "ready_for_review") {
      router.push(`/review?id=${paper.id}`);
    } else {
      router.push(`/search?paper_id=${paper.id}`);
    }
  }, [router]);

  // Extract handlers
  const handleExtractPatterns = useCallback(async () => {
    if (!extractTarget) return;
    setExtractingPatterns(true);
    setExtractError(null);
    try {
      await extractPatterns(extractTarget.id);
      // Refresh papers to update counts
      loadPapers(true);
      setExtractTarget(null);
    } catch (err) {
      setExtractError(
        err instanceof Error ? err.message : "Pattern extraction failed",
      );
    } finally {
      setExtractingPatterns(false);
    }
  }, [extractTarget, loadPapers]);

  const handleExtractAlgorithms = useCallback(async () => {
    if (!extractTarget) return;
    setExtractingAlgorithms(true);
    setExtractError(null);
    try {
      await extractAlgorithms(extractTarget.id);
      loadPapers(true);
      setExtractTarget(null);
    } catch (err) {
      setExtractError(
        err instanceof Error ? err.message : "Algorithm extraction failed",
      );
    } finally {
      setExtractingAlgorithms(false);
    }
  }, [extractTarget, loadPapers]);

  const hasMore = papers.length < total;

  return (
    <div className="mx-auto max-w-6xl px-4 py-8">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-foreground">Library</h1>
          <p className="mt-1 text-muted-foreground">
            Browse and manage your research papers and patterns.
          </p>
        </div>
        <Link href="/upload">
          <Button>
            <Plus className="h-4 w-4" />
            Upload Paper
          </Button>
        </Link>
      </div>

      {/* Filters */}
      <FilterBar
        filters={filters}
        onFiltersChange={setFilters}
        disabled={loading}
      />

      {/* Error */}
      {error && (
        <div className="mt-4 flex items-center gap-2 rounded-lg border border-destructive bg-destructive/10 p-4 text-sm text-destructive">
          <AlertCircle className="h-4 w-4 flex-shrink-0" />
          {error}
        </div>
      )}

      {/* Loading state */}
      {loading && (
        <div className="mt-8 flex items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}

      {/* Empty state */}
      {!loading && papers.length === 0 && (
        <div className="mt-8 rounded-lg border border-dashed border-input p-8 text-center">
          <p className="text-muted-foreground">
            {filters.search || filters.status || filters.type
              ? "No papers match your filters."
              : "No papers yet. Upload your first paper to get started."}
          </p>
          {!filters.search && !filters.status && !filters.type && (
            <Link href="/upload">
              <Button className="mt-4">
                <Plus className="h-4 w-4" />
                Upload Paper
              </Button>
            </Link>
          )}
        </div>
      )}

      {/* Paper list */}
      {!loading && papers.length > 0 && (
        <>
          <div className="mt-6 space-y-3">
            {papers.map((paper) => (
              <PaperCard
                key={paper.id}
                paper={paper}
                onView={() => handleView(paper)}
                onDelete={() => setDeleteTarget(paper)}
                onExtract={() => {
                  setExtractTarget(paper);
                  setExtractError(null);
                }}
              />
            ))}
          </div>

          {/* Load more */}
          {hasMore && (
            <div className="mt-6 flex justify-center">
              <Button
                variant="outline"
                onClick={handleLoadMore}
                disabled={loadingMore}
              >
                {loadingMore ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    Loading...
                  </>
                ) : (
                  `Load More (${papers.length} of ${total})`
                )}
              </Button>
            </div>
          )}
        </>
      )}

      {/* Delete confirmation dialog */}
      <Dialog
        open={deleteTarget !== null}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Paper</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete &quot;{deleteTarget?.title}&quot;?
              This will also delete all associated chunks and cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose asChild>
              <Button variant="outline" disabled={deleting}>
                Cancel
              </Button>
            </DialogClose>
            <Button
              variant="destructive"
              onClick={handleDelete}
              disabled={deleting}
            >
              {deleting ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Deleting...
                </>
              ) : (
                "Delete"
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Extract dialog */}
      <Dialog
        open={extractTarget !== null}
        onOpenChange={(open) => {
          if (!open && !extractingPatterns && !extractingAlgorithms) {
            setExtractTarget(null);
            setExtractError(null);
          }
        }}
      >
        <DialogContent className="space-y-4">
          <DialogHeader>
            <DialogTitle>Extract from Paper</DialogTitle>
            <DialogDescription>
              Use AI to extract structured data from &quot;{extractTarget?.title}&quot;.
              Each extraction takes about a minute.
            </DialogDescription>
          </DialogHeader>
          {extractError && (
            <div className="flex items-center gap-2 rounded-lg border border-destructive bg-destructive/10 p-3 text-sm text-destructive">
              <AlertCircle className="h-4 w-4 flex-shrink-0" />
              {extractError}
            </div>
          )}
          <div className="space-y-3">
            {extractTarget && extractTarget.pattern_count === 0 && (
              <Button
                className="w-full justify-start gap-2"
                variant="outline"
                onClick={handleExtractPatterns}
                disabled={extractingPatterns || extractingAlgorithms}
              >
                {extractingPatterns ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Lightbulb className="h-4 w-4" />
                )}
                {extractingPatterns
                  ? "Extracting patterns..."
                  : "Extract Patterns"}
                <span className="ml-auto text-xs text-muted-foreground">
                  Claim / Evidence / Context
                </span>
              </Button>
            )}
            {extractTarget && extractTarget.algorithm_count === 0 && (
              <Button
                className="w-full justify-start gap-2"
                variant="outline"
                onClick={handleExtractAlgorithms}
                disabled={extractingPatterns || extractingAlgorithms}
              >
                {extractingAlgorithms ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Code2 className="h-4 w-4" />
                )}
                {extractingAlgorithms
                  ? "Extracting algorithms..."
                  : "Extract Algorithms"}
                <span className="ml-auto text-xs text-muted-foreground">
                  Steps / Math / Pseudocode
                </span>
              </Button>
            )}
          </div>
          <DialogFooter>
            <DialogClose asChild>
              <Button
                variant="outline"
                disabled={extractingPatterns || extractingAlgorithms}
              >
                Close
              </Button>
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default function LibraryPage() {
  return (
    <Suspense
      fallback={
        <div className="mx-auto max-w-6xl px-4 py-8">
          <div className="flex items-center justify-center">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        </div>
      }
    >
      <LibraryContent />
    </Suspense>
  );
}
