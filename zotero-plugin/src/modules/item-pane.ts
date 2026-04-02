/**
 * Item pane section — shows RAG status, patterns, and algorithms in the right sidebar.
 */

import type { McpClient } from "../mcp/client";
import type { PatternResult } from "../mcp/types";
import { getRagPaperId, clearRagPaperId } from "../utils/zotero-item";
import { uploadSelectedItem } from "./upload";
import { extractAlgorithms } from "./extraction";
import { extractPatterns, listPatterns, openPatternDialog } from "./pattern-extraction";
import { openReviewDialog } from "./review-dialog";

/** Register the RAG Library item pane section. Returns the section ID for cleanup. */
const renderVersions = new WeakMap<HTMLElement, number>();
const FETCH_DELAY_MS = 200;

export function registerItemPane(client: McpClient, pluginId: string): string {
  const sectionId = Zotero.ItemPaneManager.registerSection({
    paneID: "rag-library",
    pluginID: pluginId,
    header: {
      l10nID: "rag-itempane-header",
      icon: "chrome://zoterorag/content/icons/icon-16.png",
    },
    sidenav: {
      l10nID: "rag-itempane-header",
      icon: "chrome://zoterorag/content/icons/icon-20.png",
    },
    onInit: ({ body }: { body: HTMLElement }) => {
      // Fluent l10n doesn't resolve for plugins — set label directly on the collapsible-section
      const section = body.closest("collapsible-section") || body.parentElement;
      if (section) {
        section.setAttribute("label", "RAG Library");
      }
    },
    onRender: ({ body, item }: { body: HTMLElement; item: any }) => {
      try {
        renderPaneShell(body, item, client);
      } catch (e) {
        Zotero.debug(`[RAG] renderPane error: ${e}`);
      }
    },
  });
  if (!sectionId) {
    throw new Error("registerSection returned false");
  }
  return sectionId;
}

/** Unregister the item pane section. */
export function unregisterItemPane(sectionId: string): void {
  Zotero.ItemPaneManager.unregisterSection(sectionId);
}

function renderPaneShell(body: HTMLElement, item: any, client: McpClient): void {
  body.textContent = "";
  const version = (renderVersions.get(body) || 0) + 1;
  renderVersions.set(body, version);

  if (!item || !item.isRegularItem()) {
    return;
  }

  const container = createEl(body, "div", "rag-pane");
  applyStyles(container, { padding: "8px 12px", fontFamily: "system-ui, sans-serif", fontSize: "13px" });

  const ragId = getRagPaperId(item);

  if (!ragId) {
    // State 1: Not uploaded
    const btn = createEl(container, "button", "rag-upload-btn");
    btn.textContent = "Upload to RAG Library";
    applyButtonStyles(btn);
    btn.addEventListener("click", () => {
      void uploadSelectedItem(client).catch((e) => Zotero.debug(`[RAG] Upload click failed: ${e}`));
    });
    return;
  }

  // State 2/3: Uploaded — show status
  const status = createEl(container, "div", "rag-status");
  status.textContent = "\u2713 Saved in RAG Library";
  applyStyles(status, { marginBottom: "8px", color: "var(--accent-green, #166534)", fontWeight: "500" });

  const patSection = createEl(container, "div", "rag-patterns");
  patSection.textContent = "Loading patterns\u2026";
  applyStyles(patSection, { color: "var(--fill-secondary, #6b7280)", marginBottom: "6px" });

  const algSection = createEl(container, "div", "rag-algorithms");
  algSection.textContent = "Loading algorithms\u2026";
  applyStyles(algSection, { color: "var(--fill-secondary, #6b7280)" });

  const win = body.ownerDocument.defaultView;
  if (!win) {
    return;
  }

  win.setTimeout(() => {
    if (renderVersions.get(body) !== version) {
      return;
    }
    startPaneFetches(body, item, client, version);
  }, FETCH_DELAY_MS);
}

