"use client";

import type { PaperType } from "@/types";
import { ChevronDown, ChevronUp } from "lucide-react";
import { useState } from "react";
import { Input } from "../ui/input";
import { Select, type SelectOption } from "../ui/select";

const PAPER_TYPE_OPTIONS: SelectOption[] = [
  { value: "research_paper", label: "Research Paper" },
  { value: "blog_post", label: "Blog Post" },
  { value: "article", label: "Article" },
  { value: "technical_report", label: "Technical Report" },
  { value: "book_chapter", label: "Book Chapter" },
];

export interface MetadataFormData {
  title: string;
  authors: string;
  source: string;
  publishedDate: string;
  type: PaperType;
}

export interface MetadataFormProps {
  data: MetadataFormData;
  onChange: (data: MetadataFormData) => void;
  disabled?: boolean;
}

export function MetadataForm({ data, onChange, disabled }: MetadataFormProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const updateField = <K extends keyof MetadataFormData>(
    field: K,
    value: MetadataFormData[K]
  ) => {
    onChange({ ...data, [field]: value });
  };

  return (
    <div className="rounded-lg border border-input bg-card">
      <button
        type="button"
        onClick={() => setIsExpanded(!isExpanded)}
        disabled={disabled}
        className="flex w-full items-center justify-between p-4 text-left transition-colors hover:bg-accent/50 disabled:cursor-not-allowed disabled:opacity-50"
      >
        <div>
          <h3 className="font-medium text-foreground">Metadata (Optional)</h3>
          <p className="text-sm text-muted-foreground">
            Override auto-extracted metadata
          </p>
        </div>
        {isExpanded ? (
          <ChevronUp className="h-5 w-5 text-muted-foreground" />
        ) : (
          <ChevronDown className="h-5 w-5 text-muted-foreground" />
        )}
      </button>

      {isExpanded && (
        <div className="space-y-4 border-t border-input p-4">
          <Input
            label="Title"
            value={data.title}
            onChange={(e) => updateField("title", e.target.value)}
            placeholder="Paper title (auto-extracted if empty)"
            disabled={disabled}
          />

          <Input
            label="Authors"
            value={data.authors}
            onChange={(e) => updateField("authors", e.target.value)}
            placeholder="Comma-separated author names"
            disabled={disabled}
          />

          <Input
            label="Source URL"
            type="url"
            value={data.source}
            onChange={(e) => updateField("source", e.target.value)}
            placeholder="https://example.com/paper"
            disabled={disabled}
          />

          <Input
            label="Published Date"
            type="date"
            value={data.publishedDate}
            onChange={(e) => updateField("publishedDate", e.target.value)}
            disabled={disabled}
          />

          <Select
            label="Document Type"
            value={data.type}
            onChange={(e) => updateField("type", e.target.value as PaperType)}
            options={PAPER_TYPE_OPTIONS}
            disabled={disabled}
          />
        </div>
      )}
    </div>
  );
}
