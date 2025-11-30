```typescript
import {
	registerPostProcessor,
	registerValidator,
	type ExtractionResult,
} from '@kreuzberg/node';

class LoggingPostProcessor {
  name = "logging-processor";
  priority = 5;

  process(result: ExtractionResult): ExtractionResult {
    console.info(`[PostProcessor] Processing ${result.mimeType}`);
    console.info(`[PostProcessor] Content length: ${result.content.length}`);

    if (result.content.length === 0) {
      console.warn("[PostProcessor] Warning: Empty content extracted");
    }

    return result;
  }
}

class LoggingValidator {
  name = "logging-validator";
  priority = 100;

  validate(result: ExtractionResult): void {
    console.info(
      `[Validator] Validating extraction result (${result.content.length} bytes)`
    );

    if (result.content.length < 50) {
      console.error("[Validator] Error: Content below minimum threshold");
      throw new Error("Content too short");
    }
  }
}

// Register plugins with logging
registerPostProcessor(new LoggingPostProcessor());
registerValidator(new LoggingValidator());

console.log("[Main] Plugins registered with logging enabled");
```