async function startPaneFetches(body: HTMLElement, item: any, client: McpClient, version: number): Promise<void> {
  const isStale = (): boolean =>
    !body.isConnected || renderVersions.get(body) !== version;

  // Validate that the paper still exists on the server
  const ragId = getRagPaperId(item);
  if (!ragId) return;

  try {
    const papers = await client.searchPapers({ query: ragId, limit: 1 });
    const stillExists = papers.papers.some((p) => p.id === ragId);
    if (!stillExists && !isStale()) {
      Zotero.debug(`[RAG] Paper ${ragId} no longer exists on server, clearing stale RAG-ID`);
      await clearRagPaperId(item);
      // Re-render as "not uploaded"
      renderPaneShell(body, item, client);
      return;
    }
  } catch (e) {
    // Server unreachable — show what we have locally
    Zotero.debug(`[RAG] Paper validation failed (server unreachable?): ${e}`);
  }

  if (isStale()) return;

  void loadPatterns(body, item, version);
  void loadAlgorithms(body, item, client, version);
}

async function loadPatterns(body: HTMLElement, item: any, version: number): Promise<void> {
  if (!item || !item.isRegularItem()) {
    return;
  }

  if (renderVersions.get(body) !== version) {
    return;
  }

  const ragId = getRagPaperId(item);
  if (!ragId) {
    return;
  }

  const patSection = body.querySelector(".rag-patterns") as HTMLElement | null;
  if (!patSection) {
    return;
  }

  const isStale = (): boolean =>
    !body.isConnected || renderVersions.get(body) !== version;

  try {
    const patterns = await listPatterns(ragId);
    if (!isStale()) {
      renderPatternsSection(patSection, item, patterns);
    }
  } catch (e) {
    if (!isStale()) {
      renderPatternFallback(patSection);
    }
    Zotero.debug(`[RAG] Failed to load patterns for pane: ${e}`);
  }
}

async function loadAlgorithms(
  body: HTMLElement,
  item: any,
  client: McpClient,
  version: number,
): Promise<void> {
  if (!item || !item.isRegularItem()) {
    return;
  }

  if (renderVersions.get(body) !== version) {
    return;
  }

  const ragId = getRagPaperId(item);
  if (!ragId) {
    return;
  }

  const algSection = body.querySelector(".rag-algorithms") as HTMLElement | null;
  if (!algSection) {
    return;
  }

  const isStale = (): boolean =>
    !body.isConnected || renderVersions.get(body) !== version;

  try {
    const result = await client.searchAlgorithms({
      paper_id: ragId,
      status: (null as unknown as string),
      limit: 50,
    });
    if (!isStale()) {
      renderAlgorithmsSection(algSection, item, result.algorithms, client);
    }
  } catch (e) {
    if (!isStale()) {
      renderAlgorithmFallback(algSection, client);
    }
    Zotero.debug(`[RAG] Failed to load algorithms for pane: ${e}`);
  }
}

function renderPatternsSection(
  section: HTMLElement,
  item: any,
  patterns: PatternResult[],
): void {
  section.textContent = "";
  if (patterns.length === 0) {
    renderPatternFallback(section);
    return;
  }
  renderDropdown(
    section,
    item,
    patterns,
    "Patterns",
    "chrome://zotero/skin/16/universal/note.svg",
    (title, all) => openPatternDialog(title, all),
    (title, one) => openPatternDialog(title, [one]),
  );
}

function renderAlgorithmsSection(
  section: HTMLElement,
  item: any,
  algorithms: Array<{ name: string }>,
  client: McpClient,
): void {
  section.textContent = "";
  if (algorithms.length === 0) {
    renderAlgorithmFallback(section, client);
    return;
  }
  renderDropdown(
    section,
    item,
    algorithms,
    "Algorithms",
    "chrome://zotero/skin/16/universal/note.svg",
    (title, all) => openReviewDialog(title, all),
    (title, one) => openReviewDialog(title, [one]),
  );
}

function renderPatternFallback(section: HTMLElement): void {
  section.textContent = "";
  const btn = createEl(section, "button", "rag-extract-patterns-btn");
  btn.textContent = "Extract Patterns";
  applyButtonStyles(btn);
  btn.addEventListener("click", () => {
    void extractPatterns().catch((e) => Zotero.debug(`[RAG] Extract patterns click failed: ${e}`));
  });
}

