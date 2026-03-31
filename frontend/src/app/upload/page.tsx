"use client";

import { extractAlgorithms, extractPatterns, uploadPaper } from "@/lib/api";
import type { PaperUploadResponse } from "@/types";
import { useRouter } from "next/navigation";
import { AlertCircle, FileText, Link2, Loader2, Type } from "lucide-react";
import Link from "next/link";
import { useCallback, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  FileUpload,
  MetadataForm,
  TextInput,
  type MetadataFormData,
} from "@/components/upload";

type InputMode = "file" | "text" | "url";

const INITIAL_METADATA: MetadataFormData = {
  title: "",
  authors: "",
  source: "",
  publishedDate: "",
  type: "research_paper",
};

export default function UploadPage() {
  const router = useRouter();

  // Input state
  const [inputMode, setInputMode] = useState<InputMode>("file");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [textContent, setTextContent] = useState("");
  const [urlContent, setUrlContent] = useState("");
  const [metadata, setMetadata] = useState<MetadataFormData>(INITIAL_METADATA);

  // Upload state
  const [isUploading, setIsUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<PaperUploadResponse | null>(null);

  // Extraction state
  const [isExtracting, setIsExtracting] = useState(false);
  const [extractError, setExtractError] = useState<string | null>(null);
  const [patternsExtracted, setPatternsExtracted] = useState(false);

  // Algorithm extraction state
  const [isExtractingAlgorithms, setIsExtractingAlgorithms] = useState(false);
  const [algorithmError, setAlgorithmError] = useState<string | null>(null);
  const [algorithmsExtracted, setAlgorithmsExtracted] = useState(false);

  const hasContent =
    inputMode === "file"
      ? selectedFile !== null
      : inputMode === "text"
      ? textContent.trim().length > 0
      : urlContent.trim().length > 0;

  const handleUpload = useCallback(async () => {
    if (!hasContent) return;

    setIsUploading(true);
    setError(null);

    try {
      const formData = new FormData();

      // Add content
      if (inputMode === "file" && selectedFile) {
        formData.append("file", selectedFile);
      } else if (inputMode === "text") {
        formData.append("text", textContent);
      } else if (inputMode === "url") {
        formData.append("url", urlContent.trim());
      }

      // Add metadata (only non-empty values)
      if (metadata.title.trim()) {
        formData.append("title", metadata.title.trim());
      }
      if (metadata.authors.trim()) {
        formData.append("authors", metadata.authors.trim());
      }
      if (metadata.source.trim()) {
        formData.append("source", metadata.source.trim());
      }
      if (metadata.publishedDate) {
        formData.append("published_date", metadata.publishedDate);
      }
      if (metadata.type) {
        formData.append("paper_type", metadata.type);
      }

      const response = await uploadPaper(formData);
      setResult(response);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Upload failed. Please try again."
      );
    } finally {
      setIsUploading(false);
    }
  }, [hasContent, inputMode, selectedFile, textContent, urlContent, metadata]);

  const handleUploadAnother = useCallback(() => {
    setSelectedFile(null);
    setTextContent("");
    setUrlContent("");
    setMetadata(INITIAL_METADATA);
    setResult(null);
    setError(null);
    setExtractError(null);
  }, []);

  const handleExtractPatterns = useCallback(async () => {
    if (!result) return;
    setIsExtracting(true);
    setExtractError(null);
    try {
      await extractPatterns(result.paper.id);
      setPatternsExtracted(true);
    } catch (err) {
      setExtractError(
        err instanceof Error ? err.message : "Pattern extraction failed",
      );
    } finally {
      setIsExtracting(false);
    }
  }, [result]);

  const handleExtractAlgorithms = useCallback(async () => {
    if (!result) return;
    setIsExtractingAlgorithms(true);
    setAlgorithmError(null);
    try {
      await extractAlgorithms(result.paper.id);
      setAlgorithmsExtracted(true);
    } catch (err) {
      setAlgorithmError(
        err instanceof Error ? err.message : "Algorithm extraction failed",
      );
    } finally {
      setIsExtractingAlgorithms(false);
    }
  }, [result]);

  // Show result after successful upload
  if (result) {
    const { paper, chunk_count, duration_ms } = result;
    return (
      <div className="mx-auto max-w-2xl px-4 py-8">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-green-600 dark:text-green-400">
              <FileText className="h-5 w-5" />
              Upload Successful
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* Paper Details */}
            <div className="space-y-2">
              <h3 className="font-medium">Paper Details</h3>
              <dl className="space-y-1 text-sm">
                <div className="flex gap-2">
                  <dt className="text-muted-foreground">Title:</dt>
                  <dd className="font-medium">{paper.title}</dd>
                </div>
                {paper.authors.length > 0 && (
                  <div className="flex gap-2">
                    <dt className="text-muted-foreground">Authors:</dt>
                    <dd>{paper.authors.join(", ")}</dd>
                  </div>
                )}
              </dl>
            </div>

            {/* Indexing Stats */}
            <div className="rounded-lg bg-primary/10 p-4">
              <p className="text-lg font-medium">
                {chunk_count} chunk{chunk_count !== 1 ? "s" : ""} indexed
              </p>
              <p className="text-sm text-muted-foreground">
                Completed in {duration_ms}ms. The paper is ready for semantic
                search.
              </p>
            </div>

            {/* Extract Patterns */}
            <div className="rounded-lg border border-input bg-card p-4">
              <h3 className="font-medium">Extract Patterns</h3>
              <p className="mt-1 text-sm text-muted-foreground">
                Use AI to extract structured research patterns
                (Claim/Evidence/Context) for review.
              </p>
              {extractError && (
                <div className="mt-2 flex items-center gap-2 text-sm text-destructive">
                  <AlertCircle className="h-4 w-4 flex-shrink-0" />
                  {extractError}
                </div>
              )}
              {patternsExtracted ? (
                <div className="mt-3 flex items-center gap-2">
                  <span className="text-sm text-green-600">Patterns extracted.</span>
                  <Link href={`/review?id=${result.paper.id}`}>
                    <Button size="sm" variant="outline">Review Patterns</Button>
                  </Link>
                </div>
              ) : (
                <Button
                  className="mt-3 w-full"
                  onClick={handleExtractPatterns}
                  disabled={isExtracting || isExtractingAlgorithms}
                >
                  {isExtracting ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      Extracting patterns (this may take a minute)...
                    </>
                  ) : (
                    "Extract Patterns"
                  )}
                </Button>
              )}
            </div>

            {/* Extract Algorithms */}
            <div className="rounded-lg border border-input bg-card p-4">
              <h3 className="font-medium">Extract Algorithms</h3>
              <p className="mt-1 text-sm text-muted-foreground">
                Use AI to extract step-by-step algorithm/protocol/scheme
                definitions with LaTeX math notation.
              </p>
              {algorithmError && (
                <div className="mt-2 flex items-center gap-2 text-sm text-destructive">
                  <AlertCircle className="h-4 w-4 flex-shrink-0" />
                  {algorithmError}
                </div>
              )}
              {algorithmsExtracted ? (
                <div className="mt-3 flex items-center gap-2">
                  <span className="text-sm text-green-600">Algorithms extracted.</span>
                  <Link href={`/review?id=${result.paper.id}`}>
                    <Button size="sm" variant="outline">Review Algorithms</Button>
                  </Link>
                </div>
              ) : (
                <Button
                  className="mt-3 w-full"
                  onClick={handleExtractAlgorithms}
                  disabled={isExtractingAlgorithms || isExtracting}
                >
                  {isExtractingAlgorithms ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      Extracting algorithms (this may take a minute)...
                    </>
                  ) : (
                    "Extract Algorithms"
                  )}
                </Button>
              )}
            </div>

            {/* Actions */}
            <div className="flex gap-3">
              <Link href="/library" className="flex-1">
                <Button variant="outline" className="w-full">
                  Go to Library
                </Button>
              </Link>
              <Link href={`/search?paper_id=${paper.id}`} className="flex-1">
                <Button variant="outline" className="w-full">
                  Search This Paper
                </Button>
              </Link>
              <Button variant="ghost" onClick={handleUploadAnother}>
                Upload Another
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-2xl px-4 py-8">
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-foreground">Upload Paper</h1>
        <p className="mt-1 text-muted-foreground">
          Upload a PDF or paste text to index for semantic search.
        </p>
      </div>

      <div className="space-y-6">
        {/* Input Mode Toggle */}
        <div className="flex gap-2 rounded-lg border border-input bg-card p-1">
          <button
            type="button"
            onClick={() => setInputMode("file")}
            className={`flex flex-1 items-center justify-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
              inputMode === "file"
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <FileText className="h-4 w-4" />
            PDF Upload
          </button>
          <button
            type="button"
            onClick={() => setInputMode("text")}
            className={`flex flex-1 items-center justify-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
              inputMode === "text"
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <Type className="h-4 w-4" />
            Text Input
          </button>
          <button
            type="button"
            onClick={() => setInputMode("url")}
            className={`flex flex-1 items-center justify-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
              inputMode === "url"
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            <Link2 className="h-4 w-4" />
            URL
          </button>
        </div>

        {/* Content Input */}
        {inputMode === "file" ? (
          <FileUpload
            selectedFile={selectedFile}
            onFileSelect={setSelectedFile}
            onFileRemove={() => setSelectedFile(null)}
            disabled={isUploading}
          />
        ) : inputMode === "text" ? (
          <TextInput
            value={textContent}
            onChange={setTextContent}
            disabled={isUploading}
          />
        ) : (
          <div className="space-y-2">
            <label className="text-sm font-medium text-foreground">
              PDF URL
            </label>
            <input
              type="url"
              value={urlContent}
              onChange={(e) => setUrlContent(e.target.value)}
              placeholder="https://example.com/paper.pdf"
              disabled={isUploading}
              className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
            />
            <p className="text-xs text-muted-foreground">
              Direct link to a PDF file. The file will be downloaded and processed.
            </p>
          </div>
        )}

        {/* Metadata Form */}
        <MetadataForm
          data={metadata}
          onChange={setMetadata}
          disabled={isUploading}
        />

        {/* Error Display */}
        {error && (
          <div className="flex items-center gap-2 rounded-lg border border-destructive bg-destructive/10 p-4 text-sm text-destructive">
            <AlertCircle className="h-4 w-4 flex-shrink-0" />
            {error}
          </div>
        )}

        {/* Upload Button */}
        <Button
          onClick={handleUpload}
          disabled={!hasContent || isUploading}
          className="w-full"
          size="lg"
        >
          {isUploading ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Processing...
            </>
          ) : (
            "Upload & Index"
          )}
        </Button>

        <p className="text-center text-xs text-muted-foreground">
          The paper will be split into chunks, embedded using a local model, and
          stored for fast semantic search.
        </p>
      </div>
    </div>
  );
}
