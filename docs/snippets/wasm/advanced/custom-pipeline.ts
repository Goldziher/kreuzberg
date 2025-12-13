import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionResult } from "@kreuzberg/wasm";

interface ProcessingStep {
  name: string;
  process: (result: ExtractionResult) => Promise<ExtractionResult>;
}

async function createExtractionPipeline(
  steps: ProcessingStep[],
  bytes: Uint8Array,
  mimeType: string
) {
  await initWasm();

  let result = await extractBytes(bytes, mimeType);

  for (const step of steps) {
    console.log(`Executing step: ${step.name}`);
    result = await step.process(result);
  }

  return result;
}

const pipeline: ProcessingStep[] = [
  {
    name: "Text Normalization",
    process: async (result) => ({
      ...result,
      content: result.content.replace(/\s+/g, " ").trim(),
    }),
  },
  {
    name: "Language Detection",
    process: async (result) => result,
  },
  {
    name: "Chunking",
    process: async (result) => result,
  },
];

createExtractionPipeline(
  pipeline,
  new Uint8Array(await fetch("doc.pdf").then((r) => r.arrayBuffer())),
  "application/pdf"
)
  .then((r) => console.log("Pipeline complete:", r))
  .catch(console.error);
