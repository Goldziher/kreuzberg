import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import archiver from "archiver";
import { expect } from "vitest";
import type { ExtractionResult, Metadata } from "../../src/types.js";

/**
 * Resolve the workspace root robustly regardless of the current working directory.
 */
function resolveWorkspaceRoot(): string {
	const envRoot = process.env.KREUZBERG_WORKSPACE_ROOT ?? process.env.GITHUB_WORKSPACE;
	if (envRoot && existsSync(envRoot)) {
		return envRoot;
	}

	let current = process.cwd();
	while (true) {
		if (existsSync(join(current, "Cargo.toml"))) {
			return current;
		}
		const parent = dirname(current);
		if (parent === current) {
			break;
		}
		current = parent;
	}

	// Fallback to original behaviour (two levels up) to avoid breaking local runs
	return join(process.cwd(), "../..");
}

/**
 * Get path to test document in the repository's test_documents directory.
 *
 * @param relativePath - Path relative to test_documents (e.g., "pdfs/simple.pdf")
 * @returns Absolute path to the test document
 */
export function getTestDocumentPath(relativePath: string): string {
	const workspaceRoot = resolveWorkspaceRoot();
	return join(workspaceRoot, "test_documents", relativePath);
}

/**
 * Check if test documents directory is available.
 *
 * @returns true if test_documents exists
 */
export function testDocumentsAvailable(): boolean {
	const workspaceRoot = resolveWorkspaceRoot();
	return existsSync(join(workspaceRoot, "test_documents"));
}

/**
 * Assert that extraction result has expected MIME type.
 *
 * @param result - Extraction result
 * @param expectedMimeType - Expected MIME type (can be partial match)
 */
export function assertMimeType(result: ExtractionResult, expectedMimeType: string): void {
	expect(result.mimeType).toContain(expectedMimeType);
}

/**
 * Assert that extraction result has non-empty content.
 *
 * @param result - Extraction result
 * @param minLength - Minimum content length (default: 1)
 */
export function assertNonEmptyContent(result: ExtractionResult, minLength = 1): void {
	expect(result.content).toBeTruthy();
	expect(result.content.length).toBeGreaterThanOrEqual(minLength);
}

/**
 * Assert that extraction result structure is valid.
 * Validates all required fields exist and have correct types.
 *
 * @param result - Extraction result
 */
export function assertValidExtractionResult(result: ExtractionResult): void {
	expect(result).toHaveProperty("content");
	expect(result).toHaveProperty("mimeType");
	expect(result).toHaveProperty("metadata");
	expect(result).toHaveProperty("tables");
	expect(result).toHaveProperty("detectedLanguages");
	expect(result).toHaveProperty("chunks");
	expect(result).toHaveProperty("images");

	expect(typeof result.content).toBe("string");
	expect(typeof result.mimeType).toBe("string");
	expect(typeof result.metadata).toBe("object");
	expect(result.metadata).not.toBeNull();
	expect(Array.isArray(result.tables)).toBe(true);

	if (result.detectedLanguages !== null) {
		expect(Array.isArray(result.detectedLanguages)).toBe(true);
	}

	if (result.chunks !== null) {
		expect(Array.isArray(result.chunks)).toBe(true);
	}

	if (result.images !== null) {
		expect(Array.isArray(result.images)).toBe(true);
	}
}

/**
 * Assert that metadata contains expected PDF fields.
 * Note: PDF metadata is optional - this validates structure if present.
 *
 * @param metadata - Extraction metadata
 */
export function assertPdfMetadata(metadata: Metadata): void {
	if (metadata.pdf) {
		if (metadata.pdf.pageCount !== undefined) {
			expect(metadata.pdf.pageCount).toBeGreaterThan(0);
		}
	}
}

/**
 * Assert that metadata contains expected Excel fields.
 * Note: Excel metadata is optional - this validates structure if present.
 *
 * @param metadata - Extraction metadata
 */
export function assertExcelMetadata(metadata: Metadata): void {
	if (metadata.excel) {
		if (metadata.excel.sheetCount !== undefined) {
			expect(metadata.excel.sheetCount).toBeGreaterThan(0);
		}
		if (metadata.excel.sheetNames !== undefined) {
			expect(Array.isArray(metadata.excel.sheetNames)).toBe(true);
		}
	}
}

/**
 * Assert that metadata contains expected image fields.
 *
 * @param metadata - Extraction metadata
 */
export function assertImageMetadata(metadata: Metadata): void {
	if (metadata.image) {
		expect(metadata.image.width).toBeTruthy();
		expect(metadata.image.height).toBeTruthy();
		expect(metadata.image.width).toBeGreaterThan(0);
		expect(metadata.image.height).toBeGreaterThan(0);

		if (metadata.image.format) {
			expect(typeof metadata.image.format).toBe("string");
		}
		return;
	}

	expect(typeof (metadata as any).width).toBe("number");
	expect(typeof (metadata as any).height).toBe("number");
	expect((metadata as any).width).toBeGreaterThan(0);
	expect((metadata as any).height).toBeGreaterThan(0);

	if ((metadata as any).format) {
		expect(typeof (metadata as any).format).toBe("string");
	}
}

