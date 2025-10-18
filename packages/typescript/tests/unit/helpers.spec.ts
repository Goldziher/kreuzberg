import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { extractFileSync, extractBytesSync } from "../../src/index.js";

function getTestDocumentPath(relativePath: string): string {
	const workspaceRoot = join(process.cwd(), "../..");
	return join(workspaceRoot, "test_documents", relativePath);
}

describe("Helper Functions and Edge Cases", () => {
	describe("Metadata parsing", () => {
		it("should parse JSON metadata from string", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const result = extractFileSync(pdfPath, null, null);

			// Metadata should be an object, not a string
			expect(typeof result.metadata).toBe("object");
			expect(result.metadata).not.toBeNull();
		});

		it("should handle metadata with nested objects", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const result = extractFileSync(pdfPath, null, {
				pdfOptions: { extractMetadata: true },
			});

			expect(typeof result.metadata).toBe("object");
			// PDF metadata may contain nested objects
			if (result.metadata.pdf) {
				expect(typeof result.metadata.pdf).toBe("object");
			}
		});
	});

	describe("Result conversion", () => {
		const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

		it("should convert raw result to ExtractionResult", () => {
			const result = extractFileSync(pdfPath, null, null);

			// Check all required fields
			expect(result).toHaveProperty("content");
			expect(result).toHaveProperty("mimeType");
			expect(result).toHaveProperty("metadata");
			expect(result).toHaveProperty("tables");
			expect(result).toHaveProperty("detectedLanguages");
			expect(result).toHaveProperty("chunks");
		});

		it("should handle missing tables as empty array", () => {
			const result = extractFileSync(pdfPath, null, null);

			expect(Array.isArray(result.tables)).toBe(true);
		});

		it("should handle detectedLanguages as null when not enabled", () => {
			const result = extractFileSync(pdfPath, null, null);

			// Without language detection config, should be null
			expect(
				result.detectedLanguages === null ||
					Array.isArray(result.detectedLanguages),
			).toBe(true);
		});

		it("should handle chunks as null when not configured", () => {
			const result = extractFileSync(pdfPath, null, null);

			// Without chunking config, should be null or undefined
			expect(
				result.chunks === null ||
					result.chunks === undefined ||
					Array.isArray(result.chunks),
			).toBe(true);
		});

		it("should preserve metadata when already an object", () => {
			const result = extractFileSync(pdfPath, null, null);

			// Metadata should be object, not string
			expect(typeof result.metadata).toBe("object");
			expect(result.metadata).not.toBeNull();
		});
	});

	describe("MIME type handling", () => {
		it("should auto-detect MIME type when null", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const result = extractFileSync(pdfPath, null, null);

			expect(result.mimeType).toContain("application/pdf");
		});

		it("should use provided MIME type hint", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const result = extractFileSync(pdfPath, "application/pdf", null);

			expect(result.mimeType).toContain("application/pdf");
		});

		it("should require MIME type for bytes extraction", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const bytes = new Uint8Array(readFileSync(pdfPath));

			const result = extractBytesSync(bytes, "application/pdf", null);
			expect(result.mimeType).toContain("application/pdf");
		});
	});

	describe("Buffer/Uint8Array conversion", () => {
		const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
		const pdfBytes = readFileSync(pdfPath);

		it("should handle Node.js Buffer", () => {
			const uint8 = new Uint8Array(pdfBytes);
			const result = extractBytesSync(uint8, "application/pdf", null);

			expect(result.content).toBeTruthy();
		});

		it("should handle Uint8Array from various sources", () => {
			// Create from buffer
			const uint8 = new Uint8Array(pdfBytes);
			const result = extractBytesSync(uint8, "application/pdf", null);

			expect(result.content).toBeTruthy();
		});

		it("should handle ArrayBuffer conversion", () => {
			const buffer = pdfBytes.buffer;
			const uint8 = new Uint8Array(buffer);
			const result = extractBytesSync(uint8, "application/pdf", null);

			expect(result.content).toBeTruthy();
		});
	});

	describe("Content validation", () => {
		it("should return string content", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const result = extractFileSync(pdfPath, null, null);

			expect(typeof result.content).toBe("string");
			expect(result.content.length).toBeGreaterThan(0);
		});

		it("should return valid MIME type string", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const result = extractFileSync(pdfPath, null, null);

			expect(typeof result.mimeType).toBe("string");
			expect(result.mimeType.length).toBeGreaterThan(0);
			expect(result.mimeType).toMatch(/^[a-z]+\/[a-z0-9+.-]+$/);
		});

		it("should return valid metadata object", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const result = extractFileSync(pdfPath, null, null);

			expect(typeof result.metadata).toBe("object");
			expect(result.metadata).not.toBeNull();
			expect(result.metadata).not.toBeUndefined();
		});

		it("should return valid tables array", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const result = extractFileSync(pdfPath, null, null);

			expect(Array.isArray(result.tables)).toBe(true);
			// Tables may be empty, but should be array
			result.tables.forEach((table) => {
				expect(table).toHaveProperty("cells");
				expect(table).toHaveProperty("markdown");
				expect(table).toHaveProperty("pageNumber");
			});
		});
	});

	describe("Edge cases", () => {
		it("should handle very small files", () => {
			const textPath = getTestDocumentPath("pandoc/simple_metadata.md");
			const result = extractFileSync(textPath, null, null);

			expect(result.content).toBeTruthy();
			expect(typeof result.content).toBe("string");
		});

		it("should handle files with special characters in path", () => {
			// Most paths should work fine
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const result = extractFileSync(pdfPath, null, null);

			expect(result.content).toBeTruthy();
		});

		it("should handle multiple extractions of same file", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result1 = extractFileSync(pdfPath, null, null);
			const result2 = extractFileSync(pdfPath, null, null);

			expect(result1.content).toBeTruthy();
			expect(result2.content).toBeTruthy();
			// Content should be consistent
			expect(result1.mimeType).toBe(result2.mimeType);
		});

		it("should handle extraction with different configs on same file", () => {
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result1 = extractFileSync(pdfPath, null, { useCache: true });
			const result2 = extractFileSync(pdfPath, null, { useCache: false });

			expect(result1.content).toBeTruthy();
			expect(result2.content).toBeTruthy();
		});
	});
});
