# TypeScript Validator Development

Validators enforce quality requirements and validation rules on extraction results. Unlike post-processors, validators **fail fast** - if validation fails, the extraction stops immediately. This guide covers implementing custom validators in TypeScript.

## Overview

TypeScript Validators allow you to:
- Enforce content quality requirements
- Validate metadata completeness
- Check extraction success criteria
- Reject low-quality results
- Leverage TypeScript's type safety

### Validators vs Post-Processors

| Feature | Validator | PostProcessor |
|---------|-----------|---------------|
| **Purpose** | Quality checks | Transform results |
| **Error behavior** | Fail-fast (stops extraction) | Continue processing |
| **Execution order** | After all post-processors | Before validators |
| **Return value** | `void` (throws on failure) | Modified `ExtractionResult` |
| **Use cases** | Quality gates, compliance | Enrichment, transformation |

## Basic Validator

### Minimal Implementation

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

class SimpleValidator {
    validate(result: ExtractionResult): void {
        if (result.content.length < 10) {
            throw new Error('ValidationError: Content too short');
        }
    }

    name(): string {
        return 'simple_validator';
    }
}

// Register the validator
registerValidator(new SimpleValidator());
```

### Key Requirements

1. **`validate()` method**: Takes `ExtractionResult`, throws `Error` on failure, returns `void` or `Promise<void>` on success
2. **`name()` method**: Returns unique string identifier
3. **Fail-fast**: Throw errors to reject extraction
4. **Thread-safe**: Avoid shared mutable state
5. **Registration**: Call `registerValidator()` to activate

### Error Format

Include "ValidationError" in error messages for proper error classification:

```typescript
// ✅ Correct - will be classified as ValidationError
throw new Error('ValidationError: Content too short');

// ⚠️  Less specific - may be classified as PluginError
throw new Error('Content too short');
```

## Complete Example: Quality Validator

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

interface QualityValidatorOptions {
    minLength?: number;
    maxLength?: number;
    requireMetadata?: string[];
    minWordCount?: number;
}

class QualityValidator {
    private options: Required<QualityValidatorOptions>;

    constructor(options: QualityValidatorOptions = {}) {
        this.options = {
            minLength: 10,
            maxLength: 10_000_000,
            requireMetadata: [],
            minWordCount: 5,
            ...options,
        };
    }

    validate(result: ExtractionResult): void {
        this.validateContentLength(result.content);
        this.validateWordCount(result.content);
        this.validateRequiredMetadata(result.metadata);
    }

    private validateContentLength(content: string): void {
        const length = content.length;

        if (length < this.options.minLength) {
            throw new Error(
                `ValidationError: Content too short (${length} < ${this.options.minLength} characters)`
            );
        }

        if (length > this.options.maxLength) {
            throw new Error(
                `ValidationError: Content too long (${length} > ${this.options.maxLength} characters)`
            );
        }
    }

    private validateWordCount(content: string): void {
        const words = content.split(/\s+/).filter(w => w.trim().length > 0);
        const wordCount = words.length;

        if (wordCount < this.options.minWordCount) {
            throw new Error(
                `ValidationError: Too few words (${wordCount} < ${this.options.minWordCount})`
            );
        }
    }

    private validateRequiredMetadata(metadata: Record<string, any>): void {
        for (const key of this.options.requireMetadata) {
            if (!(key in metadata) || metadata[key] === null || metadata[key] === undefined) {
                throw new Error(
                    `ValidationError: Missing required metadata field: ${key}`
                );
            }
        }
    }

    name(): string {
        return 'quality_validator';
    }

    priority(): number {
        return 100; // Run early (higher priority = runs first)
    }
}

// Register with configuration
registerValidator(new QualityValidator({
    minLength: 50,
    minWordCount: 10,
    requireMetadata: ['pdf.pageCount', 'pdf.author'],
}));
```

### Usage

