/**
 * Extended coverage for sync/async extraction functions.
 *
 * Tests for:
 * - extractFileSync
 * - extractFile
 * - extractBytesSync
 * - extractBytes
 * - batchExtractFilesSync
 * - batchExtractFiles
 * - batchExtractBytesSync
 * - batchExtractBytes
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import {
	batchExtractBytes,
	batchExtractBytesSync,
	batchExtractFiles,
	batchExtractFilesSync,
	extractBytes,
	extractBytesSync,
	extractFile,
	extractFileSync,
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

describe("extractFileSync - comprehensive tests", () => {
	it("should extract PDF file synchronously without config", () => {
		const result = extractFileSync(samplePdfPath);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
		expect(typeof result.content).toBe("string");
		expect(result.mimeType).toContain("pdf");
		expect(result.metadata).toBeTruthy();
	});

	it("should extract with explicit null config", () => {
		const result = extractFileSync(samplePdfPath, null, null);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});

	it("should extract with explicit MIME type", () => {
		const result = extractFileSync(samplePdfPath, "application/pdf", null);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});

	it("should extract text file synchronously", () => {
		const result = extractFileSync(sampleTextPath);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
		expect(result.mimeType).toContain("text");
	});

	it("should return metadata with content", () => {
		const result = extractFileSync(samplePdfPath);

		expect(result.metadata).toBeDefined();
		expect(typeof result.metadata).toBe("object");
	});

	it("should return tables array", () => {
		const result = extractFileSync(samplePdfPath);

		expect(Array.isArray(result.tables)).toBe(true);
	});

	it("should handle config with useCache false", () => {
		const result = extractFileSync(samplePdfPath, null, { useCache: false });

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});
});

describe("extractFile - comprehensive async tests", () => {
	it("should extract PDF file asynchronously", async () => {
		const result = await extractFile(samplePdfPath);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
		expect(typeof result.content).toBe("string");
	});

	it("should extract with null MIME type (auto-detect)", async () => {
		const result = await extractFile(samplePdfPath, null);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});

	it("should extract with explicit MIME type", async () => {
		const result = await extractFile(samplePdfPath, "application/pdf");

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});

	it("should extract with config", async () => {
		const result = await extractFile(samplePdfPath, null, {
			useCache: false,
			enableQualityProcessing: true,
		});

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});

	it("should extract text file asynchronously", async () => {
		const result = await extractFile(sampleTextPath);

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});

	it("should handle full config object", async () => {
		const result = await extractFile(samplePdfPath, null, {
			useCache: false,
			enableQualityProcessing: false,
			forceOcr: false,
		});

		expect(result).toBeTruthy();
		expect(result.content).toBeTruthy();
	});
});

describe("batchExtractFilesSync - comprehensive tests", () => {
	it("should extract single file in batch", () => {
		const results = batchExtractFilesSync([samplePdfPath]);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should extract multiple files", () => {
		const results = batchExtractFilesSync([samplePdfPath, sampleTextPath]);

		expect(results.length).toBe(2);
		expect(results[0].content).toBeTruthy();
		expect(results[1].content).toBeTruthy();
	});

	it("should maintain order of results", () => {
		const paths = [samplePdfPath, sampleTextPath, samplePdfPath];
		const results = batchExtractFilesSync(paths);

		expect(results.length).toBe(3);
		expect(results[0].mimeType).toContain("pdf");
		expect(results[1].mimeType).toContain("text");
		expect(results[2].mimeType).toContain("pdf");
	});

	it("should batch extract with config", () => {
		const results = batchExtractFilesSync([samplePdfPath], {
			useCache: false,
		});

		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should batch extract with null config", () => {
		const results = batchExtractFilesSync([samplePdfPath, sampleTextPath], null);

		expect(results.length).toBe(2);
	});
});

describe("batchExtractFiles - comprehensive async tests", () => {
	it("should extract single file asynchronously in batch", async () => {
		const results = await batchExtractFiles([samplePdfPath]);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should extract multiple files asynchronously", async () => {
		const results = await batchExtractFiles([samplePdfPath, sampleTextPath]);

		expect(results.length).toBe(2);
		expect(results[0].content).toBeTruthy();
		expect(results[1].content).toBeTruthy();
	});

	it("should batch extract with configuration", async () => {
		const results = await batchExtractFiles([samplePdfPath], {
			useCache: false,
			enableQualityProcessing: true,
		});

		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should handle multiple identical files", async () => {
		const results = await batchExtractFiles([samplePdfPath, samplePdfPath]);

		expect(results.length).toBe(2);
		expect(results[0].mimeType).toBe(results[1].mimeType);
	});
});

describe("batchExtractBytesSync - comprehensive tests", () => {
	it("should batch extract single bytes synchronously", () => {
		const results = batchExtractBytesSync([samplePdfBytes], ["application/pdf"]);

		expect(Array.isArray(results)).toBe(true);
		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should batch extract multiple bytes", () => {
		const results = batchExtractBytesSync([samplePdfBytes, sampleTextBytes], ["application/pdf", "text/markdown"]);

		expect(results.length).toBe(2);
		expect(results[0].mimeType).toContain("pdf");
		expect(results[1].mimeType).toContain("text");
	});

	it("should validate matching data and MIME type counts", () => {
		expect(() => {
			batchExtractBytesSync([samplePdfBytes, sampleTextBytes], ["application/pdf"]);
		}).toThrow();
	});

	it("should accept null config", () => {
		const results = batchExtractBytesSync([samplePdfBytes], ["application/pdf"], null);

		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should accept extraction config", () => {
		const results = batchExtractBytesSync([samplePdfBytes], ["application/pdf"], {
			useCache: false,
		});

		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should maintain order of batch results", () => {
		const dataList = [samplePdfBytes, sampleTextBytes, samplePdfBytes];
		const mimeTypes = ["application/pdf", "text/markdown", "application/pdf"];

		const results = batchExtractBytesSync(dataList, mimeTypes);

		expect(results.length).toBe(3);
		expect(results[0].mimeType).toContain("pdf");
		expect(results[1].mimeType).toContain("text");
		expect(results[2].mimeType).toContain("pdf");
	});
});

describe("batchExtractBytes - comprehensive async tests", () => {
	it("should batch extract bytes asynchronously", async () => {
		const results = await batchExtractBytes([samplePdfBytes], ["application/pdf"]);

		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should batch extract multiple byte arrays", async () => {
		const results = await batchExtractBytes([samplePdfBytes, sampleTextBytes], ["application/pdf", "text/markdown"]);

		expect(results.length).toBe(2);
		expect(results[0].mimeType).toContain("pdf");
		expect(results[1].mimeType).toContain("text");
	});

	it("should validate array lengths match", async () => {
		await expect(batchExtractBytes([samplePdfBytes, sampleTextBytes], ["application/pdf"])).rejects.toThrow();
	});

	it("should batch extract with config", async () => {
		const results = await batchExtractBytes([samplePdfBytes], ["application/pdf"], {
			useCache: false,
			enableQualityProcessing: true,
		});

		expect(results.length).toBe(1);
		expect(results[0].content).toBeTruthy();
	});

	it("should handle multiple identical byte arrays", async () => {
		const results = await batchExtractBytes([samplePdfBytes, samplePdfBytes], ["application/pdf", "application/pdf"]);

		expect(results.length).toBe(2);
		expect(results[0].mimeType).toBe(results[1].mimeType);
	});
});

describe("Extraction function argument validation", () => {
	it("extractBytesSync should validate Uint8Array argument", () => {
		expect(() => {
			extractBytesSync("not a uint8array" as any, "application/pdf");
		}).toThrow();
	});

	it("batchExtractBytesSync should validate Uint8Array array", () => {
		expect(() => {
			batchExtractBytesSync(["not uint8array"] as any, ["application/pdf"]);
		}).toThrow();
	});

	it("extractBytes should validate Uint8Array argument", async () => {
		await expect(extractBytes("not a uint8array" as any, "application/pdf")).rejects.toThrow();
	});

	it("batchExtractBytes should validate Uint8Array array length", async () => {
		await expect(batchExtractBytes([samplePdfBytes, sampleTextBytes], ["application/pdf"])).rejects.toThrow();
	});
});
