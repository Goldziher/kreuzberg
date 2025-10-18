import { join } from "node:path";
import { existsSync } from "node:fs";
import { expect } from "vitest";
import type { ExtractionResult, Metadata } from "../../src/types.js";

/**
 * Get path to test document in the repository's test_documents directory.
 *
 * @param relativePath - Path relative to test_documents (e.g., "pdfs/simple.pdf")
 * @returns Absolute path to the test document
 */
export function getTestDocumentPath(relativePath: string): string {
	// From packages/typescript/tests -> workspace root
	const workspaceRoot = join(process.cwd(), "../..");
	return join(workspaceRoot, "test_documents", relativePath);
}

/**
 * Check if test documents directory is available.
 *
 * @returns true if test_documents exists
 */
export function testDocumentsAvailable(): boolean {
	const workspaceRoot = join(process.cwd(), "../..");
	return existsSync(join(workspaceRoot, "test_documents"));
}

/**
 * Assert that extraction result has expected MIME type.
 *
 * @param result - Extraction result
 * @param expectedMimeType - Expected MIME type (can be partial match)
 */
export function assertMimeType(
	result: ExtractionResult,
	expectedMimeType: string,
): void {
	expect(result.mimeType).toContain(expectedMimeType);
}

/**
 * Assert that extraction result has non-empty content.
 *
 * @param result - Extraction result
 * @param minLength - Minimum content length (default: 1)
 */
export function assertNonEmptyContent(
	result: ExtractionResult,
	minLength = 1,
): void {
	expect(result.content).toBeTruthy();
	expect(result.content.length).toBeGreaterThanOrEqual(minLength);
}

/**
 * Assert that extraction result structure is valid.
 * Validates all required fields exist and have correct types.
 *
 * @param result - Extraction result
 */
export function assertValidExtractionResult(
	result: ExtractionResult,
): void {
	// Required fields
	expect(result).toHaveProperty("content");
	expect(result).toHaveProperty("mimeType");
	expect(result).toHaveProperty("metadata");
	expect(result).toHaveProperty("tables");
	expect(result).toHaveProperty("detectedLanguages");

	// Type checks
	expect(typeof result.content).toBe("string");
	expect(typeof result.mimeType).toBe("string");
	expect(typeof result.metadata).toBe("object");
	expect(result.metadata).not.toBeNull();
	expect(Array.isArray(result.tables)).toBe(true);

	// detectedLanguages can be null or array
	if (result.detectedLanguages !== null) {
		expect(Array.isArray(result.detectedLanguages)).toBe(true);
	}

	// chunks can be undefined, null, or array
	if (result.chunks !== undefined && result.chunks !== null) {
		expect(Array.isArray(result.chunks)).toBe(true);
	}
}

/**
 * Assert that metadata contains expected PDF fields.
 * Note: PDF metadata is optional - this validates structure if present.
 *
 * @param metadata - Extraction metadata
 */
export function assertPdfMetadata(metadata: Metadata): void {
	// PDF metadata is optional, but if present should be valid
	if (metadata.pdf) {
		// If page count exists, it should be > 0
		if (metadata.pdf.pageCount !== undefined) {
			expect(metadata.pdf.pageCount).toBeGreaterThan(0);
		}
	}
}

/**
 * Assert that metadata contains expected Excel fields.
 * Note: Excel metadata is optional - this validates structure if present.
 *
 * @param metadata - Extraction metadata
 */
export function assertExcelMetadata(metadata: Metadata): void {
	// Excel metadata is optional, but if present should be valid
	if (metadata.excel) {
		if (metadata.excel.sheetCount !== undefined) {
			expect(metadata.excel.sheetCount).toBeGreaterThan(0);
		}
		if (metadata.excel.sheetNames !== undefined) {
			expect(Array.isArray(metadata.excel.sheetNames)).toBe(true);
		}
	}
}

/**
 * Assert that metadata contains expected image fields.
 *
 * @param metadata - Extraction metadata
 */
export function assertImageMetadata(metadata: Metadata): void {
	expect(metadata.image).toBeTruthy();
	if (metadata.image) {
		// Should have width and height
		expect(metadata.image.width).toBeTruthy();
		expect(metadata.image.height).toBeTruthy();
		expect(metadata.image.width).toBeGreaterThan(0);
		expect(metadata.image.height).toBeGreaterThan(0);

		// Format should be present
		if (metadata.image.format) {
			expect(typeof metadata.image.format).toBe("string");
		}
	}
}

/**
 * Assert that OCR result contains expected text with confidence validation.
 *
 * @param result - Extraction result
 * @param expectedWords - Words expected in the content
 * @param minConfidence - Minimum acceptable confidence (default: 0.3)
 */
export function assertOcrResult(
	result: ExtractionResult,
	expectedWords: string[],
	minConfidence = 0.3,
): void {
	assertValidExtractionResult(result);

	// Normalize content for comparison
	const contentLower = result.content.toLowerCase().replace(/\n/g, " ").trim();

	// Check for expected words
	const foundWords = expectedWords.filter((word) =>
		contentLower.includes(word.toLowerCase()),
	);

	// At least one expected word should be found
	expect(foundWords.length).toBeGreaterThan(0);

	// Validate OCR metadata if present
	if (result.metadata.ocr) {
		// Confidence should be in valid range [0, 1] if present
		// Note: OCR metadata structure varies by backend
		// We check for common confidence field patterns
		const metadata: any = result.metadata;
		if (metadata.confidence !== undefined) {
			expect(metadata.confidence).toBeGreaterThanOrEqual(0.0);
			expect(metadata.confidence).toBeLessThanOrEqual(1.0);

			// If text was detected, confidence should be reasonable
			if (foundWords.length > 0) {
				expect(metadata.confidence).toBeGreaterThanOrEqual(minConfidence);
			}
		}
	}
}

/**
 * Assert that result contains substantial content (for large documents).
 *
 * @param result - Extraction result
 * @param minBytes - Minimum content size in bytes
 */
export function assertSubstantialContent(
	result: ExtractionResult,
	minBytes = 1000,
): void {
	assertNonEmptyContent(result, minBytes);
	expect(result.content.length).toBeGreaterThanOrEqual(minBytes);
}

/**
 * Assert that tables were extracted.
 *
 * @param result - Extraction result
 * @param minTables - Minimum number of tables expected
 */
export function assertTablesExtracted(
	result: ExtractionResult,
	minTables = 1,
): void {
	expect(result.tables.length).toBeGreaterThanOrEqual(minTables);

	// Validate table structure
	for (const table of result.tables) {
		expect(table.cells).toBeTruthy();
		expect(Array.isArray(table.cells)).toBe(true);
		expect(table.cells.length).toBeGreaterThan(0);

		// Validate cells are 2D array
		expect(Array.isArray(table.cells[0])).toBe(true);

		// Markdown representation should exist
		expect(table.markdown).toBeTruthy();
		expect(typeof table.markdown).toBe("string");
	}
}

/**
 * Assert that HTML was converted to Markdown.
 *
 * @param result - Extraction result
 */
export function assertMarkdownConversion(result: ExtractionResult): void {
	assertNonEmptyContent(result);

	// Check for common Markdown patterns
	const hasHeaders = result.content.includes("##") || result.content.includes("#");
	const hasTables = result.content.includes("|");
	const hasLinks = result.content.includes("[");
	const hasBold = result.content.includes("**");

	// At least one markdown pattern should be present
	expect(hasHeaders || hasTables || hasLinks || hasBold).toBe(true);
}
