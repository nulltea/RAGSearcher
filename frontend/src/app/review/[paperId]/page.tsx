"use client";

import { Button } from "@/components/ui/button";
import { PatternCard, ReviewSummary } from "@/components/review";
import { getPaper, listPatterns, submitPatternReview } from "@/lib/api";
import type { ExtractedPatternResponse, PatternDecision } from "@/types";
import { AlertCircle, ArrowLeft, CheckCircle, Loader2 } from "lucide-react";
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";

type DecisionState = "pending" | "approved" | "rejected";

interface ReviewData {
  paperTitle: string;
  patterns: ExtractedPatternResponse[];
}

export default function ReviewPage() {
  const params = useParams();
  const router = useRouter();
  const paperId = params.paperId as string;

  const [data, setData] = useState<ReviewData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [decisions, setDecisions] = useState<Map<string, DecisionState>>(
    new Map(),
  );
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitResult, setSubmitResult] = useState<{
    approved: number;
    rejected: number;
  } | null>(null);

  useEffect(() => {
    async function loadData() {
      try {
        setLoading(true);
        setError(null);

        const [paper, patternsResponse] = await Promise.all([
          getPaper(paperId),
          listPatterns(paperId, "pending"),
        ]);

        const patterns: ExtractedPatternResponse[] =
          patternsResponse.patterns.map((p) => ({
            temp_id: p.id,
            name: p.name,
            claim: p.claim,
            evidence: p.evidence,
            context: p.context,
            tags: p.tags,
            confidence: (p.confidence as "high" | "medium" | "low") || "medium",
          }));

        setData({ paperTitle: paper.title, patterns });

        const initialDecisions = new Map<string, DecisionState>();
        patterns.forEach((p) => initialDecisions.set(p.temp_id, "pending"));
        setDecisions(initialDecisions);
      } catch (err) {
        setError(
          err instanceof Error ? err.message : "Failed to load paper data",
        );
      } finally {
        setLoading(false);
      }
    }

    loadData();
  }, [paperId]);

  const handleApprove = useCallback((tempId: string) => {
    setDecisions((prev) => {
      const next = new Map(prev);
      next.set(tempId, next.get(tempId) === "approved" ? "pending" : "approved");
      return next;
    });
  }, []);

  const handleReject = useCallback((tempId: string) => {
    setDecisions((prev) => {
      const next = new Map(prev);
      next.set(tempId, next.get(tempId) === "rejected" ? "pending" : "rejected");
      return next;
    });
  }, []);

  const handleApproveAll = useCallback(() => {
    setDecisions((prev) => {
      const next = new Map(prev);
      prev.forEach((_, key) => next.set(key, "approved"));
      return next;
    });
  }, []);

  const handleRejectAll = useCallback(() => {
    setDecisions((prev) => {
      const next = new Map(prev);
      prev.forEach((_, key) => next.set(key, "rejected"));
      return next;
    });
  }, []);

  const counts = useMemo(() => {
    let approved = 0;
    let rejected = 0;
    decisions.forEach((state) => {
      if (state === "approved") approved++;
      if (state === "rejected") rejected++;
    });
    return { approved, rejected };
  }, [decisions]);

  const handleSubmit = useCallback(async () => {
    if (!data) return;

    const decisionList: PatternDecision[] = [];
    decisions.forEach((state, tempId) => {
      if (state !== "pending") {
        decisionList.push({
          pattern_id: tempId,
          approved: state === "approved",
        });
      }
    });

    if (decisionList.length === 0) {
      setError("Please approve or reject at least one pattern");
      return;
    }

    setIsSubmitting(true);
    setError(null);

    try {
      const result = await submitPatternReview(paperId, decisionList);
      setSubmitResult({
        approved: result.approved_count,
        rejected: result.rejected_count,
      });
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to submit review",
      );
    } finally {
      setIsSubmitting(false);
    }
  }, [data, decisions, paperId]);

  if (loading) {
    return (
      <div className="flex min-h-[400px] items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error && !data) {
    return (
      <div className="mx-auto max-w-2xl px-4 py-8">
        <div className="flex items-center gap-2 rounded-lg border border-destructive bg-destructive/10 p-4 text-destructive">
          <AlertCircle className="h-5 w-5 flex-shrink-0" />
          <div>
            <p className="font-medium">Error loading paper</p>
            <p className="text-sm">{error}</p>
          </div>
        </div>
        <div className="mt-4">
          <Link href="/library">
            <Button variant="outline">
              <ArrowLeft className="h-4 w-4" />
              Back to Library
            </Button>
          </Link>
        </div>
      </div>
    );
  }

  if (submitResult) {
    return (
      <div className="mx-auto max-w-2xl px-4 py-8">
        <div className="rounded-lg border border-green-500 bg-green-50 p-6 dark:bg-green-950/20">
          <div className="flex items-center gap-3">
            <CheckCircle className="h-8 w-8 text-green-600" />
            <div>
              <h2 className="text-lg font-semibold text-foreground">
                Review Complete
              </h2>
              <p className="text-muted-foreground">
                {submitResult.approved} pattern
                {submitResult.approved !== 1 ? "s" : ""} approved,{" "}
                {submitResult.rejected} rejected
              </p>
            </div>
          </div>
          <div className="mt-6 flex gap-3">
            <Button onClick={() => router.push("/library")}>
              Go to Library
            </Button>
            <Button variant="outline" onClick={() => router.push("/upload")}>
              Upload Another
            </Button>
          </div>
        </div>
      </div>
    );
  }

  if (!data || data.patterns.length === 0) {
    return (
      <div className="mx-auto max-w-2xl px-4 py-8">
        <div className="rounded-lg border border-input bg-card p-6 text-center">
          <p className="text-muted-foreground">
            No pending patterns to review for this paper.
          </p>
          <div className="mt-4">
            <Link href="/library">
              <Button variant="outline">
                <ArrowLeft className="h-4 w-4" />
                Back to Library
              </Button>
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-4xl px-4 py-8">
      <div className="mb-6">
        <Link
          href="/library"
          className="mb-4 inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to Library
        </Link>
        <h1 className="text-2xl font-bold text-foreground">Review Patterns</h1>
        <p className="mt-1 text-muted-foreground">
          Approve or reject extracted patterns to add them to your knowledge
          base.
        </p>
      </div>

      <ReviewSummary
        paperTitle={data.paperTitle}
        totalPatterns={data.patterns.length}
        approvedCount={counts.approved}
        rejectedCount={counts.rejected}
        onApproveAll={handleApproveAll}
        onRejectAll={handleRejectAll}
        disabled={isSubmitting}
      />

      {error && (
        <div className="mt-4 flex items-center gap-2 rounded-lg border border-destructive bg-destructive/10 p-4 text-sm text-destructive">
          <AlertCircle className="h-4 w-4 flex-shrink-0" />
          {error}
        </div>
      )}

      <div className="mt-6 space-y-4">
        {data.patterns.map((pattern) => (
          <PatternCard
            key={pattern.temp_id}
            pattern={pattern}
            decision={decisions.get(pattern.temp_id) || "pending"}
            onApprove={() => handleApprove(pattern.temp_id)}
            onReject={() => handleReject(pattern.temp_id)}
            disabled={isSubmitting}
          />
        ))}
      </div>

      <div className="sticky bottom-0 mt-6 border-t border-input bg-background py-4">
        <div className="flex items-center justify-between">
          <p className="text-sm text-muted-foreground">
            {counts.approved + counts.rejected} of {data.patterns.length}{" "}
            patterns reviewed
          </p>
          <Button
            onClick={handleSubmit}
            disabled={isSubmitting || counts.approved + counts.rejected === 0}
            size="lg"
          >
            {isSubmitting ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Submitting...
              </>
            ) : (
              "Submit Review"
            )}
          </Button>
        </div>
      </div>
    </div>
  );
}
