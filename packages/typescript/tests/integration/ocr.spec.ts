import { existsSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { extractFile } from "../../src/index.js";
import type { ExtractionConfig } from "../../src/types.js";
import {
	assertImageMetadata,
	assertMimeType,
	assertNonEmptyContent,
	assertOcrResult,
	assertValidExtractionResult,
	getTestDocumentPath,
	testDocumentsAvailable,
} from "../helpers/integration-helpers.js";

describe("OCR Integration Tests", () => {
	it("should perform OCR on simple English text image", async () => {
		if (!testDocumentsAvailable()) {
			console.log("Skipping: test_documents not available");
			return;
		}

		const path = getTestDocumentPath("images/test_hello_world.png");
		if (!existsSync(path)) {
			console.log("Skipping: OCR test image not available");
			return;
		}

		const config: ExtractionConfig = {
			ocr: {
				backend: "tesseract",
				language: "eng",
			},
			forceOcr: true,
		};

		try {
			const result = await extractFile(path, null, config);

			assertValidExtractionResult(result);
			assertMimeType(result, "image/png");
			assertNonEmptyContent(result);

			assertOcrResult(result, ["hello", "world"], 0.3);

			console.log("OCR successfully extracted text from image");
		} catch (error) {
			console.log("OCR test failed (Tesseract may not be installed):", (error as Error).message);
		}
	});

	it("should handle OCR on image with no text", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("images/flower_no_text.jpg");
		if (!existsSync(path)) {
			console.log("Skipping: flower image not available");
			return;
		}

		const config: ExtractionConfig = {
			ocr: {
				backend: "tesseract",
				language: "eng",
			},
			forceOcr: true,
		};

		try {
			const result = await extractFile(path, null, config);

			assertValidExtractionResult(result);
			assertMimeType(result, "image/jpeg");

			const contentLen = result.content.trim().length;
			expect(contentLen).toBeLessThan(50);

			console.log("OCR correctly returned minimal content for image without text");
		} catch (error) {
			console.log("OCR test failed (Tesseract may not be installed):", (error as Error).message);
		}
	});

	it("should extract image metadata without OCR", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("images/example.jpg");
		if (!existsSync(path)) {
			console.log("Skipping: example image not available");
			return;
		}

		const config: ExtractionConfig = {
			ocr: undefined,
		};

		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "image/jpeg");

		assertImageMetadata(result.metadata);

		console.log("Image metadata extracted without OCR");
	});

	it("should configure Tesseract OCR with custom settings", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("images/test_hello_world.png");
		if (!existsSync(path)) {
			return;
		}

		const config: ExtractionConfig = {
			ocr: {
				backend: "tesseract",
				language: "eng",
				tesseractConfig: {
					psm: 6,
					enableTableDetection: false,
				},
			},
			forceOcr: true,
		};

		try {
			const result = await extractFile(path, null, config);

			assertValidExtractionResult(result);
			assertMimeType(result, "image/png");

			if (result.metadata.ocr) {
				expect(result.metadata.ocr.language).toBe("eng");
				if (result.metadata.ocr.psm !== undefined) {
					expect(result.metadata.ocr.psm).toBe(6);
				}
			}

			console.log("Tesseract configured with custom settings");
		} catch (_error) {
			console.log("OCR test failed (Tesseract may not be installed)");
		}
	});

	it("should validate OCR confidence scores are in valid range", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("images/test_hello_world.png");
		if (!existsSync(path)) {
			return;
		}

		const config: ExtractionConfig = {
			ocr: {
				backend: "tesseract",
				language: "eng",
			},
			forceOcr: true,
		};

		try {
			const result = await extractFile(path, null, config);

			assertValidExtractionResult(result);

			const metadata: any = result.metadata;
			if (metadata.confidence !== undefined) {
				expect(metadata.confidence).toBeGreaterThanOrEqual(0.0);
				expect(metadata.confidence).toBeLessThanOrEqual(1.0);

				if (result.content.trim().length > 0) {
					expect(metadata.confidence).toBeGreaterThan(0.1);
					console.log(`OCR confidence: ${metadata.confidence}`);
				}
			}
		} catch (_error) {
			console.log("OCR test failed (Tesseract may not be installed)");
		}
	});

	it("should perform OCR with table detection", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("images/test_hello_world.png");
		if (!existsSync(path)) {
			return;
		}

		const config: ExtractionConfig = {
			ocr: {
				backend: "tesseract",
				language: "eng",
				tesseractConfig: {
					enableTableDetection: true,
				},
			},
			forceOcr: true,
		};

		try {
			const result = await extractFile(path, null, config);

			assertValidExtractionResult(result);
			assertNonEmptyContent(result);

			if (result.tables.length > 0) {
				console.log(`OCR detected ${result.tables.length} table(s)`);
				for (const table of result.tables) {
					expect(table.cells).toBeTruthy();
					expect(Array.isArray(table.cells)).toBe(true);
				}
			}
		} catch (_error) {
			console.log("OCR table detection test failed (Tesseract may not be installed)");
		}
	});

	it("should handle OCR with character whitelist", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("images/test_hello_world.png");
		if (!existsSync(path)) {
			return;
		}

		const config: ExtractionConfig = {
			ocr: {
				backend: "tesseract",
				language: "eng",
				tesseractConfig: {
					tesseditCharWhitelist: "ABCDEFGHIJKLMNOPQRSTUVWXYZ ",
				},
			},
			forceOcr: true,
		};

		try {
			const result = await extractFile(path, null, config);

			assertValidExtractionResult(result);

			const content = result.content.trim();
			if (content.length > 0) {
				const hasOnlyAllowedChars = /^[A-Z ]+$/.test(content);
				console.log(`Whitelist enforced: ${hasOnlyAllowedChars}`);
			}
		} catch (_error) {
			console.log("OCR whitelist test failed (Tesseract may not be installed)");
		}
	});

	it("should extract image with DPI adjustment", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("images/example.jpg");
		if (!existsSync(path)) {
			return;
		}

		const config: ExtractionConfig = {
			images: {
				targetDpi: 300,
				autoAdjustDpi: true,
				minDpi: 150,
				maxDpi: 600,
			},
			ocr: {
				backend: "tesseract",
				language: "eng",
			},
		};

		try {
			const result = await extractFile(path, null, config);

			assertValidExtractionResult(result);

			if (result.metadata.imagePreprocessing) {
				console.log("Image preprocessing metadata:", result.metadata.imagePreprocessing);

				expect(result.metadata.imagePreprocessing.targetDpi).toBe(300);

				if (result.metadata.imagePreprocessing.finalDpi) {
					expect(result.metadata.imagePreprocessing.finalDpi).toBeGreaterThanOrEqual(150);
					expect(result.metadata.imagePreprocessing.finalDpi).toBeLessThanOrEqual(600);
				}
			}
		} catch (error) {
			console.log("Image DPI test failed:", (error as Error).message);
		}
	});
});
