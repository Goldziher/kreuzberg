/**
 * Custom Validator Example
 *
 * Demonstrates implementing custom validator plugins.
 */

import {
    registerValidator,
    unregisterValidator,
    clearValidators,
    extractFileSync,
    extractFile,
    ExtractionResult,
    ValidatorProtocol,
} from '@goldziher/kreuzberg';

/**
 * Validator that enforces minimum content length.
 */
class MinLengthValidator implements ValidatorProtocol {
    constructor(private minLength: number = 100) {}

    name(): string {
        return 'min_length_validator';
    }

    validate(result: ExtractionResult): void {
        if (result.content.length < this.minLength) {
            throw new Error(
                `ValidationError: Content too short (${result.content.length} < ${this.minLength} characters)`
            );
        }
        console.log(`[MinLengthValidator] ✓ Content length OK: ${result.content.length} chars`);
    }

    priority(): number {
        return 100;  // High priority - run first
    }
}

/**
 * Validator that checks for required metadata fields.
 */
class MetadataValidator implements ValidatorProtocol {
    constructor(private requiredFields: string[]) {}

    name(): string {
        return 'metadata_validator';
    }

    validate(result: ExtractionResult): void {
        for (const field of this.requiredFields) {
            const value = this.getNestedProperty(result.metadata, field);
            if (value === null || value === undefined) {
                throw new Error(
                    `ValidationError: Missing required metadata field: ${field}`
                );
            }
        }
        console.log(`[MetadataValidator] ✓ All required fields present`);
    }

    priority(): number {
        return 90;  // Run after basic validation
    }

    private getNestedProperty(obj: any, path: string): any {
        return path.split('.').reduce((current, key) => current?.[key], obj);
    }
}

/**
 * Validator that checks PDF-specific requirements.
 */
class PDFValidator implements ValidatorProtocol {
    constructor(
        private minPages: number = 1,
        private maxPages: number = 1000
    ) {}

    name(): string {
        return 'pdf_validator';
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

        const pageCount = pdfMetadata.pageCount;
        if (!pageCount || pageCount < this.minPages) {
            throw new Error(
                `ValidationError: PDF has too few pages (${pageCount})`
            );
        }

        if (pageCount > this.maxPages) {
            throw new Error(
                `ValidationError: PDF has too many pages (${pageCount} > ${this.maxPages})`
            );
        }

        console.log(`[PDFValidator] ✓ Page count OK: ${pageCount} pages`);
    }

    priority(): number {
        return 80;  // Run after metadata validation
    }
}

/**
 * Validator that checks language requirements.
 */
class LanguageValidator implements ValidatorProtocol {
    constructor(private allowedLanguages: string[]) {}

    name(): string {
        return 'language_validator';
    }

    validate(result: ExtractionResult): void {
        if (!result.detectedLanguages || result.detectedLanguages.length === 0) {
            throw new Error('ValidationError: No languages detected in content');
        }

        const primaryLanguage = result.detectedLanguages[0];
        if (!this.allowedLanguages.includes(primaryLanguage)) {
            throw new Error(
                `ValidationError: Detected language '${primaryLanguage}' not in allowed list: ${this.allowedLanguages.join(', ')}`
            );
        }

        console.log(`[LanguageValidator] ✓ Language OK: ${primaryLanguage}`);
    }

    priority(): number {
        return 75;  // Run before content-specific validators
    }
}

/**
 * Validator that checks content quality.
 */
class QualityValidator implements ValidatorProtocol {
    name(): string {
        return 'quality_validator';
    }

    validate(result: ExtractionResult): void {
        const content = result.content.trim();

        // Check for empty content
        if (content.length === 0) {
            throw new Error('ValidationError: Content is empty');
        }

        // Check word count
        const words = content.split(/\s+/).filter(w => w.length > 0);
        if (words.length < 5) {
            throw new Error(
                `ValidationError: Too few words (${words.length} < 5)`
            );
        }

        // Check for suspicious patterns (e.g., OCR errors)
        const nonAlphanumericRatio = (content.match(/[^a-zA-Z0-9\s]/g) || []).length / content.length;
        if (nonAlphanumericRatio > 0.5) {
            throw new Error(
                `ValidationError: Too many non-alphanumeric characters (${(nonAlphanumericRatio * 100).toFixed(1)}%)`
            );
        }

        console.log(`[QualityValidator] ✓ Content quality OK`);
    }

