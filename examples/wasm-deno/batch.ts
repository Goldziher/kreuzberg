/**
 * Batch Processing WASM Extraction Example
 *
 * Demonstrates efficient batch processing of multiple documents.
 * Shows error handling for mixed results and performance metrics.
 *
 * Run with: deno run --allow-read batch.ts
 */

import type { ExtractionConfig, ExtractionResult } from "@kreuzberg/wasm";
import init, { extractBytes } from "@kreuzberg/wasm";

interface ExtractionRecord {
	filename: string;
	mimeType: string;
	size: number;
}

interface ExtractionError {
	filename: string;
	error: string;
}

interface BatchResult {
	successful: ExtractionRecord[];
	failed: ExtractionError[];
	startTime: number;
	endTime: number;
}

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
 * Extract from a single document with error handling
 */
async function extractDocument(
	filename: string,
	data: Uint8Array,
	mimeType: string,
	config?: ExtractionConfig,
): Promise<ExtractionResult | null> {
	return await extractBytes(data, mimeType, config);
}

/**
 * Process multiple documents sequentially
 */
async function batchExtractSequential(
	documents: Array<{ filename: string; data: Uint8Array; mimeType: string }>,
	config?: ExtractionConfig,
): Promise<BatchResult> {
	const result: BatchResult = {
		successful: [],
		failed: [],
		startTime: Date.now(),
		endTime: 0,
	};

	console.log(`Processing ${documents.length} documents sequentially...`);

	for (const doc of documents) {
		try {
			const extracted = await extractDocument(doc.filename, doc.data, doc.mimeType, config);
			if (extracted) {
				result.successful.push({
					filename: doc.filename,
					mimeType: extracted.mimeType,
					size: extracted.content.length,
				});
				console.log(`  ✓ ${doc.filename}: ${extracted.content.length} characters`);
			}
		} catch (error) {
			result.failed.push({
				filename: doc.filename,
				error: error instanceof Error ? error.message : String(error),
			});
			console.log(`  ✗ ${doc.filename}: ${error instanceof Error ? error.message : "Unknown error"}`);
		}
	}

	result.endTime = Date.now();
	return result;
}

/**
 * Process multiple documents in parallel
 */
async function batchExtractParallel(
	documents: Array<{ filename: string; data: Uint8Array; mimeType: string }>,
	config?: ExtractionConfig,
	concurrency: number = 4,
): Promise<BatchResult> {
	const result: BatchResult = {
		successful: [],
		failed: [],
		startTime: Date.now(),
		endTime: 0,
	};

	console.log(`Processing ${documents.length} documents in parallel (concurrency: ${concurrency})...`);

	// Process in batches to limit concurrency
	for (let i = 0; i < documents.length; i += concurrency) {
		const batch = documents.slice(i, i + concurrency);
		const promises = batch.map((doc) =>
			extractDocument(doc.filename, doc.data, doc.mimeType, config)
				.then((extracted) => ({ doc, extracted, error: null }))
				.catch((error) => ({ doc, extracted: null, error })),
		);

		const batchResults = await Promise.all(promises);

		for (const { doc, extracted, error } of batchResults) {
			if (extracted) {
				result.successful.push({
					filename: doc.filename,
					mimeType: extracted.mimeType,
					size: extracted.content.length,
				});
				console.log(`  ✓ ${doc.filename}: ${extracted.content.length} characters`);
			} else {
				result.failed.push({
					filename: doc.filename,
					error: error instanceof Error ? error.message : String(error),
				});
				console.log(`  ✗ ${doc.filename}: ${error instanceof Error ? error.message : "Unknown error"}`);
			}
		}
	}

	result.endTime = Date.now();
	return result;
}

/**
 * Print batch results summary
 */
