/**
 * OCR Extraction WASM Example
 *
 * Demonstrates OCR extraction from scanned documents and images.
 * Shows language selection, force OCR, and Tesseract configuration.
 *
 * Run with: deno run --allow-read ocr.ts
 *
 * Note: OCR functionality requires Tesseract to be available in your environment.
 */

import type { ExtractionConfig, ExtractionResult, OcrConfig } from "@kreuzberg/wasm";
import init, { extractBytes } from "@kreuzberg/wasm";

/**
 * Load a fixture file from the fixtures directory
 */
async function loadFixture(filename: string): Promise<Uint8Array | null> {
	try {
		const path = new URL(`./fixtures/${filename}`, import.meta.url);
		return await Deno.readFile(path.pathname);
	} catch {
		return null;
	}
}

/**
 * Extract text with OCR configuration
 */
async function extractWithOcr(
	data: Uint8Array,
	mimeType: string,
	language: string = "eng",
	forceOcr: boolean = false,
): Promise<ExtractionResult | null> {
	const ocrConfig: OcrConfig = {
		backend: "tesseract",
		language: language,
	};

	const config: ExtractionConfig = {
		ocr: ocrConfig,
		forceOcr: forceOcr,
	};

	try {
		return await extractBytes(data, mimeType, config);
	} catch (error) {
		// Check if error is due to missing Tesseract
		if (error instanceof Error && error.message.toLowerCase().includes("tesseract")) {
			console.log("  (Tesseract OCR not available - skipping)");
			return null;
		}
		throw error;
	}
}

/**
 * Print metadata from extraction result
 */
function printMetadata(result: ExtractionResult | null) {
	if (!result) return;

	console.log("  Metadata:");
	if (result.metadata) {
		Object.entries(result.metadata).forEach(([key, value]) => {
			console.log(`    - ${key}: ${JSON.stringify(value)}`);
		});
	}
}

/**
 * Main example function
 */
