"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, type SelectOption } from "@/components/ui/select";
import type { PaperStatus, PaperType } from "@/types";
import { Search, X } from "lucide-react";

const STATUS_OPTIONS: SelectOption[] = [
  { value: "", label: "All Statuses" },
  { value: "processing", label: "Processing" },
  { value: "active", label: "Active" },
  { value: "archived", label: "Archived" },
];

const TYPE_OPTIONS: SelectOption[] = [
  { value: "", label: "All Types" },
  { value: "research_paper", label: "Research Paper" },
  { value: "blog_post", label: "Blog Post" },
  { value: "article", label: "Article" },
  { value: "technical_report", label: "Technical Report" },
  { value: "book_chapter", label: "Book Chapter" },
];

export interface FilterState {
  search: string;
  status: PaperStatus | "";
  type: PaperType | "";
}

export interface FilterBarProps {
  filters: FilterState;
  onFiltersChange: (filters: FilterState) => void;
  disabled?: boolean;
}

export function FilterBar({ filters, onFiltersChange, disabled }: FilterBarProps) {
  const hasFilters = filters.search || filters.status || filters.type;

  const handleClearFilters = () => {
    onFiltersChange({ search: "", status: "", type: "" });
  };

  return (
    <div className="flex flex-col gap-4 rounded-lg border border-input bg-card p-4 sm:flex-row sm:items-end">
      <div className="flex-1">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={filters.search}
            onChange={(e) =>
              onFiltersChange({ ...filters, search: e.target.value })
            }
            placeholder="Search papers..."
            disabled={disabled}
            className="pl-9"
          />
        </div>
      </div>
      <div className="flex gap-2">
        <Select
          value={filters.status}
          onChange={(e) =>
            onFiltersChange({
              ...filters,
              status: e.target.value as PaperStatus | "",
            })
          }
          options={STATUS_OPTIONS}
          disabled={disabled}
          className="w-[160px]"
        />
        <Select
          value={filters.type}
          onChange={(e) =>
            onFiltersChange({
              ...filters,
              type: e.target.value as PaperType | "",
            })
          }
          options={TYPE_OPTIONS}
          disabled={disabled}
          className="w-[160px]"
        />
        {hasFilters && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClearFilters}
            disabled={disabled}
            className="h-10"
          >
            <X className="h-4 w-4" />
            Clear
          </Button>
        )}
      </div>
    </div>
  );
}