```typescript
import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

try {
    const result = extractFileSync('document.pdf', null, new ExtractionConfig());
    console.log('Validation passed:', result.content.length);
} catch (error) {
    if (error.message.includes('ValidationError')) {
        console.error('Validation failed:', error.message);
    } else {
        console.error('Extraction failed:', error);
    }
}
```

## Async Validator

For validators that need to perform I/O operations (database lookups, API calls):

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';
import axios from 'axios';

interface ContentPolicyCheckerOptions {
    apiUrl: string;
    apiKey: string;
    timeout?: number;
}

class ContentPolicyChecker {
    private config: Required<ContentPolicyCheckerOptions>;
    private httpClient: ReturnType<typeof axios.create>;

    constructor(options: ContentPolicyCheckerOptions) {
        this.config = {
            timeout: 5000,
            ...options,
        };

        this.httpClient = axios.create({
            baseURL: this.config.apiUrl,
            timeout: this.config.timeout,
            headers: {
                'Authorization': `Bearer ${this.config.apiKey}`,
                'Content-Type': 'application/json',
            },
        });
    }

    async validate(result: ExtractionResult): Promise<void> {
        try {
            const response = await this.httpClient.post('/check-policy', {
                content: result.content.substring(0, 5000), // Send preview
                metadata: result.metadata,
            });

            if (!response.data.compliant) {
                throw new Error(
                    `ValidationError: Content violates policy: ${response.data.reason}`
                );
            }

        } catch (error) {
            if (error instanceof Error && error.message.includes('ValidationError')) {
                throw error; // Re-throw validation errors
            }

            // Handle network errors
            throw new Error(
                `ValidationError: Policy check failed: ${error instanceof Error ? error.message : 'Unknown error'}`
            );
        }
    }

    name(): string {
        return 'content_policy_checker';
    }

    priority(): number {
        return 50; // Default priority
    }
}

// Register
registerValidator(new ContentPolicyChecker({
    apiUrl: 'https://api.example.com',
    apiKey: process.env.API_KEY || '',
}));
```

### Using with Async Extraction

```typescript
import { extractFile, ExtractionConfig } from '@goldziher/kreuzberg';

async function main() {
    try {
        const result = await extractFile('document.pdf', null, new ExtractionConfig());
        console.log('Validation passed:', result.content);
    } catch (error) {
        console.error('Validation or extraction failed:', error);
    }
}

main();
```

## Validation Examples

### PDF-Specific Validation

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

interface PDFValidatorOptions {
    requirePages?: boolean;
    minPages?: number;
    maxPages?: number;
    requireAuthor?: boolean;
}

class PDFValidator {
    private options: PDFValidatorOptions;

    constructor(options: PDFValidatorOptions = {}) {
        this.options = {
            requirePages: true,
            minPages: 1,
            maxPages: 1000,
            requireAuthor: false,
            ...options,
        };
    }

    validate(result: ExtractionResult): void {
        // Only validate PDFs
        if (result.mimeType !== 'application/pdf') {
            return;
        }

        const pdfMetadata = result.metadata.pdf;

        if (!pdfMetadata) {
            throw new Error('ValidationError: PDF metadata missing');
        }

        // Validate page count
        if (this.options.requirePages) {
            const pageCount = pdfMetadata.pageCount;

            if (!pageCount || pageCount < (this.options.minPages || 0)) {
                throw new Error(
                    `ValidationError: PDF has too few pages (${pageCount})`
                );
            }

            if (this.options.maxPages && pageCount > this.options.maxPages) {
                throw new Error(
                    `ValidationError: PDF has too many pages (${pageCount} > ${this.options.maxPages})`
                );
            }
        }

        // Validate author if required
        if (this.options.requireAuthor && !pdfMetadata.author) {
            throw new Error('ValidationError: PDF author metadata missing');
        }
    }

    name(): string {
        return 'pdf_validator';
    }
}

registerValidator(new PDFValidator({
    minPages: 1,
    maxPages: 500,
    requireAuthor: true,
}));
```

### Language Detection Validation

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

class LanguageValidator {
    constructor(
        private allowedLanguages: string[],
        private requireDetection: boolean = true
    ) {}

