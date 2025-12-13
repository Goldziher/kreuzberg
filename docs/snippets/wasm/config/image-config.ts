import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionConfig } from "@kreuzberg/wasm";

async function extractImagesWithConfig() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    images: {
      extractImages: true,
      targetDpi: 300,
      maxDimension: 2048,
      preserveAspectRatio: true,
    },
  };

  const result = await extractBytes(bytes, "application/pdf", config);

  if (result.images) {
    console.log(`Extracted ${result.images.length} images`);

    result.images.forEach((image) => {
      console.log(
        `Image: ${image.width}x${image.height}, Format: ${image.format}, DPI: ${image.description}`
      );
    });
  }
}

extractImagesWithConfig().catch(console.error);
