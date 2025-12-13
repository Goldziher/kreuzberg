import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionConfig } from "@kreuzberg/wasm";

async function extractWithImages() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    images: {
      extractImages: true,
      targetDpi: 150,
    },
  };

  const result = await extractBytes(bytes, "application/pdf", config);

  if (result.images && result.images.length > 0) {
    console.log(`Found ${result.images.length} images`);

    result.images.forEach((image, index) => {
      console.log(`Image ${index}: ${image.format} (${image.width}x${image.height})`);

      const blob = new Blob([image.data], { type: `image/${image.format}` });
      const url = URL.createObjectURL(blob);

      const img = document.createElement("img");
      img.src = url;
      document.body.appendChild(img);
    });
  }
}

extractWithImages().catch(console.error);
