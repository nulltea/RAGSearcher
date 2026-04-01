/**
 * MCP process lifecycle manager.
 * Spawns rag-searcher as a child process, keeps it alive for the session.
 */

import type { JsonRpcRequest, JsonRpcResponse } from "./types";

// Firefox/Zotero Subprocess types (simplified)
interface SubprocessPipe {
  write(data: string): Promise<number>;
  close(): Promise<void>;
  readString(count?: number): Promise<string>;
}

interface SubprocessHandle {
  stdin: SubprocessPipe;
  stdout: SubprocessPipe;
  stderr?: SubprocessPipe;
  wait(): Promise<{ exitCode: number }>;
  kill(timeout?: number): void;
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

// Subprocess is loaded lazily from Mozilla platform
let subprocessModule: SubprocessModule | null = null;

function getSubprocess(): SubprocessModule {
  if (!subprocessModule) {
    // Zotero 7 / Firefox 115+ ESM import
    const mod = ChromeUtils.importESModule(
      "resource://gre/modules/Subprocess.sys.mjs",
    );
    subprocessModule = mod.Subprocess as SubprocessModule;
  }
  return subprocessModule;
}

export class McpProcess {
  private process: SubprocessHandle | null = null;
  private binaryPath: string;
  private requestId = 0;
  private pendingRequests = new Map<
    number,
    {
      resolve: (value: JsonRpcResponse) => void;
      reject: (error: Error) => void;
    }
  >();
  private buffer = "";
  private readLoopRunning = false;

  constructor(binaryPath?: string) {
    this.binaryPath = binaryPath || "rag-searcher";
  }

  /** Check if the process is running */
  get isRunning(): boolean {
    return this.process !== null;
  }

  /** Start the MCP server process if not already running */
  async ensureRunning(): Promise<void> {
    if (this.process) return;

    const Sub = getSubprocess();

    // Resolve the binary path
    let command = this.binaryPath;
    if (!command.startsWith("/")) {
      // Try PATH first
      try {
        command = await Sub.pathSearch(command);
      } catch {
        // Zotero's PATH often lacks ~/.cargo/bin — check common install locations
        const homeDir = Services.dirsvc.get("Home", Ci.nsIFile).path;
        const cargoPath = PathUtils.join(homeDir, ".cargo", "bin", this.binaryPath);
        if (await IOUtils.exists(cargoPath)) {
          command = cargoPath;
        } else {
          throw new Error(
            `"${this.binaryPath}" not found in PATH or ~/.cargo/bin. Set the full path in RAG Library preferences.`,
          );
        }
      }
    }

    Zotero.debug(`[RAG] Spawning MCP server: ${command} serve`);

    // Build augmented PATH — Zotero's environment often lacks user bin dirs
    const homeDir = Services.dirsvc.get("Home", Ci.nsIFile).path;
    const extraPaths = [
      PathUtils.join(homeDir, ".local", "bin"),
      PathUtils.join(homeDir, ".cargo", "bin"),
      "/usr/local/bin",
      "/opt/homebrew/bin",
    ];
    const currentPath = Services.env?.get?.("PATH") || "/usr/bin:/bin:/usr/sbin:/sbin";
    const augmentedPath = [...extraPaths, currentPath].join(":");

    this.process = await Sub.call({
      command,
      arguments: ["serve"],
      environment: { PATH: augmentedPath },
      environmentAppend: true,
      stderr: "pipe",
    });

    // Start reading stdout in the background
    this.startReadLoop();

    // Drain stderr in background to prevent pipe blocking
    this.drainStderr();
  }

  /** Send a JSON-RPC request and wait for the response */
  async request(method: string, params?: Record<string, unknown>): Promise<JsonRpcResponse> {
    await this.ensureRunning();

    const id = ++this.requestId;
    const request: JsonRpcRequest = {
      jsonrpc: "2.0",
      id,
      method,
      ...(params !== undefined && { params }),
    };

    const promise = new Promise<JsonRpcResponse>((resolve, reject) => {
      this.pendingRequests.set(id, { resolve, reject });

      // Timeout after 600 seconds (extraction runs 3 Claude CLI passes)
      const timer = setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error(`MCP request timed out after 600s: ${method}`));
        }
      }, 600_000);

