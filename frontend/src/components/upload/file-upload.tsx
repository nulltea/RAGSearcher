"use client";

import { cn } from "@/lib/utils";
import { FileText, Upload, X } from "lucide-react";
import { useCallback, useState, type DragEvent } from "react";
import { Button } from "../ui/button";

const MAX_FILE_SIZE = 10 * 1024 * 1024; // 10MB

export interface FileUploadProps {
  onFileSelect: (file: File) => void;
  onFileRemove: () => void;
  selectedFile: File | null;
  disabled?: boolean;
  error?: string;
}

export function FileUpload({
  onFileSelect,
  onFileRemove,
  selectedFile,
  disabled,
  error,
}: FileUploadProps) {
  const [isDragging, setIsDragging] = useState(false);

  const validateAndSelectFile = useCallback(
    (file: File) => {
      // Validate file type
      if (file.type !== "application/pdf") {
        return "Only PDF files are accepted";
      }

      // Validate file size
      if (file.size > MAX_FILE_SIZE) {
        return "File size must be less than 10MB";
      }

      onFileSelect(file);
      return null;
    },
    [onFileSelect]
  );

  const handleDragOver = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  }, []);

  const handleDrop = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      e.preventDefault();
      e.stopPropagation();
      setIsDragging(false);

      if (disabled) return;

      const files = e.dataTransfer.files;
      if (files.length > 0) {
        validateAndSelectFile(files[0]);
      }
    },
    [disabled, validateAndSelectFile]
  );

  const handleFileInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (files && files.length > 0) {
        validateAndSelectFile(files[0]);
      }
      // Reset input so same file can be selected again
      e.target.value = "";
    },
    [validateAndSelectFile]
  );

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  if (selectedFile) {
    return (
      <div className="rounded-lg border border-input bg-background p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <FileText className="h-5 w-5 text-primary" />
            </div>
            <div>
              <p className="font-medium text-foreground">{selectedFile.name}</p>
              <p className="text-sm text-muted-foreground">
                {formatFileSize(selectedFile.size)}
              </p>
            </div>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={onFileRemove}
            disabled={disabled}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
        {error && <p className="mt-2 text-sm text-destructive">{error}</p>}
      </div>
    );
  }

  return (
    <div>
      <div
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        className={cn(
          "relative flex min-h-[200px] cursor-pointer flex-col items-center justify-center rounded-lg border-2 border-dashed transition-colors",
          isDragging
            ? "border-primary bg-primary/5"
            : "border-muted-foreground/25 hover:border-primary/50",
          disabled && "cursor-not-allowed opacity-50",
          error && "border-destructive"
        )}
      >
        <input
          type="file"
          accept="application/pdf"
          onChange={handleFileInput}
          disabled={disabled}
          className="absolute inset-0 cursor-pointer opacity-0"
        />
        <Upload className="mb-4 h-10 w-10 text-muted-foreground" />
        <p className="mb-1 text-sm font-medium text-foreground">
          {isDragging ? "Drop your PDF here" : "Drag and drop your PDF here"}
        </p>
        <p className="text-sm text-muted-foreground">or click to browse</p>
        <p className="mt-2 text-xs text-muted-foreground">
          PDF files only, max 10MB
        </p>
      </div>
      {error && <p className="mt-2 text-sm text-destructive">{error}</p>}
    </div>
  );
}
