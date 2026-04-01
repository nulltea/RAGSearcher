/**
 * Paper upload flow: Zotero item → PDF path → MCP index_paper
 */

import type { McpClient } from "../mcp/client";
import { getItemMetadata, getPdfPath, getRagPaperId, setRagPaperId, clearRagPaperId } from "../utils/zotero-item";
import { showProgress, showSuccess, showError } from "../utils/notify";

export async function uploadSelectedItem(client: McpClient): Promise<void> {
  const zoteroPane = (Zotero.getMainWindow() as any).ZoteroPane;
  if (!zoteroPane) return;
  const items = zoteroPane.getSelectedItems();
  if (items.length !== 1) return;

  const item = items[0];

  // Check if already uploaded — validate against server
  const existingId = getRagPaperId(item);
  if (existingId) {
    try {
      const papers = await client.searchPapers({ query: existingId, limit: 1 });
      const stillExists = papers.papers.some((p) => p.id === existingId);
      if (stillExists) {
        showSuccess(`Already uploaded (ID: ${existingId.slice(0, 8)}...)`);
        return;
      }
      // Paper was deleted from RAG server — clear stale ID and re-upload
      Zotero.debug(`[RAG] Paper ${existingId} no longer exists on server, clearing stale RAG-ID`);
      await clearRagPaperId(item);
    } catch {
      // Server not reachable — proceed with upload attempt anyway
    }
  }

  // Get PDF path
  const pdfPath = await getPdfPath(item);
  if (!pdfPath) {
    showError("No PDF attachment found.");
    return;
  }

  const progress = showProgress("Uploading to RAG Library");
  progress.update("Reading PDF and indexing...");

  try {
    const meta = getItemMetadata(item);

    const result = await client.indexPaper({
      file_path: pdfPath,
      title: meta.title || undefined,
      authors: meta.authors || undefined,
      source: meta.source,
      paper_type: meta.paperType,
    });

    // Store the RAG paper ID in the item's Extra field
    await setRagPaperId(item, result.paper_id);

    // Close in-progress window and show fresh success notification
    progress.close();
    showSuccess(`Uploaded! ${result.chunk_count} chunks indexed.`);
  } catch (e: any) {
    Zotero.debug(`[RAG] Upload failed: ${e}`);
    progress.close();
    showError(`Upload failed: ${e.message || e}`);
  }
}
