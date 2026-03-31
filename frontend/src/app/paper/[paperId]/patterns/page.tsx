"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { LatexRenderer } from "@/components/latex-renderer";
import { getPaper, listPatterns } from "@/lib/api";
import type { PaperResponse, PatternResponse } from "@/types";
import { AlertCircle, ArrowLeft, Loader2 } from "lucide-react";
import Link from "next/link";
import { useParams } from "next/navigation";
import { useEffect, useState } from "react";

export default function PatternsPage() {
  const params = useParams();
  const paperId = params.paperId as string;

  const [paper, setPaper] = useState<PaperResponse | null>(null);
  const [patterns, setPatterns] = useState<PatternResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      try {
        setLoading(true);
        const [p, res] = await Promise.all([
          getPaper(paperId),
          listPatterns(paperId),
        ]);
        setPaper(p);
        setPatterns(res.patterns);
      } catch (err) {
        setError(
          err instanceof Error ? err.message : "Failed to load patterns",
        );
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [paperId]);

  if (loading) {
    return (
      <div className="flex min-h-[400px] items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="mx-auto max-w-4xl px-4 py-8">
        <div className="flex items-center gap-2 rounded-lg border border-destructive bg-destructive/10 p-4 text-destructive">
          <AlertCircle className="h-5 w-5 flex-shrink-0" />
          <p>{error}</p>
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
        <h1 className="text-2xl font-bold text-foreground">Patterns</h1>
        {paper && (
          <p className="mt-1 text-muted-foreground">
            Extracted from: {paper.title}
          </p>
        )}
      </div>

      {patterns.length === 0 ? (
        <div className="rounded-lg border border-input bg-card p-6 text-center">
          <p className="text-muted-foreground">
            No patterns found for this paper.
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {patterns.map((pattern) => (
            <PatternViewCard key={pattern.id} pattern={pattern} />
          ))}
        </div>
      )}
    </div>
  );
}

const CONFIDENCE_VARIANT: Record<string, "success" | "warning" | "secondary"> = {
  high: "success",
  medium: "warning",
  low: "secondary",
};

const STATUS_VARIANT: Record<string, "success" | "secondary" | "destructive"> = {
  approved: "success",
  pending: "secondary",
  rejected: "destructive",
};

function PatternViewCard({ pattern }: { pattern: PatternResponse }) {
  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-2">
          <h3 className="font-semibold text-foreground">{pattern.name}</h3>
          <div className="flex gap-2">
            <Badge variant={STATUS_VARIANT[pattern.status] || "secondary"}>
              {pattern.status}
            </Badge>
            <Badge variant={CONFIDENCE_VARIANT[pattern.confidence] || "secondary"}>
              {pattern.confidence}
            </Badge>
          </div>
        </div>

        {pattern.tags.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-1">
            {pattern.tags.map((tag) => (
              <Badge key={tag} variant="outline">
                {tag}
              </Badge>
            ))}
          </div>
        )}

        <div className="mt-3 space-y-2 text-sm">
          {pattern.claim && (
            <div>
              <span className="font-medium text-muted-foreground">Claim: </span>
              <span className="text-foreground">
                <LatexRenderer text={pattern.claim} />
              </span>
            </div>
          )}
          {pattern.evidence && (
            <div>
              <span className="font-medium text-muted-foreground">Evidence: </span>
              <span className="text-foreground">
                <LatexRenderer text={pattern.evidence} />
              </span>
            </div>
          )}
          {pattern.context && (
            <div>
              <span className="font-medium text-muted-foreground">Context: </span>
              <span className="text-foreground">
                <LatexRenderer text={pattern.context} />
              </span>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