    validate(result: ExtractionResult): void {
        if (!result.detectedLanguages || result.detectedLanguages.length === 0) {
            if (this.requireDetection) {
                throw new Error(
                    'ValidationError: No languages detected in content'
                );
            }
            return;
        }

        const primaryLanguage = result.detectedLanguages[0];

        if (!this.allowedLanguages.includes(primaryLanguage)) {
            throw new Error(
                `ValidationError: Detected language '${primaryLanguage}' not in allowed list: ${this.allowedLanguages.join(', ')}`
            );
        }
    }

    name(): string {
        return 'language_validator';
    }

    priority(): number {
        return 75; // Run before content-specific validators
    }
}

registerValidator(new LanguageValidator(['eng', 'deu', 'fra']));
```

### OCR Quality Validation

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

class OCRQualityValidator {
    constructor(
        private minConfidence: number = 0.7,
        private minTextDensity: number = 0.1
    ) {}

    validate(result: ExtractionResult): void {
        const ocrMetadata = result.metadata.ocr;

        // Skip if OCR wasn't used
        if (!ocrMetadata) {
            return;
        }

        // Check text density (characters per image pixel)
        const imageMetadata = result.metadata.image;
        if (imageMetadata) {
            const pixels = (imageMetadata.width || 1) * (imageMetadata.height || 1);
            const textDensity = result.content.length / pixels;

            if (textDensity < this.minTextDensity) {
                throw new Error(
                    `ValidationError: OCR text density too low (${textDensity.toFixed(4)} < ${this.minTextDensity})`
                );
            }
        }

        // Validate minimum content was extracted
        if (result.content.trim().length < 10) {
            throw new Error(
                'ValidationError: OCR extracted insufficient content'
            );
        }
    }

    name(): string {
        return 'ocr_quality_validator';
    }
}

registerValidator(new OCRQualityValidator(0.8, 0.05));
```

### Table Extraction Validation

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

class TableValidator {
    constructor(
        private requireTables: boolean = false,
        private minTables: number = 0,
        private minRows: number = 2,
        private minCols: number = 2
    ) {}

    validate(result: ExtractionResult): void {
        const tables = result.tables || [];

        if (this.requireTables && tables.length === 0) {
            throw new Error(
                'ValidationError: Document must contain at least one table'
            );
        }

        if (tables.length < this.minTables) {
            throw new Error(
                `ValidationError: Too few tables (${tables.length} < ${this.minTables})`
            );
        }

        // Validate table structure
        for (let i = 0; i < tables.length; i++) {
            const table = tables[i];
            const rows = table.cells.length;
            const cols = rows > 0 ? table.cells[0].length : 0;

            if (rows < this.minRows) {
                throw new Error(
                    `ValidationError: Table ${i} has too few rows (${rows} < ${this.minRows})`
                );
            }

            if (cols < this.minCols) {
                throw new Error(
                    `ValidationError: Table ${i} has too few columns (${cols} < ${this.minCols})`
                );
            }
        }
    }

    name(): string {
        return 'table_validator';
    }
}

registerValidator(new TableValidator(true, 1, 3, 2));
```

## Advanced Patterns

### Priority-Based Execution

Validators run in priority order (higher priority first). Use this for efficient validation:

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

// Fast validation - runs first
class FastValidator {
    validate(result: ExtractionResult): void {
        if (result.content.length === 0) {
            throw new Error('ValidationError: Empty content');
        }
    }

    name(): string {
        return 'fast_validator';
    }

    priority(): number {
        return 100; // High priority - runs first
    }
}

// Expensive validation - runs last
class ExpensiveValidator {
    async validate(result: ExtractionResult): Promise<void> {
        // Expensive API call or computation
        await this.performExpensiveCheck(result);
    }

    private async performExpensiveCheck(result: ExtractionResult): Promise<void> {
        // Implementation
    }

    name(): string {
        return 'expensive_validator';
    }

    priority(): number {
        return 10; // Low priority - runs last
    }
}

registerValidator(new FastValidator());
registerValidator(new ExpensiveValidator());
```

