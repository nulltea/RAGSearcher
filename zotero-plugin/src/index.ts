/**
 * Zotero RAG Library plugin — main entry point.
 *
 * Exported as the global `ZoteroRAG` object, called from bootstrap.js.
 * Manages MCP client lifecycle and window menu registration.
 */

import { McpClient } from "./mcp/client";
import { addMenuToWindow, removeMenuFromWindow } from "./modules/menu";

let client: McpClient | null = null;
let pluginId: string;

/** Root URI of the plugin, used to resolve addon content paths */
export let pluginRootURI: string;

/** Initialize the plugin */
function init(params: { id: string; version: string; rootURI: string }): void {
  pluginId = params.id;
  pluginRootURI = params.rootURI;

  // Read binary path from preferences, fallback to auto-detect
  const prefKey = "extensions.zotero.zoterorag.binaryPath";
  let binaryPath: string | undefined;
  try {
    const pref = Zotero.Prefs.get(prefKey, true) as string | undefined;
    if (pref && pref.trim()) {
      binaryPath = pref.trim();
    }
  } catch {
    // Pref not set
  }

  client = new McpClient(binaryPath || undefined);
  Zotero.debug(`[RAG] Plugin initialized (id: ${pluginId})`);
}

/** Add menu items to a Zotero window */
function addToWindow(window: Window): void {
  if (!client) return;
  addMenuToWindow(window, client);
}

/** Remove menu items from a Zotero window */
function removeFromWindow(window: Window): void {
  removeMenuFromWindow(window);
}

/** Add to all currently open windows */
function addToAllWindows(): void {
  const windows = Zotero.getMainWindows();
  for (const win of windows) {
    if ((win as any).ZoteroPane) {
      addToWindow(win);
    }
  }
}

/** Remove from all currently open windows */
function removeFromAllWindows(): void {
  const windows = Zotero.getMainWindows();
  for (const win of windows) {
    removeFromWindow(win);
  }
}

/** Shutdown: kill MCP process */
function shutdown(): void {
  if (client) {
    client.shutdown().catch((e: any) => {
      Zotero.debug(`[RAG] Shutdown error: ${e}`);
    });
    client = null;
  }
  Zotero.debug("[RAG] Plugin shut down");
}

export { init, addToWindow, removeFromWindow, addToAllWindows, removeFromAllWindows, shutdown };
