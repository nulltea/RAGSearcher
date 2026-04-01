/**
 * Search UI: toolbar/menu integration, semantic search dialog, and result opening.
 */

import type { McpClient } from "../mcp/client";
import type { QueryResponse, SemanticSearchResult } from "../mcp/types";
import { getPdfAttachmentItem, getRagPaperId } from "../utils/zotero-item";
import { showError } from "../utils/notify";

const TOOLBAR_BUTTON_ID = "zotero-rag-search-button";
const TOOLS_MENU_ITEM_ID = "zotero-rag-search-menuitem";
const ACTIVE_PAPER_MENU_ITEM_ID = "zotero-itemmenu-rag-search-paper";

const SEARCH_BOX_SELECTORS = [
  "#zotero-tb-search",
  "#zotero-tb-search-textbox",
  'search-textbox[placeholder*="All Fields"]',
  'textbox[placeholder*="All Fields"]',
];

interface SearchDialogOptions {
  mode: "global" | "paper";
  client: McpClient;
  paperId?: string;
  paperTitle?: string;
  initialQuery?: string;
}

interface SearchDialogResult extends SemanticSearchResult {
  paperId?: string;
  paperTitle?: string;
  sourceKind?: string;
}

interface SearchDialogResponse extends QueryResponse {
  results: SearchDialogResult[];
}

interface SearchDialogArgs {
  mode: "global" | "paper";
  paperId?: string;
  paperTitle?: string;
  initialQuery: string;
  searchFn: (query: string) => Promise<SearchDialogResponse>;
  openResultFn: (result: SearchDialogResult, query: string) => Promise<void>;
}

export function addSearchUiToWindow(window: Window, client: McpClient): void {
  const addedToolbarButton = addToolbarButton(window, client);
  if (!addedToolbarButton) {
    addToolsMenuItem(window, client);
  }
}

export function removeSearchUiFromWindow(window: Window): void {
  const doc = window.document;
  doc.getElementById(TOOLBAR_BUTTON_ID)?.remove();
  doc.getElementById(TOOLS_MENU_ITEM_ID)?.remove();
}

export async function openGlobalSearchDialog(client: McpClient, initialQuery = ""): Promise<void> {
  openSearchDialog({
    mode: "global",
    client,
    initialQuery,
  });
}

export async function openActivePaperSearchDialog(client: McpClient): Promise<void> {
  const item = getActiveRegularItem();
  if (!item) {
    showError("Select one indexed paper first.");
    return;
  }

  const paperId = getRagPaperId(item);
  if (!paperId) {
    showError("Paper is not uploaded to RAG Library yet.");
    return;
  }

  openSearchDialog({
    mode: "paper",
    client,
    paperId,
    paperTitle: (item.getField("title") as string) || "Untitled",
  });
}

function addToolbarButton(window: Window, client: McpClient): boolean {
  const doc = window.document;
  if (doc.getElementById(TOOLBAR_BUTTON_ID)) {
    return true;
  }

  const mount = findToolbarButtonMount(doc);
  if (!mount) {
    return false;
  }

  try {
    const button = doc.createXULElement("toolbarbutton");
    button.id = TOOLBAR_BUTTON_ID;
    button.setAttribute("class", "toolbarbutton-1");
    button.setAttribute("image", "chrome://zoterorag/content/icons/icon-20.png");
    button.setAttribute("tooltiptext", "Search indexed papers");
    button.setAttribute("label", "");
    button.setAttribute("style", [
      "min-width: 40px",
      "width: 40px",
      "min-height: 40px",
      "height: 40px",
      "padding: 8px",
      "margin-inline-end: 8px",
      "border-radius: 6px",
      "list-style-image: url(chrome://zoterorag/content/icons/icon-20.png)",
    ].join("; "));
    button.addEventListener("click", (event) => {
      event.stopPropagation();
    }, true);
    button.addEventListener("command", () => {
      void openGlobalSearchDialog(client);
    });
    mount.parent.insertBefore(button, mount.before);
    return true;
  } catch (e) {
    Zotero.debug(`[RAG] Failed to add toolbar search button: ${e}`);
    doc.getElementById(TOOLBAR_BUTTON_ID)?.remove();
    return false;
  }
}

