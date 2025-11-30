```typescript
import {
	extractFile,
	extractFileSync,
	extractBytes,
	extractBytesSync,
	type ExtractionResult,
	type ExtractionConfig,
} from '@kreuzberg/node';
import * as readline from "node:readline";

/**
 * MCP Server for Kreuzberg
 * Exposes document extraction as MCP tools
 * @example
 * const server = new KreuzbergMcpServer();
 * await server.start();
 */
class KreuzbergMcpServer {
  private config?: ExtractionConfig;
  private rl: readline.Interface;

  constructor(config?: ExtractionConfig) {
    this.config = config;
    this.rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
      terminal: false,
    });
  }

  /**
   * Start MCP server
   */
  async start(): Promise<void> {
    console.error("[MCP Server] Starting Kreuzberg MCP server");

    this.rl.on("line", async (line) => {
      try {
        const request = JSON.parse(line) as {
          id: number;
          method: string;
          params: Record<string, unknown>;
        };

        const response = await this.handleRequest(request);
        process.stdout.write(JSON.stringify(response) + "\n");
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : "Unknown error";
        process.stdout.write(
          JSON.stringify({
            id: 0,
            error: { message: errorMessage },
          }) + "\n"
        );
      }
    });
  }

  /**
   * Handle incoming MCP request
   */
  private async handleRequest(request: {
    id: number;
    method: string;
    params: Record<string, unknown>;
  }): Promise<Record<string, unknown>> {
    const { id, method, params } = request;

    if (method === "tools/list") {
      return {
        id,
        result: this.listTools(),
      };
    }

    if (method === "tools/call") {
      const result = await this.callTool(
        params.name as string,
        params.arguments as Record<string, unknown>
      );
      return {
        id,
        result,
      };
    }

    throw new Error(`Unknown method: ${method}`);
  }

  /**
   * List available tools
   */
  private listTools(): Array<{
    name: string;
    description: string;
    inputSchema: Record<string, unknown>;
  }> {
    return [
      {
        name: "extract_file",
        description:
          "Extract content from a file by path",
        inputSchema: {
          type: "object",
          properties: {
            path: { type: "string", description: "Path to file" },
            async: { type: "boolean", description: "Use async extraction" },
            config: {
              type: "object",
              description: "Optional extraction config",
            },
          },
          required: ["path"],
        },
      },
      {
        name: "extract_bytes",
        description:
          "Extract content from raw bytes",
        inputSchema: {
          type: "object",
          properties: {
            data: { type: "string", description: "Base64-encoded data" },
            mimeType: { type: "string", description: "MIME type" },
            async: { type: "boolean", description: "Use async extraction" },
            config: {
              type: "object",
              description: "Optional extraction config",
            },
          },
          required: ["data", "mimeType"],
        },
      },
    ];
  }

  /**
   * Call tool
   */
  private async callTool(
    name: string,
    args: Record<string, unknown>
  ): Promise<unknown> {
    if (name === "extract_file") {
      const path = args.path as string;
      const useAsync = (args.async as boolean) ?? true;
      const config = (args.config as ExtractionConfig) ?? this.config;

      if (useAsync) {
        return extractFile(path, null, config);
      } else {
        return extractFileSync(path, null, config);
      }
    }

    if (name === "extract_bytes") {
      const data = Buffer.from(args.data as string, "base64");
      const mimeType = args.mimeType as string;
      const useAsync = (args.async as boolean) ?? true;
      const config = (args.config as ExtractionConfig) ?? this.config;

      if (useAsync) {
        return extractBytes(data, mimeType, config);
      } else {
        return extractBytesSync(data, mimeType, config);
      }
    }

    throw new Error(`Unknown tool: ${name}`);
  }
}

/**
 * Main entry point
 */
async function main(): Promise<void> {
  const config: ExtractionConfig = {
    ocr: {
      enabled: true,
      backend: "tesseract",
    },
  };

  const server = new KreuzbergMcpServer(config);
  await server.start();
}

// Start server when invoked as MCP
if (process.argv[2] === "mcp") {
  main().catch((error) => {
    console.error("Server error:", error);
    process.exit(1);
  });
}

export { KreuzbergMcpServer };
```
