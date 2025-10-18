import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { ExtractionResult, Metadata } from "../../src/types.js";

/**
 * Load a test document from the shared test_documents directory.
 * Path should be relative to test_documents/ (e.g., "pdfs/simple.pdf")
 */
export function loadTestDocument(relativePath: string): Buffer {
	const testDocPath = join(
		process.cwd(),
		"../../test_documents",
		relativePath,
	);
	return readFileSync(testDocPath);
}

/**
 * Get absolute path to a test document.
 */
export function getTestDocumentPath(relativePath: string): string {
	return join(process.cwd(), "../../test_documents", relativePath);
}

/**
 * Assert that an extraction result is valid and has required fields.
 */
export function assertValidExtractionResult(result: ExtractionResult): void {
	// Required fields
	if (typeof result.content !== "string") {
		throw new Error("result.content must be a string");
	}
	if (typeof result.mimeType !== "string") {
		throw new Error("result.mimeType must be a string");
	}
	if (typeof result.metadata !== "object" || result.metadata === null) {
		throw new Error("result.metadata must be an object");
	}

	// Tables can be null or array
	if (result.tables !== null && !Array.isArray(result.tables)) {
		throw new Error("result.tables must be null or an array");
	}

	// Detected languages can be null or array
	if (
		result.detectedLanguages !== null &&
		!Array.isArray(result.detectedLanguages)
	) {
		throw new Error("result.detectedLanguages must be null or an array");
	}

	// Chunks can be undefined, null, or array
	if (
		result.chunks !== undefined &&
		result.chunks !== null &&
		!Array.isArray(result.chunks)
	) {
		throw new Error("result.chunks must be undefined, null, or an array");
	}
}

/**
 * Assert that metadata has expected structure.
 */
export function assertValidMetadata(metadata: Metadata): void {
	if (typeof metadata !== "object" || metadata === null) {
		throw new Error("metadata must be an object");
	}

	// Check optional fields have correct types if present
	if (metadata.language !== undefined && metadata.language !== null) {
		if (typeof metadata.language !== "string") {
			throw new Error("metadata.language must be a string or null");
		}
	}

	if (metadata.date !== undefined && metadata.date !== null) {
		if (typeof metadata.date !== "string") {
			throw new Error("metadata.date must be a string or null");
		}
	}

	if (metadata.format !== undefined && metadata.format !== null) {
		if (typeof metadata.format !== "string") {
			throw new Error("metadata.format must be a string or null");
		}
	}
}

/**
 * Assert that extraction throws an error with expected message pattern.
 */
export async function assertExtractionError(
	fn: () => Promise<ExtractionResult> | ExtractionResult,
	expectedMessagePattern?: RegExp | string,
): Promise<void> {
	let error: Error | null = null;

	try {
		await fn();
	} catch (e) {
		error = e as Error;
	}

	if (!error) {
		throw new Error("Expected extraction to throw an error, but it succeeded");
	}

	if (expectedMessagePattern) {
		const pattern =
			typeof expectedMessagePattern === "string"
				? new RegExp(expectedMessagePattern)
				: expectedMessagePattern;

		if (!pattern.test(error.message)) {
			throw new Error(
				`Error message "${error.message}" does not match expected pattern ${pattern}`,
			);
		}
	}
}

/**
 * Create a temporary test file for testing.
 */
export function createTempFile(content: Buffer | string, extension = "txt"): {
	path: string;
	cleanup: () => void;
} {
	const tmpPath = join(
		process.cwd(),
		`temp-test-${Date.now()}-${Math.random().toString(36).slice(2)}.${extension}`,
	);
	const fs = require("node:fs");

	if (typeof content === "string") {
		fs.writeFileSync(tmpPath, content, "utf-8");
	} else {
		fs.writeFileSync(tmpPath, content);
	}

	return {
		path: tmpPath,
		cleanup: () => {
			try {
				fs.unlinkSync(tmpPath);
			} catch {}
		},
	};
}

/**
 * Sleep for specified milliseconds (useful for async tests).
 */
export function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}
