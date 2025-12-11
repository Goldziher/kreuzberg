```typescript
import { describe, it, expect } from 'vitest';
import {
	registerPostProcessor,
	registerValidator,
	unregisterPostProcessor,
	unregisterValidator,
	type ExtractionResult,
} from '@kreuzberg/node';

describe("Plugin Testing", () => {
  describe("PostProcessor", () => {
    it("should add metadata to extraction result", () => {
      const processor = {
        name: "test-processor",
        priority: 10,
        process(result: ExtractionResult): ExtractionResult {
          return {
            ...result,
            metadata: {
              ...result.metadata,
              processed: true,
              processedAt: new Date().toISOString(),
            },
          };
        },
      };

      registerPostProcessor(processor);

      const mockResult: ExtractionResult = {
        content: "Test content",
        mimeType: "text/plain",
        metadata: { custom: "value" },
        tables: [],
        detectedLanguages: [],
        chunks: undefined,
        images: undefined,
      };

      const processed = processor.process(mockResult);

      expect(processed.metadata.processed).toBe(true);
      expect(processed.metadata.custom).toBe("value");

      unregisterPostProcessor("test-processor");
    });
  });

  describe("Validator", () => {
    it("should validate content length", () => {
      const validator = {
        name: "length-validator",
        priority: 10,
        validate(result: ExtractionResult): void {
          if (result.content.length < 10) {
            throw new Error("Content too short");
          }
        },
      };

      registerValidator(validator);

      const mockResult: ExtractionResult = {
        content: "Short",
        mimeType: "text/plain",
        metadata: {},
        tables: [],
        detectedLanguages: [],
        chunks: undefined,
        images: undefined,
      };

      expect(() => validator.validate(mockResult)).toThrow(
        "Content too short"
      );

      unregisterValidator("length-validator");
    });

    it("should pass validation for valid content", () => {
      const validator = {
        name: "length-validator-pass",
        priority: 10,
        validate(result: ExtractionResult): void {
          if (result.content.length < 10) {
            throw new Error("Content too short");
          }
        },
      };

      const mockResult: ExtractionResult = {
        content: "This is a valid long content",
        mimeType: "text/plain",
        metadata: {},
        tables: [],
        detectedLanguages: [],
        chunks: undefined,
        images: undefined,
      };

      expect(() => validator.validate(mockResult)).not.toThrow();
    });
  });
});
```