### Conditional Validation

Skip validation based on result properties:

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

class ConditionalValidator {
    validate(result: ExtractionResult): void {
        // Only validate PDFs
        if (result.mimeType !== 'application/pdf') {
            return;
        }

        // Only validate if OCR was used
        if (!result.metadata.ocr) {
            return;
        }

        // Perform validation
        if (result.content.length < 100) {
            throw new Error(
                'ValidationError: OCR-extracted PDF content too short'
            );
        }
    }

    name(): string {
        return 'conditional_validator';
    }
}

registerValidator(new ConditionalValidator());
```

### Multi-Rule Validation

Collect multiple validation failures:

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

class MultiRuleValidator {
    private rules: Array<{
        name: string;
        check: (result: ExtractionResult) => boolean;
        message: string;
    }>;

    constructor() {
        this.rules = [
            {
                name: 'min_length',
                check: (r) => r.content.length >= 10,
                message: 'Content must be at least 10 characters',
            },
            {
                name: 'has_text',
                check: (r) => r.content.trim().length > 0,
                message: 'Content must not be empty or whitespace only',
            },
            {
                name: 'has_mime_type',
                check: (r) => !!r.mimeType,
                message: 'MIME type must be present',
            },
        ];
    }

    validate(result: ExtractionResult): void {
        const failures: string[] = [];

        for (const rule of this.rules) {
            if (!rule.check(result)) {
                failures.push(`${rule.name}: ${rule.message}`);
            }
        }

        if (failures.length > 0) {
            throw new Error(
                `ValidationError: Multiple validation failures:\n${failures.join('\n')}`
            );
        }
    }

    name(): string {
        return 'multi_rule_validator';
    }
}

registerValidator(new MultiRuleValidator());
```

### Schema Validation

Validate metadata structure:

```typescript
import { registerValidator, ExtractionResult } from '@goldziher/kreuzberg';

interface ExpectedMetadataSchema {
    pdf?: {
        pageCount: number;
        author?: string;
    };
    date?: string;
}

class MetadataSchemaValidator {
    validate(result: ExtractionResult): void {
        // Check PDF metadata structure
        if (result.mimeType === 'application/pdf') {
            const pdf = result.metadata.pdf as any;

            if (!pdf) {
                throw new Error('ValidationError: PDF metadata missing');
            }

            if (typeof pdf.pageCount !== 'number') {
                throw new Error(
                    'ValidationError: pdf.pageCount must be a number'
                );
            }

            if (pdf.author !== undefined && typeof pdf.author !== 'string') {
                throw new Error(
                    'ValidationError: pdf.author must be a string'
                );
            }
        }
    }

    name(): string {
        return 'metadata_schema_validator';
    }
}

registerValidator(new MetadataSchemaValidator());
```

## Testing Validators

### Unit Testing with Jest

