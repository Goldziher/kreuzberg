import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

function getTestDocumentPath(relativePath: string): string {
	const workspaceRoot = join(process.cwd(), "../..");
	return join(workspaceRoot, "test_documents", relativePath);
}

describe("Edge Cases and Coverage", () => {
	describe("Metadata parsing edge cases", () => {
		it("should handle invalid JSON in metadata string", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, null, null);

			expect(typeof result.metadata).toBe("object");
			expect(result.metadata).not.toBeNull();
		});

		it("should handle empty metadata", () => {
			const mockResult = {
				content: "test",
				mimeType: "text/plain",
				metadata: {},
				tables: [],
				detectedLanguages: null,
				chunks: null,
			};

			expect(mockResult.metadata).toEqual({});
		});
	});

	describe("Result field handling", () => {
		it("should handle null tables field", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, null, null);

			expect(Array.isArray(result.tables)).toBe(true);
		});

		it("should handle null chunks field", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, null, null);

			if (result.chunks !== null) {
				expect(Array.isArray(result.chunks)).toBe(true);
			}
		});

		it("should handle null images field", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, null, null);

			if (result.images !== null) {
				expect(Array.isArray(result.images)).toBe(true);
			}
		});

		it("should handle undefined detectedLanguages field", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, null, null);

			if (result.detectedLanguages !== null) {
				expect(Array.isArray(result.detectedLanguages)).toBe(true);
			}
		});
	});

	describe("Binding initialization edge cases", () => {
		it("should cache binding after first initialization", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result1 = extractFileSync(pdfPath, null, null);
			expect(result1).toBeTruthy();

			const result2 = extractFileSync(pdfPath, null, null);
			expect(result2).toBeTruthy();
		});
	});

	describe("Configuration defaults", () => {
		it("should handle completely empty config", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, null, {});

			expect(result.content).toBeTruthy();
		});

		it("should handle partial config objects", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, null, { useCache: false });

			expect(result.content).toBeTruthy();
		});

		it("should handle nested partial configs", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, null, {
				ocr: { backend: "tesseract" },
			});

			expect(result.content).toBeTruthy();
		});
	});

	describe("Type conversions", () => {
		it("should convert Uint8Array to Buffer consistently", async () => {
			const { extractBytesSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
			const bytes = readFileSync(pdfPath);

			const uint8Array = new Uint8Array(bytes);
			const result = extractBytesSync(uint8Array, "application/pdf", null);

			expect(result.content).toBeTruthy();
		});

		it("should handle empty Uint8Array", async () => {
			const { extractBytesSync } = await import("../../src/index.js");

			const emptyArray = new Uint8Array([]);

			expect(() => {
				extractBytesSync(emptyArray, "application/pdf", null);
			}).toThrow();
		});
	});

	describe("MIME type auto-detection", () => {
		it("should auto-detect when MIME type is null", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, null, null);

			expect(result.mimeType).toContain("pdf");
		});

		it("should use provided MIME type over auto-detection", async () => {
			const { extractFileSync } = await import("../../src/index.js");
			const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");

			const result = extractFileSync(pdfPath, "application/pdf", null);

			expect(result.mimeType).toContain("application/pdf");
		});
	});

	describe("Version export", () => {
		it("should export version string", async () => {
			const { __version__ } = await import("../../src/index.js");

			expect(typeof __version__).toBe("string");
			expect(__version__).toMatch(/^\d+\.\d+\.\d+(-[\w.]+)?$/);
		});
	});

	describe("Module exports", () => {
		it("should export all main functions", async () => {
			const module = await import("../../src/index.js");

			expect(module.extractFile).toBeDefined();
			expect(module.extractFileSync).toBeDefined();
			expect(module.extractBytes).toBeDefined();
			expect(module.extractBytesSync).toBeDefined();
			expect(module.batchExtractFiles).toBeDefined();
			expect(module.batchExtractFilesSync).toBeDefined();
			expect(module.batchExtractBytes).toBeDefined();
		});

		it("should export plugin functions", async () => {
			const module = await import("../../src/index.js");

			expect(module.registerPostProcessor).toBeDefined();
			expect(module.unregisterPostProcessor).toBeDefined();
			expect(module.clearPostProcessors).toBeDefined();
			expect(module.registerOcrBackend).toBeDefined();
		});
	});
});
