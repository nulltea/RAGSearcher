"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Check, X } from "lucide-react";

export interface ReviewSummaryProps {
  paperTitle: string;
  totalPatterns: number;
  approvedCount: number;
  rejectedCount: number;
  onApproveAll: () => void;
  onRejectAll: () => void;
  disabled?: boolean;
}

export function ReviewSummary({
  paperTitle,
  totalPatterns,
  approvedCount,
  rejectedCount,
  onApproveAll,
  onRejectAll,
  disabled,
}: ReviewSummaryProps) {
  const pendingCount = totalPatterns - approvedCount - rejectedCount;

  return (
    <div className="rounded-lg border border-input bg-card p-4">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">
            {paperTitle}
          </h2>
          <div className="mt-2 flex flex-wrap gap-2">
            <Badge variant="outline">{totalPatterns} patterns</Badge>
            {approvedCount > 0 && (
              <Badge variant="success">{approvedCount} approved</Badge>
            )}
            {rejectedCount > 0 && (
              <Badge variant="destructive">{rejectedCount} rejected</Badge>
            )}
            {pendingCount > 0 && (
              <Badge variant="secondary">{pendingCount} pending</Badge>
            )}
          </div>
        </div>
        <div className="flex gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={onApproveAll}
            disabled={disabled || approvedCount === totalPatterns}
          >
            <Check className="h-4 w-4" />
            Approve All
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={onRejectAll}
            disabled={disabled || rejectedCount === totalPatterns}
          >
            <X className="h-4 w-4" />
            Reject All
          </Button>
        </div>
      </div>
    </div>
  );
}