```typescript
import { ExtractionResult } from '@goldziher/kreuzberg';
import { QualityValidator } from './quality-validator';

describe('QualityValidator', () => {
    test('should pass valid content', () => {
        const result: ExtractionResult = {
            content: 'This is valid content with enough words and length',
            mimeType: 'text/plain',
            metadata: {},
            tables: [],
            detectedLanguages: null,
            chunks: null,
        };

        const validator = new QualityValidator({ minLength: 10, minWordCount: 5 });

        expect(() => validator.validate(result)).not.toThrow();
    });

    test('should reject short content', () => {
        const result: ExtractionResult = {
            content: 'Short',
            mimeType: 'text/plain',
            metadata: {},
            tables: [],
            detectedLanguages: null,
            chunks: null,
        };

        const validator = new QualityValidator({ minLength: 50 });

        expect(() => validator.validate(result)).toThrow('ValidationError');
        expect(() => validator.validate(result)).toThrow('too short');
    });

    test('should validate required metadata', () => {
        const result: ExtractionResult = {
            content: 'Valid content',
            mimeType: 'application/pdf',
            metadata: { pdf: { pageCount: 5 } },
            tables: [],
            detectedLanguages: null,
            chunks: null,
        };

        const validator = new QualityValidator({
            requireMetadata: ['pdf.pageCount'],
        });

        expect(() => validator.validate(result)).not.toThrow();
    });

    test('should reject missing metadata', () => {
        const result: ExtractionResult = {
            content: 'Valid content',
            mimeType: 'application/pdf',
            metadata: {},
            tables: [],
            detectedLanguages: null,
            chunks: null,
        };

        const validator = new QualityValidator({
            requireMetadata: ['pdf.author'],
        });

        expect(() => validator.validate(result)).toThrow('Missing required metadata');
    });
});

describe('AsyncValidator', () => {
    test('should handle async validation', async () => {
        const result: ExtractionResult = {
            content: 'test',
            mimeType: 'text/plain',
            metadata: {},
            tables: [],
            detectedLanguages: null,
            chunks: null,
        };

        const validator = new ContentPolicyChecker({
            apiUrl: 'http://localhost:3000',
            apiKey: 'test',
        });

        await expect(validator.validate(result)).resolves.not.toThrow();
    });
});
```

### Integration Testing

```typescript
import {
    extractFileSync,
    ExtractionConfig,
    registerValidator,
    clearValidators,
} from '@goldziher/kreuzberg';
import { QualityValidator } from './quality-validator';

describe('Validator Integration', () => {
    beforeEach(() => {
        // Clear validators before each test
        clearValidators();
    });

    test('should fail extraction on validation error', () => {
        registerValidator(new QualityValidator({ minLength: 1000 }));

        expect(() => {
            extractFileSync('small_document.pdf', null, new ExtractionConfig());
        }).toThrow('ValidationError');
    });

    test('should pass extraction when validation succeeds', () => {
        registerValidator(new QualityValidator({ minLength: 10 }));

        const result = extractFileSync(
            'large_document.pdf',
            null,
            new ExtractionConfig()
        );

        expect(result.content.length).toBeGreaterThan(10);
    });

    test('should execute validators in priority order', () => {
        const executionOrder: string[] = [];

        class FirstValidator {
            validate(): void {
                executionOrder.push('first');
            }
            name(): string {
                return 'first';
            }
            priority(): number {
                return 100;
            }
        }

        class SecondValidator {
            validate(): void {
                executionOrder.push('second');
            }
            name(): string {
                return 'second';
            }
            priority(): number {
                return 50;
            }
        }

        registerValidator(new SecondValidator());
        registerValidator(new FirstValidator());

        extractFileSync('document.pdf', null, new ExtractionConfig());

        expect(executionOrder).toEqual(['first', 'second']);
    });
});
```

## Best Practices

### Clear Error Messages

Provide actionable error messages:

```typescript
// ✅ Good - specific and actionable
throw new Error(
    `ValidationError: Content too short (${length} < ${minLength} characters). ` +
    `Consider reducing minLength or improving extraction.`
);

// ❌ Bad - vague
throw new Error('ValidationError: Invalid content');
```

### Type Safety

Leverage TypeScript for configuration:

```typescript
interface StrictValidatorConfig {
    readonly minLength: number;
    readonly maxLength: number;
    readonly allowedMimeTypes: readonly string[];
}

class TypeSafeValidator {
    private readonly config: StrictValidatorConfig;

    constructor(config: StrictValidatorConfig) {
        this.config = Object.freeze({...config});
    }

    validate(result: ExtractionResult): void {
        if (!this.config.allowedMimeTypes.includes(result.mimeType)) {
            throw new Error(
                `ValidationError: MIME type '${result.mimeType}' not allowed`
            );
        }
    }

    name(): string {
        return 'type_safe_validator';
    }
}
```

### Efficient Validation

Run cheap checks first:

```typescript
class EfficientValidator {
    validate(result: ExtractionResult): void {
        // Fast checks first
        if (!result.content) {
            throw new Error('ValidationError: No content');
        }

        if (result.content.length < 10) {
            throw new Error('ValidationError: Content too short');
        }

        // Expensive checks last
        this.performExpensiveValidation(result);
    }

    private performExpensiveValidation(result: ExtractionResult): void {
        // Regex, API calls, etc.
    }

    name(): string {
        return 'efficient_validator';
    }

    priority(): number {
        return 100; // Run early to fail fast
    }
}
```

### Documentation

Use JSDoc for validator documentation:

```typescript
/**
 * Validates extraction quality based on content length and metadata.
 *
 * @example
 * ```typescript
 * import { registerValidator } from '@goldziher/kreuzberg';
 * import { QualityValidator } from './quality-validator';
 *
 * registerValidator(new QualityValidator({
 *     minLength: 100,
 *     minWordCount: 20,
 *     requireMetadata: ['pdf.pageCount'],
 * }));
 * ```
 */
class QualityValidator {
    /**
     * Creates a new QualityValidator instance.
     *
     * @param options - Validation configuration
     * @param options.minLength - Minimum content length in characters
     * @param options.minWordCount - Minimum word count
     * @param options.requireMetadata - Required metadata fields
     */
    constructor(options?: QualityValidatorOptions) {
        // Implementation
    }

    /**
     * Validates an extraction result.
     *
     * @param result - The extraction result to validate
     * @throws {Error} ValidationError if validation fails
     */
    validate(result: ExtractionResult): void {
        // Implementation
    }
}
```

## Common Pitfalls

### 1. Not Throwing Errors

```typescript
// ❌ Wrong - validation failure not signaled
validate(result: ExtractionResult): void {
    if (result.content.length < 10) {
        console.log('Validation failed'); // Silent failure!
    }
}

// ✅ Correct - throw error on failure
validate(result: ExtractionResult): void {
    if (result.content.length < 10) {
        throw new Error('ValidationError: Content too short');
    }
}
```

### 2. Swallowing Errors

```typescript
// ❌ Wrong - errors silently caught
validate(result: ExtractionResult): void {
    try {
        this.checkContent(result);
    } catch (error) {
        console.log('Error:', error); // Validation appears to pass!
    }
}

// ✅ Correct - let errors propagate
validate(result: ExtractionResult): void {
    this.checkContent(result); // Errors propagate naturally
}
```

### 3. Modifying Results

```typescript
// ❌ Wrong - validators should not modify results
validate(result: ExtractionResult): void {
    result.metadata.validated = true; // Don't modify!
    if (result.content.length < 10) {
        throw new Error('ValidationError');
    }
}

// ✅ Correct - validators only check
validate(result: ExtractionResult): void {
    if (result.content.length < 10) {
        throw new Error('ValidationError');
    }
    // No modifications
}
```

### 4. Missing Error Prefix

```typescript
// ⚠️  Less specific - may be classified as generic PluginError
throw new Error('Content too short');

// ✅ Better - explicitly marked as ValidationError
throw new Error('ValidationError: Content too short');
```

## Validator Lifecycle

### Registration and Unregistration

```typescript
import {
    registerValidator,
    unregisterValidator,
    clearValidators,
} from '@goldziher/kreuzberg';

// Register
const validator = new QualityValidator();
registerValidator(validator);

// Unregister specific validator
unregisterValidator('quality_validator');

// Clear all validators
clearValidators();
```

### Testing Cleanup

Always clean up validators in tests:

```typescript
describe('Validator Tests', () => {
    afterEach(() => {
        clearValidators();
    });

    test('some test', () => {
        registerValidator(new QualityValidator());
        // Test code
    });
});
```

## Next Steps

- [Plugin Development Overview](overview.md) - Compare plugin types
- [TypeScript PostProcessor Development](typescript-postprocessor.md) - Transform results
- [Python Validator Development](python-postprocessor.md#validation-pattern) - Python examples
- [API Reference](../api-reference/python/) - Complete API documentation