async function main() {
	// Initialize the WASM module
	await init();

	console.log("=".repeat(60));
	console.log("Kreuzberg WASM OCR Extraction Examples");
	console.log("=".repeat(60));

	// Example 1: Basic OCR with English
	console.log("\n--- Example 1: Basic OCR (English) ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData && sampleData.length > 0) {
			console.log("Extracting text with English OCR...");
			const result = await extractWithOcr(sampleData, "application/pdf", "eng");

			if (result) {
				console.log(`  Content length: ${result.content.length} characters`);
				console.log(`  MIME type: ${result.mimeType}`);
				console.log(`  Preview: ${result.content.substring(0, 100)}...`);
				printMetadata(result);
			}
		} else {
			console.log("Sample PDF not available in fixtures/");
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 2: OCR with German language
	console.log("\n--- Example 2: OCR with German Language ---");
	console.log("German OCR configuration example (language: deu)");
	const germanConfig: ExtractionConfig = {
		ocr: {
			backend: "tesseract",
			language: "deu",
		},
	};
	console.log(`  Configuration: ${JSON.stringify(germanConfig, null, 2)}`);
	console.log("  (Requires German language pack for Tesseract)");

	// Example 3: Force OCR on text PDFs
	console.log("\n--- Example 3: Force OCR on Text PDFs ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData && sampleData.length > 0) {
			console.log("Forcing OCR even if PDF contains extractable text...");
			const result = await extractWithOcr(sampleData, "application/pdf", "eng", true);

			if (result) {
				console.log(`  Forced OCR extraction: ${result.content.length} characters`);
			}
		} else {
			console.log("Sample PDF not available in fixtures/");
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 4: OCR with Tesseract configuration
	console.log("\n--- Example 4: Advanced Tesseract Configuration ---");
	const psmModes: Record<number, string> = {
		0: "Orientation and script detection",
		1: "Automatic page segmentation with OSD",
		3: "Fully automatic page segmentation",
		6: "Uniform block of text",
		7: "Single text line",
		8: "Single word",
		11: "Sparse text",
		13: "Raw line",
	};

	console.log("Available Tesseract PSM (Page Segmentation Mode) options:");
	Object.entries(psmModes).forEach(([mode, description]) => {
		console.log(`  - PSM ${mode}: ${description}`);
	});

	console.log("\nExample configuration with PSM 6 (uniform block of text):");
	const tesseractConfig: ExtractionConfig = {
		ocr: {
			backend: "tesseract",
			language: "eng",
			tesseractConfig: {
				psm: 6,
				enableTableDetection: true,
			},
		},
	};
	console.log(`  ${JSON.stringify(tesseractConfig, null, 2)}`);

	// Example 5: Language options
	console.log("\n--- Example 5: Supported OCR Languages ---");
	const languages = [
		{ code: "eng", name: "English" },
		{ code: "deu", name: "German" },
		{ code: "fra", name: "French" },
		{ code: "spa", name: "Spanish" },
		{ code: "ita", name: "Italian" },
		{ code: "por", name: "Portuguese" },
		{ code: "rus", name: "Russian" },
		{ code: "jpn", name: "Japanese" },
		{ code: "chi_sim", name: "Chinese (Simplified)" },
		{ code: "chi_tra", name: "Chinese (Traditional)" },
		{ code: "ara", name: "Arabic" },
	];

	console.log("Common language codes (ISO 639-3 format):");
	languages.forEach((lang) => {
		console.log(`  - ${lang.code.padEnd(8)}: ${lang.name}`);
	});

	// Example 6: Table detection
	console.log("\n--- Example 6: OCR with Table Detection ---");
	const tableDetectionConfig: ExtractionConfig = {
		ocr: {
			backend: "tesseract",
			language: "eng",
			tesseractConfig: {
				psm: 3, // Automatic page segmentation
				enableTableDetection: true,
			},
		},
	};

	console.log("Configuration for detecting tables in scanned documents:");
	console.log(`  ${JSON.stringify(tableDetectionConfig, null, 2)}`);

	// Example 7: Error handling for OCR
	console.log("\n--- Example 7: OCR Error Handling ---");
	console.log("OCR operations may fail if:");
	console.log("  - Tesseract is not installed on the system");
	console.log("  - Required language data files are missing");
	console.log("  - Image quality is too poor for text recognition");
	console.log("  - Document format is not supported");
	console.log("\nRecommended fallback strategy:");
	console.log("  1. Try OCR extraction");
	console.log("  2. Fall back to regular text extraction");
	console.log("  3. Return structured metadata");

	// Example 8: Batch OCR processing
	console.log("\n--- Example 8: Batch OCR Processing ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData && sampleData.length > 0) {
			const documents = [
				{
					filename: "english.pdf",
					data: sampleData,
					language: "eng",
				},
				{
					filename: "german.pdf",
					data: sampleData,
					language: "deu",
				},
			];

			console.log("Processing multiple documents with different languages:");
			for (const doc of documents) {
				try {
					console.log(`\n  ${doc.filename} (${doc.language}):`);
					const result = await extractWithOcr(doc.data, "application/pdf", doc.language);

					if (result) {
						console.log(`    - Extracted: ${result.content.length} characters`);
					}
				} catch (error) {
					console.log(`    - Error: ${error instanceof Error ? error.message : String(error)}`);
				}
			}
		} else {
			console.log("Sample PDF not available in fixtures/");
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 9: Configuration best practices
	console.log("\n--- Example 9: OCR Configuration Best Practices ---");
	const configs: Record<string, ExtractionConfig> = {
		basic: {
			ocr: {
				backend: "tesseract",
				language: "eng",
			},
		},

		highQuality: {
			ocr: {
				backend: "tesseract",
				language: "eng",
				tesseractConfig: {
					psm: 3,
					enableTableDetection: true,
				},
			},
		},

		multilingual: {
			ocr: {
				backend: "tesseract",
				language: "eng+fra+deu",
			},
		},

		performance: {
			forceOcr: false,
			ocr: {
				backend: "tesseract",
				language: "eng",
				tesseractConfig: {
					psm: 6,
				},
			},
		},
	};

	console.log("Recommended configurations:");
	Object.entries(configs).forEach(([name, config]) => {
		console.log(`\n  ${name}:`);
		if (config.ocr) {
			console.log(`    - Backend: ${config.ocr.backend}`);
			console.log(`    - Language: ${config.ocr.language}`);
			if (config.ocr.tesseractConfig) {
				console.log(`    - PSM: ${config.ocr.tesseractConfig.psm}`);
				if (config.ocr.tesseractConfig.enableTableDetection) {
					console.log(`    - Table Detection: enabled`);
				}
			}
		}
	});

	console.log(`\n${"=".repeat(60)}`);
	console.log("OCR examples completed");
	console.log("=".repeat(60));
}

// Run main function
main().catch((error) => {
	console.error("Fatal error:", error);
	Deno.exit(1);
});
