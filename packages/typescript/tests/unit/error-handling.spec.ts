import { describe, it, expect } from "vitest";
import {
	extractFile,
	extractFileSync,
	extractBytes,
	extractBytesSync,
	batchExtractFiles,
	batchExtractFilesSync,
	registerPostProcessor,
	unregisterPostProcessor,
	clearPostProcessors,
	registerOcrBackend,
} from "../../src/index.js";

describe("Error Handling", () => {
	describe("File extraction errors", () => {
		it("should throw error for non-existent file (sync)", () => {
			expect(() => {
				extractFileSync("/nonexistent/file/path.pdf", null, null);
			}).toThrow();
		});

		it("should throw error for non-existent file (async)", async () => {
			await expect(
				extractFile("/nonexistent/file/path.pdf", null, null),
			).rejects.toThrow();
		});

		it("should throw error for invalid file path", () => {
			expect(() => {
				extractFileSync("", null, null);
			}).toThrow();
		});
	});

	describe("Bytes extraction errors", () => {
		it("should throw error for unsupported MIME type (sync)", () => {
			const data = new Uint8Array([1, 2, 3, 4]);
			expect(() => {
				extractBytesSync(data, "application/x-fake-mime-type", null);
			}).toThrow();
		});

		it("should throw error for unsupported MIME type (async)", async () => {
			const data = new Uint8Array([1, 2, 3, 4]);
			await expect(
				extractBytes(data, "application/x-fake-mime-type", null),
			).rejects.toThrow();
		});

		it("should throw error for empty MIME type", () => {
			const data = new Uint8Array([1, 2, 3, 4]);
			expect(() => {
				extractBytesSync(data, "", null);
			}).toThrow();
		});

		it("should throw error for malformed data", () => {
			// Invalid PDF header
			const data = new Uint8Array([0, 0, 0, 0]);
			expect(() => {
				extractBytesSync(data, "application/pdf", null);
			}).toThrow();
		});
	});

	describe("Batch extraction errors", () => {
		it("should return error metadata for failed files in batch (sync)", () => {
			// Batch extraction returns results with error metadata instead of throwing
			const paths = ["/nonexistent/file1.pdf"];

			const results = batchExtractFilesSync(paths, null);

			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(1);
			// Error information should be in metadata
			expect(results[0].metadata.error).toBeTruthy();
			expect(results[0].content).toContain("Error:");
		});

		it("should return error metadata for failed files in batch (async)", async () => {
			// Batch extraction returns results with error metadata instead of throwing
			const paths = ["/nonexistent/file1.pdf"];

			const results = await batchExtractFiles(paths, null);

			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(1);
			// Error information should be in metadata
			expect(results[0].metadata.error).toBeTruthy();
			expect(results[0].content).toContain("Error:");
		});
	});

	describe("Plugin registration errors (not yet implemented)", () => {
		it("should throw error when registering postprocessor", () => {
			const processor = {
				name: () => "test_processor",
				process: (result: any) => result,
				processingStage: () => "middle" as const,
			};

			expect(() => {
				registerPostProcessor(processor);
			}).toThrow("registerPostProcessor not yet implemented");
		});

		it("should throw error when unregistering postprocessor", () => {
			expect(() => {
				unregisterPostProcessor("test_processor");
			}).toThrow("unregisterPostProcessor not yet implemented");
		});

		it("should throw error when clearing postprocessors", () => {
			expect(() => {
				clearPostProcessors();
			}).toThrow("clearPostProcessors not yet implemented");
		});

		it("should throw error when registering OCR backend", () => {
			const backend = {
				name: () => "test_ocr",
				extractText: async () => ({ text: "test", confidence: 1.0 }),
			};

			expect(() => {
				registerOcrBackend(backend);
			}).toThrow("registerOcrBackend not yet implemented");
		});
	});
});
