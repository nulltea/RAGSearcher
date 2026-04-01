/**
 * Algorithm extraction flow: trigger extraction → open review dialog
 */

import type { McpClient } from "../mcp/client";
import { getRagPaperId } from "../utils/zotero-item";
import { showProgress, showSuccess, showError } from "../utils/notify";
import { openReviewDialog } from "./review-dialog";

export async function extractAlgorithms(client: McpClient): Promise<void> {
  const zoteroPane = (Zotero.getMainWindow() as any).ZoteroPane;
  if (!zoteroPane) return;
  const items = zoteroPane.getSelectedItems();
  if (items.length !== 1) return;

  const item = items[0];
  const paperId = getRagPaperId(item);
  if (!paperId) {
    const progress = showProgress("RAG Library");
    progress.fail("Paper not uploaded to RAG Library yet. Upload it first.");
    return;
  }

  // Check if algorithms already exist — skip expensive extraction
  try {
    // Pass status: null to override serde default ("approved") and find any status
    const existing = await client.searchAlgorithms({
      paper_id: paperId,
      status: (null as unknown as string),
      limit: 50,
    });
    if (existing.algorithms.length > 0) {
      showSuccess(`Already extracted (${existing.algorithms.length} algorithms).`);
      const title = (item.getField("title") as string) || "Untitled";
      openReviewDialog(title, existing.algorithms);
      return;
    }
  } catch (e) {
    Zotero.debug(`[RAG] Algorithm check failed, proceeding with extraction: ${e}`);
  }

  const progress = showProgress("Extracting Algorithms");
  progress.update("Running 3-pass AI pipeline (this may take a few minutes)...");

  try {
    const result = await client.extractAlgorithms({ paper_id: paperId });
    progress.close();
    showSuccess(`Extracted ${result.algorithm_count} algorithms.`);

    // Fetch the full algorithm data for the dialog
    const algorithms = await client.searchAlgorithms({
      paper_id: paperId,
      status: "approved",
      limit: 50,
    });

    if (algorithms.algorithms.length > 0) {
      const title = (item.getField("title") as string) || "Untitled";
      openReviewDialog(title, algorithms.algorithms);
    }
  } catch (e: any) {
    Zotero.debug(`[RAG] Extraction failed: ${e}`);
    progress.close();
    showError(`Extraction failed: ${e.message || e}`);
  }
}

export async function viewAlgorithms(client: McpClient): Promise<void> {
  const zoteroPane = (Zotero.getMainWindow() as any).ZoteroPane;
  if (!zoteroPane) return;
  const items = zoteroPane.getSelectedItems();
  if (items.length !== 1) return;

  const item = items[0];
  const paperId = getRagPaperId(item);
  if (!paperId) return;

  const progress = showProgress("RAG Library");
  progress.update("Loading algorithms...");

  try {
    const algorithms = await client.searchAlgorithms({
      paper_id: paperId,
      limit: 50,
    });

    progress.close();

    if (algorithms.algorithms.length > 0) {
      const title = (item.getField("title") as string) || "Untitled";
      openReviewDialog(title, algorithms.algorithms);
    } else {
      showSuccess("No algorithms found for this paper.");
    }
  } catch (e: any) {
    Zotero.debug(`[RAG] Failed to load algorithms: ${e}`);
    progress.close();
    showError(`Failed: ${e.message || e}`);
  }
}
