import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionResult } from "@kreuzberg/wasm";

interface Plugin {
  name: string;
  execute: (result: ExtractionResult) => Promise<ExtractionResult>;
}

class TextCleanerPlugin implements Plugin {
  name = "text-cleaner";

  async execute(result: ExtractionResult): Promise<ExtractionResult> {
    const cleaned = result.content
      .replace(/\x00/g, "")
      .replace(/\s+/g, " ")
      .trim();

    return { ...result, content: cleaned };
  }
}

class MetadataEnricherPlugin implements Plugin {
  name = "metadata-enricher";

  async execute(result: ExtractionResult): Promise<ExtractionResult> {
    return {
      ...result,
      metadata: {
        ...result.metadata,
        processedAt: new Date().toISOString(),
        contentLength: result.content.length,
      },
    };
  }
}

async function executePipeline(
  bytes: Uint8Array,
  mimeType: string,
  plugins: Plugin[]
): Promise<ExtractionResult> {
  await initWasm();

  let result = await extractBytes(bytes, mimeType);

  for (const plugin of plugins) {
    console.log(`Executing plugin: ${plugin.name}`);
    result = await plugin.execute(result);
  }

  return result;
}

const pipeline = [new TextCleanerPlugin(), new MetadataEnricherPlugin()];

executePipeline(new Uint8Array([1, 2, 3]), "application/pdf", pipeline)
  .then((r) => console.log("Pipeline complete", r))
  .catch(console.error);
