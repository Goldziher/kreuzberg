/**
 * Unit tests for OCR backend registration.
 *
 * These tests verify that OCR backends can be registered and that the
 * registration process validates inputs correctly.
 */

import { afterEach, describe, expect, it, vi } from "vitest";
import { __resetBindingForTests, __setBindingForTests, registerOcrBackend } from "../../dist/index.js";
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
			initialize: async () => {},
			shutdown: async () => {},
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

		const testBytes = new Uint8Array([0xff, 0xd8, 0xff]);
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

describe("OCR backend bridge wiring", () => {
	const createOcrBinding = () => ({
		registerOcrBackend: vi.fn(),
	});

	afterEach(() => {
		__resetBindingForTests();
		delete process.env.KREUZBERG_DEBUG_GUTEN;
		vi.restoreAllMocks();
	});

	it("should unwrap tuple payloads and forward bytes", async () => {
		const mockBinding = createOcrBinding();
		__setBindingForTests(mockBinding);

		const processSpy = vi.fn().mockResolvedValue({
			content: "tuple result",
			mime_type: "text/plain",
			metadata: {},
			tables: [],
		});

		const backend: OcrBackendProtocol = {
			name: () => "tuple-ocr",
			supportedLanguages: () => ["en"],
			processImage: processSpy,
		};

		registerOcrBackend(backend);

		const wrapped = mockBinding.registerOcrBackend.mock.calls[0][0];
		const buffer = Buffer.from("abc");

		await wrapped.processImage([[buffer, "en"]]);
		await wrapped.processImage([buffer, "de"]);

		expect(processSpy).toHaveBeenCalledWith(expect.any(Uint8Array), "en");
		expect(processSpy).toHaveBeenCalledWith(expect.any(Uint8Array), "de");
	});

	it("should decode base64 payloads and require language", async () => {
		const mockBinding = createOcrBinding();
		__setBindingForTests(mockBinding);

		const backend: OcrBackendProtocol = {
			name: () => "base64-ocr",
			supportedLanguages: () => ["en"],
			processImage: vi.fn().mockResolvedValue({
				content: "ok",
				mime_type: "text/plain",
				metadata: {},
				tables: [],
			}),
		};

		registerOcrBackend(backend);
		const wrapped = mockBinding.registerOcrBackend.mock.calls[0][0];

		const payload = Buffer.from("debug").toString("base64");
		await wrapped.processImage(payload, "en");

		await expect(wrapped.processImage(Buffer.from("raw-bytes"))).rejects.toThrow(/language parameter/);
	});

	it("should log debug information when enabled", async () => {
		const mockBinding = createOcrBinding();
		__setBindingForTests(mockBinding);
		process.env.KREUZBERG_DEBUG_GUTEN = "1";

		const backend: OcrBackendProtocol = {
			name: () => "debug-ocr",
			supportedLanguages: () => ["en"],
			processImage: vi.fn().mockResolvedValue({
				content: "ok",
				mime_type: "text/plain",
				metadata: {},
				tables: [],
			}),
		};

		registerOcrBackend(backend);
		const wrapped = mockBinding.registerOcrBackend.mock.calls[0][0];

		const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
		await wrapped.processImage(Buffer.from("header-test"), "en");
		expect(consoleSpy).toHaveBeenCalled();
	});

	it("should describe string payloads when debug logging is enabled", async () => {
		const mockBinding = createOcrBinding();
		__setBindingForTests(mockBinding);
		process.env.KREUZBERG_DEBUG_GUTEN = "1";

		const processSpy = vi.fn().mockResolvedValue({
			content: "tuple string content",
			mime_type: "text/plain",
			metadata: {},
			tables: [],
		});

		const backend: OcrBackendProtocol = {
			name: () => "tuple-string-ocr",
			supportedLanguages: () => ["fr"],
			processImage: processSpy,
		};

		registerOcrBackend(backend);
		const wrapped = mockBinding.registerOcrBackend.mock.calls[0][0];

		const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
		const base64Payload = Buffer.from("tuple-log").toString("base64");

		await wrapped.processImage([[base64Payload, "fr"]]);

		expect(processSpy).toHaveBeenCalledWith(expect.any(Uint8Array), "fr");

		const receivedPayloadLog = consoleSpy.mock.calls.find(
			(args) => args[0] === "[registerOcrBackend] Received payload",
		);
		expect(receivedPayloadLog).toBeDefined();
		expect(receivedPayloadLog?.[2]).toBe("ctor");
		expect(receivedPayloadLog?.[3]).toBe("String");
		expect(receivedPayloadLog?.[4]).toBe("length");
		expect(receivedPayloadLog?.[5]).toBe(base64Payload.length);
		consoleSpy.mockRestore();
	});
});
