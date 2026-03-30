"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { Confidence, ExtractedPatternResponse } from "@/types";
import { Check, ChevronDown, ChevronUp, X } from "lucide-react";
import { useState } from "react";

type DecisionState = "pending" | "approved" | "rejected";

export interface PatternCardProps {
  pattern: ExtractedPatternResponse;
  decision: DecisionState;
  onApprove: () => void;
  onReject: () => void;
  disabled?: boolean;
}

const CONFIDENCE_VARIANTS: Record<
  Confidence,
  "success" | "warning" | "secondary"
> = {
  high: "success",
  medium: "warning",
  low: "secondary",
};

export function PatternCard({
  pattern,
  decision,
  onApprove,
  onReject,
  disabled,
}: PatternCardProps) {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(["claim"]),
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

  const sections = [
    { key: "claim", label: "Claim", content: pattern.claim },
    { key: "evidence", label: "Evidence", content: pattern.evidence },
    { key: "context", label: "Context", content: pattern.context },
  ].filter((s) => s.content);

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
            <h3 className="font-semibold text-foreground">{pattern.name}</h3>
            <div className="mt-2 flex flex-wrap gap-2">
              <Badge variant={CONFIDENCE_VARIANTS[pattern.confidence]}>
                {pattern.confidence} confidence
              </Badge>
              {pattern.tags.map((tag) => (
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
        {sections.map(({ key, label, content }) => (
          <div key={key} className="rounded-md border border-input">
            <button
              type="button"
              onClick={() => toggleSection(key)}
              className="flex w-full items-center justify-between p-3 text-left text-sm font-medium hover:bg-accent/50"
            >
              <span className="text-muted-foreground">{label}</span>
              {expandedSections.has(key) ? (
                <ChevronUp className="h-4 w-4 text-muted-foreground" />
              ) : (
                <ChevronDown className="h-4 w-4 text-muted-foreground" />
              )}
            </button>
            {expandedSections.has(key) && (
              <div className="border-t border-input p-3">
                <p className="whitespace-pre-wrap text-sm text-foreground">
                  {content}
                </p>
              </div>
            )}
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
