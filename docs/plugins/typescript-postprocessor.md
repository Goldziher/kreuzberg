# TypeScript PostProcessor Development

PostProcessors transform extraction results after initial extraction. This guide covers implementing custom post-processors in TypeScript.

## Overview

TypeScript PostProcessors allow you to:
- Modify extracted content
- Add or enrich metadata
- Filter or clean results
- Perform async operations (API calls, database queries)
- Leverage TypeScript's type safety

## Basic PostProcessor

### Minimal Implementation

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';

class SimplePostProcessor {
    process(result: ExtractionResult): ExtractionResult {
        // Modify the result
        result.metadata.processed = true;
        return result;
    }

    name(): string {
        return 'simple_processor';
    }
}

// Register the processor
registerPostProcessor(new SimplePostProcessor());
```

### Key Requirements

1. **`process()` method**: Takes `ExtractionResult`, returns modified `ExtractionResult` or `Promise<ExtractionResult>`
2. **`name()` method**: Returns unique string identifier
3. **Thread-safe**: Avoid shared mutable state
4. **Registration**: Call `registerPostProcessor()` to activate

## Complete Example: Metadata Enricher

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';

interface MetadataEnricherOptions {
    addTimestamp?: boolean;
    addStats?: boolean;
}

class MetadataEnricher {
    private options: MetadataEnricherOptions;

    constructor(options: MetadataEnricherOptions = {}) {
        this.options = {
            addTimestamp: true,
            addStats: true,
            ...options,
        };
    }

    process(result: ExtractionResult): ExtractionResult {
        if (this.options.addTimestamp) {
            result.metadata.processedAt = new Date().toISOString();
        }

        if (this.options.addStats) {
            Object.assign(result.metadata, this.calculateStats(result));
        }

        return result;
    }

    private calculateStats(result: ExtractionResult): Record<string, any> {
        const content = result.content;
        return {
            charCount: content.length,
            wordCount: content.split(/\s+/).filter(w => w.length > 0).length,
            lineCount: content.split('\n').length,
            tableCount: result.tables.length,
            hasContent: content.trim().length > 0,
        };
    }

    name(): string {
        return 'metadata_enricher';
    }
}

// Register with configuration
registerPostProcessor(new MetadataEnricher({
    addTimestamp: true,
    addStats: true,
}));
```

### Usage

```typescript
import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

// Extract with our post-processor
const result = extractFileSync('document.pdf', null, new ExtractionConfig());

// Check added metadata
console.log(result.metadata.processedAt);
console.log(result.metadata.wordCount);
```

## Async PostProcessor

For I/O-bound operations (API calls, database queries), use async:

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';
import axios from 'axios';

interface APIEnricherConfig {
    apiKey: string;
    apiUrl: string;
    timeout?: number;
}

class AsyncAPIEnricher {
    private config: APIEnricherConfig;
    private httpClient: ReturnType<typeof axios.create>;

