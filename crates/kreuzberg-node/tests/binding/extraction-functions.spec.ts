import { readFileSync } from "node:fs";
import { join } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import {
	batchExtractBytes,
	batchExtractFiles,
	batchExtractFilesSync,
	extractBytes,
	extractBytesSync,
} from "../../dist/index.js";

function getTestDocumentPath(relativePath: string): string {
	const workspaceRoot = join(process.cwd(), "../..");
	return join(workspaceRoot, "test_documents", relativePath);
}

let samplePdfPath: string;
let samplePdfBytes: Uint8Array;
let sampleTextPath: string;
let sampleTextBytes: Uint8Array;

beforeAll(() => {
	samplePdfPath = getTestDocumentPath("pdfs/code_and_formula.pdf");
	samplePdfBytes = new Uint8Array(readFileSync(samplePdfPath));

	sampleTextPath = getTestDocumentPath("pandoc/simple_metadata.md");
	sampleTextBytes = new Uint8Array(readFileSync(sampleTextPath));
});

describe("extractBytesSync", () => {
	it("should extract PDF from bytes synchronously", () => {
		const result = extractBytesSync(samplePdfBytes, "application/pdf", null);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
		expect(typeof result.content).toBe("string");
		expect(result.mimeType).toContain("application/pdf");
		expect(result.metadata).toBeTruthy();
		expect(Array.isArray(result.tables)).toBe(true);
	});

	it("should extract text from bytes synchronously", () => {
		const result = extractBytesSync(sampleTextBytes, "text/markdown", null);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
		expect(result.mimeType).toContain("text/markdown");
	});

	it("should handle extraction config with bytes", () => {
		const config = {
			useCache: false,
			enableQualityProcessing: true,
		};
		const result = extractBytesSync(samplePdfBytes, "application/pdf", config);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});

	it("should handle Uint8Array conversion to Buffer", () => {
		const uint8Data = new Uint8Array(samplePdfBytes);
		const result = extractBytesSync(uint8Data, "application/pdf", null);

		expect(result).toBeTruthy();
	});
});

describe("extractBytes (async)", () => {
	it("should extract PDF from bytes asynchronously", async () => {
		const result = await extractBytes(samplePdfBytes, "application/pdf", null);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
		expect(typeof result.content).toBe("string");
		expect(result.mimeType).toContain("application/pdf");
		expect(result.metadata).toBeTruthy();
		expect(Array.isArray(result.tables)).toBe(true);
	});

	it("should extract text from bytes asynchronously", async () => {
		const result = await extractBytes(sampleTextBytes, "text/markdown", null);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
		expect(result.mimeType).toContain("text/markdown");
	});

	it("should handle extraction config with async bytes", async () => {
		const config = {
			useCache: false,
			enableQualityProcessing: false,
		};
		const result = await extractBytes(samplePdfBytes, "application/pdf", config);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});

	it("should handle Uint8Array conversion to Buffer (async)", async () => {
		const uint8Data = new Uint8Array(samplePdfBytes);
		const result = await extractBytes(uint8Data, "application/pdf", null);

		expect(result).toBeTruthy();
	});
});

describe("batchExtractFilesSync", () => {
	it("should extract multiple files synchronously", () => {
		const paths = [samplePdfPath, sampleTextPath];
		const results = batchExtractFilesSync(paths, null);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(2);

		expect(results[0].content).toBeTruthy();
		expect(results[0].mimeType).toContain("application/pdf");

		expect(results[1].content).toBeTruthy();
		expect(results[1].mimeType).toContain("text/markdown");
	});

	it("should handle empty file list", () => {
		const results = batchExtractFilesSync([], null);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(0);
	});

	it("should handle single file in batch", () => {
		const results = batchExtractFilesSync([samplePdfPath], null);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should handle extraction config in batch sync", () => {
		const config = {
			useCache: false,
			enableQualityProcessing: true,
		};
		const results = batchExtractFilesSync([samplePdfPath], config);

		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});
});

describe("batchExtractFiles (async)", () => {
	it("should extract multiple files asynchronously", async () => {
		const paths = [samplePdfPath, sampleTextPath];
		const results = await batchExtractFiles(paths, null);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(2);

		expect(results[0].content).toBeTruthy();
		expect(results[0].mimeType).toContain("application/pdf");

		expect(results[1].content).toBeTruthy();
		expect(results[1].mimeType).toContain("text/markdown");
	});

	it("should handle empty file list (async)", async () => {
		const results = await batchExtractFiles([], null);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(0);
	});

	it("should handle single file in batch (async)", async () => {
		const results = await batchExtractFiles([samplePdfPath], null);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should handle extraction config in batch async", async () => {
		const config = {
			useCache: true,
			enableQualityProcessing: false,
		};
		const results = await batchExtractFiles([samplePdfPath], config);

		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should handle large batch of files", async () => {
		const paths = [samplePdfPath, sampleTextPath, samplePdfPath];
		const results = await batchExtractFiles(paths, null);

		expect(results.length).toBe(3);
		results.forEach((result) => {
			expect(result.content).toBeTruthy();
		});
	});
});

describe("batchExtractBytes (async)", () => {
	it("should extract multiple byte arrays asynchronously", async () => {
		const dataList = [samplePdfBytes, sampleTextBytes];
		const mimeTypes = ["application/pdf", "text/markdown"];
		const results = await batchExtractBytes(dataList, mimeTypes, null);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(2);

		expect(results[0].content).toBeTruthy();
		expect(results[0].mimeType).toContain("application/pdf");

		expect(results[1].content).toBeTruthy();
		expect(results[1].mimeType).toContain("text/markdown");
	});

	it("should handle empty data list", async () => {
		const results = await batchExtractBytes([], [], null);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(0);
	});

	it("should handle single byte array in batch", async () => {
		const results = await batchExtractBytes([samplePdfBytes], ["application/pdf"], null);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should handle extraction config in batch bytes", async () => {
		const config = {
			useCache: false,
			enableQualityProcessing: true,
		};
		const results = await batchExtractBytes([samplePdfBytes], ["application/pdf"], config);

		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should convert Uint8Array to Buffer for each item", async () => {
		const data1 = new Uint8Array(samplePdfBytes);
		const data2 = new Uint8Array(sampleTextBytes);

		const results = await batchExtractBytes([data1, data2], ["application/pdf", "text/plain"], null);

		expect(results.length).toBe(2);
		results.forEach((result) => {
			expect(result.content).toBeTruthy();
		});
	});

	it("should handle large batch of byte arrays", async () => {
		const dataList = [samplePdfBytes, sampleTextBytes, samplePdfBytes];
		const mimeTypes = ["application/pdf", "text/plain", "application/pdf"];
		const results = await batchExtractBytes(dataList, mimeTypes, null);

		expect(results.length).toBe(3);
		results.forEach((result) => {
			expect(result.content).toBeTruthy();
		});
	});
});
