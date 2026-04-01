/**
 * Pattern extraction flow: runs CLI commands (not MCP) to extract/list patterns.
 */

import type {
  PatternResult,
  ExtractPatternsCliResult,
  ListPatternsCliResult,
} from "../mcp/types";
import { getRagPaperId } from "../utils/zotero-item";
import { showProgress, showSuccess, showError } from "../utils/notify";

// Subprocess types (same as process.ts)
interface SubprocessPipe {
  readString(count?: number): Promise<string>;
  close(): Promise<void>;
}

interface SubprocessHandle {
  stdout: SubprocessPipe;
  stderr?: SubprocessPipe;
  wait(): Promise<{ exitCode: number }>;
}

interface SubprocessModule {
  call(options: {
    command: string;
    arguments?: string[];
    environment?: Record<string, string>;
    environmentAppend?: boolean;
    stderr?: "pipe" | "stdout";
  }): Promise<SubprocessHandle>;
  pathSearch(name: string): Promise<string>;
}

let subprocessModule: SubprocessModule | null = null;

function getSubprocess(): SubprocessModule {
  if (!subprocessModule) {
    const mod = ChromeUtils.importESModule(
      "resource://gre/modules/Subprocess.sys.mjs",
    );
    subprocessModule = mod.Subprocess as SubprocessModule;
  }
  return subprocessModule;
}

/** Resolve the rag-searcher binary path (same logic as process.ts) */
async function resolveBinaryPath(): Promise<string> {
  const Sub = getSubprocess();

  // Check preferences first
  const prefKey = "extensions.zotero.zoterorag.binaryPath";
  let binaryName = "rag-searcher";
  try {
    const pref = Zotero.Prefs.get(prefKey, true) as string | undefined;
    if (pref && pref.trim()) {
      return pref.trim();
    }
  } catch { /* not set */ }

  // Try PATH
  try {
    return await Sub.pathSearch(binaryName);
  } catch { /* not in PATH */ }

  // Check common install locations
  const homeDir = Services.dirsvc.get("Home", Ci.nsIFile).path;
  const cargoPath = PathUtils.join(homeDir, ".cargo", "bin", binaryName);
  if (await IOUtils.exists(cargoPath)) {
    return cargoPath;
  }

  throw new Error(
    `"${binaryName}" not found in PATH or ~/.cargo/bin. Set the full path in RAG Library preferences.`,
  );
}

/** Run a one-shot rag-searcher CLI command and return stdout */
async function runCliCommand(args: string[]): Promise<string> {
  const Sub = getSubprocess();
  const command = await resolveBinaryPath();

  // Build augmented PATH
  const homeDir = Services.dirsvc.get("Home", Ci.nsIFile).path;
  const extraPaths = [
    PathUtils.join(homeDir, ".local", "bin"),
    PathUtils.join(homeDir, ".cargo", "bin"),
    "/usr/local/bin",
    "/opt/homebrew/bin",
  ];
  const currentPath = Services.env?.get?.("PATH") || "/usr/bin:/bin:/usr/sbin:/sbin";
  const augmentedPath = [...extraPaths, currentPath].join(":");

  const proc = await Sub.call({
    command,
    arguments: args,
    environment: { PATH: augmentedPath },
    environmentAppend: true,
    stderr: "pipe",
  });

  // Read all stdout
  let stdout = "";
  while (true) {
    const chunk = await proc.stdout.readString();
    if (!chunk) break;
    stdout += chunk;
  }

  // Drain stderr (for logging)
  let stderr = "";
  if (proc.stderr) {
    try {
      while (true) {
        const chunk = await proc.stderr.readString();
        if (!chunk) break;
        stderr += chunk;
      }
    } catch { /* expected at EOF */ }
  }

  const { exitCode } = await proc.wait();
  if (exitCode !== 0) {
    Zotero.debug(`[RAG] CLI stderr: ${stderr}`);
    throw new Error(stderr.trim() || `rag-searcher exited with code ${exitCode}`);
  }

  return stdout;
}

/** List patterns for a paper via CLI */
export async function listPatterns(paperId: string): Promise<PatternResult[]> {
  const json = await runCliCommand(["list-patterns", paperId]);
  const result = JSON.parse(json) as ListPatternsCliResult;
  return result.patterns;
}

/** Open the pattern detail dialog */
export function openPatternDialog(paperTitle: string, patterns: PatternResult[]): void {
  const win = Zotero.getMainWindow();
  const windowName = `rag-patterns-${Date.now()}`;
  win.openDialog(
    "chrome://zoterorag/content/pattern-dialog.xhtml",
    windowName,
    "chrome,centerscreen,resizable,width=850,height=700",
    { paperTitle, patterns },
  );
}

/** Extract patterns from a paper (checks for existing first) */
export async function extractPatterns(): Promise<void> {
  const zoteroPane = (Zotero.getMainWindow() as any).ZoteroPane;
  if (!zoteroPane) return;
  const items = zoteroPane.getSelectedItems();
  if (items.length !== 1) return;

  const item = items[0];
  const paperId = getRagPaperId(item);
  if (!paperId) {
    const progress = showProgress("RAG Library");
    progress.fail("Paper not uploaded to RAG Library yet. Upload it first.");
    return;
  }

  // Check if patterns already exist
  try {
    const existing = await listPatterns(paperId);
    if (existing.length > 0) {
      showSuccess(`Already extracted (${existing.length} patterns).`);
      const title = (item.getField("title") as string) || "Untitled";
      openPatternDialog(title, existing);
      return;
    }
  } catch (e) {
    Zotero.debug(`[RAG] Pattern check failed, proceeding with extraction: ${e}`);
  }

  const progress = showProgress("Extracting Patterns");
  progress.update("Running 3-pass AI pipeline (this may take a few minutes)...");

  try {
    const json = await runCliCommand(["extract-patterns", paperId]);
    const result = JSON.parse(json) as ExtractPatternsCliResult;
    progress.close();
    showSuccess(`Extracted ${result.pattern_count} patterns.`);

    // Fetch full pattern data
    const patterns = await listPatterns(paperId);
    if (patterns.length > 0) {
      const title = (item.getField("title") as string) || "Untitled";
      openPatternDialog(title, patterns);
    }
  } catch (e: any) {
    Zotero.debug(`[RAG] Pattern extraction failed: ${e}`);
    progress.close();
    showError(`Pattern extraction failed: ${e.message || e}`);
  }
}

/** View existing patterns for a paper */
export async function viewPatterns(): Promise<void> {
  const zoteroPane = (Zotero.getMainWindow() as any).ZoteroPane;
  if (!zoteroPane) return;
  const items = zoteroPane.getSelectedItems();
  if (items.length !== 1) return;

  const item = items[0];
  const paperId = getRagPaperId(item);
  if (!paperId) return;

  const progress = showProgress("RAG Library");
  progress.update("Loading patterns...");

  try {
    const patterns = await listPatterns(paperId);
    progress.close();

    if (patterns.length > 0) {
      const title = (item.getField("title") as string) || "Untitled";
      openPatternDialog(title, patterns);
    } else {
      showSuccess("No patterns found for this paper.");
    }
  } catch (e: any) {
    Zotero.debug(`[RAG] Failed to load patterns: ${e}`);
    progress.close();
    showError(`Failed: ${e.message || e}`);
  }
}
