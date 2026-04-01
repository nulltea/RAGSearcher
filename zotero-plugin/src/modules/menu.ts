/**
 * Right-click context menu registration for Zotero library items.
 */

import type { McpClient } from "../mcp/client";
import { getRagPaperId } from "../utils/zotero-item";
import { uploadSelectedItem } from "./upload";
import { extractAlgorithms, viewAlgorithms } from "./extraction";
import { extractPatterns, viewPatterns } from "./pattern-extraction";

const MENU_IDS = {
  upload: "zotero-itemmenu-rag-upload",
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

  const uploadItem = doc.createXULElement("menuitem");
  uploadItem.id = MENU_IDS.upload;
  uploadItem.setAttribute("label", "Upload to RAG Library");
  uploadItem.addEventListener("command", () => {
    void uploadSelectedItem(client);
  });
  menu.appendChild(uploadItem);

  const extractPatternsItem = doc.createXULElement("menuitem");
  extractPatternsItem.id = MENU_IDS.extractPatterns;
  extractPatternsItem.setAttribute("label", "Extract Patterns");
  extractPatternsItem.addEventListener("command", () => {
    void extractPatterns();
  });
  menu.appendChild(extractPatternsItem);

  const extractItem = doc.createXULElement("menuitem");
  extractItem.id = MENU_IDS.extract;
  extractItem.setAttribute("label", "Extract Algorithms");
  extractItem.addEventListener("command", () => {
    void extractAlgorithms(client);
  });
  menu.appendChild(extractItem);

  const viewPatternsItem = doc.createXULElement("menuitem");
  viewPatternsItem.id = MENU_IDS.viewPatterns;
  viewPatternsItem.setAttribute("label", "View Patterns");
  viewPatternsItem.addEventListener("command", () => {
    void viewPatterns();
  });
  menu.appendChild(viewPatternsItem);

  const viewItem = doc.createXULElement("menuitem");
  viewItem.id = MENU_IDS.view;
  viewItem.setAttribute("label", "View Algorithms");
  viewItem.addEventListener("command", () => {
    void viewAlgorithms(client);
  });
  menu.appendChild(viewItem);

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
  const singleItem = items.length === 1 ? items[0] : null;
  const isRegular = singleItem?.isRegularItem() ?? false;

  const uploadEl = doc.getElementById(MENU_IDS.upload);
  const extractPatternsEl = doc.getElementById(MENU_IDS.extractPatterns);
  const extractEl = doc.getElementById(MENU_IDS.extract);
  const viewPatternsEl = doc.getElementById(MENU_IDS.viewPatterns);
  const viewEl = doc.getElementById(MENU_IDS.view);
  const sepEl = doc.getElementById(MENU_IDS.separator);

  if (!isRegular || !singleItem) {
    uploadEl?.setAttribute("hidden", "true");
    extractPatternsEl?.setAttribute("hidden", "true");
    extractEl?.setAttribute("hidden", "true");
    viewPatternsEl?.setAttribute("hidden", "true");
    viewEl?.setAttribute("hidden", "true");
    sepEl?.setAttribute("hidden", "true");
    return;
  }

  sepEl?.removeAttribute("hidden");
  uploadEl?.removeAttribute("hidden");

  const ragId = getRagPaperId(singleItem);
  if (ragId) {
    extractPatternsEl?.removeAttribute("hidden");
    extractEl?.removeAttribute("hidden");
    viewPatternsEl?.removeAttribute("hidden");
    viewEl?.removeAttribute("hidden");
  } else {
    extractPatternsEl?.setAttribute("hidden", "true");
    extractEl?.setAttribute("hidden", "true");
    viewPatternsEl?.setAttribute("hidden", "true");
    viewEl?.setAttribute("hidden", "true");
  }
}
