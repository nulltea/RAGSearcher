/**
 * Algorithm review dialog — opens an XHTML window showing extracted algorithms.
 */

import type { AlgorithmResult } from "../mcp/types";

export function openReviewDialog(paperTitle: string, algorithms: AlgorithmResult[]): void {
  const win = Zotero.getMainWindow();
  win.openDialog(
    "chrome://zoterorag/content/review-dialog.xhtml",
    "rag-review",
    "chrome,centerscreen,resizable,width=850,height=700",
    { paperTitle, algorithms },
  );
}