function addToolsMenuItem(window: Window, client: McpClient): void {
  const doc = window.document;
  if (doc.getElementById(TOOLS_MENU_ITEM_ID)) {
    return;
  }

  const popup = findToolsPopup(doc);
  if (!popup) {
    return;
  }

  const item = doc.createXULElement("menuitem");
  item.id = TOOLS_MENU_ITEM_ID;
  item.setAttribute("label", "Search RAG Library");
  item.addEventListener("command", () => {
    void openGlobalSearchDialog(client);
  });
  popup.appendChild(item);
}

export function addActivePaperSearchMenuItem(
  doc: Document,
  popup: XULElement,
  client: McpClient,
): void {
  if (doc.getElementById(ACTIVE_PAPER_MENU_ITEM_ID)) {
    return;
  }

  const item = doc.createXULElement("menuitem");
  item.id = ACTIVE_PAPER_MENU_ITEM_ID;
  item.setAttribute("label", "Search This Paper");
  item.addEventListener("command", () => {
    void openActivePaperSearchDialog(client);
  });
  popup.appendChild(item);
}

function openSearchDialog(options: SearchDialogOptions): void {
  const win = Zotero.getMainWindow();
  const args: SearchDialogArgs = {
    mode: options.mode,
    paperId: options.paperId,
    paperTitle: options.paperTitle,
    initialQuery: options.initialQuery || "",
    searchFn: (query: string) => executeSearch(options.client, query, options.paperId),
    openResultFn: (result: SearchDialogResult, query: string) => openSearchResult(result, query),
  };

  win.openDialog(
    "chrome://zoterorag/content/search-dialog.xhtml",
    `rag-search-${Date.now()}`,
    "chrome,centerscreen,resizable,width=980,height=760",
    args,
  );
}

async function executeSearch(
  client: McpClient,
  query: string,
  paperId?: string,
): Promise<SearchDialogResponse> {
  const response = await client.search({
    query,
    limit: paperId ? 100 : 25,
    min_score: 0.5,
    hybrid: true,
  });

  const filtered = paperId
    ? response.results.filter((result) => result.file_path === `papers/${paperId}`)
    : response.results;

  const paperCache = new Map<string, string | undefined>();
  const hydrated = await Promise.all(filtered.map(async (result) => {
    const parsed = parseSearchPath(result.file_path);
    const resultPaperId = parsed?.paperId;
    let paperTitle: string | undefined;
    if (resultPaperId) {
      if (!paperCache.has(resultPaperId)) {
        paperCache.set(resultPaperId, await findPaperTitleByRagId(resultPaperId));
      }
      paperTitle = paperCache.get(resultPaperId);
    }

    return {
      ...result,
      paperId: resultPaperId,
      paperTitle,
      sourceKind: parsed?.kind,
    } satisfies SearchDialogResult;
  }));

  return {
    ...response,
    results: hydrated,
  };
}

async function openSearchResult(result: SearchDialogResult, _query: string): Promise<void> {
  if (!result.paperId) {
    showError("Could not resolve paper for this result.");
    return;
  }

  const item = await findItemByRagId(result.paperId);
  if (!item) {
    showError("Paper not found in Zotero library.");
    return;
  }

  const attachment = getPdfAttachmentItem(item);
  if (!attachment) {
    showError("No PDF attachment found for this paper.");
    return;
  }

  const readerApi = (Zotero as any).Reader;
  if (!readerApi?.open) {
    showError("Zotero PDF reader is not available.");
    return;
  }

  try {
    const opened = await readerApi.open(attachment.id, undefined, { allowDuplicate: false });
    const reader = opened || findOpenReaderForItem(attachment.id);
    if (!reader) {
      return;
    }

    await waitForReaderReady(reader);
    await triggerReaderFind(reader, _query, result.content);
  } catch (e) {
    Zotero.debug(`[RAG] Failed to open PDF result: ${e}`);
    showError("Failed to open PDF.");
  }
}

