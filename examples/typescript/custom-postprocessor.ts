/**
 * Custom PostProcessor Example
 *
 * Demonstrates implementing custom post-processor plugins.
 */

import {
    registerPostProcessor,
    unregisterPostProcessor,
    clearPostProcessors,
    extractFileSync,
    extractFile,
    ExtractionResult,
    PostProcessorProtocol,
} from '@goldziher/kreuzberg';

/**
 * Post-processor that enriches extraction results with metadata.
 */
class MetadataEnricher implements PostProcessorProtocol {
    name(): string {
        return 'metadata_enricher';
    }

    process(result: ExtractionResult): ExtractionResult {
        const content = result.content;

        // Calculate statistics
        result.metadata.processed_at = new Date().toISOString();
        result.metadata.word_count = content.split(/\s+/).filter(w => w.length > 0).length;
        result.metadata.char_count = content.length;
        result.metadata.line_count = content.split('\n').length;
        result.metadata.has_content = content.trim().length > 0;

        // Content type hints
        result.metadata.has_urls = /https?:\/\//.test(content);
        result.metadata.has_emails = /\S+@\S+\.\S+/.test(content);
        result.metadata.has_phone_numbers = /\d{3}[-.]?\d{3}[-.]?\d{4}/.test(content);

        console.log(`[MetadataEnricher] Added statistics: ${result.metadata.word_count} words`);

        return result;
    }
}

/**
 * Post-processor that redacts Personally Identifiable Information.
 */
class PIIRedactor implements PostProcessorProtocol {
    name(): string {
        return 'pii_redactor';
    }

    process(result: ExtractionResult): ExtractionResult {
        let content = result.content;

        // Redact emails
        content = content.replace(
            /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/g,
            '[EMAIL REDACTED]'
        );

        // Redact phone numbers (various formats)
        content = content.replace(/\(\d{3}\)\s*\d{3}[-.]?\d{4}/g, '[PHONE REDACTED]');
        content = content.replace(/\d{3}[-.]?\d{3}[-.]?\d{4}/g, '[PHONE REDACTED]');

        // Redact SSN
        content = content.replace(/\b\d{3}-\d{2}-\d{4}\b/g, '[SSN REDACTED]');

        // Redact credit card numbers
        content = content.replace(/\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b/g, '[CARD REDACTED]');

        result.content = content;
        result.metadata.pii_redacted = true;

        console.log('[PIIRedactor] Redacted PII from content');

        return result;
    }
}

/**
 * Post-processor that normalizes text formatting.
 */
class TextNormalizer implements PostProcessorProtocol {
    name(): string {
        return 'text_normalizer';
    }

    process(result: ExtractionResult): ExtractionResult {
        let content = result.content;

        // Normalize whitespace
        content = content.replace(/ +/g, ' ');  // Multiple spaces -> single space
        content = content.replace(/\n{3,}/g, '\n\n');  // Multiple newlines -> max 2

        // Remove empty lines
        const lines = content.split('\n').filter(line => line.trim().length > 0);
        content = lines.join('\n');

        // Normalize Unicode and trim
        content = content.normalize('NFC').trim();

        result.content = content;
        result.metadata.text_normalized = true;

        console.log('[TextNormalizer] Normalized text formatting');

        return result;
    }
}

/**
 * Post-processor that generates a summary of the content.
 */
class SummaryGenerator implements PostProcessorProtocol {
    constructor(private maxSummaryLength: number = 500) {}

    name(): string {
        return 'summary_generator';
    }

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
        result.metadata.is_truncated = content.length > this.maxSummaryLength;

        console.log(`[SummaryGenerator] Generated summary: ${summary.length} chars`);

        return result;
    }
}

/**
 * Post-processor that extracts keywords from content.
 */
class KeywordExtractor implements PostProcessorProtocol {
    name(): string {
        return 'keyword_extractor';
    }

