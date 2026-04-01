/**
 * Right-click context menu registration for Zotero library items.
 */

import type { McpClient } from "../mcp/client";
import { getRagPaperId } from "../utils/zotero-item";
import { uploadSelectedItem } from "./upload";
import { extractAlgorithms, viewAlgorithms } from "./extraction";

const MENU_IDS = {
  upload: "zotero-itemmenu-rag-upload",
  extract: "zotero-itemmenu-rag-extract",
  view: "zotero-itemmenu-rag-view",
  separator: "zotero-itemmenu-rag-separator",
};

// Store listeners per window for cleanup
const windowListeners = new WeakMap<Window, () => void>();

/** Add RAG menu items to a Zotero window */
export function addMenuToWindow(window: Window, client: McpClient): void {
  const doc = window.document;
  const menu = doc.getElementById("zotero-itemmenu");
  if (!menu) return;

  // Prevent double-registration
  if (doc.getElementById(MENU_IDS.separator)) return;

  // Separator
  const sep = doc.createXULElement("menuseparator");
  sep.id = MENU_IDS.separator;
  menu.appendChild(sep);

  // Upload to RAG Library
  const uploadItem = doc.createXULElement("menuitem");
  uploadItem.id = MENU_IDS.upload;
  uploadItem.setAttribute("label", "Upload to RAG Library");
  uploadItem.addEventListener("command", () => uploadSelectedItem(client));
  menu.appendChild(uploadItem);

  // Extract Algorithms
  const extractItem = doc.createXULElement("menuitem");
  extractItem.id = MENU_IDS.extract;
  extractItem.setAttribute("label", "Extract Algorithms");
  extractItem.addEventListener("command", () => extractAlgorithms(client));
  menu.appendChild(extractItem);

  // View Algorithms
  const viewItem = doc.createXULElement("menuitem");
  viewItem.id = MENU_IDS.view;
  viewItem.setAttribute("label", "View Algorithms");
  viewItem.addEventListener("command", () => viewAlgorithms(client));
  menu.appendChild(viewItem);

  // Update visibility on menu showing
  const listener = () => updateMenuVisibility(window);
  menu.addEventListener("popupshowing", listener);
  windowListeners.set(window, listener);
}

/** Remove RAG menu items from a Zotero window */
export function removeMenuFromWindow(window: Window): void {
  const doc = window.document;

  // Remove popupshowing listener
  const listener = windowListeners.get(window);
  if (listener) {
    const menu = doc.getElementById("zotero-itemmenu");
    menu?.removeEventListener("popupshowing", listener);
    windowListeners.delete(window);
  }

  // Remove menu elements
  for (const id of Object.values(MENU_IDS)) {
    const el = doc.getElementById(id);
    el?.remove();
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
  const extractEl = doc.getElementById(MENU_IDS.extract);
  const viewEl = doc.getElementById(MENU_IDS.view);
  const sepEl = doc.getElementById(MENU_IDS.separator);

  // Hide all if not a single regular item
  if (!isRegular || !singleItem) {
    uploadEl?.setAttribute("hidden", "true");
    extractEl?.setAttribute("hidden", "true");
    viewEl?.setAttribute("hidden", "true");
    sepEl?.setAttribute("hidden", "true");
    return;
  }

  sepEl?.removeAttribute("hidden");

  const ragId = getRagPaperId(singleItem);

  // Always show Upload (handles stale RAG-IDs via server validation)
  uploadEl?.removeAttribute("hidden");

  if (ragId) {
    // RAG-ID exists: also show extract and view
    extractEl?.removeAttribute("hidden");
    viewEl?.removeAttribute("hidden");
  } else {
    extractEl?.setAttribute("hidden", "true");
    viewEl?.setAttribute("hidden", "true");
  }
}
