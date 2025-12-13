```typescript title="TypeScript"
import {
	registerOcrBackend,
	type OcrBackendProtocol,
} from '@kreuzberg/node';

/**
 * Mock OCR backend for testing
 * Simulates OCR results without calling external service
 * @example
 * const backend = new MockOcrBackend();
 * await backend.initialize();
 * registerOcrBackend(backend);
 */
class MockOcrBackend implements OcrBackendProtocol {
  private callCount: number = 0;

  name(): string {
    return "mock-ocr-backend";
  }

  supportedLanguages(): string[] {
    return ["en", "de", "fr", "es"];
  }

  async initialize(): Promise<void> {
    console.log("Mock OCR backend initialized");
  }

  async shutdown(): Promise<void> {
    console.log("Mock OCR backend shutdown");
  }

  /**
   * Return mock OCR results based on image size
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
    this.callCount++;

    const buffer = typeof imageData === "string"
      ? Buffer.from(imageData, "base64")
      : Buffer.from(imageData);

    // Simulate OCR processing time
    await new Promise((resolve) => setTimeout(resolve, 100));

    const mockText =
      `This is mock OCR result for ${language} detected in ${buffer.length} bytes of image data.`;

    return {
      content: mockText,
      mime_type: "text/plain",
      metadata: { confidence: 0.95, language },
      tables: []
    };
  }

  /**
   * Get backend statistics
   */
  getStats(): { callCount: number } {
    return { callCount: this.callCount };
  }
}

// Register mock OCR backend for testing
const mockBackend = new MockOcrBackend();
await mockBackend.initialize();
registerOcrBackend(mockBackend);

// Usage in tests
// const result = await extractFile("image.png");
// console.log(mockBackend.getStats()); // { callCount: 1 }
```
