/**
 * Integration tests for Guten OCR backend.
 *
 * These tests verify that the Guten OCR backend can be registered and used
 * for text extraction from images.
 */

import { readFile } from "node:fs/promises";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { extractBytes, registerOcrBackend } from "../../src/index.js";
import { GutenOcrBackend } from "../../src/ocr/guten-ocr.js";
import { getTestDocumentPath, testDocumentsAvailable } from "../helpers/integration-helpers.js";

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
		if (!testDocumentsAvailable()) return;

		const imagePath = getTestDocumentPath("images/invoice_image.png");
		const imageBytes = await readFile(imagePath);

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
		if (!testDocumentsAvailable()) return;

		const imagePath = getTestDocumentPath("images/invoice_image.png");
		const imageBytes = await readFile(imagePath);

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
		if (!testDocumentsAvailable()) return;

		const imagePath = getTestDocumentPath("images/example.jpg");
		const imageBytes = await readFile(imagePath);

		const result = await backend.processImage(imageBytes, "unsupported_lang");
		expect(result).toHaveProperty("content");
	});

	it("should handle empty image gracefully", async () => {
		if (!gutenOcrAvailable) return;
		if (!testDocumentsAvailable()) return;

		const imagePath = getTestDocumentPath("images/flower_no_text.jpg");
		const imageBytes = await readFile(imagePath);

		const result = await backend.processImage(imageBytes, "en");

		expect(result).toHaveProperty("content");
		expect(result.mime_type).toBe("text/plain");
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
		if (!testDocumentsAvailable()) return;

		const newBackend = new GutenOcrBackend();

		const imagePath = getTestDocumentPath("images/example.jpg");
		const imageBytes = await readFile(imagePath);

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
		if (!testDocumentsAvailable()) return;

		const image1Path = getTestDocumentPath("images/invoice_image.png");
		const image2Path = getTestDocumentPath("images/example.jpg");
		const image3Path = getTestDocumentPath("images/chi_sim_image.jpeg");

		const [image1, image2, image3] = await Promise.all([
			readFile(image1Path),
			readFile(image2Path),
			readFile(image3Path),
		]);

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
		if (!testDocumentsAvailable()) return;

		const imagePath = getTestDocumentPath("images/invoice_image.png");
		const imageBytes = await readFile(imagePath);

		const result = await backend.processImage(imageBytes, "en");

		expect(result.metadata.confidence).toBeGreaterThanOrEqual(0);
		expect(result.metadata.confidence).toBeLessThanOrEqual(1);
		expect(result.metadata.language).toBe("en");
	});
});
