```typescript
import {
	registerPostProcessor,
	unregisterPostProcessor,
	type PostProcessorProtocol,
	type ExtractionResult,
} from 'kreuzberg';

/**
 * Custom post-processor for cleaning extraction results
 * Removes common artifacts and normalizes whitespace
 * @example
 * const processor = new CleaningPostProcessor();
 * registerPostProcessor(processor);
 */
class CleaningPostProcessor implements PostProcessorProtocol {
  name(): string {
    return "cleaning-postprocessor";
  }

  processingStage(): "early" | "middle" | "late" {
    return "middle";
  }

  /**
   * Process extraction result for cleanup
   */
  process(result: ExtractionResult): ExtractionResult {
    return {
      ...result,
      content: this.cleanContent(result.content),
    };
  }

  /**
   * Remove artifacts and normalize whitespace
   */
  private cleanContent(content: string): string {
    // Remove multiple spaces
    let cleaned = content.replace(/\s+/g, " ");

    // Remove common OCR artifacts
    cleaned = cleaned.replace(/\|/g, "l");
    cleaned = cleaned.replace(/0O/g, "00");

    // Remove leading/trailing whitespace from lines
    cleaned = cleaned
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0)
      .join("\n");

    return cleaned.trim();
  }
}

// Register the post-processor
const processor = new CleaningPostProcessor();
registerPostProcessor(processor);

// Later, unregister if needed
// unregisterPostProcessor("cleaning-postprocessor");
```