    constructor(config: APIEnricherConfig) {
        this.config = {
            timeout: 5000,
            ...config,
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

    async process(result: ExtractionResult): Promise<ExtractionResult> {
        try {
            // Call external API
            const response = await this.httpClient.post('/enrich', {
                text: result.content.substring(0, 1000), // Send preview
                metadata: result.metadata,
            });

            // Add API response to metadata
            result.metadata.apiEnrichment = response.data;

        } catch (error) {
            // Handle errors gracefully
            console.error('API enrichment error:', error);
            result.metadata.apiEnrichmentError = error instanceof Error
                ? error.message
                : 'Unknown error';
        }

        return result;
    }

    name(): string {
        return 'async_api_enricher';
    }
}

// Register
registerPostProcessor(new AsyncAPIEnricher({
    apiKey: process.env.API_KEY || '',
    apiUrl: 'https://api.example.com',
}));
```

### Using with Async Extraction

```typescript
import { extractFile, ExtractionConfig } from '@goldziher/kreuzberg';

async function main() {
    const result = await extractFile('document.pdf', null, new ExtractionConfig());
    console.log(result.metadata.apiEnrichment);
}

main();
```

## Content Transformation Examples

### PII Redaction

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';

interface PIIRedactorOptions {
    redactEmails?: boolean;
    redactPhones?: boolean;
    redactSSN?: boolean;
}

class PIIRedactor {
    private options: PIIRedactorOptions;

    constructor(options: PIIRedactorOptions = {}) {
        this.options = {
            redactEmails: true,
            redactPhones: true,
            redactSSN: false,
            ...options,
        };
    }

    process(result: ExtractionResult): ExtractionResult {
        let content = result.content;

        if (this.options.redactEmails) {
            content = this.redactEmails(content);
        }

        if (this.options.redactPhones) {
            content = this.redactPhones(content);
        }

        if (this.options.redactSSN) {
            content = this.redactSSN(content);
        }

        result.content = content;
        result.metadata.piiRedacted = true;

        return result;
    }

    private redactEmails(text: string): string {
        return text.replace(
            /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/g,
            '[EMAIL REDACTED]'
        );
    }

    private redactPhones(text: string): string {
        // (555) 123-4567, 555-123-4567, 555.123.4567
        return text
            .replace(/\(\d{3}\)\s*\d{3}[-.\s]?\d{4}/g, '[PHONE REDACTED]')
            .replace(/\d{3}[-.\s]\d{3}[-.\s]\d{4}/g, '[PHONE REDACTED]');
    }

    private redactSSN(text: string): string {
        return text.replace(/\b\d{3}-\d{2}-\d{4}\b/g, '[SSN REDACTED]');
    }

    name(): string {
        return 'pii_redactor';
    }
}

registerPostProcessor(new PIIRedactor({
    redactEmails: true,
    redactPhones: true,
    redactSSN: true,
}));
```

### Text Normalization

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';

interface TextNormalizerOptions {
    normalizeWhitespace?: boolean;
    normalizeUnicode?: boolean;
    removeEmptyLines?: boolean;
}

class TextNormalizer {
    private options: TextNormalizerOptions;

    constructor(options: TextNormalizerOptions = {}) {
        this.options = {
            normalizeWhitespace: true,
            normalizeUnicode: true,
            removeEmptyLines: true,
            ...options,
        };
    }

    process(result: ExtractionResult): ExtractionResult {
        let content = result.content;

        if (this.options.normalizeUnicode) {
            // Normalize Unicode to NFC form
            content = content.normalize('NFC');
        }

        if (this.options.normalizeWhitespace) {
            // Replace multiple spaces with single space
            content = content.replace(/ +/g, ' ');
            // Replace multiple newlines with max 2
            content = content.replace(/\n{3,}/g, '\n\n');
        }

        if (this.options.removeEmptyLines) {
            // Remove lines with only whitespace
            content = content
                .split('\n')
                .filter(line => line.trim().length > 0)
                .join('\n');
        }

        result.content = content;
        result.metadata.textNormalized = true;

        return result;
    }

    name(): string {
        return 'text_normalizer';
    }
}

registerPostProcessor(new TextNormalizer());
```

### Content Summarization

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';

class ContentSummarizer {
    constructor(private maxSummaryLength: number = 500) {}

    process(result: ExtractionResult): ExtractionResult {
        const content = result.content;
        let summary = content.substring(0, this.maxSummaryLength);

        // Try to break at sentence boundary
        if (content.length > this.maxSummaryLength) {
            const lastPeriod = summary.lastIndexOf('.');
            const lastNewline = summary.lastIndexOf('\n');
            const breakPoint = Math.max(lastPeriod, lastNewline);

            if (breakPoint > 0) {
                summary = summary.substring(0, breakPoint + 1);
            } else {
                summary += '...';
            }
        }

        result.metadata.summary = summary.trim();
        result.metadata.isTruncated = content.length > this.maxSummaryLength;

        return result;
    }

    name(): string {
        return 'content_summarizer';
    }
}

registerPostProcessor(new ContentSummarizer(500));
```

## Advanced Patterns

### Conditional Processing

Process differently based on result properties:

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';

class ConditionalProcessor {
    process(result: ExtractionResult): ExtractionResult {
        // Check MIME type
        const mimeType = result.metadata.mimeType as string || '';

        if (mimeType === 'application/pdf') {
            this.processPDF(result);
        } else if (mimeType.startsWith('image/')) {
            this.processImage(result);
        }

        // Check content length
        const contentLength = result.content.length;
        if (contentLength < 100) {
            result.metadata.contentQuality = 'low';
        } else if (contentLength < 1000) {
            result.metadata.contentQuality = 'medium';
        } else {
            result.metadata.contentQuality = 'high';
        }

        return result;
    }

    private processPDF(result: ExtractionResult): void {
        result.metadata.format = 'pdf';
        // Add PDF-specific metadata
    }

    private processImage(result: ExtractionResult): void {
        result.metadata.format = 'image';
        // Add image-specific metadata
    }

    name(): string {
        return 'conditional_processor';
    }
}

registerPostProcessor(new ConditionalProcessor());
```

### Chaining with State

Maintain state across multiple processors:

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';

class StatefulProcessor {
    private processingCount: number = 0;
    private totalWords: number = 0;

    process(result: ExtractionResult): ExtractionResult {
        // Update state
        this.processingCount++;
        const words = result.content.split(/\s+/).filter(w => w.length > 0).length;
        this.totalWords += words;

        // Add state to metadata
        result.metadata.processingSequence = this.processingCount;
        result.metadata.cumulativeWords = this.totalWords;

        return result;
    }

    getStats() {
        return {
            totalProcessed: this.processingCount,
            totalWords: this.totalWords,
            avgWords: this.totalWords / Math.max(1, this.processingCount),
        };
    }

    name(): string {
        return 'stateful_processor';
    }
}

// Create and register
const processor = new StatefulProcessor();
registerPostProcessor(processor);

// Later, get stats
const stats = processor.getStats();
console.log(`Processed ${stats.totalProcessed} documents`);
```

### Error Recovery

Handle errors gracefully:

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';

class RobustProcessor {
    process(result: ExtractionResult): ExtractionResult {
        try {
            // Risky operation
            this.riskyOperation(result);
        } catch (error) {
            if (error instanceof TypeError) {
                // Handle specific error
                console.warn('TypeError in processing:', error);
                result.metadata.processingWarning = error.message;
            } else {
                // Handle unexpected errors
                console.error('Unexpected error in processing:', error);
                result.metadata.processingError = error instanceof Error
                    ? error.message
                    : 'Unknown error';
            }
            // Continue processing, don't fail
        }

        return result;
    }

    private riskyOperation(result: ExtractionResult): void {
        // Operation that might fail
    }

    name(): string {
        return 'robust_processor';
    }
}

registerPostProcessor(new RobustProcessor());
```

## Testing PostProcessors

### Unit Testing with Jest

```typescript
import { ExtractionResult } from '@goldziher/kreuzberg';
import { MetadataEnricher } from './metadata-enricher';

describe('MetadataEnricher', () => {
    test('should add statistics', () => {
        const result: ExtractionResult = {
            content: 'Hello world test',
            metadata: {},
            tables: [],
            chunks: [],
            detectedLanguages: [],
            keywords: [],
        };

        const processor = new MetadataEnricher({ addStats: true });
        const processed = processor.process(result);

        expect(processed.metadata.wordCount).toBe(3);
        expect(processed.metadata.charCount).toBe(16);
    });

    test('should add timestamp', () => {
        const result: ExtractionResult = {
            content: 'test',
            metadata: {},
            tables: [],
            chunks: [],
            detectedLanguages: [],
            keywords: [],
        };

        const processor = new MetadataEnricher({ addTimestamp: true });
        const processed = processor.process(result);

        expect(processed.metadata.processedAt).toBeDefined();
        expect(typeof processed.metadata.processedAt).toBe('string');
    });
});

describe('AsyncAPIEnricher', () => {
    test('should enrich with API data', async () => {
        const result: ExtractionResult = {
            content: 'test',
            metadata: {},
            tables: [],
            chunks: [],
            detectedLanguages: [],
            keywords: [],
        };

        const processor = new AsyncAPIEnricher({
            apiKey: 'test',
            apiUrl: 'http://localhost:3000',
        });

        const processed = await processor.process(result);
        expect(processed).toBeDefined();
    });
});
```

### Integration Testing

```typescript
import { extractFileSync, ExtractionConfig, registerPostProcessor } from '@goldziher/kreuzberg';
import { MetadataEnricher } from './metadata-enricher';

describe('Integration Tests', () => {
    test('should apply processor in extraction pipeline', () => {
        // Register processor
        registerPostProcessor(new MetadataEnricher());

        // Extract file
        const result = extractFileSync('test_document.pdf', null, new ExtractionConfig());

        // Verify processor ran
        expect(result.metadata.processedAt).toBeDefined();
        expect(result.metadata.wordCount).toBeDefined();
    });
});
```

## Best Practices

### Type Safety

Leverage TypeScript's type system:

```typescript
import { registerPostProcessor, ExtractionResult } from '@goldziher/kreuzberg';

interface ProcessorConfig {
    enabled: boolean;
    options: Record<string, any>;
}

class TypeSafeProcessor {
    private config: ProcessorConfig;

    constructor(config: ProcessorConfig) {
        this.config = config;
    }

    process(result: ExtractionResult): ExtractionResult {
        if (!this.config.enabled) {
            return result;
        }

        // Type-safe operations
        const metadata: Record<string, string | number | boolean> = {
            processed: true,
            timestamp: Date.now(),
            version: '1.0.0',
        };

        Object.assign(result.metadata, metadata);
        return result;
    }

    name(): string {
        return 'type_safe_processor';
    }
}
```

### Immutability

Avoid mutating shared state:

```typescript
class ImmutableProcessor {
    // ❌ Avoid mutable shared state
    // private counter: number = 0;

    // ✅ Use const for configuration
    private readonly config: Readonly<{
        maxLength: number;
        format: string;
    }>;

    constructor(config: { maxLength: number; format: string }) {
        this.config = Object.freeze(config);
    }

    process(result: ExtractionResult): ExtractionResult {
        // Create new objects instead of mutating
        return {
            ...result,
            metadata: {
                ...result.metadata,
                maxLength: this.config.maxLength,
            },
        };
    }

    name(): string {
        return 'immutable_processor';
    }
}
```

### Documentation

Use JSDoc for documentation:

```typescript
/**
 * A post-processor that enriches extraction results with metadata.
 *
 * @example
 * ```typescript
 * import { registerPostProcessor } from '@goldziher/kreuzberg';
 * import { MetadataEnricher } from './metadata-enricher';
 *
 * registerPostProcessor(new MetadataEnricher({
 *     addStats: true,
 *     addTimestamp: true,
 * }));
 * ```
 */
class MetadataEnricher {
    /**
     * Creates a new MetadataEnricher instance.
     *
     * @param options - Configuration options
     * @param options.addStats - Whether to add content statistics
     * @param options.addTimestamp - Whether to add processing timestamp
     */
    constructor(options?: MetadataEnricherOptions) {
        // Implementation
    }

    /**
     * Processes an extraction result by adding metadata.
     *
     * @param result - The extraction result to process
     * @returns The modified extraction result
     */
    process(result: ExtractionResult): ExtractionResult {
        // Implementation
    }
}
```

## Common Pitfalls

### 1. Forgetting to Return Result

```typescript
// ❌ Wrong - missing return
process(result: ExtractionResult): ExtractionResult {
    result.metadata.processed = true;
    // Missing return!
}

// ✅ Correct - always return
process(result: ExtractionResult): ExtractionResult {
    result.metadata.processed = true;
    return result;
}
```

### 2. Shared Mutable State

```typescript
// ❌ Wrong - shared mutable state
class BadProcessor {
    private results: ExtractionResult[] = [];

    process(result: ExtractionResult): ExtractionResult {
        this.results.push(result); // Dangerous!
        return result;
    }
}

// ✅ Correct - avoid shared state
class GoodProcessor {
    process(result: ExtractionResult): ExtractionResult {
        // Process without storing state
        return result;
    }
}
```

### 3. Blocking Operations

```typescript
// ❌ Wrong - synchronous I/O in async processor
async process(result: ExtractionResult): Promise<ExtractionResult> {
    const fs = require('fs');
    const data = fs.readFileSync('file.txt'); // Blocking!
    return result;
}

// ✅ Correct - use async I/O
async process(result: ExtractionResult): Promise<ExtractionResult> {
    const fs = require('fs').promises;
    const data = await fs.readFile('file.txt');
    return result;
}
```

## Next Steps

- [Plugin Development Overview](overview.md) - Compare plugin types
- [Python PostProcessor Development](python-postprocessor.md) - Python examples
- [API Reference](../api-reference/python/) - Complete API documentation
