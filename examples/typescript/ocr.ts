/**
 * OCR Extraction Example
 *
 * Demonstrates OCR extraction from scanned PDFs and images.
 */

import { ExtractionConfig, extractFile, extractFileSync, OcrConfig, TesseractConfig } from "@kreuzberg/node";

async function main() {
	console.log("=== Basic OCR ===");
	const config = new ExtractionConfig({
		ocr: new OcrConfig({
			backend: "tesseract",
			language: "eng",
		}),
	});

	const result = extractFileSync("scanned_document.pdf", null, config);
	console.log(`Extracted: ${result.content.length} characters`);
	console.log(`First 200 chars: ${result.content.substring(0, 200)}...`);

	console.log("\n=== OCR with German ===");
	const germanConfig = new ExtractionConfig({
		ocr: new OcrConfig({
			backend: "tesseract",
			language: "deu",
		}),
	});

	const germanResult = extractFileSync("german_document.pdf", null, germanConfig);
	console.log(`Extracted German text: ${germanResult.content.length} characters`);

	console.log("\n=== Force OCR ===");
	const forceConfig = new ExtractionConfig({
		ocr: new OcrConfig({
			backend: "tesseract",
			language: "eng",
		}),
		forceOcr: true,
	});

	const forcedResult = extractFileSync("mixed_document.pdf", null, forceConfig);
	console.log(`Forced OCR extraction: ${forcedResult.content.length} characters`);

	console.log("\n=== OCR from Image ===");
	const imageConfig = new ExtractionConfig({
		ocr: new OcrConfig({
			backend: "tesseract",
			language: "eng",
		}),
	});

	const imageResult = extractFileSync("screenshot.png", null, imageConfig);
	console.log(`Extracted from image: ${imageResult.content.length} characters`);

	if (imageResult.metadata.ocr) {
		console.log(`OCR Language: ${imageResult.metadata.ocr.language}`);
		console.log(`Table Count: ${imageResult.metadata.ocr.tableCount}`);
	}

	console.log("\n=== OCR Table Extraction ===");
	const tableConfig = new ExtractionConfig({
		ocr: new OcrConfig({
			backend: "tesseract",
			language: "eng",
			tesseractConfig: new TesseractConfig({
				enableTableDetection: true,
			}),
		}),
	});

	const tableResult = extractFileSync("table_document.pdf", null, tableConfig);
	console.log(`Found ${tableResult.tables.length} tables`);

	tableResult.tables.forEach((table, i) => {
		console.log(`\nTable ${i + 1}:`);
		console.log(`  Rows: ${table.cells.length}`);
		console.log(`  Columns: ${table.cells[0]?.length || 0}`);
		console.log(`  Markdown:\n${table.markdown.substring(0, 200)}...`);
	});

	console.log("\n=== Async OCR ===");
	const asyncResult = await extractFile("scanned_document.pdf", null, config);
	console.log(`Async OCR extracted: ${asyncResult.content.length} characters`);

	console.log("\n=== Custom PSM Mode ===");
	const psmConfig = new ExtractionConfig({
		ocr: new OcrConfig({
			backend: "tesseract",
			language: "eng",
			tesseractConfig: new TesseractConfig({
				psm: 6,
			}),
		}),
	});

	const psmResult = extractFileSync("document.pdf", null, psmConfig);
	console.log(`Extracted with PSM 6: ${psmResult.content.length} characters`);
}

main().catch(console.error);
