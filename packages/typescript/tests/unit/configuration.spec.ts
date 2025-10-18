import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { extractFileSync, extractBytesSync } from "../../src/index.js";
import type { ExtractionConfig } from "../../src/types.js";

function getTestDocumentPath(relativePath: string): string {
	const workspaceRoot = join(process.cwd(), "../..");
	return join(workspaceRoot, "test_documents", relativePath);
}

describe("Configuration Options", () => {
	const pdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
	const pdfBytes = new Uint8Array(readFileSync(pdfPath));

	describe("Basic configuration", () => {
		it("should handle useCache: true", () => {
			const config: ExtractionConfig = { useCache: true };
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle useCache: false", () => {
			const config: ExtractionConfig = { useCache: false };
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle enableQualityProcessing: true", () => {
			const config: ExtractionConfig = { enableQualityProcessing: true };
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle enableQualityProcessing: false", () => {
			const config: ExtractionConfig = { enableQualityProcessing: false };
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});
	});

	describe("OCR configuration", () => {
		it("should handle OCR with tesseract backend", () => {
			const config: ExtractionConfig = {
				ocr: {
					backend: "tesseract",
					language: "eng",
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle OCR with tesseract config options", () => {
			const config: ExtractionConfig = {
				ocr: {
					backend: "tesseract",
					language: "eng",
					tesseractConfig: {
						psm: 6,
						enableTableDetection: true,
						tesseditCharWhitelist: "0123456789",
					},
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle forceOcr: true", () => {
			const config: ExtractionConfig = {
				forceOcr: true,
				ocr: {
					backend: "tesseract",
					language: "eng",
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle forceOcr: false", () => {
			const config: ExtractionConfig = {
				forceOcr: false,
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});
	});

	describe("Chunking configuration", () => {
		it("should handle chunking with maxChars", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 1000,
					maxOverlap: 100,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
			// Chunks may or may not be present depending on content length
		});

		it("should handle chunking with different settings", () => {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 500,
					maxOverlap: 50,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});
	});

	describe("PDF options", () => {
		it("should handle PDF extractImages: true", () => {
			const config: ExtractionConfig = {
				pdfOptions: {
					extractImages: true,
					extractMetadata: true,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle PDF extractImages: false", () => {
			const config: ExtractionConfig = {
				pdfOptions: {
					extractImages: false,
					extractMetadata: true,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle PDF password configuration", () => {
			const config: ExtractionConfig = {
				pdfOptions: {
					passwords: ["test123", "password"],
					extractMetadata: true,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});
	});

	describe("Image configuration", () => {
		it("should handle image extraction config", () => {
			const config: ExtractionConfig = {
				images: {
					extractImages: true,
					targetDpi: 300,
					maxImageDimension: 4096,
					autoAdjustDpi: true,
					minDpi: 72,
					maxDpi: 600,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle custom DPI settings", () => {
			const config: ExtractionConfig = {
				images: {
					targetDpi: 150,
					minDpi: 100,
					maxDpi: 300,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});
	});

	describe("Token reduction", () => {
		it("should handle token reduction: off", () => {
			const config: ExtractionConfig = {
				tokenReduction: {
					mode: "off",
					preserveImportantWords: true,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle token reduction: aggressive", () => {
			const config: ExtractionConfig = {
				tokenReduction: {
					mode: "aggressive",
					preserveImportantWords: false,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});
	});

	describe("Language detection", () => {
		it("should handle language detection enabled", () => {
			const config: ExtractionConfig = {
				languageDetection: {
					enabled: true,
					minConfidence: 0.8,
					detectMultiple: false,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle language detection with multiple languages", () => {
			const config: ExtractionConfig = {
				languageDetection: {
					enabled: true,
					minConfidence: 0.7,
					detectMultiple: true,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});
	});

	describe("Postprocessor configuration", () => {
		it("should handle postprocessor enabled: true", () => {
			const config: ExtractionConfig = {
				postprocessor: {
					enabled: true,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle postprocessor enabled: false", () => {
			const config: ExtractionConfig = {
				postprocessor: {
					enabled: false,
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle enabled processors list", () => {
			const config: ExtractionConfig = {
				postprocessor: {
					enabled: true,
					enabledProcessors: ["processor1", "processor2"],
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle disabled processors list", () => {
			const config: ExtractionConfig = {
				postprocessor: {
					enabled: true,
					disabledProcessors: ["processor3", "processor4"],
				},
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});
	});

	describe("Max concurrent extractions", () => {
		it("should handle maxConcurrentExtractions setting", () => {
			const config: ExtractionConfig = {
				maxConcurrentExtractions: 4,
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle maxConcurrentExtractions: 1", () => {
			const config: ExtractionConfig = {
				maxConcurrentExtractions: 1,
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});
	});

	describe("Combined configurations", () => {
		it("should handle complex configuration with multiple options", () => {
			const config: ExtractionConfig = {
				useCache: false,
				enableQualityProcessing: true,
				ocr: {
					backend: "tesseract",
					language: "eng",
				},
				chunking: {
					maxChars: 1000,
					maxOverlap: 200,
				},
				images: {
					targetDpi: 300,
				},
				tokenReduction: {
					mode: "off",
				},
				languageDetection: {
					enabled: true,
				},
				maxConcurrentExtractions: 2,
			};
			const result = extractFileSync(pdfPath, null, config);
			expect(result.content).toBeTruthy();
		});

		it("should handle configuration with bytes extraction", () => {
			const config: ExtractionConfig = {
				useCache: false,
				enableQualityProcessing: true,
				pdfOptions: {
					extractImages: true,
					extractMetadata: true,
				},
			};
			const result = extractBytesSync(pdfBytes, "application/pdf", config);
			expect(result.content).toBeTruthy();
		});
	});
});
