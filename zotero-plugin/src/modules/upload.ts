/**
 * Paper upload flow: Zotero item → PDF path → MCP index_paper
 */

import type { McpClient } from "../mcp/client";
import { getItemMetadata, getPdfPath, getRagPaperId, setRagPaperId, clearRagPaperId } from "../utils/zotero-item";
import { showProgress, showSuccess, showError } from "../utils/notify";
import { refreshItemPane } from "./item-pane";

type UploadOutcome = "uploaded" | "skipped" | "failed";

interface UploadItemResult {
  outcome: UploadOutcome;
  title: string;
  reason?: string;
  chunkCount?: number;
}

interface UploadBatchSummary {
  uploaded: number;
  skipped: number;
  failed: number;
  results: UploadItemResult[];
}

export async function uploadSelectedItem(client: McpClient): Promise<void> {
  const zoteroPane = (Zotero.getMainWindow() as any).ZoteroPane;
  if (!zoteroPane) return;
  const selectedItems = zoteroPane.getSelectedItems() as Zotero.Item[];
  const summary = await uploadItems(client, selectedItems);
  if (summary.results.length === 0) {
    return;
  }

  // Refresh item pane if any uploads succeeded (RAG-ID changed in Extra field)
  if (summary.uploaded > 0) {
    refreshItemPane();
  }

  if (summary.results.length === 1) {
    const [result] = summary.results;
    if (result.outcome === "uploaded") {
      showSuccess(`Uploaded! ${result.chunkCount ?? 0} chunks indexed.`);
    } else if (result.outcome === "skipped") {
      showSuccess(result.reason || "Skipped.");
    } else {
      showError(result.reason || "Upload failed.");
    }
    return;
  }

  const summaryText = [
    `Uploaded ${summary.uploaded}`,
    `Skipped ${summary.skipped}`,
    `Failed ${summary.failed}`,
  ].join(" • ");
  const details = buildSummaryDetails(summary.results);
  showSuccess(details ? `${summaryText}\n${details}` : summaryText);
}

async function uploadItems(client: McpClient, items: Zotero.Item[]): Promise<UploadBatchSummary> {
  const progress = showProgress("Uploading to RAG Library");
  const results: UploadItemResult[] = [];
  let uploaded = 0;
  let skipped = 0;
  let failed = 0;

  const total = items.length;
  for (let index = 0; index < items.length; index += 1) {
    const item = items[index];
    const title = getItemTitle(item);
    progress.update(`[${index + 1}/${total}] ${title}`);

    const result = await uploadItem(client, item);
    results.push(result);
    if (result.outcome === "uploaded") uploaded += 1;
    if (result.outcome === "skipped") skipped += 1;
    if (result.outcome === "failed") failed += 1;
  }

  progress.close();
  return { uploaded, skipped, failed, results };
}

async function uploadItem(client: McpClient, item: Zotero.Item): Promise<UploadItemResult> {
  const title = getItemTitle(item);

  if (!item.isRegularItem()) {
    return {
      outcome: "skipped",
      title,
      reason: `${title}: unsupported item type`,
    };
  }

  const existingId = getRagPaperId(item);
  if (existingId) {
    try {
      const papers = await client.searchPapers({ query: existingId, limit: 1 });
      const stillExists = papers.papers.some((p) => p.id === existingId);
      if (stillExists) {
        return {
          outcome: "skipped",
          title,
          reason: `${title}: already uploaded`,
        };
      }
      Zotero.debug(`[RAG] Paper ${existingId} no longer exists on server, clearing stale RAG-ID`);
      await clearRagPaperId(item);
    } catch {
      // Server not reachable — proceed with upload attempt anyway
    }
  }

  const pdfPath = await getPdfPath(item);
  if (!pdfPath) {
    return {
      outcome: "skipped",
      title,
      reason: `${title}: no PDF attachment found`,
    };
  }

  try {
    const meta = getItemMetadata(item);

    const result = await client.indexPaper({
      file_path: pdfPath,
      title: meta.title || undefined,
      authors: meta.authors || undefined,
      source: meta.source,
      paper_type: meta.paperType,
    });

    await setRagPaperId(item, result.paper_id);
    return {
      outcome: "uploaded",
      title,
      chunkCount: result.chunk_count,
    };
  } catch (e: any) {
    Zotero.debug(`[RAG] Upload failed: ${e}`);
    return {
      outcome: "failed",
      title,
      reason: `${title}: ${e.message || e}`,
    };
  }
}

function getItemTitle(item: Zotero.Item): string {
  return (item.getField("title") as string) || "Untitled";
}

function buildSummaryDetails(results: UploadItemResult[]): string {
  const details = results
    .filter((result) => result.outcome !== "uploaded" && result.reason)
    .slice(0, 4)
    .map((result) => result.reason);
  if (details.length === 0) {
    return "";
  }
  return details.join("; ");
}
