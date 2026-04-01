/**
 * Zotero RAG Library plugin — main entry point.
 *
 * Exported as the global `ZoteroRAG` object, called from bootstrap.js.
 * Manages MCP client lifecycle and window menu registration.
 */

import { McpClient } from "./mcp/client";
import { addMenuToWindow, removeMenuFromWindow } from "./modules/menu";
import { registerItemPane, unregisterItemPane } from "./modules/item-pane";
import { addSearchUiToWindow, removeSearchUiFromWindow } from "./modules/search";

let client: McpClient | null = null;
let pluginId: string;
let itemPaneSectionId: string | null = null;

/** Root URI of the plugin, used to resolve addon content paths */
export let pluginRootURI: string;

/** Initialize the plugin */
function init(params: { id: string; version: string; rootURI: string }): void {
  pluginId = params.id;
  pluginRootURI = params.rootURI;

  const prefKey = "extensions.zotero.zoterorag.binaryPath";
  let binaryPath: string | undefined;
  try {
    const pref = Zotero.Prefs.get(prefKey, true) as string | undefined;
    if (pref && pref.trim()) binaryPath = pref.trim();
  } catch { /* pref not set */ }

  client = new McpClient(binaryPath || undefined);

  // Register item pane section
  try {
    itemPaneSectionId = registerItemPane(client, pluginId);
  } catch (e) {
    Zotero.debug(`[RAG] Failed to register item pane: ${e}`);
  }

  Zotero.debug(`[RAG] Plugin initialized (id: ${pluginId})`);
}

/** Add menu items to a Zotero window */
function addToWindow(window: Window): void {
  if (!client) return;
  try { addMenuToWindow(window, client); } catch (e) { Zotero.debug(`[RAG] addMenuToWindow failed: ${e}`); }
  try { addSearchUiToWindow(window, client); } catch (e) { Zotero.debug(`[RAG] addSearchUiToWindow failed: ${e}`); }
}

/** Remove menu items from a Zotero window */
function removeFromWindow(window: Window): void {
  removeMenuFromWindow(window);
  removeSearchUiFromWindow(window);
}

/** Add to all currently open windows */
function addToAllWindows(): void {
  for (const win of Zotero.getMainWindows()) {
    if ((win as any).ZoteroPane) addToWindow(win);
  }
}

/** Remove from all currently open windows */
function removeFromAllWindows(): void {
  for (const win of Zotero.getMainWindows()) removeFromWindow(win);
}

/** Shutdown */
function shutdown(): void {
  if (itemPaneSectionId) {
    try { unregisterItemPane(itemPaneSectionId); } catch { /* already removed */ }
    itemPaneSectionId = null;
  }
  if (client) {
    client.shutdown().catch((e: any) => Zotero.debug(`[RAG] Shutdown error: ${e}`));
    client = null;
  }
  Zotero.debug("[RAG] Plugin shut down");
}

export { init, addToWindow, removeFromWindow, addToAllWindows, removeFromAllWindows, shutdown };
