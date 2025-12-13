import { extractFromFile, initWasm } from "@kreuzberg/wasm";
import type { ExtractionResult } from "@kreuzberg/wasm";

async function cacheResultInSessionStorage(file: File): Promise<ExtractionResult> {
  await initWasm();

  const cacheKey = `extraction_${file.name}_${file.size}`;

  const cached = sessionStorage.getItem(cacheKey);
  if (cached) {
    console.log("Loading from session storage");
    return JSON.parse(cached);
  }

  console.log("Extracting and caching result");
  const result = await extractFromFile(file);

  try {
    sessionStorage.setItem(cacheKey, JSON.stringify(result));
  } catch (error) {
    if (error instanceof Error && error.name === "QuotaExceededError") {
      console.warn("Session storage full, skipping cache");
    }
  }

  return result;
}

async function clearExtractionCache() {
  const keys = Object.keys(sessionStorage);
  let cleared = 0;

  for (const key of keys) {
    if (key.startsWith("extraction_")) {
      sessionStorage.removeItem(key);
      cleared++;
    }
  }

  console.log(`Cleared ${cleared} cached results`);
}

clearExtractionCache();