function printResults(results: BatchResult, label: string) {
	const duration = results.endTime - results.startTime;
	const totalChars = results.successful.reduce((sum, r) => sum + r.size, 0);

	console.log(`\n${label} Results:`);
	console.log(`  Successful: ${results.successful.length}/${results.successful.length + results.failed.length}`);
	console.log(`  Total characters extracted: ${totalChars}`);
	console.log(`  Duration: ${duration}ms`);

	if (results.successful.length > 0) {
		const avgSize = totalChars / results.successful.length;
		console.log(`  Average size per document: ${Math.round(avgSize)} characters`);
	}

	if (results.failed.length > 0) {
		console.log(`  Failed documents:`);
		results.failed.forEach((fail) => {
			console.log(`    - ${fail.filename}: ${fail.error}`);
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
	console.log("Kreuzberg WASM Batch Processing Examples");
	console.log("=".repeat(60));

	// Example 1: Sequential batch processing
	console.log("\n--- Example 1: Sequential Batch Processing ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData && sampleData.length > 0) {
			const documents = [
				{
					filename: "sample1.pdf",
					data: sampleData,
					mimeType: "application/pdf",
				},
				{
					filename: "sample2.pdf",
					data: sampleData,
					mimeType: "application/pdf",
				},
				{
					filename: "sample3.pdf",
					data: sampleData,
					mimeType: "application/pdf",
				},
			];

			const results = await batchExtractSequential(documents);
			printResults(results, "Sequential");
		} else {
			console.log("Sample PDF not available in fixtures/. Skipping batch processing.");
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 2: Parallel batch processing
	console.log("\n--- Example 2: Parallel Batch Processing ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData && sampleData.length > 0) {
			const documents = [
				{
					filename: "parallel1.pdf",
					data: sampleData,
					mimeType: "application/pdf",
				},
				{
					filename: "parallel2.pdf",
					data: sampleData,
					mimeType: "application/pdf",
				},
				{
					filename: "parallel3.pdf",
					data: sampleData,
					mimeType: "application/pdf",
				},
				{
					filename: "parallel4.pdf",
					data: sampleData,
					mimeType: "application/pdf",
				},
			];

			const results = await batchExtractParallel(documents, undefined, 2);
			printResults(results, "Parallel");
		} else {
			console.log("Sample PDF not available in fixtures/. Skipping batch processing.");
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 3: Batch with configuration
	console.log("\n--- Example 3: Batch with Configuration ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData && sampleData.length > 0) {
			const config: ExtractionConfig = {
				chunking: {
					maxChars: 1000,
					maxOverlap: 100,
				},
			};

			const documents = [
				{
					filename: "config1.pdf",
					data: sampleData,
					mimeType: "application/pdf",
				},
				{
					filename: "config2.pdf",
					data: sampleData,
					mimeType: "application/pdf",
				},
			];

			const results = await batchExtractSequential(documents, config);
			console.log(`\nBatch with configuration completed: ${results.successful.length} successful`);
		} else {
			console.log("Sample PDF not available in fixtures/. Skipping batch processing.");
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 4: Error handling
	console.log("\n--- Example 4: Error Handling ---");
	try {
		const documents = [
			{
				filename: "valid.pdf",
				data: new Uint8Array([0x25, 0x50, 0x44, 0x46]), // PDF header
				mimeType: "application/pdf",
			},
			{
				filename: "invalid.pdf",
				data: new Uint8Array([0xff, 0xff, 0xff, 0xff]), // Invalid data
				mimeType: "application/pdf",
			},
		];

		const results = await batchExtractSequential(documents);

		console.log(`\nResults summary:`);
		console.log(`  Successful: ${results.successful.length}`);
		console.log(`  Failed: ${results.failed.length}`);
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 5: Performance comparison
	console.log("\n--- Example 5: Performance Comparison ---");
	try {
		const sampleData = await loadFixture("sample.pdf");

		if (sampleData && sampleData.length > 0) {
			const documents = Array.from({ length: 3 }, (_, i) => ({
				filename: `perf${i}.pdf`,
				data: sampleData,
				mimeType: "application/pdf",
			}));

			console.log("\nSequential processing:");
			const seqResults = await batchExtractSequential(documents);

			console.log("\nParallel processing:");
			const parResults = await batchExtractParallel(documents, undefined, 3);

			const improvement =
				((seqResults.endTime - seqResults.startTime - (parResults.endTime - parResults.startTime)) /
					(seqResults.endTime - seqResults.startTime)) *
				100;

			console.log(`\nPerformance improvement: ${improvement.toFixed(1)}%`);
		}
	} catch (error) {
		console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
	}

	// Example 6: Mixed document types
	console.log("\n--- Example 6: Mixed Document Types ---");
	console.log("Example batch processing with different document types:");
	const documentTypes = [
		{ type: "application/pdf", ext: ".pdf" },
		{ type: "text/plain", ext: ".txt" },
		{ type: "text/html", ext: ".html" },
		{
			type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			ext: ".docx",
		},
	];

	documentTypes.forEach((doc) => {
		console.log(`  - ${doc.ext}: ${doc.type}`);
	});

	console.log(`\n${"=".repeat(60)}`);
	console.log("Batch processing examples completed");
	console.log("=".repeat(60));
}

// Run main function
main().catch((error) => {
	console.error("Fatal error:", error);
	Deno.exit(1);
});
