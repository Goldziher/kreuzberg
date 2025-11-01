import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GutenOcrBackend } from "../../src/ocr/guten-ocr.js";

describe("GutenOcrBackend", () => {
	describe("Constructor and basic properties", () => {
		it("should create instance without options", () => {
			const backend = new GutenOcrBackend();
			expect(backend).toBeInstanceOf(GutenOcrBackend);
		});

		it("should create instance with custom options", () => {
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

		it("should return correct backend name", () => {
			const backend = new GutenOcrBackend();
			expect(backend.name()).toBe("guten-ocr");
		});

		it("should return supported languages", () => {
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
			vi.resetModules();
			mockOcrInstance = {
				detect: vi.fn(),
			};
			mockOcrModule = {
				create: vi.fn().mockResolvedValue(mockOcrInstance),
			};
		});

		afterEach(() => {
			vi.unstubAllGlobals();
		});

		it("should initialize successfully", async () => {
			const backend = new GutenOcrBackend();

			// Mock the dynamic import
			vi.doMock("@gutenye/ocr-node", () => ({
				default: mockOcrModule,
			}));

			await backend.initialize();
			// Should succeed without throwing
		});

		it("should throw error if @gutenye/ocr-node is not installed", async () => {
			const backend = new GutenOcrBackend();

			// Mock import failure
			vi.doMock("@gutenye/ocr-node", () => {
				throw new Error("MODULE_NOT_FOUND");
			});

			await expect(backend.initialize()).rejects.toThrow(/requires the '@gutenye\/ocr-node' package/);
		});

		it("should throw error if OCR creation fails", async () => {
			const backend = new GutenOcrBackend();

			const failingModule = {
				create: vi.fn().mockRejectedValue(new Error("Creation failed")),
			};

			vi.doMock("@gutenye/ocr-node", () => ({
				default: failingModule,
			}));

			await expect(backend.initialize()).rejects.toThrow(/Failed to initialize Guten OCR/);
		});

		it("should not reinitialize if already initialized", async () => {
			const backend = new GutenOcrBackend();

			vi.doMock("@gutenye/ocr-node", () => ({
				default: mockOcrModule,
			}));

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
			const backend = new GutenOcrBackend(options);

			vi.doMock("@gutenye/ocr-node", () => ({
				default: mockOcrModule,
			}));

			await backend.initialize();
			expect(mockOcrModule.create).toHaveBeenCalledWith(options);
		});
	});

	describe("Shutdown", () => {
		it("should cleanup resources", async () => {
			const backend = new GutenOcrBackend();
			await backend.shutdown();
			// Should not throw
		});
	});

	describe("processImage", () => {
		let mockOcrInstance: any;
		let mockOcrModule: any;
		let mockSharp: any;

		beforeEach(() => {
			vi.resetModules();

			// Mock OCR result
			mockOcrInstance = {
				detect: vi.fn().mockResolvedValue({
					texts: [
						{ text: "Hello", score: 0.95, frame: { top: 0, left: 0, width: 100, height: 20 } },
						{ text: "World", score: 0.9, frame: { top: 25, left: 0, width: 100, height: 20 } },
					],
					resizedImageWidth: 800,
					resizedImageHeight: 600,
				}),
			};

			mockOcrModule = {
				create: vi.fn().mockResolvedValue(mockOcrInstance),
			};

			// Mock sharp
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
			vi.unstubAllGlobals();
		});

		it("should process image successfully", async () => {
			const backend = new GutenOcrBackend();

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const imageBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]); // PNG header
			const result = await backend.processImage(imageBytes, "en");

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
			const backend = new GutenOcrBackend();

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const imageBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
			await backend.processImage(imageBytes, "en");

			expect(mockOcrModule.create).toHaveBeenCalled();
		});

		it("should handle empty text detection", async () => {
			mockOcrInstance.detect.mockResolvedValue({
				texts: [],
				resizedImageWidth: 800,
				resizedImageHeight: 600,
			});

			const backend = new GutenOcrBackend();

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const imageBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
			const result = await backend.processImage(imageBytes, "en");

			expect(result.content).toBe("");
			expect(result.metadata.confidence).toBe(0);
			expect(result.metadata.text_regions).toBe(0);
		});

		it("should calculate average confidence correctly", async () => {
			mockOcrInstance.detect.mockResolvedValue({
				texts: [
					{ text: "One", score: 1.0, frame: { top: 0, left: 0, width: 100, height: 20 } },
					{ text: "Two", score: 0.8, frame: { top: 25, left: 0, width: 100, height: 20 } },
					{ text: "Three", score: 0.7, frame: { top: 50, left: 0, width: 100, height: 20 } },
				],
				resizedImageWidth: 800,
				resizedImageHeight: 600,
			});

			const backend = new GutenOcrBackend();

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const imageBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
			const result = await backend.processImage(imageBytes, "en");

			expect(result.metadata.confidence).toBeCloseTo(0.8333, 3);
		});

		it("should throw error if initialization fails during processImage", async () => {
			const backend = new GutenOcrBackend();

			vi.doMock("@gutenye/ocr-node", () => {
				throw new Error("MODULE_NOT_FOUND");
			});

			const imageBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
			await expect(backend.processImage(imageBytes, "en")).rejects.toThrow();
		});

		it("should throw error if OCR detection fails", async () => {
			mockOcrInstance.detect.mockRejectedValue(new Error("Detection failed"));

			const backend = new GutenOcrBackend();

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: mockSharp }));

			const imageBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
			await expect(backend.processImage(imageBytes, "en")).rejects.toThrow(/Guten OCR processing failed/);
		});

		it("should throw error if sharp processing fails", async () => {
			const failingSharp = vi.fn().mockImplementation(() => {
				throw new Error("Invalid image");
			});

			const backend = new GutenOcrBackend();

			vi.doMock("@gutenye/ocr-node", () => ({ default: mockOcrModule }));
			vi.doMock("sharp", () => ({ default: failingSharp }));

			const imageBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
			await expect(backend.processImage(imageBytes, "en")).rejects.toThrow(/Guten OCR processing failed/);
		});

		it("should throw error if OCR instance is null after initialization", async () => {
			const nullModule = {
				create: vi.fn().mockResolvedValue(null),
			};

			const backend = new GutenOcrBackend();

			vi.doMock("@gutenye/ocr-node", () => ({ default: nullModule }));

			const imageBytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
			await expect(backend.processImage(imageBytes, "en")).rejects.toThrow(/Guten OCR backend failed to initialize/);
		});
	});
});
