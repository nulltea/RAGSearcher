"use client";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { getPaper, listAlgorithms } from "@/lib/api";
import type { AlgorithmResponse, PaperResponse } from "@/types";
import { AlertCircle, ArrowLeft, ChevronDown, ChevronUp, Loader2 } from "lucide-react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { Suspense, useEffect, useState } from "react";
import { LatexRenderer } from "@/components/latex-renderer";

function AlgorithmsContent() {
  const searchParams = useSearchParams();
  const paperId = searchParams.get("id") || "";

  const [paper, setPaper] = useState<PaperResponse | null>(null);
  const [algorithms, setAlgorithms] = useState<AlgorithmResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!paperId) return;
    async function load() {
      try {
        setLoading(true);
        const [p, res] = await Promise.all([
          getPaper(paperId),
          listAlgorithms(paperId, "approved"),
        ]);
        setPaper(p);
        setAlgorithms(res.algorithms);
      } catch (err) {
        setError(
          err instanceof Error ? err.message : "Failed to load algorithms",
        );
      } finally {
        setLoading(false);
      }
    }
    load();
  }, [paperId]);

  if (!paperId) {
    return (
      <div className="mx-auto max-w-4xl px-4 py-8">
        <p className="text-muted-foreground">No paper ID provided.</p>
        <Link href="/library">
          <Button variant="outline" className="mt-4">
            <ArrowLeft className="h-4 w-4" />
            Back to Library
          </Button>
        </Link>
      </div>
    );
  }

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
        <h1 className="text-2xl font-bold text-foreground">Algorithms</h1>
        {paper && (
          <p className="mt-1 text-muted-foreground">
            Extracted from: {paper.title}
          </p>
        )}
      </div>

      {algorithms.length === 0 ? (
        <div className="rounded-lg border border-input bg-card p-6 text-center">
          <p className="text-muted-foreground">
            No approved algorithms for this paper.
          </p>
        </div>
      ) : (
        <div className="space-y-6">
          {algorithms.map((algo) => (
            <AlgorithmViewCard key={algo.id} algorithm={algo} />
          ))}
        </div>
      )}
    </div>
  );
}

export default function AlgorithmsPage() {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-[400px] items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <AlgorithmsContent />
    </Suspense>
  );
}

function AlgorithmViewCard({ algorithm }: { algorithm: AlgorithmResponse }) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(["steps"]),
  );

  const toggle = (key: string) => {
    setExpandedSections((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">{algorithm.name}</CardTitle>
        {algorithm.description && (
          <p className="text-sm text-muted-foreground">
            {algorithm.description}
          </p>
        )}
        <div className="mt-2 flex flex-wrap gap-2">
          {algorithm.complexity && (
            <Badge variant="outline">{algorithm.complexity}</Badge>
          )}
          {algorithm.tags.map((tag) => (
            <Badge key={tag} variant="outline">
              {tag}
            </Badge>
          ))}
        </div>
      </CardHeader>
      <CardContent className="space-y-2">
        {/* Steps */}
        <Section
          label={`Steps (${algorithm.steps.length})`}
          sectionKey="steps"
          expanded={expandedSections}
          onToggle={toggle}
        >
          <ol className="space-y-3">
            {algorithm.steps.map((step) => (
              <li key={step.number} className="text-sm">
                <div className="flex gap-2">
                  <span className="font-mono text-muted-foreground">
                    {step.number}.
                  </span>
                  <div className="flex-1">
                    <p className="font-medium text-foreground">
                      <LatexRenderer text={step.action} />
                    </p>
                    {step.details && (
                      <p className="mt-0.5 text-muted-foreground">
                        <LatexRenderer text={step.details} />
                      </p>
                    )}
                    {step.math && (
                      <div className="mt-1 rounded bg-muted px-2 py-1">
                        <LatexRenderer text={step.math} block />
                      </div>
                    )}
                  </div>
                </div>
              </li>
            ))}
          </ol>
        </Section>

        {/* Inputs/Outputs */}
        {(algorithm.inputs.length > 0 || algorithm.outputs.length > 0) && (
          <Section
            label="Inputs / Outputs"
            sectionKey="io"
            expanded={expandedSections}
            onToggle={toggle}
          >
            <div className="space-y-3 text-sm">
              {algorithm.inputs.length > 0 && (
                <div>
                  <p className="mb-1 font-medium text-foreground">Inputs</p>
                  <ul className="space-y-1">
                    {algorithm.inputs.map((io) => (
                      <li key={io.name} className="text-muted-foreground">
                        <span className="font-mono text-foreground">
                          <LatexRenderer text={io.name} />
                        </span>{" "}
                        <span className="text-xs">(<LatexRenderer text={io.type} />)</span>
                        {io.description && <> &mdash; <LatexRenderer text={io.description} /></>}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              {algorithm.outputs.length > 0 && (
                <div>
                  <p className="mb-1 font-medium text-foreground">Outputs</p>
                  <ul className="space-y-1">
                    {algorithm.outputs.map((io) => (
                      <li key={io.name} className="text-muted-foreground">
                        <span className="font-mono text-foreground">
                          <LatexRenderer text={io.name} />
                        </span>{" "}
                        <span className="text-xs">(<LatexRenderer text={io.type} />)</span>
                        {io.description && <> &mdash; <LatexRenderer text={io.description} /></>}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </Section>
        )}

        {/* Preconditions */}
        {algorithm.preconditions.length > 0 && (
          <Section
            label="Preconditions"
            sectionKey="preconditions"
            expanded={expandedSections}
            onToggle={toggle}
          >
            <ul className="list-disc space-y-1 pl-4 text-sm text-foreground">
              {algorithm.preconditions.map((p, i) => (
                <li key={i}><LatexRenderer text={p} /></li>
              ))}
            </ul>
          </Section>
        )}

        {/* Math */}
        {algorithm.mathematical_notation && (
          <Section
            label="Mathematical Notation"
            sectionKey="math"
            expanded={expandedSections}
            onToggle={toggle}
          >
            <div className="rounded bg-muted p-3">
              <LatexRenderer text={algorithm.mathematical_notation} block />
            </div>
          </Section>
        )}

        {/* Pseudocode */}
        {algorithm.pseudocode && (
          <Section
            label="Pseudocode"
            sectionKey="pseudocode"
            expanded={expandedSections}
            onToggle={toggle}
          >
            <pre className="overflow-x-auto rounded bg-muted p-3 font-mono text-sm">
              {algorithm.pseudocode}
            </pre>
          </Section>
        )}
      </CardContent>
    </Card>
  );
}

function Section({
  label,
  sectionKey,
  expanded,
  onToggle,
  children,
}: {
  label: string;
  sectionKey: string;
  expanded: Set<string>;
  onToggle: (key: string) => void;
  children: React.ReactNode;
}) {
  const isExpanded = expanded.has(sectionKey);
  return (
    <div className="rounded-md border border-input">
      <button
        type="button"
        onClick={() => onToggle(sectionKey)}
        className="flex w-full items-center justify-between p-3 text-left text-sm font-medium hover:bg-accent/50"
      >
        <span className="text-muted-foreground">{label}</span>
        {isExpanded ? (
          <ChevronUp className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        )}
      </button>
      {isExpanded && (
        <div className="border-t border-input p-3">{children}</div>
      )}
    </div>
  );
}
