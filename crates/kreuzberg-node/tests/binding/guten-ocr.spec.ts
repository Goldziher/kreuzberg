import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Valid minimal 1x1 PNG image for testing
// This is a complete, valid PNG file that can be processed by image libraries
const validMinimalPng = new Uint8Array([
	0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
	0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49,
	0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00,
	0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
]);

describe("GutenOcrBackend", () => {
	describe("Constructor and basic properties", async () => {
		it("should create instance without options", async () => {
			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();
			expect(backend).toBeInstanceOf(GutenOcrBackend);
		});

		it("should create instance with custom options", async () => {
			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend({
				models: {
					detectionPath: "/custom/detection.onnx",
					recognitionPath: "/custom/recognition.onnx",
					dictionaryPath: "/custom/dict.txt",
				},
				isDebug: true,
				debugOutputDir: "/debug",
			});
			expect(backend).toBeInstanceOf(GutenOcrBackend);
		});

		it("should return correct backend name", async () => {
			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();
			expect(backend.name()).toBe("guten-ocr");
		});

		it("should return supported languages", async () => {
			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();
			const languages = backend.supportedLanguages();
			expect(languages).toContain("en");
			expect(languages).toContain("eng");
			expect(languages).toContain("ch_sim");
			expect(languages).toContain("ch_tra");
			expect(languages).toContain("chinese");
		});
	});

	describe("Initialization", () => {
		let mockOcrModule: any;
		let mockOcrInstance: any;

		beforeEach(() => {
			mockOcrInstance = {
				detect: vi.fn(),
			};
			mockOcrModule = {
				create: vi.fn().mockResolvedValue(mockOcrInstance),
			};
		});

		afterEach(() => {
			vi.unmock("@gutenye/ocr-node");
			vi.unmock("sharp");
		});

		it("should initialize successfully", async () => {
			vi.doMock("@gutenye/ocr-node", () => ({
				default: mockOcrModule,
			}));
			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			await backend.initialize();
		});

		it("should throw error if @gutenye/ocr-node is not installed", async () => {
			vi.doMock("@gutenye/ocr-node", () => {
				throw new Error("MODULE_NOT_FOUND");
			});

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			await expect(backend.initialize()).rejects.toThrow(/requires the '@gutenye\/ocr-node' package/);
		});

		it("should throw error if OCR creation fails", async () => {
			const failingModule = {
				create: vi.fn().mockRejectedValue(new Error("Creation failed")),
			};

			vi.doMock("@gutenye/ocr-node", () => ({
				default: failingModule,
			}));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			await expect(backend.initialize()).rejects.toThrow(/Failed to initialize Guten OCR/);
		});

		it("should not reinitialize if already initialized", async () => {
			vi.doMock("@gutenye/ocr-node", () => ({
				default: mockOcrModule,
			}));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			await backend.initialize();
			const firstCallCount = mockOcrModule.create.mock.calls.length;

			await backend.initialize();
			expect(mockOcrModule.create).toHaveBeenCalledTimes(firstCallCount);
		});

		it("should pass options to OCR create", async () => {
			const options = {
				models: {
					detectionPath: "/custom/detection.onnx",
					recognitionPath: "/custom/recognition.onnx",
					dictionaryPath: "/custom/dict.txt",
				},
				isDebug: true,
			};

			vi.doMock("@gutenye/ocr-node", () => ({
				default: mockOcrModule,
			}));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend(options);

			await backend.initialize();
			expect(mockOcrModule.create).toHaveBeenCalledWith(options);
		});
	});

	describe("Shutdown", () => {
		it("should cleanup resources", async () => {
			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();
			await backend.shutdown();
		});
	});

	describe("processImage", () => {
		let mockOcrInstance: any;
		let mockOcrModule: any;
		let mockSharp: any;

		beforeEach(() => {
			mockOcrInstance = {
				detect: vi.fn().mockResolvedValue([
					{
						text: "Hello",
						mean: 0.95,
						box: [
							[0, 0],
							[100, 0],
							[100, 20],
							[0, 20],
						],
					},
					{
						text: "World",
						mean: 0.9,
						box: [
							[0, 25],
							[100, 25],
							[100, 45],
							[0, 45],
						],
					},
				]),
			};

			mockOcrModule = {
				create: vi.fn().mockResolvedValue(mockOcrInstance),
			};

			const mockImageInstance = {
				metadata: vi.fn().mockResolvedValue({
					width: 800,
					height: 600,
					format: "png",
				}),
				raw: vi.fn().mockReturnThis(),
				toBuffer: vi.fn().mockResolvedValue({
					data: Buffer.alloc(800 * 600 * 3),
					info: {
						width: 800,
						height: 600,
						channels: 3,
					},
				}),
			};

			mockSharp = vi.fn().mockReturnValue(mockImageInstance);
		});

		afterEach(() => {
			vi.unmock("@gutenye/ocr-node");
			vi.unmock("sharp");
		});

		it("should process image successfully", async () => {
			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			const result = await backend.processImage(validMinimalPng, "en");

			expect(result.content).toBe("Hello\nWorld");
			expect(result.mime_type).toBe("text/plain");
			expect(result.metadata.width).toBe(800);
			expect(result.metadata.height).toBe(600);
			expect(result.metadata.confidence).toBeCloseTo(0.925);
			expect(result.metadata.text_regions).toBe(2);
			expect(result.metadata.language).toBe("en");
			expect(result.tables).toEqual([]);
		});

		it("should auto-initialize if not initialized", async () => {
			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			await backend.processImage(validMinimalPng, "en");

			expect(mockOcrModule.create).toHaveBeenCalled();
		});

		it("should handle empty text detection", async () => {
			mockOcrInstance.detect.mockResolvedValue([]);

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			const result = await backend.processImage(validMinimalPng, "en");

			expect(result.content).toBe("");
			expect(result.metadata.confidence).toBe(0);
			expect(result.metadata.text_regions).toBe(0);
		});

		it("should calculate average confidence correctly", async () => {
			mockOcrInstance.detect.mockResolvedValue([
				{
					text: "One",
					mean: 1.0,
					box: [
						[0, 0],
						[100, 0],
						[100, 20],
						[0, 20],
					],
				},
				{
					text: "Two",
					mean: 0.8,
					box: [
						[0, 25],
						[100, 25],
						[100, 45],
						[0, 45],
					],
				},
				{
					text: "Three",
					mean: 0.7,
					box: [
						[0, 50],
						[100, 50],
						[100, 70],
						[0, 70],
					],
				},
			]);

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			const result = await backend.processImage(validMinimalPng, "en");

			expect(result.metadata.confidence).toBeCloseTo(0.8333, 3);
		});

		it.skip("should throw error if initialization fails during processImage", async () => {
			vi.doMock("@gutenye/ocr-node", () => {
				throw new Error("MODULE_NOT_FOUND");
			});

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			await expect(backend.processImage(validMinimalPng, "en")).rejects.toThrow();
		});

		it.skip("should throw error if OCR detection fails", async () => {
			mockOcrInstance.detect.mockRejectedValue(new Error("Detection failed"));

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			await expect(backend.processImage(validMinimalPng, "en")).rejects.toThrow(/Guten OCR processing failed/);
		});

		it.skip("should continue if sharp processing fails", async () => {
			const failingSharp = vi.fn().mockImplementation(() => {
				throw new Error("Invalid image");
			});

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: failingSharp }));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			const result = await backend.processImage(validMinimalPng, "en");

			expect(result.metadata.width).toBe(0);
			expect(result.metadata.height).toBe(0);
			expect(result.content).toContain("Hello");
		});

		it.skip("should throw error if OCR instance is null after initialization", async () => {
			const nullModule = {
				create: vi.fn().mockResolvedValue(null),
			};

			vi.doMock("@gutenye/ocr-node", () => ({ default: nullModule }));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();

			await expect(backend.processImage(validMinimalPng, "en")).rejects.toThrow(
				/Guten OCR backend failed to initialize/,
			);
		});

		it.skip("should emit debug logs when KREUZBERG_DEBUG_GUTEN is enabled", async () => {
			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({
				default: vi.fn().mockReturnValue({
					metadata: vi.fn().mockResolvedValue({ width: 1, height: 1 }),
				}),
			}));

			const { GutenOcrBackend } = await import("../../dist/index.js");
			const backend = new GutenOcrBackend();
			(backend as any).ocr = {
				detect: vi.fn().mockResolvedValue([
					{
						text: "debug",
						mean: 0.5,
						box: [
							[0, 0],
							[1, 0],
							[1, 1],
							[0, 1],
						],
					},
				]),
			};

			process.env.KREUZBERG_DEBUG_GUTEN = "1";
			const logSpy = vi.spyOn(console, "log").mockImplementation(() => {});

			await backend.processImage(Buffer.from("header-bytes"), "en");

			expect(logSpy).toHaveBeenCalledWith(
				expect.stringContaining("[Guten OCR] Debug input header:"),
				expect.any(Array),
			);
			expect(logSpy).toHaveBeenCalledWith(
				expect.stringContaining("[Guten OCR] Buffer?"),
				expect.any(Boolean),
				"constructor",
				expect.any(String),
				"length",
				expect.any(Number),
				"type",
				expect.any(String),
			);

			process.env.KREUZBERG_DEBUG_GUTEN = undefined;
			logSpy.mockRestore();
		});
	});
});
