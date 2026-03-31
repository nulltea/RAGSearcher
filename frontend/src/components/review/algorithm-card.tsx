"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { AlgorithmResponse } from "@/types";
import { Check, ChevronDown, ChevronUp, X } from "lucide-react";
import { useState } from "react";
import { LatexBlock, LatexRenderer } from "@/components/latex-renderer";

type DecisionState = "pending" | "approved" | "rejected";

export interface AlgorithmCardProps {
  algorithm: AlgorithmResponse;
  decision: DecisionState;
  onApprove: () => void;
  onReject: () => void;
  disabled?: boolean;
}

export function AlgorithmCard({
  algorithm,
  decision,
  onApprove,
  onReject,
  disabled,
}: AlgorithmCardProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(["steps"]),
  );

  const toggleSection = (section: string) => {
    setExpandedSections((prev) => {
      const next = new Set(prev);
      if (next.has(section)) {
        next.delete(section);
      } else {
        next.add(section);
      }
      return next;
    });
  };

  const confidenceVariant =
    algorithm.confidence === "high"
      ? "success"
      : algorithm.confidence === "low"
        ? "secondary"
        : "warning";

  return (
    <Card
      className={cn(
        "transition-colors",
        decision === "approved" &&
          "border-green-500 bg-green-50 dark:bg-green-950/20",
        decision === "rejected" &&
          "border-red-500 bg-red-50 dark:bg-red-950/20",
      )}
    >
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1">
            <h3 className="font-semibold text-foreground">{algorithm.name}</h3>
            {algorithm.description && (
              <p className="mt-1 text-sm text-muted-foreground">
                {algorithm.description}
              </p>
            )}
            <div className="mt-2 flex flex-wrap gap-2">
              <Badge variant={confidenceVariant}>
                {algorithm.confidence} confidence
              </Badge>
              {algorithm.complexity && (
                <Badge variant="outline">{algorithm.complexity}</Badge>
              )}
              {algorithm.tags.map((tag) => (
                <Badge key={tag} variant="outline">
                  {tag}
                </Badge>
              ))}
            </div>
          </div>
          <div className="flex gap-2">
            <Button
              size="sm"
              variant={decision === "approved" ? "primary" : "outline"}
              onClick={onApprove}
              disabled={disabled}
              className={cn(
                decision === "approved" &&
                  "bg-green-600 hover:bg-green-700 text-white",
              )}
            >
              <Check className="h-4 w-4" />
              {decision === "approved" ? "Approved" : "Approve"}
            </Button>
            <Button
              size="sm"
              variant={decision === "rejected" ? "destructive" : "outline"}
              onClick={onReject}
              disabled={disabled}
            >
              <X className="h-4 w-4" />
              {decision === "rejected" ? "Rejected" : "Reject"}
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-2">
        {/* Steps */}
        <CollapsibleSection
          label={`Steps (${algorithm.steps.length})`}
          sectionKey="steps"
          expanded={expandedSections}
          onToggle={toggleSection}
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
        </CollapsibleSection>

        {/* Inputs/Outputs */}
        {(algorithm.inputs.length > 0 || algorithm.outputs.length > 0) && (
          <CollapsibleSection
            label="Inputs / Outputs"
            sectionKey="io"
            expanded={expandedSections}
            onToggle={toggleSection}
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
          </CollapsibleSection>
        )}

        {/* Preconditions */}
        {algorithm.preconditions.length > 0 && (
          <CollapsibleSection
            label="Preconditions"
            sectionKey="preconditions"
            expanded={expandedSections}
            onToggle={toggleSection}
          >
            <ul className="list-disc space-y-1 pl-4 text-sm text-foreground">
              {algorithm.preconditions.map((p, i) => (
                <li key={i}><LatexRenderer text={p} /></li>
              ))}
            </ul>
          </CollapsibleSection>
        )}

        {/* Math notation */}
        {algorithm.mathematical_notation && (
          <CollapsibleSection
            label="Mathematical Notation"
            sectionKey="math"
            expanded={expandedSections}
            onToggle={toggleSection}
          >
            <div className="rounded bg-muted p-3">
              <LatexRenderer text={algorithm.mathematical_notation} block />
            </div>
          </CollapsibleSection>
        )}

        {/* Pseudocode */}
        {algorithm.pseudocode && (
          <CollapsibleSection
            label="Pseudocode"
            sectionKey="pseudocode"
            expanded={expandedSections}
            onToggle={toggleSection}
          >
            <pre className="overflow-x-auto rounded bg-muted p-3 font-mono text-sm">
              {algorithm.pseudocode}
            </pre>
          </CollapsibleSection>
        )}
      </CardContent>
    </Card>
  );
}

function CollapsibleSection({
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
