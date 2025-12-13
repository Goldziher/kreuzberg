import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionConfig } from "@kreuzberg/wasm";

async function extractImageMetadata() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    images: {
      extractImages: true,
      targetDpi: 150,
    },
  };

  const result = await extractBytes(bytes, "application/pdf", config);

  if (result.images) {
    result.images.forEach((image, index) => {
      console.log(`Image ${index}:`, {
        format: image.format,
        width: image.width,
        height: image.height,
        pageNumber: image.pageNumber,
        colorspace: image.colorspace,
        bitsPerComponent: image.bitsPerComponent,
        isMask: image.isMask,
        dataSize: image.data.byteLength,
      });
    });
  }
}

extractImageMetadata().catch(console.error);