/**
 * Assert that OCR result contains expected text with confidence validation.
 *
 * @param result - Extraction result
 * @param expectedWords - Words expected in the content
 * @param minConfidence - Minimum acceptable confidence (default: 0.3)
 */
export function assertOcrResult(result: ExtractionResult, expectedWords: string[], minConfidence = 0.3): void {
	assertValidExtractionResult(result);

	const contentLower = result.content.toLowerCase().replace(/\n/g, " ").trim();

	const foundWords = expectedWords.filter((word) => contentLower.includes(word.toLowerCase()));

	expect(foundWords.length).toBeGreaterThan(0);

	if (result.metadata.ocr) {
		const metadata: any = result.metadata;
		if (metadata.confidence !== undefined) {
			expect(metadata.confidence).toBeGreaterThanOrEqual(0.0);
			expect(metadata.confidence).toBeLessThanOrEqual(1.0);

			if (foundWords.length > 0) {
				expect(metadata.confidence).toBeGreaterThanOrEqual(minConfidence);
			}
		}
	}
}

/**
 * Assert that result contains substantial content (for large documents).
 *
 * @param result - Extraction result
 * @param minBytes - Minimum content size in bytes
 */
export function assertSubstantialContent(result: ExtractionResult, minBytes = 1000): void {
	assertNonEmptyContent(result, minBytes);
	expect(result.content.length).toBeGreaterThanOrEqual(minBytes);
}

/**
 * Assert that tables were extracted.
 *
 * @param result - Extraction result
 * @param minTables - Minimum number of tables expected
 */
export function assertTablesExtracted(result: ExtractionResult, minTables = 1): void {
	expect(result.tables.length).toBeGreaterThanOrEqual(minTables);

	for (const table of result.tables) {
		expect(table.cells).toBeTruthy();
		expect(Array.isArray(table.cells)).toBe(true);
		expect(table.cells.length).toBeGreaterThan(0);

		expect(Array.isArray(table.cells[0])).toBe(true);

		expect(table.markdown).toBeTruthy();
		expect(typeof table.markdown).toBe("string");
	}
}

/**
 * Assert that HTML was converted to Markdown.
 *
 * @param result - Extraction result
 */
export function assertMarkdownConversion(result: ExtractionResult): void {
	assertNonEmptyContent(result);

	const hasHeaders = result.content.includes("##") || result.content.includes("#");
	const hasTables = result.content.includes("|");
	const hasLinks = result.content.includes("[");
	const hasBold = result.content.includes("**");

	expect(hasHeaders || hasTables || hasLinks || hasBold).toBe(true);
}

/**
 * Create a ZIP archive from a map of file paths to contents.
 *
 * @param files - Map of file paths to file contents (string or Buffer)
 * @returns Promise resolving to ZIP archive as Uint8Array
 */
export async function createZip(files: Record<string, string | Buffer | Uint8Array>): Promise<Uint8Array> {
	return new Promise<Uint8Array>((resolve, reject) => {
		const archive = archiver("zip", { zlib: { level: 9 } });
		const chunks: Buffer[] = [];

		archive.on("data", (chunk: Buffer) => {
			chunks.push(chunk);
		});

		archive.on("end", () => {
			resolve(new Uint8Array(Buffer.concat(chunks)));
		});

		archive.on("error", (err) => {
			reject(err);
		});

		for (const [path, content] of Object.entries(files)) {
			if (typeof content === "string") {
				archive.append(content, { name: path });
			} else {
				archive.append(content instanceof Uint8Array ? Buffer.from(content) : (content as Buffer), { name: path });
			}
		}

		archive.finalize();
	});
}

/**
 * Create a TAR archive from a map of file paths to contents.
 *
 * @param files - Map of file paths to file contents (string or Buffer)
 * @returns Promise resolving to TAR archive as Uint8Array
 */
export async function createTar(files: Record<string, string | Buffer | Uint8Array>): Promise<Uint8Array> {
	return new Promise<Uint8Array>((resolve, reject) => {
		const archive = archiver("tar", {});
		const chunks: Buffer[] = [];

		archive.on("data", (chunk: Buffer) => {
			chunks.push(chunk);
		});

		archive.on("end", () => {
			resolve(new Uint8Array(Buffer.concat(chunks)));
		});

		archive.on("error", (err) => {
			reject(err);
		});

		for (const [path, content] of Object.entries(files)) {
			if (typeof content === "string") {
				archive.append(content, { name: path });
			} else {
				archive.append(content instanceof Uint8Array ? Buffer.from(content) : (content as Buffer), { name: path });
			}
		}

		archive.finalize();
	});
}
