import type { ExtractionConfig } from "@kreuzberg/core";
import { describe, expect, it } from "vitest";
import {
	configToJS,
	fileToUint8Array,
	isValidExtractionResult,
	jsToExtractionResult,
	wrapWasmError,
} from "./wasm-adapter.js";

describe("WASM Adapter", () => {
	describe("fileToUint8Array", () => {
		it("should handle successful file conversion", async () => {
			const mockFile = {
				size: 5,
				arrayBuffer: async () => new ArrayBuffer(5),
			};

			const result = await fileToUint8Array(mockFile as unknown as Blob);

			expect(result).toBeInstanceOf(Uint8Array);
		});

		it("should throw if file exceeds max size", async () => {
			const largeBlob = {
				size: 512 * 1024 * 1024 + 1,
				arrayBuffer: async () => new ArrayBuffer(0),
			};

			await expect(fileToUint8Array(largeBlob as unknown as Blob)).rejects.toThrow("exceeds maximum");
		});

		it("should validate file size before reading", async () => {
			const mockFile = {
				size: 1024 * 1024,
				arrayBuffer: async () => new ArrayBuffer(1024 * 1024),
			};

			const result = await fileToUint8Array(mockFile as unknown as Blob);

			expect(result).toBeInstanceOf(Uint8Array);
		});

		it("should handle error during arrayBuffer read", async () => {
			const mockFile = {
				size: 100,
				arrayBuffer: async () => {
					throw new Error("Read failed");
				},
			};

			await expect(fileToUint8Array(mockFile as unknown as Blob)).rejects.toThrow("Failed to read file");
		});
	});

	describe("configToJS", () => {
		it("should return empty object for null config", () => {
			const result = configToJS(null);
			expect(result).toEqual({});
		});

		it("should normalize simple config", () => {
			const config: ExtractionConfig = {
				ocr: { backend: "tesseract" },
			};

			const result = configToJS(config);

			expect(result).toHaveProperty("ocr");
			expect((result.ocr as Record<string, unknown>).backend).toBe("tesseract");
		});

		it("should remove undefined values", () => {
			const config: Record<string, unknown> = {
				ocr: { backend: "tesseract" },
				chunking: undefined,
			};

			const result = configToJS(config as ExtractionConfig);

			expect(result).not.toHaveProperty("chunking");
			expect(result).toHaveProperty("ocr");
		});

		it("should remove null values", () => {
			const config: Record<string, unknown> = {
				ocr: { backend: "tesseract" },
				chunking: null,
			};

			const result = configToJS(config as ExtractionConfig);

			expect(result).not.toHaveProperty("chunking");
		});

		it("should handle nested objects", () => {
			const config: ExtractionConfig = {
				ocr: {
					backend: "tesseract",
					language: "eng",
				},
				chunking: {
					maxChars: 1000,
					chunkOverlap: 100,
				},
			};

			const result = configToJS(config);

			expect(result.ocr).toEqual({ backend: "tesseract", language: "eng" });
			expect(result.chunking).toEqual({
				maxChars: 1000,
				chunkOverlap: 100,
			});
		});

		it("should handle arrays in config", () => {
			const config: Record<string, unknown> = {
				languages: ["eng", "deu", "fra"],
			};

			const result = configToJS(config as ExtractionConfig);

			expect(Array.isArray(result.languages)).toBe(true);
			expect(result.languages).toEqual(["eng", "deu", "fra"]);
		});

		it("should handle deeply nested structures", () => {
			const config: Record<string, unknown> = {
				level1: {
					level2: {
						level3: {
							value: "deep",
						},
					},
				},
			};

			const result = configToJS(config as ExtractionConfig);

			expect((result.level1 as Record<string, unknown>).level2).toBeDefined();
		});

		it("should remove empty objects", () => {
			const config: Record<string, unknown> = {
				ocr: { backend: "tesseract" },
				empty: {},
			};

			const result = configToJS(config as ExtractionConfig);

			expect(result).not.toHaveProperty("empty");
		});

		it("should preserve numeric values", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 1000,
					chunkOverlap: 100,
				},
			};

			const result = configToJS(config);
			const chunking = result.chunking as Record<string, unknown>;

			expect(chunking.maxChars).toBe(1000);
			expect(chunking.chunkOverlap).toBe(100);
		});

		it("should preserve boolean values", () => {
			const config: Record<string, unknown> = {
				images: {
					extractImages: true,
				},
			};

			const result = configToJS(config as ExtractionConfig);
			const images = result.images as Record<string, unknown>;

			expect(images.extractImages).toBe(true);
		});
	});

	describe("jsToExtractionResult", () => {
		it("should parse valid extraction result", () => {
			const jsValue = {
				content: "Hello world",
				mimeType: "text/plain",
				metadata: { pageCount: 1 },
				tables: [],
				detectedLanguages: ["en"],
			};

			const result = jsToExtractionResult(jsValue);

			expect(result.content).toBe("Hello world");
			expect(result.mimeType).toBe("text/plain");
		});

		it("should throw if value is not an object", () => {
			expect(() => jsToExtractionResult("string")).toThrow("not an object");
			expect(() => jsToExtractionResult(null)).toThrow("not an object");
			expect(() => jsToExtractionResult(undefined)).toThrow("not an object");
		});

		it("should throw if content is missing", () => {
			const jsValue = {
				mimeType: "text/plain",
				metadata: {},
			};

			expect(() => jsToExtractionResult(jsValue)).toThrow("missing or invalid content");
		});

		it("should throw if mimeType is missing", () => {
			const jsValue = {
				content: "Hello",
				metadata: {},
			};

			expect(() => jsToExtractionResult(jsValue)).toThrow("missing or invalid mimeType");
		});

		it("should throw if metadata is missing", () => {
			const jsValue = {
				content: "Hello",
				mimeType: "text/plain",
			};

			expect(() => jsToExtractionResult(jsValue)).toThrow("missing or invalid metadata");
		});

		it("should parse tables correctly", () => {
			const jsValue = {
				content: "test",
				mimeType: "application/pdf",
				metadata: {},
				tables: [
					{
						cells: [
							["a", "b"],
							["c", "d"],
						],
						markdown: "| a | b |\n| c | d |",
						pageNumber: 1,
					},
				],
				detectedLanguages: null,
			};

			const result = jsToExtractionResult(jsValue);

			expect(result.tables).toHaveLength(1);
			expect(result.tables[0].cells).toEqual([
				["a", "b"],
				["c", "d"],
			]);
		});

		it("should skip invalid tables", () => {
			const jsValue = {
				content: "test",
				mimeType: "application/pdf",
				metadata: {},
				tables: [
					{
						cells: [["a", "b"]],
						markdown: "valid",
						pageNumber: 1,
					},
					{
						cells: [[1, 2]],
						markdown: "invalid",
						pageNumber: 1,
					},
				],
			};

			const result = jsToExtractionResult(jsValue);

			expect(result.tables).toHaveLength(1);
		});

		it("should parse chunks with metadata", () => {
			const jsValue = {
				content: "test",
				mimeType: "application/pdf",
				metadata: {},
				chunks: [
					{
						content: "chunk 1",
						metadata: {
							charStart: 0,
							charEnd: 7,
							tokenCount: 2,
							chunkIndex: 0,
							totalChunks: 1,
						},
						embedding: null,
					},
				],
			};

			const result = jsToExtractionResult(jsValue);

			expect(result.chunks).toHaveLength(1);
			expect(result.chunks?.[0].content).toBe("chunk 1");
		});

		it("should parse images with validation", () => {
			const jsValue = {
				content: "test",
				mimeType: "application/pdf",
				metadata: {},
				images: [
					{
						data: new Uint8Array([1, 2, 3]),
						format: "png",
						imageIndex: 0,
						pageNumber: 1,
						width: 100,
						height: 100,
						colorspace: "RGB",
						bitsPerComponent: 8,
						isMask: false,
						description: "test image",
						ocrResult: null,
					},
				],
			};

			const result = jsToExtractionResult(jsValue);

			expect(result.images).toHaveLength(1);
			expect(result.images?.[0].format).toBe("png");
		});

		it("should handle detectedLanguages", () => {
			const jsValue = {
				content: "Bonjour",
				mimeType: "text/plain",
				metadata: {},
				detectedLanguages: ["fr", "en"],
			};

			const result = jsToExtractionResult(jsValue);

			expect(result.detectedLanguages).toEqual(["fr", "en"]);
		});

		it("should throw if chunk has invalid metadata", () => {
			const jsValue = {
				content: "test",
				mimeType: "application/pdf",
				metadata: {},
				chunks: [
					{
						content: "chunk",
						metadata: {
							charStart: "not a number",
							charEnd: 10,
							tokenCount: null,
							chunkIndex: 0,
							totalChunks: 1,
						},
					},
				],
			};

			expect(() => jsToExtractionResult(jsValue)).toThrow("charStart must be a number");
		});

		it("should throw if image data is not Uint8Array", () => {
			const jsValue = {
				content: "test",
				mimeType: "application/pdf",
				metadata: {},
				images: [
					{
						data: "not bytes",
						format: "png",
						imageIndex: 0,
						pageNumber: 1,
						width: 100,
						height: 100,
						bitsPerComponent: 8,
						isMask: false,
					},
				],
			};

			expect(() => jsToExtractionResult(jsValue)).toThrow("data must be Uint8Array");
		});

		it("should validate detectedLanguages are strings", () => {
			const jsValue = {
				content: "test",
				mimeType: "text/plain",
				metadata: {},
				detectedLanguages: ["en", 123],
			};

			expect(() => jsToExtractionResult(jsValue)).toThrow("detectedLanguages must contain only strings");
		});
	});

	describe("wrapWasmError", () => {
		it("should wrap Error objects", () => {
			const original = new Error("Test error");
			const wrapped = wrapWasmError(original, "testing");

			expect(wrapped).toBeInstanceOf(Error);
			expect(wrapped.message).toContain("Error testing");
			expect(wrapped.message).toContain("Test error");
		});

		it("should wrap string errors", () => {
			const wrapped = wrapWasmError("String error", "context");

			expect(wrapped).toBeInstanceOf(Error);
			expect(wrapped.message).toContain("String error");
		});

		it("should include context in message", () => {
			const original = new Error("Original");
			const wrapped = wrapWasmError(original, "extracting from bytes");

			expect(wrapped.message).toContain("extracting from bytes");
		});

		it("should preserve original error as cause", () => {
			const original = new Error("Original message");
			const wrapped = wrapWasmError(original, "context");

			expect(wrapped.cause).toBe(original);
		});

		it("should handle unknown error types", () => {
			const wrapped = wrapWasmError({}, "context");

			expect(wrapped).toBeInstanceOf(Error);
			expect(wrapped.message).toContain("context");
		});
	});

	describe("isValidExtractionResult", () => {
		it("should return true for valid result", () => {
			const result = {
				content: "test",
				mimeType: "text/plain",
				metadata: {},
				tables: [],
			};

			expect(isValidExtractionResult(result)).toBe(true);
		});

		it("should return false if not an object", () => {
			expect(isValidExtractionResult("string")).toBe(false);
			expect(isValidExtractionResult(null)).toBe(false);
			expect(isValidExtractionResult(123)).toBe(false);
		});

		it("should return false if content is not string", () => {
			const result = {
				content: 123,
				mimeType: "text/plain",
				metadata: {},
				tables: [],
			};

			expect(isValidExtractionResult(result)).toBe(false);
		});

		it("should return false if mimeType is not string", () => {
			const result = {
				content: "test",
				mimeType: 123,
				metadata: {},
				tables: [],
			};

			expect(isValidExtractionResult(result)).toBe(false);
		});

		it("should return false if metadata is not object", () => {
			const result = {
				content: "test",
				mimeType: "text/plain",
				metadata: null,
				tables: [],
			};

			expect(isValidExtractionResult(result)).toBe(false);
		});

		it("should return false if tables is not array", () => {
			const result = {
				content: "test",
				mimeType: "text/plain",
				metadata: {},
				tables: "not array",
			};

			expect(isValidExtractionResult(result)).toBe(false);
		});

		it("should return true even if optional fields missing", () => {
			const result = {
				content: "test",
				mimeType: "text/plain",
				metadata: {},
				tables: [],
			};

			expect(isValidExtractionResult(result)).toBe(true);
		});
	});
});
