/**
 * High-level MCP client wrapping tool calls.
 */

import { McpProcess } from "./process";
import type {
  ExtractAlgorithmsArgs,
  ExtractAlgorithmsResult,
  IndexPaperArgs,
  IndexPaperResult,
  McpToolCallResult,
  SearchAlgorithmsArgs,
  SearchAlgorithmsResult,
  SearchPapersArgs,
  SearchPapersResult,
} from "./types";

export class McpClient {
  private process: McpProcess;
  private initialized = false;

  constructor(binaryPath?: string) {
    this.process = new McpProcess(binaryPath);
  }

  /** Ensure the process is running and initialized */
  private async ensureInitialized(): Promise<void> {
    if (this.initialized) return;
    await this.process.ensureRunning();
    await this.process.initialize();
    this.initialized = true;
  }

  /** Call an MCP tool and parse the JSON result */
  private async callTool<T>(name: string, args: Record<string, unknown>): Promise<T> {
    await this.ensureInitialized();

    const response = await this.process.request("tools/call", {
      name,
      arguments: args,
    });

    if (response.error) {
      throw new Error(`Tool ${name} failed: ${response.error.message}`);
    }

    const result = response.result as McpToolCallResult;
    if (result.isError) {
      const text = result.content?.[0]?.text || "Unknown error";
      throw new Error(`Tool ${name} error: ${text}`);
    }

    const text = result.content?.[0]?.text;
    if (!text) {
      throw new Error(`Tool ${name} returned no content`);
    }

    return JSON.parse(text) as T;
  }

  /** Index a paper from a local file path */
  async indexPaper(args: IndexPaperArgs): Promise<IndexPaperResult> {
    return this.callTool<IndexPaperResult>("index_paper", args as Record<string, unknown>);
  }

  /** Extract algorithms from an indexed paper */
  async extractAlgorithms(args: ExtractAlgorithmsArgs): Promise<ExtractAlgorithmsResult> {
    return this.callTool<ExtractAlgorithmsResult>(
      "extract_algorithms",
      args as Record<string, unknown>,
    );
  }

  /** Search algorithms across papers */
  async searchAlgorithms(args: SearchAlgorithmsArgs): Promise<SearchAlgorithmsResult> {
    return this.callTool<SearchAlgorithmsResult>(
      "search_algorithms",
      args as Record<string, unknown>,
    );
  }

  /** Search papers by metadata */
  async searchPapers(args: SearchPapersArgs): Promise<SearchPapersResult> {
    return this.callTool<SearchPapersResult>("search_papers", args as Record<string, unknown>);
  }

  /** Shutdown the MCP process */
  async shutdown(): Promise<void> {
    this.initialized = false;
    await this.process.shutdown();
  }

  /** Check if the MCP process is running */
  get isRunning(): boolean {
    return this.process.isRunning;
  }
}
