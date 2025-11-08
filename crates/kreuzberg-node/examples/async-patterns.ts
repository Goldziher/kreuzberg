/**
 * NAPI-RS Async Patterns for Kreuzberg Node.js Bindings
 *
 * This example demonstrates how NAPI-RS provides built-in async support
 * that automatically converts Rust async functions to JavaScript Promises.
 */

import { readFile } from "fs/promises";
import {
	batchExtractBytes,
	batchExtractFiles,
	type ExtractionConfig,
	type ExtractionResult,
	extractBytes,
	extractFile,
	type OcrConfig,
} from "../index";

/**
 * Basic async extraction
 *
 * NAPI-RS automatically converts Rust async fn to Promise<T>
 */
async function basicExtraction(): Promise<void> {
	const result: ExtractionResult = await extractFile("document.pdf", null, null);

	console.log("Content:", result.content);
	console.log("Metadata:", result.metadata);
}

/**
 * Extraction with configuration
 */
async function extractionWithConfig(): Promise<void> {
	const config: ExtractionConfig = {
		ocr: {
			enabled: true,
			backend: "tesseract",
			language: "eng",
		} as OcrConfig,
		forceOcr: false,
		maxFileSize: null,
	};

	const result: ExtractionResult = await extractFile("scanned.pdf", "application/pdf", config);

	console.log("Extracted with OCR:", result.content);
}

/**
 * Concurrent processing with Promise.all
 *
 * Node.js event loop continues during async operations
 */
async function concurrentExtraction(): Promise<void> {
	const files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"];

	const results: ExtractionResult[] = await Promise.all(files.map((file) => extractFile(file, null, null)));

	results.forEach((result, index) => {
		console.log(`File ${index + 1}:`, result.content.slice(0, 100));
	});
}

/**
 * Batch extraction API (optimized Rust implementation)
 */
async function batchExtraction(): Promise<void> {
	const files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"];

	const results: ExtractionResult[] = await batchExtractFiles(files, null, null);

	console.log(`Processed ${results.length} files`);
}

/**
 * Extract from bytes (in-memory processing)
 */
async function extractFromBytes(): Promise<void> {
	const buffer = await readFile("document.pdf");
	const bytes = new Uint8Array(buffer);

	const result: ExtractionResult = await extractBytes(bytes, "application/pdf", null);

	console.log("Extracted from memory:", result.content);
}

/**
 * Batch extraction from bytes
 */
async function batchExtractFromBytes(): Promise<void> {
	const files = ["doc1.pdf", "doc2.pdf"];
	const buffers = await Promise.all(files.map((f) => readFile(f)));
	const bytesArray = buffers.map((b) => new Uint8Array(b));

	const results: ExtractionResult[] = await batchExtractBytes(bytesArray, "application/pdf", null);

	console.log(`Processed ${results.length} files from memory`);
}

/**
 * Error handling with try-catch
 */
async function errorHandling(): Promise<void> {
	try {
		const result = await extractFile("nonexistent.pdf", null, null);
		console.log(result.content);
	} catch (error) {
		console.error("Extraction failed:", error);
	}
}

/**
 * Promise chaining (alternative to async/await)
 */
function promiseChaining(): void {
	extractFile("document.pdf", null, null)
		.then((result) => {
			console.log("Content:", result.content);
			return result;
		})
		.then((result) => {
			console.log("Word count:", result.content.split(/\s+/).length);
		})
		.catch((error) => {
			console.error("Error:", error);
		});
}

/**
 * Sequential processing with for-await-of
 */
async function sequentialProcessing(): Promise<void> {
	const files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"];

	for (const file of files) {
		const result = await extractFile(file, null, null);
		console.log(`Processed ${file}:`, result.content.slice(0, 50));
	}
}

/**
 * Advanced: Custom timeout wrapper
 */
async function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
	return Promise.race([
		promise,
		new Promise<T>((_, reject) => setTimeout(() => reject(new Error("Timeout")), timeoutMs)),
	]);
}

async function extractionWithTimeout(): Promise<void> {
	try {
		const result = await withTimeout(extractFile("large-document.pdf", null, null), 30000);
		console.log("Extracted:", result.content);
	} catch (error) {
		console.error("Extraction timed out or failed:", error);
	}
}

/**
 * Main demonstration
 */
async function main(): Promise<void> {
	console.log("=== Basic Extraction ===");
	await basicExtraction();

	console.log("\n=== Extraction with Config ===");
	await extractionWithConfig();

	console.log("\n=== Concurrent Extraction ===");
	await concurrentExtraction();

	console.log("\n=== Batch Extraction ===");
	await batchExtraction();

	console.log("\n=== Extract from Bytes ===");
	await extractFromBytes();

	console.log("\n=== Error Handling ===");
	await errorHandling();

	console.log("\n=== Sequential Processing ===");
	await sequentialProcessing();

	console.log("\n=== Extraction with Timeout ===");
	await extractionWithTimeout();
}

if (require.main === module) {
	main().catch(console.error);
}

/**
 * Key Takeaways:
 *
 * 1. NAPI-RS automatically converts Rust `async fn` to JavaScript Promise<T>
 * 2. Zero configuration needed - just use async/await
 * 3. No overhead - transparent Tokio integration (~0ms)
 * 4. Natural JavaScript patterns (Promise.all, for-await-of, try-catch)
 * 5. Event loop continues during async operations
 * 6. Superior to PyO3's pyo3_async_runtimes (simpler API, better performance)
 *
 * Performance Comparison:
 * - NAPI-RS: ~0ms overhead, automatic Promise conversion
 * - PyO3 (optimized): ~0.17ms overhead, manual future_into_py()
 * - PyO3 (unoptimized): ~4.8ms overhead, spawn_blocking
 *
 * See ASYNC_COMPARISON.md for detailed comparison with Python bindings.
 */
