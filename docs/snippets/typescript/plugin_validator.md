```ts
import {
	extractFileSync,
	registerValidator,
	unregisterValidator,
	ValidationError,
	type ExtractionResult,
} from 'kreuzberg';

class MinLengthValidator {
  name = "min_length_validator";
  priority = 10;

  validate(result: ExtractionResult): void {
    if (result.content.length < 50) {
      throw new ValidationError(`Content too short: ${result.content.length}`);
    }
  }
}

registerValidator(new MinLengthValidator());

const result = extractFileSync("document.pdf");
console.log(`Validated content length: ${result.content.length}`);

unregisterValidator("min_length_validator");
```
