/**
 * Basic Extraction Example
 *
 * Demonstrates basic document extraction with Kreuzberg.
 */

import {
    extractFile,
    extractFileSync,
    extractBytes,
    extractBytesSync,
    ExtractionConfig
} from '@goldziher/kreuzberg';
import { readFileSync } from 'fs';
import { readFile } from 'fs/promises';

async function main() {
    // Synchronous extraction - simplest approach
    console.log('=== Synchronous Extraction ===');
    const result = extractFileSync('document.pdf');
    console.log(`Content length: ${result.content.length} characters`);
    console.log(`MIME type: ${result.mimeType}`);
    console.log(`First 200 chars: ${result.content.substring(0, 200)}...`);

    // With configuration
    console.log('\n=== With Configuration ===');
    const config = new ExtractionConfig({
        enableQualityProcessing: true,
        useCache: true,
    });
    const configResult = extractFileSync('document.pdf', null, config);
    console.log(`Extracted ${configResult.content.length} characters with quality processing`);

    // Async extraction - for I/O-bound workloads
    console.log('\n=== Async Extraction ===');
    const asyncResult = await extractFile('document.pdf');
    console.log(`Async extracted: ${asyncResult.content.length} characters`);

    // Extract from bytes
    console.log('\n=== Extract from Bytes ===');
    const data = readFileSync('document.pdf');
    const bytesResult = extractBytesSync(data, 'application/pdf');
    console.log(`Extracted from bytes: ${bytesResult.content.length} characters`);

    // Extract from bytes (async)
    console.log('\n=== Extract from Bytes (Async) ===');
    const asyncData = await readFile('document.pdf');
    const asyncBytesResult = await extractBytes(asyncData, 'application/pdf');
    console.log(`Async extracted from bytes: ${asyncBytesResult.content.length} characters`);

    // Access metadata
    console.log('\n=== Metadata ===');
    const pdfResult = extractFileSync('document.pdf');
    if (pdfResult.metadata.pdf) {
        console.log(`PDF Pages: ${pdfResult.metadata.pdf.pageCount}`);
        console.log(`Author: ${pdfResult.metadata.pdf.author}`);
        console.log(`Title: ${pdfResult.metadata.pdf.title}`);
    }

    // Error handling
    console.log('\n=== Error Handling ===');
    try {
        extractFileSync('nonexistent.pdf');
    } catch (error) {
        console.error(`Caught error: ${error instanceof Error ? error.message : error}`);
    }
}

main().catch(console.error);
