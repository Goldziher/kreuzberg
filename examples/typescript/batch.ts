/**
 * Batch Processing Example
 *
 * Demonstrates efficient batch processing of multiple documents.
 */

import { readFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { basename } from "node:path";
import { batchExtractBytes, batchExtractBytesSync, batchExtractFiles, batchExtractFilesSync } from "@kreuzberg/node";
import { glob } from "glob";

async function main() {
	console.log("=== Synchronous Batch Processing ===");
	const files = ["document1.pdf", "document2.docx", "document3.txt", "document4.html"];

	const results = batchExtractFilesSync(files);

	files.forEach((file: string, i: number) => {
		const result = results[i];
		if (result) {
			console.log(`\n${file}:`);
			console.log(`  Length: ${result.content.length} chars`);
			console.log(`  MIME: ${result.mimeType}`);
			console.log(`  Preview: ${result.content.substring(0, 100)}...`);
		}
	});

	console.log("\n=== Async Batch Processing ===");
	const manyFiles = Array.from({ length: 10 }, (_, i) => `doc${i}.pdf`);
	const asyncResults = await batchExtractFiles(manyFiles);

	const totalChars = asyncResults.reduce((sum, r) => sum + r.content.length, 0);
	console.log(`Processed ${asyncResults.length} files`);
	console.log(`Total characters: ${totalChars}`);

	console.log("\n=== Batch with Configuration ===");
	const config = {
		enableQualityProcessing: true,
		useCache: true,
	};

	const configResults = batchExtractFilesSync(files, config);
	console.log(`Processed ${configResults.length} files with configuration`);

	console.log("\n=== Process Directory ===");
	const pdfFiles = glob.sync("data/*.pdf");
	if (pdfFiles.length > 0) {
		const dirResults = batchExtractFilesSync(pdfFiles.slice(0, 5));

		pdfFiles.slice(0, 5).forEach((file: string, i: number) => {
			const filename = basename(file);
			const dirResult = dirResults[i];
			if (dirResult) {
				console.log(`${filename}: ${dirResult.content.length} chars`);
			}
		});
	}

	console.log("\n=== Batch Extract from Bytes ===");
	const dataList: Buffer[] = [];
	const mimeTypes: string[] = [];

	for (const file of files.slice(0, 3)) {
		dataList.push(readFileSync(file));

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

	console.log("\n=== Async Batch from Bytes ===");
	const asyncDataList = await Promise.all(files.slice(0, 3).map((file) => readFile(file)));
	const asyncBytesResults = await batchExtractBytes(asyncDataList, mimeTypes);
	console.log(`Async extracted ${asyncBytesResults.length} documents from bytes`);

	console.log("\n=== Batch with Error Handling ===");
	const filesWithInvalid = ["valid1.pdf", "nonexistent.pdf", "valid2.txt"];

	try {
		batchExtractFilesSync(filesWithInvalid);
	} catch (error) {
		console.error(`Batch error: ${error instanceof Error ? error.message : error}`);
		console.log("Note: Batch operations fail fast on first error");
		console.log("Process files individually for better error handling");
	}

	console.log("\n=== Individual Processing with Error Handling ===");
	for (const file of filesWithInvalid) {
		try {
			const results = batchExtractFilesSync([file]);
			const result = results[0];
			if (result) {
				console.log(`✓ ${file}: ${result.content.length} chars`);
			}
		} catch (error) {
			const err = error as Error;
			console.log(`✗ ${file}: ${err.constructor.name}: ${err.message}`);
		}
	}

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

	parallelResults.forEach((result: any, i: number) => {
		if ("error" in result && result.error) {
			console.log(`✗ ${result.file}: ${result.message}`);
		} else if ("content" in result && result.content) {
			console.log(`✓ ${files[i]}: ${result.content.length} chars`);
		}
	});
}

import { extractFile } from "@kreuzberg/node";

main().catch(console.error);
