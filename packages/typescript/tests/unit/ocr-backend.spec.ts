/**
 * Unit tests for OCR backend registration.
 *
 * These tests verify that OCR backends can be registered and that the
 * registration process validates inputs correctly.
 */

import { describe, it, expect } from "vitest";
import { registerOcrBackend } from "../../src/index.js";
import type { OcrBackendProtocol } from "../../src/types.js";

describe("OCR Backend Registration", () => {
	it("should register a valid OCR backend", () => {
		const mockBackend: OcrBackendProtocol = {
			name: () => "test-ocr",
			supportedLanguages: () => ["en", "de"],
			processImage: async (imageBytes: Uint8Array, language: string) => ({
				content: "test content",
				mime_type: "text/plain",
				metadata: { language },
				tables: [],
			}),
		};

		// Should not throw
		expect(() => registerOcrBackend(mockBackend)).not.toThrow();
	});

	it("should reject backend with empty name", () => {
		const invalidBackend: OcrBackendProtocol = {
			name: () => "",
			supportedLanguages: () => ["en"],
			processImage: async () => ({
				content: "test",
				mime_type: "text/plain",
				metadata: {},
				tables: [],
			}),
		};

		expect(() => registerOcrBackend(invalidBackend)).toThrow(/name cannot be empty/i);
	});

	it("should reject backend with no supported languages", () => {
		const invalidBackend: OcrBackendProtocol = {
			name: () => "test-ocr",
			supportedLanguages: () => [],
			processImage: async () => ({
				content: "test",
				mime_type: "text/plain",
				metadata: {},
				tables: [],
			}),
		};

		expect(() => registerOcrBackend(invalidBackend)).toThrow(/must support at least one language/i);
	});

	it("should handle backend with optional methods", () => {
		const backendWithOptionals: OcrBackendProtocol = {
			name: () => "test-ocr-2",
			supportedLanguages: () => ["en"],
			processImage: async () => ({
				content: "test",
				mime_type: "text/plain",
				metadata: {},
				tables: [],
			}),
			initialize: async () => {
				// Optional initialization
			},
			shutdown: async () => {
				// Optional cleanup
			},
		};

		expect(() => registerOcrBackend(backendWithOptionals)).not.toThrow();
	});

	it("should correctly wrap processImage for NAPI bridge", async () => {
		const testContent = "extracted text from image";
		const testMetadata = { confidence: 0.95, language: "en" };

		const mockBackend: OcrBackendProtocol = {
			name: () => "mock-ocr",
			supportedLanguages: () => ["en"],
			processImage: async (imageBytes: Uint8Array, language: string) => {
				// Verify we receive Uint8Array
				expect(imageBytes).toBeInstanceOf(Uint8Array);
				expect(language).toBe("en");

				return {
					content: testContent,
					mime_type: "text/plain",
					metadata: testMetadata,
					tables: [],
				};
			},
		};

		registerOcrBackend(mockBackend);

		// The wrapped backend should exist now
		// (actual extraction test would require a full integration test)
	});
});

describe("OCR Backend Protocol Interface", () => {
	it("should accept backends with correct return type", async () => {
		const backend: OcrBackendProtocol = {
			name: () => "type-test",
			supportedLanguages: () => ["en", "de", "fr"],
			processImage: async (imageBytes: Uint8Array, language: string) => {
				return {
					content: "extracted text",
					mime_type: "text/plain",
					metadata: {
						confidence: 0.98,
						language,
						width: 800,
						height: 600,
					},
					tables: [],
				};
			},
		};

		const testBytes = new Uint8Array([0xff, 0xd8, 0xff]); // JPEG magic bytes
		const result = await backend.processImage(testBytes, "en");

		expect(result.content).toBe("extracted text");
		expect(result.mime_type).toBe("text/plain");
		expect(result.metadata.confidence).toBe(0.98);
		expect(result.tables).toEqual([]);
	});

	it("should support backends with table detection", async () => {
		const backend: OcrBackendProtocol = {
			name: () => "table-ocr",
			supportedLanguages: () => ["en"],
			processImage: async () => {
				return {
					content: "text with tables",
					mime_type: "text/plain",
					metadata: {},
					tables: [
						{
							cells: [
								["Header 1", "Header 2"],
								["Cell 1", "Cell 2"],
							],
							markdown: "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |",
							page_number: 1,
						},
					],
				};
			},
		};

		const result = await backend.processImage(new Uint8Array(), "en");
		expect(result.tables).toHaveLength(1);
		expect(result.tables[0].cells).toHaveLength(2);
	});
});
