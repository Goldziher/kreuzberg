```typescript
import {
	registerValidator,
	unregisterValidator,
	extractFile,
	type ValidatorProtocol,
	type ExtractionResult,
	ValidationError,
} from 'kreuzberg';

/**
 * Custom validator for quality checking
 * Ensures extracted content meets minimum quality standards
 * @example
 * const validator = new QualityValidator();
 * registerValidator(validator);
 */
class QualityValidator implements ValidatorProtocol {
  name(): string {
    return "quality-validator";
  }

  priority(): number {
    return 10;
  }

  /**
   * Validate extraction result meets quality standards
   */
  validate(result: ExtractionResult): void {
    this.checkMinimumLength(result);
    this.checkEmptyContent(result);
    this.checkMetadata(result);
  }

  /**
   * Ensure minimum content length
   */
  private checkMinimumLength(result: ExtractionResult): void {
    const minLength = 50;
    if (result.content.length < minLength) {
      throw new ValidationError(
        `Content too short: ${result.content.length} bytes (minimum ${minLength})`
      );
    }
  }

  /**
   * Ensure content is not empty
   */
  private checkEmptyContent(result: ExtractionResult): void {
    const trimmed = result.content.trim();
    if (trimmed.length === 0) {
      throw new ValidationError("Extracted content is empty");
    }
  }

  /**
   * Validate metadata is present
   */
  private checkMetadata(result: ExtractionResult): void {
    if (!result.metadata || Object.keys(result.metadata).length === 0) {
      throw new ValidationError("Missing extraction metadata");
    }
  }
}

// Register the validator
const validator = new QualityValidator();
registerValidator(validator);

// Usage with error handling (must use async extraction for custom validators)
try {
  const result = await extractFile("document.pdf");
  console.log(
    `Validated content length: ${result.content.length} characters`
  );
} catch (error) {
  if (error instanceof ValidationError) {
    console.error(`Validation failed: ${error.message}`);
  } else {
    throw error;
  }
}

// Later, unregister if needed
// unregisterValidator("quality-validator");
```
