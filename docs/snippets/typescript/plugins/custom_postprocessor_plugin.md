```typescript
import {
  registerPostProcessor,
  unregisterPostProcessor,
  type PostProcessorProtocol,
  type ExtractionResult,
} from "@goldziher/kreuzberg";

/**
 * Metadata enrichment post-processor
 * Adds custom metadata to extraction results
 * @example
 * const processor = new MetadataEnrichmentProcessor();
 * registerPostProcessor(processor);
 */
class MetadataEnrichmentProcessor implements PostProcessorProtocol {
  private processedCount: number = 0;

  name(): string {
    return "metadata-enrichment-processor";
  }

  processingStage(): "early" | "middle" | "late" {
    return "middle";
  }

  /**
   * Enrich result with additional metadata
   */
  process(result: ExtractionResult): ExtractionResult {
    this.processedCount++;

    return {
      ...result,
      metadata: {
        ...result.metadata,
        processedAt: new Date().toISOString(),
        processingIndex: this.processedCount,
        contentStats: {
          characterCount: result.content.length,
          wordCount: result.content.split(/\s+/).length,
          lineCount: result.content.split("\n").length,
        },
        extractionQuality: this.calculateQuality(result.content),
      },
    };
  }

  /**
   * Calculate quality score based on content
   */
  private calculateQuality(content: string): number {
    let score = 0;

    // Check if content has reasonable length
    if (content.length > 100) score += 20;

    // Check for diverse character types
    const hasUpperCase = /[A-Z]/.test(content);
    const hasLowerCase = /[a-z]/.test(content);
    const hasNumbers = /\d/.test(content);
    const hasPunctuation = /[.!?;,]/.test(content);

    if (hasUpperCase) score += 15;
    if (hasLowerCase) score += 15;
    if (hasNumbers) score += 10;
    if (hasPunctuation) score += 10;

    // Check for line breaks (indicates structure)
    if (content.includes("\n")) score += 15;

    return Math.min(score, 100);
  }

  /**
   * Get processor statistics
   */
  getStats(): { processedCount: number } {
    return { processedCount: this.processedCount };
  }
}

/**
 * Format post-processor
 * Standardizes text formatting
 * @example
 * const processor = new FormatPostProcessor();
 * registerPostProcessor(processor);
 */
class FormatPostProcessor implements PostProcessorProtocol {
  name(): string {
    return "format-postprocessor";
  }

  processingStage(): "early" | "middle" | "late" {
    return "late";
  }

  /**
   * Format content for consistency
   */
  process(result: ExtractionResult): ExtractionResult {
    return {
      ...result,
      content: this.formatContent(result.content),
    };
  }

  /**
   * Apply formatting rules
   */
  private formatContent(content: string): string {
    // Normalize line endings
    let formatted = content.replace(/\r\n/g, "\n");

    // Remove trailing whitespace from lines
    formatted = formatted
      .split("\n")
      .map((line) => line.trimEnd())
      .join("\n");

    // Remove excessive blank lines
    formatted = formatted.replace(/\n\n\n+/g, "\n\n");

    // Ensure single space after periods
    formatted = formatted.replace(/\.  +/g, ". ");

    return formatted.trim();
  }
}

// Register both processors
const metadataProcessor = new MetadataEnrichmentProcessor();
const formatProcessor = new FormatPostProcessor();

registerPostProcessor(metadataProcessor);
registerPostProcessor(formatProcessor);

// Later usage with statistics
// console.log(metadataProcessor.getStats()); // { processedCount: X }

// Cleanup
// unregisterPostProcessor("metadata-enrichment-processor");
// unregisterPostProcessor("format-postprocessor");
```
