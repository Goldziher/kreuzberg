```typescript title="TypeScript"
import {
	registerOcrBackend,
	type OcrBackendProtocol,
} from '@kreuzberg/node';

/**
 * Custom OCR backend implementation
 * Allows integration with custom OCR services
 * @example
 * const backend = new CustomOcrBackend();
 * await backend.initialize();
 * registerOcrBackend(backend);
 */
class CustomOcrBackend implements OcrBackendProtocol {
  private apiUrl: string;

  constructor(apiUrl: string) {
    this.apiUrl = apiUrl;
  }

  name(): string {
    return "custom-ocr-backend";
  }

  supportedLanguages(): string[] {
    return ["en", "de", "fr", "es"];
  }

  async initialize(): Promise<void> {
    console.log(`Initializing custom OCR backend at ${this.apiUrl}`);
  }

  async shutdown(): Promise<void> {
    console.log("Shutting down custom OCR backend");
  }

  /**
   * Process image and extract text via OCR
   */
  async processImage(
    imageData: Uint8Array | string,
    language: string
  ): Promise<{
    content: string;
    mime_type: string;
    metadata: Record<string, unknown>;
    tables: unknown[];
  }> {
    const buffer = typeof imageData === "string"
      ? Buffer.from(imageData, "base64")
      : Buffer.from(imageData);

    const formData = new FormData();
    const blob = new Blob([buffer], { type: "image/png" });
    formData.append("image", blob);
    formData.append("language", language);

    const response = await fetch(`${this.apiUrl}/ocr`, {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      throw new Error(`OCR service failed: ${response.statusText}`);
    }

    const result = await response.json();
    return {
      content: result.text,
      mime_type: "text/plain",
      metadata: { confidence: result.confidence, language },
      tables: result.tables || []
    };
  }
}

// Register custom OCR backend
const backend = new CustomOcrBackend("http://localhost:8000");
await backend.initialize();
registerOcrBackend(backend);
```
