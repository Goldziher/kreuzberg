import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { detectMimeTypeFromPath, validateMimeType } from "../../src/index.js";

describe("MIME Utility Functions", () => {
	let tempDir: string;

	beforeEach(() => {
		// Create a unique temp directory for each test
		tempDir = join(tmpdir(), `kreuzberg-mime-test-${Date.now()}`);
		if (!existsSync(tempDir)) {
			mkdirSync(tempDir, { recursive: true });
		}
	});

	afterEach(() => {
		// Cleanup is optional for temp files, OS will clean up eventually
	});

	describe("detectMimeTypeFromPath", () => {
		it("should detect MIME type for PDF files", () => {
			const testFile = join(tempDir, "test.pdf");
			writeFileSync(testFile, "dummy content");

			const mimeType = detectMimeTypeFromPath(testFile);
			expect(mimeType).toBe("application/pdf");
		});

		it("should detect MIME type for DOCX files", () => {
			const testFile = join(tempDir, "test.docx");
			writeFileSync(testFile, "dummy content");

			const mimeType = detectMimeTypeFromPath(testFile);
			expect(mimeType).toBe("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
		});

		it("should detect MIME type for image files", () => {
			const testCases = [
				{ ext: "png", expected: "image/png" },
				{ ext: "jpg", expected: "image/jpeg" },
				{ ext: "jpeg", expected: "image/jpeg" },
				{ ext: "gif", expected: "image/gif" },
				{ ext: "webp", expected: "image/webp" },
				{ ext: "tiff", expected: "image/tiff" },
			];

			for (const { ext, expected } of testCases) {
				const testFile = join(tempDir, `test.${ext}`);
				writeFileSync(testFile, "dummy content");

				const mimeType = detectMimeTypeFromPath(testFile);
				expect(mimeType).toBe(expected);
			}
		});

		it("should detect MIME type for Office document files", () => {
			const testCases = [
				{ ext: "xlsx", expected: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" },
				{ ext: "pptx", expected: "application/vnd.openxmlformats-officedocument.presentationml.presentation" },
				{ ext: "doc", expected: "application/msword" },
				{ ext: "ppt", expected: "application/vnd.ms-powerpoint" },
			];

			for (const { ext, expected } of testCases) {
				const testFile = join(tempDir, `test.${ext}`);
				writeFileSync(testFile, "dummy content");

				const mimeType = detectMimeTypeFromPath(testFile);
				expect(mimeType).toBe(expected);
			}
		});

		it("should detect MIME type for data format files", () => {
			const testCases = [
				{ ext: "json", expected: "application/json" },
				{ ext: "yaml", expected: "application/x-yaml" },
				{ ext: "yml", expected: "application/x-yaml" },
				{ ext: "xml", expected: "application/xml" },
				{ ext: "csv", expected: "text/csv" },
				{ ext: "toml", expected: "application/toml" },
			];

			for (const { ext, expected } of testCases) {
				const testFile = join(tempDir, `test.${ext}`);
				writeFileSync(testFile, "dummy content");

				const mimeType = detectMimeTypeFromPath(testFile);
				expect(mimeType).toBe(expected);
			}
		});

		it("should detect MIME type for text files", () => {
			const testCases = [
				{ ext: "txt", expected: "text/plain" },
				{ ext: "md", expected: "text/markdown" },
				{ ext: "html", expected: "text/html" },
				{ ext: "htm", expected: "text/html" },
			];

			for (const { ext, expected } of testCases) {
				const testFile = join(tempDir, `test.${ext}`);
				writeFileSync(testFile, "dummy content");

				const mimeType = detectMimeTypeFromPath(testFile);
				expect(mimeType).toBe(expected);
			}
		});

		it("should handle case-insensitive extensions", () => {
			const testFile = join(tempDir, "test.PDF");
			writeFileSync(testFile, "dummy content");

			const mimeType = detectMimeTypeFromPath(testFile);
			expect(mimeType).toBe("application/pdf");
		});

		it("should work without checking file existence when checkExists is false", () => {
			const nonExistentFile = join(tempDir, "nonexistent.pdf");

			// Should not throw when checkExists is false (default)
			const mimeType = detectMimeTypeFromPath(nonExistentFile, false);
			expect(mimeType).toBe("application/pdf");
		});

		it("should throw error for non-existent file when checkExists is true", () => {
			const nonExistentFile = join(tempDir, "nonexistent.pdf");

			expect(() => detectMimeTypeFromPath(nonExistentFile, true)).toThrow(/does not exist/i);
		});

		it("should throw error for unknown extensions", () => {
			const testFile = join(tempDir, "test.unknownext");
			writeFileSync(testFile, "dummy content");

			expect(() => detectMimeTypeFromPath(testFile)).toThrow(/unknown extension/i);
		});

		it("should throw error for files without extension", () => {
			const testFile = join(tempDir, "testfile");
			writeFileSync(testFile, "dummy content");

			expect(() => detectMimeTypeFromPath(testFile)).toThrow(/(could not determine mime type|Failed to create reference)/i);
		});
	});

	describe("validateMimeType", () => {
		it("should validate supported MIME types", () => {
			const validTypes = ["application/pdf", "text/plain", "text/html", "application/json", "text/markdown"];

			for (const mimeType of validTypes) {
				const result = validateMimeType(mimeType);
				expect(result).toBe(mimeType);
			}
		});

		it("should validate image MIME types", () => {
			const imageTypes = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/tiff"];

			for (const mimeType of imageTypes) {
				const result = validateMimeType(mimeType);
				expect(result).toBe(mimeType);
			}
		});

		it("should validate any image/* MIME type", () => {
			// Rust implementation allows any image/* MIME type
			const customImageType = "image/custom-format";
			const result = validateMimeType(customImageType);
			expect(result).toBe(customImageType);
		});

		it("should validate Office document MIME types", () => {
			const officeTypes = [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
				"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
				"application/vnd.openxmlformats-officedocument.presentationml.presentation",
				"application/msword",
				"application/vnd.ms-powerpoint",
				"application/vnd.ms-excel",
			];

			for (const mimeType of officeTypes) {
				const result = validateMimeType(mimeType);
				expect(result).toBe(mimeType);
			}
		});

		it("should throw error for unsupported MIME types", () => {
			const unsupportedTypes = ["video/mp4", "audio/mpeg", "application/unknown", "invalid/mime-type"];

			for (const mimeType of unsupportedTypes) {
				expect(() => validateMimeType(mimeType)).toThrow(/unsupported format/i);
			}
		});

		it("should validate email MIME types", () => {
			const emailTypes = ["message/rfc822", "application/vnd.ms-outlook"];

			for (const mimeType of emailTypes) {
				const result = validateMimeType(mimeType);
				expect(result).toBe(mimeType);
			}
		});
	});
});
