import type { ExtractionResult, Table, Chunk, ExtractedImage } from "@kreuzberg/wasm";

function isTable(obj: unknown): obj is Table {
  if (!obj || typeof obj !== "object") return false;
  const t = obj as Record<string, unknown>;
  return (
    Array.isArray(t.cells) &&
    typeof t.markdown === "string" &&
    typeof t.pageNumber === "number"
  );
}

function isChunk(obj: unknown): obj is Chunk {
  if (!obj || typeof obj !== "object") return false;
  const c = obj as Record<string, unknown>;
  return (
    typeof c.content === "string" &&
    c.metadata &&
    typeof c.metadata === "object" &&
    typeof (c.metadata as Record<string, unknown>).charStart === "number"
  );
}

function isExtractedImage(obj: unknown): obj is ExtractedImage {
  if (!obj || typeof obj !== "object") return false;
  const i = obj as Record<string, unknown>;
  return (
    i.data instanceof Uint8Array &&
    typeof i.format === "string" &&
    typeof i.imageIndex === "number"
  );
}

function isExtractionResult(obj: unknown): obj is ExtractionResult {
  if (!obj || typeof obj !== "object") return false;
  const r = obj as Record<string, unknown>;
  return (
    typeof r.content === "string" &&
    typeof r.mimeType === "string" &&
    r.metadata &&
    typeof r.metadata === "object" &&
    Array.isArray(r.tables)
  );
}

const result = { content: "text", mimeType: "application/pdf", metadata: {}, tables: [] };

if (isExtractionResult(result)) {
  console.log("Valid extraction result");
  console.log("Has tables:", result.tables.length > 0);
}
