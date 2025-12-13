import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionResult } from "@kreuzberg/wasm";

interface ValidationError {
  field: string;
  message: string;
}

class ContentValidator {
  private minContentLength: number = 10;
  private maxContentLength: number = 10000000;

  setMinLength(length: number) {
    this.minContentLength = length;
  }

  validate(result: ExtractionResult): ValidationError[] {
    const errors: ValidationError[] = [];

    if (result.content.length < this.minContentLength) {
      errors.push({
        field: "content",
        message: `Content length (${result.content.length}) is below minimum (${this.minContentLength})`,
      });
    }

    if (result.content.length > this.maxContentLength) {
      errors.push({
        field: "content",
        message: `Content length (${result.content.length}) exceeds maximum (${this.maxContentLength})`,
      });
    }

    if (!result.mimeType) {
      errors.push({
        field: "mimeType",
        message: "MIME type is required",
      });
    }

    return errors;
  }

  getName(): string {
    return "content-validator";
  }
}

async function demonstrateValidator() {
  await initWasm();

  const validator = new ContentValidator();
  validator.setMinLength(100);

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const result = await extractBytes(bytes, "application/pdf");
  const errors = validator.validate(result);

  if (errors.length > 0) {
    console.log("Validation errors:");
    errors.forEach((e) => console.log(`  ${e.field}: ${e.message}`));
  } else {
    console.log("Content validation passed");
  }
}

demonstrateValidator().catch(console.error);
