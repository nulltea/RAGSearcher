/**
 * Algorithm review dialog — opens an XHTML window showing extracted algorithms.
 */

import type { AlgorithmResult } from "../mcp/types";

export function openReviewDialog(paperTitle: string, algorithms: AlgorithmResult[]): void {
  const win = Zotero.getMainWindow();
  // Use unique window name so multiple dialogs can open, and new data always triggers fresh render
  const windowName = `rag-review-${Date.now()}`;
  win.openDialog(
    "chrome://zoterorag/content/review-dialog.xhtml",
    windowName,
    "chrome,centerscreen,resizable,width=850,height=700",
    { paperTitle, algorithms },
  );
}
