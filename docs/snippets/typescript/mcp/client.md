```typescript title="TypeScript"
import { spawn } from "node:child_process";
import * as readline from "node:readline";

/**
 * MCP Client for Kreuzberg
 * Communicates with Kreuzberg MCP server via stdio
 * @example
 * const client = new KreuzbergMcpClient();
 * await client.connect();
 * const result = await client.callTool("extract_file", { path: "doc.pdf" });
 */
class KreuzbergMcpClient {
  private process: ReturnType<typeof spawn> | null = null;
  private rl: readline.Interface | null = null;
  private requestId: number = 0;
  private pendingRequests: Map<
    number,
    {
      resolve: (value: unknown) => void;
      reject: (error: Error) => void;
    }
  > = new Map();

  /**
   * Connect to MCP server
   */
  async connect(): Promise<void> {
    this.process = spawn("kreuzberg", ["mcp"]);

    this.rl = readline.createInterface({
      input: this.process.stdout,
      output: this.process.stdin,
      terminal: false,
    });

    // Handle incoming responses
    this.rl.on("line", (line) => {
      try {
        const response = JSON.parse(line) as {
          id: number;
          result?: unknown;
          error?: { message: string };
        };
        const pending = this.pendingRequests.get(response.id);

        if (pending) {
          if (response.error) {
            pending.reject(new Error(response.error.message));
          } else {
            pending.resolve(response.result);
          }
          this.pendingRequests.delete(response.id);
        }
      } catch (error) {
        console.error("Failed to parse response:", error);
      }
    });

    // Handle errors
    this.process.stderr?.on("data", (data) => {
      console.error("MCP server error:", data.toString());
    });

    // Wait for initialization
    await this.listTools();
  }

  /**
   * List available tools
   */
  async listTools(): Promise<Array<{ name: string; description: string }>> {
    return this.sendRequest("tools/list", {}) as Promise<
      Array<{ name: string; description: string }>
    >;
  }

  /**
   * Call a tool on the server
   */
  async callTool(
    toolName: string,
    args: Record<string, unknown>
  ): Promise<unknown> {
    return this.sendRequest("tools/call", {
      name: toolName,
      arguments: args,
    });
  }

  /**
   * Extract file from path
   */
  async extractFile(
    path: string,
    async: boolean = false
  ): Promise<Record<string, unknown>> {
    return this.callTool("extract_file", {
      path,
      async,
    }) as Promise<Record<string, unknown>>;
  }

  /**
   * Extract from bytes
   */
  async extractBytes(
    data: Uint8Array,
    mimeType: string,
    async: boolean = false
  ): Promise<Record<string, unknown>> {
    const base64 = Buffer.from(data).toString("base64");
    return this.callTool("extract_bytes", {
      data: base64,
      mimeType,
      async,
    }) as Promise<Record<string, unknown>>;
  }

  /**
   * Send request to server
   */
  private sendRequest(
    method: string,
    params: Record<string, unknown>
  ): Promise<unknown> {
    return new Promise((resolve, reject) => {
      const id = ++this.requestId;
      this.pendingRequests.set(id, { resolve, reject });

      const request = {
        jsonrpc: "2.0",
        id,
        method,
        params,
      };

      this.process?.stdin.write(JSON.stringify(request) + "\n");
    });
  }

  /**
   * Disconnect from server
   */
  disconnect(): void {
    this.rl?.close();
    this.process?.kill();
  }
}

// Usage example
async function main(): Promise<void> {
  const client = new KreuzbergMcpClient();

  try {
    // Connect to MCP server
    await client.connect();
    console.log("Connected to Kreuzberg MCP server");

    // List available tools
    const tools = await client.listTools();
    console.log("Available tools:", tools.map((t) => t.name));

    // Extract file
    const result = await client.extractFile("document.pdf", true);
    console.log("Extraction result:", result);
  } catch (error) {
    console.error("Error:", error);
  } finally {
    client.disconnect();
  }
}

// Run if executed directly
if (require.main === module) {
  main();
}

export { KreuzbergMcpClient };
```
