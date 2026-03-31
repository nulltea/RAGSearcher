"use client";

import { Button } from "@/components/ui/button";
import { AlgorithmCard, PatternCard, ReviewSummary } from "@/components/review";
import {
  getPaper,
  listAlgorithms,
  listPatterns,
  submitAlgorithmReview,
  submitPatternReview,
} from "@/lib/api";
import type {
  AlgorithmResponse,
  ExtractedPatternResponse,
  PatternDecision,
} from "@/types";
import { AlertCircle, ArrowLeft, CheckCircle, Loader2 } from "lucide-react";
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";

type DecisionState = "pending" | "approved" | "rejected";
type Tab = "patterns" | "algorithms";

export default function ReviewPage() {
  const params = useParams();
  const router = useRouter();
  const paperId = params.paperId as string;

  const [paperTitle, setPaperTitle] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("patterns");

  // Patterns state
  const [patterns, setPatterns] = useState<ExtractedPatternResponse[]>([]);
  const [patternDecisions, setPatternDecisions] = useState<
    Map<string, DecisionState>
  >(new Map());

  // Algorithms state
  const [algorithms, setAlgorithms] = useState<AlgorithmResponse[]>([]);
  const [algorithmDecisions, setAlgorithmDecisions] = useState<
    Map<string, DecisionState>
  >(new Map());

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitResult, setSubmitResult] = useState<{
    approvedPatterns: number;
    rejectedPatterns: number;
    approvedAlgorithms: number;
    rejectedAlgorithms: number;
  } | null>(null);

  useEffect(() => {
    async function loadData() {
      try {
        setLoading(true);
        setError(null);

        const [paper, patternsRes, algorithmsRes] = await Promise.all([
          getPaper(paperId),
          listPatterns(paperId, "pending"),
          listAlgorithms(paperId, "pending"),
        ]);

        setPaperTitle(paper.title);

        const mappedPatterns: ExtractedPatternResponse[] =
          patternsRes.patterns.map((p) => ({
            temp_id: p.id,
            name: p.name,
            claim: p.claim,
            evidence: p.evidence,
            context: p.context,
            tags: p.tags,
            confidence:
              (p.confidence as "high" | "medium" | "low") || "medium",
          }));
        setPatterns(mappedPatterns);

        const pDec = new Map<string, DecisionState>();
        mappedPatterns.forEach((p) => pDec.set(p.temp_id, "pending"));
        setPatternDecisions(pDec);

        setAlgorithms(algorithmsRes.algorithms);
        const aDec = new Map<string, DecisionState>();
        algorithmsRes.algorithms.forEach((a) => aDec.set(a.id, "pending"));
        setAlgorithmDecisions(aDec);

        // Auto-select tab based on what's available
        if (mappedPatterns.length === 0 && algorithmsRes.algorithms.length > 0) {
          setActiveTab("algorithms");
        }
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

  // Pattern decision handlers
  const handlePatternApprove = useCallback((id: string) => {
    setPatternDecisions((prev) => {
      const next = new Map(prev);
      next.set(id, next.get(id) === "approved" ? "pending" : "approved");
      return next;
    });
  }, []);

  const handlePatternReject = useCallback((id: string) => {
    setPatternDecisions((prev) => {
      const next = new Map(prev);
      next.set(id, next.get(id) === "rejected" ? "pending" : "rejected");
      return next;
    });
  }, []);

  // Algorithm decision handlers
  const handleAlgorithmApprove = useCallback((id: string) => {
    setAlgorithmDecisions((prev) => {
      const next = new Map(prev);
      next.set(id, next.get(id) === "approved" ? "pending" : "approved");
      return next;
    });
  }, []);

  const handleAlgorithmReject = useCallback((id: string) => {
    setAlgorithmDecisions((prev) => {
      const next = new Map(prev);
      next.set(id, next.get(id) === "rejected" ? "pending" : "rejected");
      return next;
    });
  }, []);

  // Bulk actions for active tab
  const handleApproveAll = useCallback(() => {
    if (activeTab === "patterns") {
      setPatternDecisions((prev) => {
        const next = new Map(prev);
        prev.forEach((_, k) => next.set(k, "approved"));
        return next;
      });
    } else {
      setAlgorithmDecisions((prev) => {
        const next = new Map(prev);
        prev.forEach((_, k) => next.set(k, "approved"));
        return next;
      });
    }
  }, [activeTab]);

  const handleRejectAll = useCallback(() => {
    if (activeTab === "patterns") {
      setPatternDecisions((prev) => {
        const next = new Map(prev);
        prev.forEach((_, k) => next.set(k, "rejected"));
        return next;
      });
    } else {
      setAlgorithmDecisions((prev) => {
        const next = new Map(prev);
        prev.forEach((_, k) => next.set(k, "rejected"));
        return next;
      });
    }
  }, [activeTab]);

  const patternCounts = useMemo(() => {
    let approved = 0,
      rejected = 0;
    patternDecisions.forEach((s) => {
      if (s === "approved") approved++;
      if (s === "rejected") rejected++;
    });
    return { approved, rejected };
  }, [patternDecisions]);

  const algorithmCounts = useMemo(() => {
    let approved = 0,
      rejected = 0;
    algorithmDecisions.forEach((s) => {
      if (s === "approved") approved++;
      if (s === "rejected") rejected++;
    });
    return { approved, rejected };
  }, [algorithmDecisions]);

  const activeCounts =
    activeTab === "patterns" ? patternCounts : algorithmCounts;
  const activeTotal =
    activeTab === "patterns" ? patterns.length : algorithms.length;

  const handleSubmit = useCallback(async () => {
    setIsSubmitting(true);
    setError(null);

    try {
      let ap = 0, rp = 0, aa = 0, ra = 0;

      // Submit pattern decisions
      const patternDecs: PatternDecision[] = [];
      patternDecisions.forEach((state, id) => {
        if (state !== "pending") {
          patternDecs.push({ pattern_id: id, approved: state === "approved" });
        }
      });
      if (patternDecs.length > 0) {
        const res = await submitPatternReview(paperId, patternDecs);
        ap = res.approved_count;
        rp = res.rejected_count;
      }

      // Submit algorithm decisions
      const algoDecs: PatternDecision[] = [];
      algorithmDecisions.forEach((state, id) => {
        if (state !== "pending") {
          algoDecs.push({ pattern_id: id, approved: state === "approved" });
        }
      });
      if (algoDecs.length > 0) {
        const res = await submitAlgorithmReview(paperId, algoDecs);
        aa = res.approved_count;
        ra = res.rejected_count;
      }

      if (patternDecs.length === 0 && algoDecs.length === 0) {
        setError("Please approve or reject at least one item");
        return;
      }

      setSubmitResult({
        approvedPatterns: ap,
        rejectedPatterns: rp,
        approvedAlgorithms: aa,
        rejectedAlgorithms: ra,
      });
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to submit review",
      );
    } finally {
      setIsSubmitting(false);
    }
  }, [patternDecisions, algorithmDecisions, paperId]);

  if (loading) {
    return (
      <div className="flex min-h-[400px] items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error && patterns.length === 0 && algorithms.length === 0) {
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
    const totalApproved =
      submitResult.approvedPatterns + submitResult.approvedAlgorithms;
    const totalRejected =
      submitResult.rejectedPatterns + submitResult.rejectedAlgorithms;
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
                {totalApproved} approved, {totalRejected} rejected
              </p>
              {submitResult.approvedPatterns > 0 && (
                <p className="text-sm text-muted-foreground">
                  Patterns: {submitResult.approvedPatterns} approved,{" "}
                  {submitResult.rejectedPatterns} rejected
                </p>
              )}
              {submitResult.approvedAlgorithms > 0 && (
                <p className="text-sm text-muted-foreground">
                  Algorithms: {submitResult.approvedAlgorithms} approved,{" "}
                  {submitResult.rejectedAlgorithms} rejected
                </p>
              )}
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

  if (patterns.length === 0 && algorithms.length === 0) {
    return (
      <div className="mx-auto max-w-2xl px-4 py-8">
        <div className="rounded-lg border border-input bg-card p-6 text-center">
          <p className="text-muted-foreground">
            No pending items to review for this paper.
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

  const hasPatterns = patterns.length > 0;
  const hasAlgorithms = algorithms.length > 0;
  const showTabs = hasPatterns && hasAlgorithms;

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
        <h1 className="text-2xl font-bold text-foreground">Review</h1>
        <p className="mt-1 text-muted-foreground">
          Approve or reject extracted items for: {paperTitle}
        </p>
      </div>

      {/* Tabs */}
      {showTabs && (
        <div className="mb-4 flex gap-1 rounded-lg border border-input bg-card p-1">
          <button
            type="button"
            onClick={() => setActiveTab("patterns")}
            className={`flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
              activeTab === "patterns"
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            Patterns ({patterns.length})
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("algorithms")}
            className={`flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
              activeTab === "algorithms"
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            Algorithms ({algorithms.length})
          </button>
        </div>
      )}

      <ReviewSummary
        paperTitle={paperTitle}
        totalPatterns={activeTotal}
        approvedCount={activeCounts.approved}
        rejectedCount={activeCounts.rejected}
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
        {activeTab === "patterns"
          ? patterns.map((pattern) => (
              <PatternCard
                key={pattern.temp_id}
                pattern={pattern}
                decision={patternDecisions.get(pattern.temp_id) || "pending"}
                onApprove={() => handlePatternApprove(pattern.temp_id)}
                onReject={() => handlePatternReject(pattern.temp_id)}
                disabled={isSubmitting}
              />
            ))
          : algorithms.map((algo) => (
              <AlgorithmCard
                key={algo.id}
                algorithm={algo}
                decision={algorithmDecisions.get(algo.id) || "pending"}
                onApprove={() => handleAlgorithmApprove(algo.id)}
                onReject={() => handleAlgorithmReject(algo.id)}
                disabled={isSubmitting}
              />
            ))}
      </div>

      <div className="sticky bottom-0 mt-6 border-t border-input bg-background py-4">
        <div className="flex items-center justify-between">
          <p className="text-sm text-muted-foreground">
            {patternCounts.approved + patternCounts.rejected +
              algorithmCounts.approved + algorithmCounts.rejected}{" "}
            of {patterns.length + algorithms.length} items reviewed
          </p>
          <Button
            onClick={handleSubmit}
            disabled={
              isSubmitting ||
              patternCounts.approved +
                patternCounts.rejected +
                algorithmCounts.approved +
                algorithmCounts.rejected ===
                0
            }
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