async function findItemByRagId(paperId: string): Promise<Zotero.Item | null> {
  try {
    const search = new Zotero.Search();
    search.addCondition("extra", "contains", `RAG-ID: ${paperId}`);
    search.addCondition("itemType", "isNot", "attachment");
    const ids = await search.search();
    if (ids.length === 0) {
      return null;
    }
    return Zotero.Items.get(ids[0]) || null;
  } catch (e) {
    Zotero.debug(`[RAG] Failed to find Zotero item by RAG-ID ${paperId}: ${e}`);
    return null;
  }
}

async function findPaperTitleByRagId(paperId: string): Promise<string | undefined> {
  const item = await findItemByRagId(paperId);
  if (!item) {
    return undefined;
  }
  return (item.getField("title") as string) || undefined;
}

function parseSearchPath(filePath: string): { kind: string; paperId: string } | undefined {
  const match = /^(papers|patterns|algorithms)\/(.+)$/.exec(filePath);
  if (!match) {
    return undefined;
  }
  return {
    kind: match[1],
    paperId: match[2],
  };
}

function findOpenReaderForItem(itemID: number): any {
  const readers = ((Zotero as any).Reader?._readers || []) as any[];
  return readers.find((reader) => reader?.itemID === itemID) || null;
}

async function waitForReaderReady(reader: any): Promise<void> {
  if (reader?._initPromise) {
    await reader._initPromise;
  }
  if (reader?._primaryView?.initializedPromise) {
    await reader._primaryView.initializedPromise;
  }
  if (reader?._primaryView?._iframeWindow?.PDFViewerApplication?.initializedPromise) {
    await reader._primaryView._iframeWindow.PDFViewerApplication.initializedPromise;
  }
}

async function triggerReaderFind(reader: any, query: string, content: string): Promise<void> {
  const searchQuery = buildReaderSearchQuery(content, query);
  if (!searchQuery) {
    return;
  }

  const app = reader?._primaryView?._iframeWindow?.PDFViewerApplication;
  const findState = {
    query: searchQuery,
    phraseSearch: true,
    caseSensitive: false,
    entireWord: false,
    highlightAll: true,
    findPrevious: false,
    matchDiacritics: false,
  };

  await new Promise((resolve) => setTimeout(resolve, 150));

  if (app?.findController?.executeCommand) {
    app.findController.executeCommand("find", findState);
    return;
  }

  const eventBus = app?.eventBus;
  if (eventBus?.dispatch) {
    eventBus.dispatch("find", {
      source: reader,
      type: "",
      ...findState,
    });
  }
}

function buildReaderSearchQuery(content: string, query: string): string {
  const snippet = content
    .replace(/\s+/g, " ")
    .replace(/[^\p{L}\p{N}\s\-_,.:;()]/gu, " ")
    .trim()
    .split(/[.;!?]/)
    .map((part) => part.trim())
    .find((part) => part.length >= 24)
    || content.replace(/\s+/g, " ").trim().slice(0, 120).trim();

  if (snippet) {
    return snippet;
  }

  return query.trim();
}

function getActiveRegularItem(): Zotero.Item | null {
  const zoteroPane = (Zotero.getMainWindow() as any).ZoteroPane;
  if (!zoteroPane) {
    return null;
  }

  const items = zoteroPane.getSelectedItems() as Zotero.Item[];
  return items.length === 1 && items[0]?.isRegularItem() ? items[0] : null;
}

function findSearchAnchor(doc: Document): Element | null {
  for (const selector of SEARCH_BOX_SELECTORS) {
    const match = doc.querySelector(selector);
    if (match) {
      return match;
    }
  }
  return null;
}

function findToolbarButtonMount(doc: Document): { parent: Element; before: Element | null } | null {
  const anchor = findSearchAnchor(doc);
  if (!anchor) {
    return null;
  }

  const widget = anchor.closest("search-textbox, textbox, toolbaritem, hbox") || anchor;
  const parent = widget.parentElement;
  if (!parent) {
    return null;
  }

  return {
    parent,
    before: widget,
  };
}

function findToolsPopup(doc: Document): Element | null {
  return doc.getElementById("menu_ToolsPopup")
    || doc.getElementById("menuToolsPopup")
    || doc.getElementById("menu_Tools")?.querySelector("menupopup")
    || null;
}