    priority(): number {
        return 70;  // Run after format-specific validators
    }
}

/**
 * Async validator that performs external validation.
 */
class ExternalValidator implements ValidatorProtocol {
    constructor(private apiUrl: string, private apiKey: string) {}

    name(): string {
        return 'external_validator';
    }

    async validate(result: ExtractionResult): Promise<void> {
        try {
            console.log(`[ExternalValidator] Calling validation API: ${this.apiUrl}`);

            // Mock API call - in production, use fetch
            // const response = await fetch(this.apiUrl, {
            //     method: 'POST',
            //     headers: {
            //         'Authorization': `Bearer ${this.apiKey}`,
            //         'Content-Type': 'application/json',
            //     },
            //     body: JSON.stringify({
            //         content: result.content.substring(0, 5000),
            //         metadata: result.metadata,
            //     }),
            // });
            //
            // if (!response.ok) {
            //     throw new Error(`API returned ${response.status}`);
            // }
            //
            // const data = await response.json();
            // if (!data.valid) {
            //     throw new Error(`ValidationError: ${data.reason}`);
            // }

            console.log('[ExternalValidator] ✓ External validation passed');
        } catch (error) {
            if (error instanceof Error && error.message.includes('ValidationError')) {
                throw error;  // Re-throw validation errors
            }

            // Handle network/API errors
            throw new Error(
                `ValidationError: External validation failed: ${error instanceof Error ? error.message : 'Unknown error'}`
            );
        }
    }

    priority(): number {
        return 50;  // Default priority
    }
}

async function main() {
    // Register validators
    console.log('=== Registering Validators ===');
    registerValidator(new MinLengthValidator(50));
    registerValidator(new MetadataValidator(['mime_type', 'pdf.pageCount']));
    registerValidator(new PDFValidator(1, 500));
    registerValidator(new LanguageValidator(['eng', 'deu', 'fra']));
    registerValidator(new QualityValidator());
    registerValidator(new ExternalValidator('https://api.example.com/validate', 'api-key'));

    console.log('Registered 6 validators\n');

    // Extract with all validators - this will pass
    console.log('=== Extraction with Validators (Pass) ===');
    try {
        const result = await extractFile('document.pdf');
        console.log(`\n✓ Validation passed: ${result.content.length} chars extracted`);
    } catch (error) {
        console.error(`\n✗ Validation failed: ${error instanceof Error ? error.message : error}`);
    }

    // Extract with validators - this will fail
    console.log('\n=== Extraction with Validators (Fail) ===');
    try {
        const result = extractFileSync('short_document.pdf');
        console.log(`✓ Validation passed: ${result.content.length} chars`);
    } catch (error) {
        console.error(`✗ Validation failed: ${error instanceof Error ? error.message : error}`);
    }

    // Unregister specific validator
    console.log('\n=== Unregister Validator ===');
    unregisterValidator('min_length_validator');
    console.log('Unregistered: min_length_validator');

    try {
        const result = extractFileSync('short_document.pdf');
        console.log(`✓ Now passes (no min length check): ${result.content.length} chars`);
    } catch (error) {
        console.error(`✗ Still failed: ${error instanceof Error ? error.message : error}`);
    }

    // Clear all validators
    console.log('\n=== Clear All Validators ===');
    clearValidators();
    console.log('Cleared all validators');

    const result = extractFileSync('document.pdf');
    console.log(`✓ No validation: ${result.content.length} chars extracted`);

    // Register selective validators
    console.log('\n=== Selective Validation ===');
    registerValidator(new QualityValidator());
    registerValidator(new MinLengthValidator(100));

    try {
        const result2 = extractFileSync('document.pdf');
        console.log(`✓ Selective validation passed: ${result2.content.length} chars`);
    } catch (error) {
        console.error(`✗ Selective validation failed: ${error instanceof Error ? error.message : error}`);
    }
}

main().catch(console.error);