function renderAlgorithmFallback(section: HTMLElement, client: McpClient): void {
  section.textContent = "";
  const btn = createEl(section, "button", "rag-extract-btn");
  btn.textContent = "Extract Algorithms";
  applyButtonStyles(btn);
  btn.addEventListener("click", () => {
    void extractAlgorithms(client).catch((e) => Zotero.debug(`[RAG] Extract algorithms click failed: ${e}`));
  });
}

/** Render a dropdown section: header button + individual rows */
function renderDropdown<T extends { name: string }>(
  section: HTMLElement,
  item: any,
  items: T[],
  label: string,
  iconSrc: string,
  onHeaderClick: (title: string, all: T[]) => void,
  onRowClick: (title: string, single: T) => void,
): void {
  // Header button
  const headerBtn = createEl(section, "button", "rag-dropdown-header");
  headerBtn.textContent = `${label} (${items.length})`;
  applyStyles(headerBtn, {
    display: "block",
    width: "100%",
    textAlign: "left",
    padding: "6px 8px",
    marginBottom: "4px",
    fontWeight: "500",
    fontSize: "13px",
    background: "transparent",
    border: "none",
    cursor: "pointer",
    color: "var(--fill-primary, #1f2937)",
    borderRadius: "4px",
  });
  headerBtn.addEventListener("mouseenter", () => {
    headerBtn.style.background = "var(--fill-quinary, #f3f4f6)";
  });
  headerBtn.addEventListener("mouseleave", () => {
    headerBtn.style.background = "transparent";
  });
  headerBtn.addEventListener("click", () => {
    const title = (item.getField("title") as string) || "Untitled";
    onHeaderClick(title, items);
  });

  // Individual rows
  for (const entry of items) {
    const row = createEl(section, "div", "rag-row");
    applyStyles(row, {
      padding: "3px 8px 3px 12px",
      margin: "1px 0",
      borderRadius: "4px",
      cursor: "pointer",
      display: "flex",
      alignItems: "center",
      gap: "6px",
    });

    // Icon (uses -moz-context-properties so SVG respects fill color)
    const icon = row.ownerDocument.createElementNS("http://www.w3.org/1999/xhtml", "img") as HTMLImageElement;
    icon.src = iconSrc;
    icon.className = "rag-row-icon";
    const iconEl = icon as unknown as HTMLElement;
    applyStyles(iconEl, { width: "13px", height: "13px", flexShrink: "0" });
    iconEl.style.setProperty("-moz-context-properties", "fill", "");
    iconEl.style.setProperty("fill", "currentColor", "");
    applyStyles(iconEl, { opacity: "0.6", color: "var(--fill-primary, #1f2937)" });
    row.appendChild(icon);

    const name = createEl(row, "span", "rag-row-name");
    name.textContent = entry.name;
    applyStyles(name, { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" });

    row.addEventListener("mouseenter", () => {
      row.style.background = "var(--fill-quinary, #f3f4f6)";
    });
    row.addEventListener("mouseleave", () => {
      row.style.background = "transparent";
    });
    row.addEventListener("click", () => {
      const title = (item.getField("title") as string) || "Untitled";
      onRowClick(title, entry);
    });
  }
}

function createEl(parent: HTMLElement, tag: string, className?: string): HTMLElement {
  const el = parent.ownerDocument.createElementNS("http://www.w3.org/1999/xhtml", tag) as HTMLElement;
  if (className) el.className = className;
  parent.appendChild(el);
  return el;
}

function applyStyles(el: HTMLElement, styles: Record<string, string>): void {
  for (const [key, value] of Object.entries(styles)) {
    (el.style as any)[key] = value;
  }
}

function applyButtonStyles(btn: HTMLElement): void {
  applyStyles(btn, {
    padding: "6px 12px",
    borderRadius: "4px",
    border: "1px solid var(--fill-quinary, #d1d5db)",
    background: "var(--material-background, white)",
    color: "var(--fill-primary, #1f2937)",
    cursor: "pointer",
    fontSize: "13px",
    fontWeight: "500",
    width: "100%",
  });
}
