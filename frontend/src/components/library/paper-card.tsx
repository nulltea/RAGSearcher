"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import type { PaperResponse, PaperStatus } from "@/types";
import { Calendar, Code2, FileText, Lightbulb, Search, Sparkles, Trash2, User } from "lucide-react";
import Link from "next/link";

const STATUS_VARIANTS: Record<PaperStatus, "default" | "secondary" | "success" | "warning"> = {
  processing: "warning",
  ready_for_review: "secondary",
  active: "success",
  archived: "default",
};

const STATUS_LABELS: Record<PaperStatus, string> = {
  processing: "Processing",
  ready_for_review: "Ready for Review",
  active: "Active",
  archived: "Archived",
};

export interface PaperCardProps {
  paper: PaperResponse;
  onView: () => void;
  onDelete: () => void;
  onExtract: () => void;
  disabled?: boolean;
}

export function PaperCard({ paper, onView, onDelete, onExtract, disabled }: PaperCardProps) {
  const needsExtraction = paper.pattern_count === 0 || paper.algorithm_count === 0;
  const formattedDate = paper.published_date
    ? new Date(paper.published_date).toLocaleDateString()
    : null;

  return (
    <Card className="transition-colors hover:bg-accent/50">
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0 flex-1">
            {/* Title */}
            <h3 className="truncate font-semibold text-foreground">
              {paper.title}
            </h3>

            {/* Metadata */}
            <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-muted-foreground">
              {paper.authors.length > 0 && (
                <span className="flex items-center gap-1">
                  <User className="h-3.5 w-3.5" />
                  {paper.authors.slice(0, 2).join(", ")}
                  {paper.authors.length > 2 && ` +${paper.authors.length - 2}`}
                </span>
              )}
              {formattedDate && (
                <span className="flex items-center gap-1">
                  <Calendar className="h-3.5 w-3.5" />
                  {formattedDate}
                </span>
              )}
              <span className="flex items-center gap-1">
                <FileText className="h-3.5 w-3.5" />
                {paper.chunk_count} chunk{paper.chunk_count !== 1 ? "s" : ""}
              </span>
            </div>

            {/* Badges */}
            <div className="mt-3 flex flex-wrap gap-2">
              <Badge variant={STATUS_VARIANTS[paper.status]}>
                {STATUS_LABELS[paper.status]}
              </Badge>
              <Badge variant="outline">{formatPaperType(paper.paper_type)}</Badge>
            </div>
          </div>

          {/* Actions */}
          <div className="flex gap-2">
            {paper.status === "ready_for_review" && (
              <Button
                size="sm"
                variant="primary"
                onClick={onView}
                disabled={disabled}
              >
                Review
              </Button>
            )}
            <Link href={`/search?paper_id=${paper.id}`}>
              <Button size="sm" variant="outline" disabled={disabled}>
                <Search className="h-4 w-4" />
                Search
              </Button>
            </Link>
            {needsExtraction && paper.status !== "processing" && (
              <Button
                size="sm"
                variant="outline"
                onClick={onExtract}
                disabled={disabled}
              >
                <Sparkles className="h-4 w-4" />
                Extract
              </Button>
            )}
            {paper.pattern_count > 0 && (
              <Link href={`/paper/patterns?id=${paper.id}`}>
                <Button size="sm" variant="outline" disabled={disabled}>
                  <Lightbulb className="h-4 w-4" />
                  Patterns
                </Button>
              </Link>
            )}
            {paper.algorithm_count > 0 && (
              <Link href={`/paper/algorithms?id=${paper.id}`}>
                <Button size="sm" variant="outline" disabled={disabled}>
                  <Code2 className="h-4 w-4" />
                  Algorithms
                </Button>
              </Link>
            )}
            <Button
              size="sm"
              variant="ghost"
              onClick={onDelete}
              disabled={disabled}
              className="text-destructive hover:bg-destructive/10 hover:text-destructive"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function formatPaperType(type: string): string {
  return type
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}
