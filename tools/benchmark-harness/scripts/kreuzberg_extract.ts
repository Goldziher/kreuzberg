#!/usr/bin/env tsx
/**
 * Kreuzberg TypeScript/Node.js extraction wrapper for benchmark harness.
 *
 * Supports two modes:
 * - async: extractFile() - asynchronous extraction (default)
 * - batch: batchExtractFile() - batch extraction for multiple files
 */

import { batchExtractFile, extractFile } from '@kreuzberg/node';

interface ExtractionOutput {
	content: string;
	metadata: Record<string, unknown>;
	_extraction_time_ms: number;
	_batch_total_ms?: number;
}

async function extractAsync(filePath: string): Promise<ExtractionOutput> {
	const start = performance.now();
	const result = await extractFile(filePath);
	const durationMs = performance.now() - start;

	return {
		content: result.content,
		metadata: result.metadata || {},
		_extraction_time_ms: durationMs,
	};
}

async function extractBatch(filePaths: string[]): Promise<ExtractionOutput[]> {
	const start = performance.now();
	const results = await batchExtractFile(filePaths);
	const totalDurationMs = performance.now() - start;

	const perFileDurationMs = filePaths.length > 0 ? totalDurationMs / filePaths.length : 0;

	return results.map((result) => ({
		content: result.content,
		metadata: result.metadata || {},
		_extraction_time_ms: perFileDurationMs,
		_batch_total_ms: totalDurationMs,
	}));
}

async function main(): Promise<void> {
	const args = process.argv.slice(2);

	if (args.length < 2) {
		console.error("Usage: kreuzberg_extract.ts <mode> <file_path> [additional_files...]");
		console.error("Modes: async, batch");
		process.exit(1);
	}

	const mode = args[0];
	const filePaths = args.slice(1);

	try {
		if (mode === "async") {
			if (filePaths.length !== 1) {
				console.error("Error: async mode requires exactly one file");
				process.exit(1);
			}
			const payload = await extractAsync(filePaths[0]);
			console.log(JSON.stringify(payload));
		} else if (mode === "batch") {
			if (filePaths.length < 1) {
				console.error("Error: batch mode requires at least one file");
				process.exit(1);
			}

			const results = await extractBatch(filePaths);

			if (filePaths.length === 1) {
				console.log(JSON.stringify(results[0]));
			} else {
				console.log(JSON.stringify(results));
			}
		} else {
			console.error(`Error: Unknown mode '${mode}'. Use async or batch`);
			process.exit(1);
		}
	} catch (err) {
		const error = err as Error;
		console.error(`Error extracting with Kreuzberg: ${error.message}`);
		process.exit(1);
	}
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
