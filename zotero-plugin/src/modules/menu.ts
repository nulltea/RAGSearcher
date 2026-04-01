/**
 * Right-click context menu registration for Zotero library items.
 */

import type { McpClient } from "../mcp/client";
import { getRagPaperId } from "../utils/zotero-item";
import { addActivePaperSearchMenuItem } from "./search";
import { uploadSelectedItem } from "./upload";
import { extractAlgorithms, viewAlgorithms } from "./extraction";
import { extractPatterns, viewPatterns } from "./pattern-extraction";

const MENU_IDS = {
  menu: "zotero-itemmenu-rag-menu",
  popup: "zotero-itemmenu-rag-popup",
  upload: "zotero-itemmenu-rag-upload",
  searchPaper: "zotero-itemmenu-rag-search-paper",
  extractPatterns: "zotero-itemmenu-rag-extract-patterns",
  extract: "zotero-itemmenu-rag-extract",
  viewPatterns: "zotero-itemmenu-rag-view-patterns",
  view: "zotero-itemmenu-rag-view",
  separator: "zotero-itemmenu-rag-separator",
};

const windowListeners = new WeakMap<Window, () => void>();

/** Add RAG menu items to a Zotero window */
export function addMenuToWindow(window: Window, client: McpClient): void {
  const doc = window.document;
  const menu = doc.getElementById("zotero-itemmenu");
  if (!menu) return;

  if (doc.getElementById(MENU_IDS.separator)) return;

  const separator = doc.createXULElement("menuseparator");
  separator.id = MENU_IDS.separator;
  menu.appendChild(separator);

  const submenu = doc.createXULElement("menu");
  submenu.id = MENU_IDS.menu;
  submenu.setAttribute("label", "RAG Library");
  menu.appendChild(submenu);

  const popup = doc.createXULElement("menupopup");
  popup.id = MENU_IDS.popup;
  submenu.appendChild(popup);
  addActivePaperSearchMenuItem(doc, popup, client);

  const uploadItem = doc.createXULElement("menuitem");
  uploadItem.id = MENU_IDS.upload;
  uploadItem.setAttribute("label", "Upload to RAG Library");
  uploadItem.addEventListener("command", () => {
    void uploadSelectedItem(client);
  });
  popup.appendChild(uploadItem);

  const extractPatternsItem = doc.createXULElement("menuitem");
  extractPatternsItem.id = MENU_IDS.extractPatterns;
  extractPatternsItem.setAttribute("label", "Extract Patterns");
  extractPatternsItem.addEventListener("command", () => {
    void extractPatterns();
  });
  popup.appendChild(extractPatternsItem);

  const extractItem = doc.createXULElement("menuitem");
  extractItem.id = MENU_IDS.extract;
  extractItem.setAttribute("label", "Extract Algorithms");
  extractItem.addEventListener("command", () => {
    void extractAlgorithms(client);
  });
  popup.appendChild(extractItem);

  const viewPatternsItem = doc.createXULElement("menuitem");
  viewPatternsItem.id = MENU_IDS.viewPatterns;
  viewPatternsItem.setAttribute("label", "View Patterns");
  viewPatternsItem.addEventListener("command", () => {
    void viewPatterns();
  });
  popup.appendChild(viewPatternsItem);

  const viewItem = doc.createXULElement("menuitem");
  viewItem.id = MENU_IDS.view;
  viewItem.setAttribute("label", "View Algorithms");
  viewItem.addEventListener("command", () => {
    void viewAlgorithms(client);
  });
  popup.appendChild(viewItem);

  const listener = () => updateMenuVisibility(window);
  menu.addEventListener("popupshowing", listener);
  windowListeners.set(window, listener);
}

/** Remove RAG menu items from a Zotero window */
export function removeMenuFromWindow(window: Window): void {
  const doc = window.document;
  const listener = windowListeners.get(window);
  if (listener) {
    const menu = doc.getElementById("zotero-itemmenu");
    menu?.removeEventListener("popupshowing", listener);
    windowListeners.delete(window);
  }

  for (const id of Object.values(MENU_IDS)) {
    doc.getElementById(id)?.remove();
  }
}

/** Update menu item visibility based on selected item state */
function updateMenuVisibility(window: Window): void {
  const doc = window.document;
  const zoteroPane = (window as any).ZoteroPane;
  if (!zoteroPane) return;

  const items = zoteroPane.getSelectedItems();
  const regularItems = items.filter((item: Zotero.Item) => item?.isRegularItem?.());
  const singleItem = regularItems.length === 1 ? regularItems[0] : null;
  const hasRegularSelection = regularItems.length > 0;

  const menuEl = doc.getElementById(MENU_IDS.menu);
  const sepEl = doc.getElementById(MENU_IDS.separator);
  const uploadEl = doc.getElementById(MENU_IDS.upload);
  const searchPaperEl = doc.getElementById(MENU_IDS.searchPaper);
  const extractPatternsEl = doc.getElementById(MENU_IDS.extractPatterns);
  const extractEl = doc.getElementById(MENU_IDS.extract);
  const viewPatternsEl = doc.getElementById(MENU_IDS.viewPatterns);
  const viewEl = doc.getElementById(MENU_IDS.view);

  if (!hasRegularSelection) {
    menuEl?.setAttribute("hidden", "true");
    sepEl?.setAttribute("hidden", "true");
    return;
  }

  sepEl?.removeAttribute("hidden");
  menuEl?.removeAttribute("hidden");
  uploadEl?.removeAttribute("hidden");

  if (!singleItem) {
    searchPaperEl?.setAttribute("hidden", "true");
    extractPatternsEl?.setAttribute("hidden", "true");
    extractEl?.setAttribute("hidden", "true");
    viewPatternsEl?.setAttribute("hidden", "true");
    viewEl?.setAttribute("hidden", "true");
    return;
  }

  const ragId = getRagPaperId(singleItem);
  if (ragId) {
    searchPaperEl?.removeAttribute("hidden");
    extractPatternsEl?.removeAttribute("hidden");
    extractEl?.removeAttribute("hidden");
    viewPatternsEl?.removeAttribute("hidden");
    viewEl?.removeAttribute("hidden");
  } else {
    searchPaperEl?.setAttribute("hidden", "true");
    extractPatternsEl?.setAttribute("hidden", "true");
    extractEl?.setAttribute("hidden", "true");
    viewPatternsEl?.setAttribute("hidden", "true");
    viewEl?.setAttribute("hidden", "true");
  }
}
