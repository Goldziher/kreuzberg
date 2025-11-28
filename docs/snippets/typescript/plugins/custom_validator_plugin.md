```typescript
import {
	registerValidator,
	unregisterValidator,
	type ValidatorProtocol,
	type ExtractionResult,
	ValidationError,
} from 'kreuzberg';

/**
 * Stateful validator with call counting
 * Demonstrates maintaining state across multiple validations
 * @example
 * const validator = new StatefulValidator();
 * registerValidator(validator);
 */
class StatefulValidator implements ValidatorProtocol {
  private callCount: number = 0;
  private failureCount: number = 0;
  private cache: Map<string, boolean> = new Map();

  name(): string {
    return "stateful-validator";
  }

  priority(): number {
    return 50;
  }

  /**
   * Validate with state tracking
   */
  validate(result: ExtractionResult): void {
    this.callCount++;

    // Check cache first
    const cacheKey = this.getCacheKey(result);
    if (this.cache.has(cacheKey)) {
      return;
    }

    try {
      this.performValidation(result);
      this.cache.set(cacheKey, true);
    } catch (error) {
      this.failureCount++;
      throw error;
    }
  }

  /**
   * Perform actual validation logic
   */
  private performValidation(result: ExtractionResult): void {
    // Check content length
    if (result.content.length < 10) {
      throw new ValidationError(
        "Content too short (minimum 10 characters)"
      );
    }

    // Check for mime type
    if (!result.mimeType) {
      throw new ValidationError("Missing MIME type");
    }

    // Check metadata
    if (!result.metadata || Object.keys(result.metadata).length === 0) {
      throw new ValidationError("Missing metadata");
    }
  }

  /**
   * Create cache key from result
   */
  private getCacheKey(result: ExtractionResult): string {
    return `${result.mimeType}-${result.content.length}`;
  }

  /**
   * Get validation statistics
   */
  getStats(): {
    totalCalls: number;
    failures: number;
    successRate: number;
    cacheSize: number;
  } {
    return {
      totalCalls: this.callCount,
      failures: this.failureCount,
      successRate: this.callCount > 0
        ? (1 - this.failureCount / this.callCount) * 100
        : 100,
      cacheSize: this.cache.size,
    };
  }
}

/**
 * Content type validator
 * Ensures extracted content is appropriate for its MIME type
 * @example
 * const validator = new ContentTypeValidator();
 * registerValidator(validator);
 */
class ContentTypeValidator implements ValidatorProtocol {
  name(): string {
    return "content-type-validator";
  }

  priority(): number {
    return 20;
  }

  /**
   * Validate content matches MIME type
   */
  validate(result: ExtractionResult): void {
    this.validateContentType(result.mimeType, result.content);
  }

  /**
   * Check if content is appropriate for MIME type
   */
  private validateContentType(mimeType: string, content: string): void {
    const trimmed = content.trim();

    if (mimeType.includes("json")) {
      try {
        JSON.parse(trimmed);
      } catch {
        throw new ValidationError("Content is not valid JSON");
      }
    }

    if (mimeType.includes("xml")) {
      if (!trimmed.startsWith("<")) {
        throw new ValidationError("Content does not appear to be XML");
      }
    }

    if (mimeType.includes("html")) {
      if (!trimmed.includes("<") || !trimmed.includes(">")) {
        throw new ValidationError("Content does not appear to be HTML");
      }
    }
  }
}

// Register validators
const statefulValidator = new StatefulValidator();
const contentTypeValidator = new ContentTypeValidator();

registerValidator(statefulValidator);
registerValidator(contentTypeValidator);

// Usage with statistics
// try {
//   const result = await extractFile("document.pdf");
// } catch (error) {
//   console.error("Validation failed:", error);
// }
// console.log(statefulValidator.getStats());

// Cleanup
// unregisterValidator("stateful-validator");
// unregisterValidator("content-type-validator");
```
