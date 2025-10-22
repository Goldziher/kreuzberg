import { describe, expect, it } from "vitest";
import { extractBytes, extractBytesSync, extractFile, extractFileSync } from "../../src/index.js";
import { assertValidExtractionResult, createTempFile } from "../helpers/test-utils.js";

describe("Basic Extraction - Sync", () => {
	it("should extract simple text file synchronously", () => {
		const tempFile = createTempFile("Hello, World!");

		try {
			const result = extractFileSync(tempFile.path);

			assertValidExtractionResult(result);
			expect(result.content).toContain("Hello, World!");
			expect(result.mimeType).toBeTruthy();
		} finally {
			tempFile.cleanup();
		}
	});

	it("should extract with null config", () => {
		const tempFile = createTempFile("Test content");

		try {
			const result = extractFileSync(tempFile.path, null, null);

			assertValidExtractionResult(result);
			expect(result.content).toContain("Test content");
		} finally {
			tempFile.cleanup();
		}
	});

	it("should extract with undefined config", () => {
		const tempFile = createTempFile("Test content");

		try {
			const result = extractFileSync(tempFile.path, undefined, undefined);

			assertValidExtractionResult(result);
			expect(result.content).toContain("Test content");
		} finally {
			tempFile.cleanup();
		}
	});

	it("should extract with MIME type hint", () => {
		const tempFile = createTempFile("Plain text content", "txt");

		try {
			const result = extractFileSync(tempFile.path, "text/plain");

			assertValidExtractionResult(result);
			expect(result.mimeType).toBe("text/plain");
			expect(result.content).toContain("Plain text content");
		} finally {
			tempFile.cleanup();
		}
	});

	it("should extract bytes synchronously", () => {
		const content = "Hello from bytes!";
		const buffer = Buffer.from(content, "utf-8");

		const result = extractBytesSync(buffer, "text/plain");

		assertValidExtractionResult(result);
		expect(result.content).toContain("Hello from bytes!");
		expect(result.mimeType).toBe("text/plain");
	});
});

describe("Basic Extraction - Async", () => {
	it("should extract simple text file asynchronously", async () => {
		const tempFile = createTempFile("Async Hello, World!");

		try {
			const result = await extractFile(tempFile.path);

			assertValidExtractionResult(result);
			expect(result.content).toContain("Async Hello, World!");
			expect(result.mimeType).toBeTruthy();
		} finally {
			tempFile.cleanup();
		}
	});

	it("should extract with null config async", async () => {
		const tempFile = createTempFile("Async test content");

		try {
			const result = await extractFile(tempFile.path, null, null);

			assertValidExtractionResult(result);
			expect(result.content).toContain("Async test content");
		} finally {
			tempFile.cleanup();
		}
	});

	it("should extract bytes asynchronously", async () => {
		const content = "Async hello from bytes!";
		const buffer = Buffer.from(content, "utf-8");

		const result = await extractBytes(buffer, "text/plain");

		assertValidExtractionResult(result);
		expect(result.content).toContain("Async hello from bytes!");
		expect(result.mimeType).toBe("text/plain");
	});
});

describe("Extraction Result Structure", () => {
	it("should have all required fields", () => {
		const tempFile = createTempFile("Test");

		try {
			const result = extractFileSync(tempFile.path);

			expect(result).toHaveProperty("content");
			expect(result).toHaveProperty("mimeType");
			expect(result).toHaveProperty("metadata");
			expect(result).toHaveProperty("tables");
			expect(result).toHaveProperty("detectedLanguages");

			expect(typeof result.content).toBe("string");
			expect(typeof result.mimeType).toBe("string");
			expect(typeof result.metadata).toBe("object");
			expect(result.metadata).not.toBeNull();
		} finally {
			tempFile.cleanup();
		}
	});

	it("should have tables as null or array", () => {
		const tempFile = createTempFile("Test");

		try {
			const result = extractFileSync(tempFile.path);

			if (result.tables !== null) {
				expect(Array.isArray(result.tables)).toBe(true);
			}
		} finally {
			tempFile.cleanup();
		}
	});

	it("should have detectedLanguages as null or array", () => {
		const tempFile = createTempFile("Test");

		try {
			const result = extractFileSync(tempFile.path);

			if (result.detectedLanguages !== null) {
				expect(Array.isArray(result.detectedLanguages)).toBe(true);
			}
		} finally {
			tempFile.cleanup();
		}
	});
});

describe("Metadata Parsing", () => {
	it("should parse metadata as object", () => {
		const tempFile = createTempFile("Test content");

		try {
			const result = extractFileSync(tempFile.path);

			expect(typeof result.metadata).toBe("object");
			expect(result.metadata).not.toBeNull();
		} finally {
			tempFile.cleanup();
		}
	});

	it("should handle metadata with optional fields", () => {
		const tempFile = createTempFile("Test");

		try {
			const result = extractFileSync(tempFile.path);

			if ("language" in result.metadata && result.metadata.language !== null) {
				expect(typeof result.metadata.language).toBe("string");
			}

			if ("date" in result.metadata && result.metadata.date !== null) {
				expect(typeof result.metadata.date).toBe("string");
			}

			if ("format" in result.metadata && result.metadata.format !== null) {
				expect(typeof result.metadata.format).toBe("string");
			}
		} finally {
			tempFile.cleanup();
		}
	});
});
