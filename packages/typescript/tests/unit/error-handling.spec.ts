import { describe, expect, it, vi } from "vitest";
import {
	__resetBindingForTests,
	__setBindingForTests,
	batchExtractFiles,
	batchExtractFilesSync,
	clearPostProcessors,
	extractBytes,
	extractBytesSync,
	extractFile,
	extractFileSync,
	registerOcrBackend,
	registerPostProcessor,
	unregisterPostProcessor,
} from "../../src/index.js";

describe("Error Handling", () => {
	describe("File extraction errors", () => {
		it("should throw error for non-existent file (sync)", () => {
			expect(() => {
				extractFileSync("/nonexistent/file/path.pdf", null, null);
			}).toThrow();
		});

		it("should throw error for non-existent file (async)", async () => {
			await expect(extractFile("/nonexistent/file/path.pdf", null, null)).rejects.toThrow();
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
			await expect(extractBytes(data, "application/x-fake-mime-type", null)).rejects.toThrow();
		});

		it("should throw error for empty MIME type", () => {
			const data = new Uint8Array([1, 2, 3, 4]);
			expect(() => {
				extractBytesSync(data, "", null);
			}).toThrow();
		});

		it("should throw error for malformed data", () => {
			const data = new Uint8Array([0, 0, 0, 0]);
			expect(() => {
				extractBytesSync(data, "application/pdf", null);
			}).toThrow();
		});
	});

	describe("Batch extraction errors", () => {
		it("should return error metadata for failed files in batch (sync)", () => {
			const paths = ["/nonexistent/file1.pdf"];

			const results = batchExtractFilesSync(paths, null);

			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(1);
			expect(results[0].metadata.error).toBeTruthy();
			expect(results[0].content).toContain("Error:");
		});

		it("should return error metadata for failed files in batch (async)", async () => {
			const paths = ["/nonexistent/file1.pdf"];

			const results = await batchExtractFiles(paths, null);

			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(1);
			expect(results[0].metadata.error).toBeTruthy();
			expect(results[0].content).toContain("Error:");
		});
	});

	describe("Plugin registration", () => {
		it("should allow registering postprocessor", () => {
			const processor = {
				name: () => "test_processor",
				process: (result: any) => result,
				processingStage: () => "middle" as const,
			};

			expect(() => {
				registerPostProcessor(processor);
			}).not.toThrow();
		});

		it("should allow unregistering postprocessor", () => {
			expect(() => {
				unregisterPostProcessor("test_processor");
			}).not.toThrow();
		});

		it("should allow clearing postprocessors", () => {
			expect(() => {
				clearPostProcessors();
			}).not.toThrow();
		});

		it("should forward OCR backend registration to native binding", () => {
			const backend = {
				name: () => "test_ocr",
				supportedLanguages: () => ["eng", "spa"],
				processImage: async () => ({ content: "test", confidence: 1.0, textRegions: 1 }),
				extractText: async () => ({ text: "test", confidence: 1.0 }),
			};

			const mockBinding = {
				registerOcrBackend: vi.fn(),
			};

			__setBindingForTests(mockBinding);
			try {
				expect(() => {
					registerOcrBackend(backend);
				}).not.toThrow();
				expect(mockBinding.registerOcrBackend).toHaveBeenCalledTimes(1);
			} finally {
				__resetBindingForTests();
			}
		});
	});
});
