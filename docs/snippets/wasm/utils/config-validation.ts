import type { ExtractionConfig } from "@kreuzberg/wasm";

function validateExtractionConfig(config: unknown): config is ExtractionConfig {
  if (!config || typeof config !== "object") {
    return false;
  }

  const cfg = config as Record<string, unknown>;

  if (cfg.ocr && typeof cfg.ocr === "object") {
    const ocr = cfg.ocr as Record<string, unknown>;
    if (ocr.language && typeof ocr.language !== "string") {
      return false;
    }
    if (ocr.backend && typeof ocr.backend !== "string") {
      return false;
    }
  }

  if (cfg.chunking && typeof cfg.chunking === "object") {
    const chunking = cfg.chunking as Record<string, unknown>;
    if (chunking.maxChars && typeof chunking.maxChars !== "number") {
      return false;
    }
    if (chunking.chunkOverlap && typeof chunking.chunkOverlap !== "number") {
      return false;
    }
  }

  if (cfg.images && typeof cfg.images === "object") {
    const images = cfg.images as Record<string, unknown>;
    if (images.extractImages && typeof images.extractImages !== "boolean") {
      return false;
    }
    if (images.targetDpi && typeof images.targetDpi !== "number") {
      return false;
    }
  }

  return true;
}

const testConfig = {
  ocr: { backend: "tesseract-wasm", language: "eng" },
  chunking: { maxChars: 1000 },
};

if (validateExtractionConfig(testConfig)) {
  console.log("Configuration is valid");
} else {
  console.log("Configuration is invalid");
}
