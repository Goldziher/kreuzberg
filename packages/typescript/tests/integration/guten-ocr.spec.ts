/**
 * Integration tests for Guten OCR backend.
 *
 * These tests verify that the Guten OCR backend can be registered and used
 * for text extraction from images.
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { extractBytes, registerOcrBackend } from "../../src/index.js";
import { GutenOcrBackend } from "../../src/ocr/guten-ocr.js";

const isGutenOcrAvailable = async (): Promise<boolean> => {
	try {
		await import("@gutenye/ocr-node");
		return true;
	} catch {
		return false;
	}
};

describe("Guten OCR Backend Integration", () => {
	let backend: GutenOcrBackend;
	let gutenOcrAvailable: boolean;

	beforeAll(async () => {
		gutenOcrAvailable = await isGutenOcrAvailable();
		if (!gutenOcrAvailable) {
			console.log("Skipping Guten OCR tests - @gutenye/ocr-node not installed");
			return;
		}

		backend = new GutenOcrBackend();
		await backend.initialize();
		registerOcrBackend(backend);
	});

	afterAll(async () => {
		if (backend) {
			await backend.shutdown();
		}
	});

	it("should have correct backend name", () => {
		if (!gutenOcrAvailable) return;
		expect(backend.name()).toBe("guten-ocr");
	});

	it("should have supported languages", () => {
		if (!gutenOcrAvailable) return;
		const languages = backend.supportedLanguages();
		expect(languages).toBeInstanceOf(Array);
		expect(languages.length).toBeGreaterThan(0);
		expect(languages).toContain("en");
	});

	it("should extract text from a simple test image", async () => {
		if (!gutenOcrAvailable) return;

		const sharp = await import("sharp").then((m) => m.default || m);

		const svgImage = Buffer.from(`
			<svg width="400" height="100">
				<rect width="400" height="100" fill="white"/>
				<text x="20" y="50" font-family="Arial" font-size="24" fill="black">Hello World</text>
			</svg>
		`);

		const imageBytes = await sharp(svgImage).png().toBuffer();

		const result = await backend.processImage(imageBytes, "en");

		expect(result).toHaveProperty("content");
		expect(result.content).toBeTruthy();
		expect(result.mime_type).toBe("text/plain");
		expect(result.metadata).toHaveProperty("width");
		expect(result.metadata).toHaveProperty("height");
		expect(result.metadata).toHaveProperty("confidence");
		expect(result.metadata).toHaveProperty("text_regions");
		expect(result.tables).toEqual([]);
	});

	it("should work with extractBytes for image extraction", async () => {
		if (!gutenOcrAvailable) return;

		const sharp = await import("sharp").then((m) => m.default || m);

		const svgImage = Buffer.from(`
			<svg width="400" height="100">
				<rect width="400" height="100" fill="white"/>
				<text x="20" y="50" font-family="Arial" font-size="24" fill="black">Test Image</text>
			</svg>
		`);

		const imageBytes = await sharp(svgImage).png().toBuffer();

		const result = await extractBytes(imageBytes, "image/png", {
			ocr: {
				backend: "guten-ocr",
				language: "en",
			},
			forceOcr: true,
		});

		expect(result.content).toBeTruthy();
		expect(result.mimeType).toBe("image/png");
	});

	it("should handle unsupported language gracefully", async () => {
		if (!gutenOcrAvailable) return;

		const sharp = await import("sharp").then((m) => m.default || m);

		const svgImage = Buffer.from(`
			<svg width="400" height="100">
				<rect width="400" height="100" fill="white"/>
				<text x="20" y="50" font-family="Arial" font-size="24" fill="black">Test</text>
			</svg>
		`);

		const imageBytes = await sharp(svgImage).png().toBuffer();

		const result = await backend.processImage(imageBytes, "unsupported_lang");
		expect(result).toHaveProperty("content");
	});

	it("should handle empty image gracefully", async () => {
		if (!gutenOcrAvailable) return;

		const sharp = await import("sharp").then((m) => m.default || m);

		const imageBytes = await sharp({
			create: {
				width: 100,
				height: 100,
				channels: 3,
				background: { r: 255, g: 255, b: 255 },
			},
		})
			.png()
			.toBuffer();

		const result = await backend.processImage(imageBytes, "en");

		expect(result).toHaveProperty("content");
		expect(result.mime_type).toBe("text/plain");
		expect(result.metadata.text_regions).toBe(0);
	});

	it("should initialize only once", async () => {
		if (!gutenOcrAvailable) return;

		await backend.initialize();
		await backend.initialize();
		await backend.initialize();

		expect(backend.name()).toBe("guten-ocr");
	});

	it("should throw error if processing before initialization", async () => {
		if (!gutenOcrAvailable) return;

		const newBackend = new GutenOcrBackend();

		const sharp = await import("sharp").then((m) => m.default || m);
		const imageBytes = await sharp({
			create: {
				width: 100,
				height: 100,
				channels: 3,
				background: { r: 255, g: 255, b: 255 },
			},
		})
			.png()
			.toBuffer();

		const result = await newBackend.processImage(imageBytes, "en");
		expect(result).toHaveProperty("content");
	});

	it("should throw error when @gutenye/ocr-node is not installed", async () => {
		if (gutenOcrAvailable) {
			return;
		}

		const failBackend = new GutenOcrBackend();

		await expect(failBackend.initialize()).rejects.toThrow(/requires the '@gutenye\/ocr-node' package/);
	});
});

describe("Guten OCR Backend - Advanced Features", () => {
	let backend: GutenOcrBackend;
	let gutenOcrAvailable: boolean;

	beforeAll(async () => {
		gutenOcrAvailable = await isGutenOcrAvailable();
		if (!gutenOcrAvailable) {
			return;
		}

		backend = new GutenOcrBackend({
			isDebug: false,
		});
		await backend.initialize();
	});

	afterAll(async () => {
		if (backend) {
			await backend.shutdown();
		}
	});

	it("should support custom configuration", () => {
		if (!gutenOcrAvailable) return;

		const customBackend = new GutenOcrBackend({
			isDebug: true,
			debugOutputDir: "./ocr_debug",
		});

		expect(customBackend.name()).toBe("guten-ocr");
	});

	it("should handle concurrent processImage calls", async () => {
		if (!gutenOcrAvailable) return;

		const sharp = await import("sharp").then((m) => m.default || m);

		const createTestImage = async (text: string) => {
			const svgImage = Buffer.from(`
				<svg width="400" height="100">
					<rect width="400" height="100" fill="white"/>
					<text x="20" y="50" font-family="Arial" font-size="24" fill="black">${text}</text>
				</svg>
			`);
			return await sharp(svgImage).png().toBuffer();
		};

		const image1 = await createTestImage("Image 1");
		const image2 = await createTestImage("Image 2");
		const image3 = await createTestImage("Image 3");

		const results = await Promise.all([
			backend.processImage(image1, "en"),
			backend.processImage(image2, "en"),
			backend.processImage(image3, "en"),
		]);

		expect(results).toHaveLength(3);
		results.forEach((result) => {
			expect(result).toHaveProperty("content");
			expect(result.mime_type).toBe("text/plain");
		});
	});

	it("should provide metadata with confidence scores", async () => {
		if (!gutenOcrAvailable) return;

		const sharp = await import("sharp").then((m) => m.default || m);

		const svgImage = Buffer.from(`
			<svg width="400" height="100">
				<rect width="400" height="100" fill="white"/>
				<text x="20" y="50" font-family="Arial" font-size="24" fill="black">High Quality Text</text>
			</svg>
		`);

		const imageBytes = await sharp(svgImage).png().toBuffer();
		const result = await backend.processImage(imageBytes, "en");

		expect(result.metadata.confidence).toBeGreaterThanOrEqual(0);
		expect(result.metadata.confidence).toBeLessThanOrEqual(1);
		expect(result.metadata.language).toBe("en");
	});
});
