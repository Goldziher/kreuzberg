import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionResult } from "@kreuzberg/wasm";

class MarkdownFormatter {
  async process(result: ExtractionResult): Promise<ExtractionResult> {
    const formatted = result.content
      .replace(/^(.+)$/gm, "# $1")
      .replace(/\n\n+/g, "\n\n");

    return {
      ...result,
      content: formatted,
    };
  }

  getName(): string {
    return "markdown-formatter";
  }

  getVersion(): string {
    return "1.0.0";
  }
}

async function demonstrateCustomProcessor() {
  await initWasm();

  const processor = new MarkdownFormatter();
  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  let result = await extractBytes(bytes, "application/pdf");

  result = await processor.process(result);
  console.log("Formatted result:", result.content);

  return result;
}

demonstrateCustomProcessor().catch(console.error);
