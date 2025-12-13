import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionResult } from "@kreuzberg/wasm";

class ExtractionCache {
  private cache = new Map<string, ExtractionResult>();
  private fileHashes = new Map<File, string>();

  async getHash(file: File): Promise<string> {
    if (this.fileHashes.has(file)) {
      return this.fileHashes.get(file)!;
    }

    const buffer = await file.arrayBuffer();
    const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hashStr = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

    this.fileHashes.set(file, hashStr);
    return hashStr;
  }

  async extract(file: File): Promise<ExtractionResult> {
    const hash = await this.getHash(file);

    if (this.cache.has(hash)) {
      console.log("Cache hit for", file.name);
      return this.cache.get(hash)!;
    }

    console.log("Cache miss for", file.name);
    const bytes = new Uint8Array(await file.arrayBuffer());
    const result = await extractBytes(bytes, file.type);

    this.cache.set(hash, result);
    return result;
  }

  clear() {
    this.cache.clear();
    this.fileHashes.clear();
  }

  getSize() {
    return this.cache.size;
  }
}

async function demonstrateCaching() {
  await initWasm();

  const cache = new ExtractionCache();

  const result = await cache.extract(new File([], "test.pdf"));
  console.log("Cache size:", cache.getSize());
}

demonstrateCaching().catch(console.error);