    process(result: ExtractionResult): ExtractionResult {
        const content = result.content.toLowerCase();

        // Remove common words (stopwords)
        const stopwords = new Set(['the', 'a', 'an', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for', 'of', 'with', 'is', 'was', 'are', 'were', 'be', 'been', 'being']);

        // Extract words (4+ characters)
        const words = content.match(/\b[a-z]{4,}\b/g) || [];

        // Count word frequencies
        const wordFreq = new Map<string, number>();
        for (const word of words) {
            if (!stopwords.has(word)) {
                wordFreq.set(word, (wordFreq.get(word) || 0) + 1);
            }
        }

        // Get top 10 keywords
        const keywords = Array.from(wordFreq.entries())
            .sort((a, b) => b[1] - a[1])
            .slice(0, 10)
            .map(([word]) => word);

        result.metadata.keywords = keywords;

        console.log(`[KeywordExtractor] Extracted ${keywords.length} keywords`);

        return result;
    }
}

/**
 * Async post-processor that calls external API for enrichment.
 */
class ExternalAPIEnricher implements PostProcessorProtocol {
    constructor(private apiUrl: string, private apiKey: string) {}

    name(): string {
        return 'external_api_enricher';
    }

    async process(result: ExtractionResult): Promise<ExtractionResult> {
        try {
            // Mock API call - in production, use fetch or axios
            console.log(`[ExternalAPIEnricher] Calling API: ${this.apiUrl}`);

            // Simulated API response
            result.metadata.external_data = {
                sentiment: 'positive',
                topics: ['technology', 'business'],
                language_confidence: 0.95,
            };

            console.log('[ExternalAPIEnricher] API enrichment complete');
        } catch (error) {
            console.error('[ExternalAPIEnricher] API call failed:', error);
            result.metadata.api_error = error instanceof Error ? error.message : 'Unknown error';
        }

        return result;
    }
}

async function main() {
    // Register post-processors
    console.log('=== Registering Post-Processors ===');
    registerPostProcessor(new MetadataEnricher());
    registerPostProcessor(new PIIRedactor());
    registerPostProcessor(new TextNormalizer());
    registerPostProcessor(new SummaryGenerator(300));
    registerPostProcessor(new KeywordExtractor());
    registerPostProcessor(new ExternalAPIEnricher('https://api.example.com', 'api-key'));

    console.log('Registered 6 post-processors\n');

    // Extract with all post-processors
    console.log('=== Extraction with Post-Processors ===');
    const result = await extractFile('document.pdf');

    console.log('\nFinal result:');
    console.log(`  Content length: ${result.content.length} chars`);
    console.log(`  Word count: ${result.metadata.word_count}`);
    console.log(`  PII redacted: ${result.metadata.pii_redacted}`);
    console.log(`  Text normalized: ${result.metadata.text_normalized}`);
    console.log(`  Summary length: ${(result.metadata.summary as string)?.length || 0}`);
    console.log(`  Keywords: ${(result.metadata.keywords as string[])?.slice(0, 5)}`);

    // Unregister specific post-processor
    console.log('\n=== Unregister Post-Processor ===');
    unregisterPostProcessor('pii_redactor');
    console.log('Unregistered: pii_redactor');

    const result2 = extractFileSync('document.pdf');
    console.log(`PII redacted: ${result2.metadata.pii_redacted || false}`);  // Should be false now

    // Clear all post-processors
    console.log('\n=== Clear All Post-Processors ===');
    clearPostProcessors();
    console.log('Cleared all post-processors');

    const result3 = extractFileSync('document.pdf');
    console.log(`Word count (should be missing): ${result3.metadata.word_count}`);

    // Register selective post-processors
    console.log('\n=== Selective Post-Processing ===');
    registerPostProcessor(new MetadataEnricher());
    registerPostProcessor(new SummaryGenerator(200));

    const result4 = extractFileSync('document.pdf');
    console.log(`Only metadata enrichment and summary: ${Object.keys(result4.metadata)}`);
}

main().catch(console.error);