      // Clear timeout when resolved
      const origResolve = resolve;
      const origReject = reject;
      this.pendingRequests.set(id, {
        resolve: (v) => { clearTimeout(timer); origResolve(v); },
        reject: (e) => { clearTimeout(timer); origReject(e); },
      });
    });

    const payload = JSON.stringify(request) + "\n";
    await this.process!.stdin.write(payload);

    return promise;
  }

  /** Initialize the MCP session (required before tool calls) */
  async initialize(): Promise<void> {
    const response = await this.request("initialize", {
      protocolVersion: "2024-11-05",
      capabilities: {},
      clientInfo: {
        name: "zotero-rag-library",
        version: "0.1.0",
      },
    });

    if (response.error) {
      throw new Error(`MCP initialize failed: ${response.error.message}`);
    }

    // Send initialized notification (no response expected, no id)
    const notification = JSON.stringify({
      jsonrpc: "2.0",
      method: "notifications/initialized",
    }) + "\n";
    await this.process!.stdin.write(notification);

    Zotero.debug("[RAG] MCP session initialized");
  }

  /** Stop the MCP server process */
  async shutdown(): Promise<void> {
    if (!this.process) return;

    const proc = this.process;
    this.process = null;
    this.readLoopRunning = false;

    try {
      proc.kill();
    } catch {
      // Process may already be dead
    }

    // Reject all pending requests
    for (const [, { reject }] of this.pendingRequests) {
      reject(new Error("MCP process shutting down"));
    }
    this.pendingRequests.clear();
    this.buffer = "";

    Zotero.debug("[RAG] MCP process shut down");
  }

  /** Read stdout continuously and dispatch responses */
  private async startReadLoop(): Promise<void> {
    if (this.readLoopRunning) return;
    this.readLoopRunning = true;

    try {
      while (this.readLoopRunning && this.process) {
        const data = await this.process.stdout.readString();
        if (!data) {
          // EOF — process exited
          break;
        }

        this.buffer += data;
        this.processBuffer();
      }
    } catch (e) {
      if (this.readLoopRunning) {
        Zotero.debug(`[RAG] MCP stdout read error: ${e}`);
      }
    }

    // Process exited unexpectedly
    if (this.process) {
      Zotero.debug("[RAG] MCP process exited unexpectedly, clearing state");
      this.process = null;
      this.readLoopRunning = false;

      for (const [, { reject }] of this.pendingRequests) {
        reject(new Error("MCP process exited unexpectedly"));
      }
      this.pendingRequests.clear();
    }
  }

  /** Drain stderr to prevent pipe blocking (log to Zotero debug) */
  private async drainStderr(): Promise<void> {
    if (!this.process?.stderr) return;
    try {
      while (this.process) {
        const data = await this.process.stderr!.readString();
        if (!data) break;
        // Log stderr output for debugging
        for (const line of data.split("\n")) {
          if (line.trim()) {
            Zotero.debug(`[RAG stderr] ${line}`);
          }
        }
      }
    } catch {
      // Expected when process exits
    }
  }

  /** Parse complete JSON-RPC messages from the buffer */
  private processBuffer(): void {
    // MCP uses newline-delimited JSON
    const lines = this.buffer.split("\n");
    // Keep the last (potentially incomplete) chunk
    this.buffer = lines.pop() || "";

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;

      try {
        const message = JSON.parse(trimmed) as JsonRpcResponse;

        // Only dispatch responses (messages with an id)
        if (message.id !== undefined) {
          const pending = this.pendingRequests.get(message.id);
          if (pending) {
            this.pendingRequests.delete(message.id);
            pending.resolve(message);
          }
        }
        // Notifications (no id) are logged
        else {
          Zotero.debug(`[RAG] MCP notification: ${trimmed.slice(0, 200)}`);
        }
      } catch {
        Zotero.debug(`[RAG] Failed to parse MCP message: ${trimmed.slice(0, 200)}`);
      }
    }
  }
}
