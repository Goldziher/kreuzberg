/**
 * Batch Processing Example
 *
 * Demonstrates efficient batch processing of multiple documents.
 */

import { readFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { basename } from "node:path";
import {
	batchExtractBytes,
	batchExtractBytesSync,
	batchExtractFiles,
	batchExtractFilesSync,
	ExtractionConfig,
} from "@goldziher/kreuzberg";
import { glob } from "glob";

async function main() {
	// Synchronous batch processing
	console.log("=== Synchronous Batch Processing ===");
	const files = ["document1.pdf", "document2.docx", "document3.txt", "document4.html"];

	const results = batchExtractFilesSync(files);

	files.forEach((file, i) => {
		const result = results[i];
		console.log(`\n${file}:`);
		console.log(`  Length: ${result.content.length} chars`);
		console.log(`  MIME: ${result.mimeType}`);
		console.log(`  Preview: ${result.content.substring(0, 100)}...`);
	});

	// Async batch processing - better for large datasets
	console.log("\n=== Async Batch Processing ===");
	const manyFiles = Array.from({ length: 10 }, (_, i) => `doc${i}.pdf`);
	const asyncResults = await batchExtractFiles(manyFiles);

	const totalChars = asyncResults.reduce((sum, r) => sum + r.content.length, 0);
	console.log(`Processed ${asyncResults.length} files`);
	console.log(`Total characters: ${totalChars}`);

	// Batch with configuration
	console.log("\n=== Batch with Configuration ===");
	const config = new ExtractionConfig({
		enableQualityProcessing: true,
		useCache: true,
		ocr: undefined, // Disable OCR for faster processing
	});

	const configResults = batchExtractFilesSync(files, config);
	console.log(`Processed ${configResults.length} files with configuration`);

	// Process directory of files
	console.log("\n=== Process Directory ===");
	const pdfFiles = glob.sync("data/*.pdf");
	if (pdfFiles.length > 0) {
		const dirResults = batchExtractFilesSync(pdfFiles.slice(0, 5)); // Process first 5

		pdfFiles.slice(0, 5).forEach((file, i) => {
			const filename = basename(file);
			console.log(`${filename}: ${dirResults[i].content.length} chars`);
		});
	}

	// Batch extract from bytes
	console.log("\n=== Batch Extract from Bytes ===");
	const dataList: Buffer[] = [];
	const mimeTypes: string[] = [];

	for (const file of files.slice(0, 3)) {
		dataList.push(readFileSync(file));

		// Detect MIME type from extension
		const ext = file.toLowerCase().match(/\.[^.]+$/)?.[0];
		const mimeMap: Record<string, string> = {
			".pdf": "application/pdf",
			".docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			".txt": "text/plain",
			".html": "text/html",
		};
		mimeTypes.push(mimeMap[ext || ""] || "application/octet-stream");
	}

	const bytesResults = batchExtractBytesSync(dataList, mimeTypes);
	console.log(`Extracted ${bytesResults.length} documents from bytes`);

	// Async batch from bytes
	console.log("\n=== Async Batch from Bytes ===");
	const asyncDataList = await Promise.all(files.slice(0, 3).map((file) => readFile(file)));
	const asyncBytesResults = await batchExtractBytes(asyncDataList, mimeTypes);
	console.log(`Async extracted ${asyncBytesResults.length} documents from bytes`);

	// Error handling in batch processing
	console.log("\n=== Batch with Error Handling ===");
	const filesWithInvalid = [
		"valid1.pdf",
		"nonexistent.pdf", // This will fail
		"valid2.txt",
	];

	try {
		batchExtractFilesSync(filesWithInvalid);
	} catch (error) {
		console.error(`Batch error: ${error instanceof Error ? error.message : error}`);
		console.log("Note: Batch operations fail fast on first error");
		console.log("Process files individually for better error handling");
	}

	// Individual processing with error handling
	console.log("\n=== Individual Processing with Error Handling ===");
	for (const file of filesWithInvalid) {
		try {
			const [result] = batchExtractFilesSync([file]);
			console.log(`✓ ${file}: ${result.content.length} chars`);
		} catch (error) {
			const err = error as Error;
			console.log(`✗ ${file}: ${err.constructor.name}: ${err.message}`);
		}
	}

	// Parallel processing with Promise.all
	console.log("\n=== Parallel Async Processing ===");
	const parallelResults = await Promise.all(
		files.map((file) =>
			extractFile(file).catch((err) => ({
				error: true,
				file,
				message: err.message,
			})),
		),
	);

	parallelResults.forEach((result, i) => {
		if ("error" in result && result.error) {
			console.log(`✗ ${result.file}: ${result.message}`);
		} else if ("content" in result) {
			console.log(`✓ ${files[i]}: ${result.content.length} chars`);
		}
	});
}

// Import for parallel processing example
import { extractFile } from "@goldziher/kreuzberg";

main().catch(console.error);
