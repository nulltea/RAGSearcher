/**
 * Progress window and notification helpers for Zotero.
 */

export interface ProgressHandle {
  update(text: string): void;
  finish(text: string, success?: boolean): void;
  fail(text: string): void;
  close(): void;
}

/** Show a progress window and return a handle to update/close it */
export function showProgress(header: string): ProgressHandle {
  const pw = new Zotero.ProgressWindow({ closeOnClick: true });
  pw.changeHeadline(header);
  const progress = new pw.ItemProgress("chrome://zotero/skin/tick.png", "Starting...");
  pw.show();

  return {
    update(text: string) {
      try { progress.setText(text); } catch { /* widget may be stale */ }
    },
    finish(text: string, success = true) {
      try {
        progress.setProgress(100);
        progress.setIcon(
          success
            ? "chrome://zotero/skin/tick.png"
            : "chrome://zotero/skin/cross.png",
        );
        progress.setText(text);
        pw.startCloseTimer(4000);
      } catch {
        // Widget stale after long async — show fresh notification instead
        showSuccess(text);
      }
    },
    fail(text: string) {
      try {
        progress.setProgress(100);
        progress.setIcon("chrome://zotero/skin/cross.png");
        progress.setText(text);
        pw.startCloseTimer(8000);
      } catch {
        showError(text);
      }
    },
    close() {
      try { pw.close(); } catch { /* already closed */ }
    },
  };
}

/** Show a simple success notification */
export function showSuccess(message: string): void {
  const pw = new Zotero.ProgressWindow({ closeOnClick: true });
  pw.changeHeadline("RAG Library");
  new pw.ItemProgress("chrome://zotero/skin/tick.png", message);
  pw.show();
  pw.startCloseTimer(4000);
}

/** Show a simple error notification */
export function showError(message: string): void {
  const pw = new Zotero.ProgressWindow({ closeOnClick: true });
  pw.changeHeadline("RAG Library - Error");
  new pw.ItemProgress("chrome://zotero/skin/cross.png", message);
  pw.show();
  pw.startCloseTimer(8000);
}
