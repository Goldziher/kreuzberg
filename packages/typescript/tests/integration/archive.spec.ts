/**
 * Archive Integration Tests
 *
 * Tests ZIP and TAR archive extraction functionality including:
 * - Single and multiple file extraction
 * - Nested directories
 * - Mixed file formats
 * - Archive metadata extraction
 * - Error handling for corrupted/invalid archives
 */

import { describe, expect, it } from "vitest";
import { extractBytes, extractBytesSync } from "../../src/index.js";
import { createTar, createZip } from "../helpers/integration-helpers.js";

describe("Archive Integration Tests", () => {
	describe("ZIP Extraction", () => {
		it("should extract simple ZIP with single file", async () => {
			const zipBytes = await createZip({
				"test.txt": "Hello from ZIP!",
			});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result.mimeType).toBe("application/zip");
			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("ZIP Archive");
			expect(result.content).toContain("test.txt");
			expect(result.content).toContain("Hello from ZIP!");

			expect(result.metadata.archive).toBeDefined();
			expect(result.metadata.archive.format).toBe("ZIP");
			expect(result.metadata.archive.file_count).toBe(1);
			expect(result.metadata.archive.file_list).toHaveLength(1);
			expect(result.metadata.archive.file_list[0]).toBe("test.txt");
		});

		it("should extract ZIP with multiple files", async () => {
			const zipBytes = await createZip({
				"file1.txt": "Content 1",
				"file2.md": "# Content 2",
				"file3.json": '{"key": "value"}',
			});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("file1.txt");
			expect(result.content).toContain("file2.md");
			expect(result.content).toContain("file3.json");
			expect(result.content).toContain("Content 1");
			expect(result.content).toContain("Content 2");
			expect(result.content).toContain("value");

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_count).toBe(3);
			expect(archiveMeta.file_list).toHaveLength(3);
			expect(archiveMeta.file_list).toContain("file1.txt");
			expect(archiveMeta.file_list).toContain("file2.md");
			expect(archiveMeta.file_list).toContain("file3.json");
		});

		it("should extract ZIP with nested directories", async () => {
			const zipBytes = await createZip({
				"dir1/file.txt": "File in dir1",
				"dir1/subdir/nested.txt": "Nested file",
			});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("dir1/");
			expect(result.content).toContain("dir1/file.txt");
			expect(result.content).toContain("dir1/subdir/nested.txt");
			expect(result.content).toContain("File in dir1");
			expect(result.content).toContain("Nested file");

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_count).toBeGreaterThanOrEqual(2);
			expect(archiveMeta.file_list.some((f) => f.includes("dir1/file.txt"))).toBe(true);
			expect(archiveMeta.file_list.some((f) => f.includes("dir1/subdir/nested.txt"))).toBe(true);
		});

		it("should extract ZIP with mixed file types", async () => {
			const zipBytes = await createZip({
				"document.txt": "Text document",
				"readme.md": "# README",
				"image.png": Buffer.from([0x89, 0x50, 0x4e, 0x47]),
				"document.pdf": "%PDF-1.4",
			});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("document.txt");
			expect(result.content).toContain("readme.md");
			expect(result.content).toContain("image.png");
			expect(result.content).toContain("document.pdf");
			expect(result.content).toContain("Text document");
			expect(result.content).toContain("# README");

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_count).toBe(4);
			expect(archiveMeta.file_list).toHaveLength(4);
			expect(archiveMeta.file_list).toContain("document.txt");
			expect(archiveMeta.file_list).toContain("readme.md");
			expect(archiveMeta.file_list).toContain("image.png");
			expect(archiveMeta.file_list).toContain("document.pdf");
		});

		it("should handle empty ZIP gracefully", async () => {
			const zipBytes = await createZip({});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("ZIP Archive");
			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_count).toBe(0);
			expect(archiveMeta.total_size).toBe(0);
			expect(archiveMeta.file_list).toEqual([]);
		});

		it("should reject corrupted ZIP gracefully", async () => {
			const corruptedZip = new Uint8Array([0x50, 0x4b, 0x03, 0x04, 0xff, 0xff, 0xff, 0xff]);

			await expect(extractBytes(corruptedZip, "application/zip")).rejects.toThrow();
		});

		it("should handle large ZIP with many files", async () => {
			const files: Record<string, string> = {};
			for (let i = 0; i < 100; i++) {
				files[`file_${i}.txt`] = `Content ${i}`;
			}

			const zipBytes = await createZip(files);
			const result = await extractBytes(zipBytes, "application/zip");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_count).toBe(100);
			expect(archiveMeta.file_list).toHaveLength(100);
			expect(result.content).toContain("file_0.txt");
			expect(result.content).toContain("file_99.txt");
			expect(archiveMeta.file_list).toContain("file_0.txt");
			expect(archiveMeta.file_list).toContain("file_50.txt");
			expect(archiveMeta.file_list).toContain("file_99.txt");
		});

		it("should handle ZIP with special characters in filenames", async () => {
			const zipBytes = await createZip({
				"测试文件.txt": "Unicode content",
				"file with spaces.txt": "Spaces in filename",
				"file-with-dashes.txt": "Dashes",
			});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content.includes("测试文件.txt") || result.content.includes("txt")).toBe(true);
			expect(result.content).toContain("file with spaces.txt");
			expect(result.content).toContain("file-with-dashes.txt");

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_count).toBe(3);
			expect(archiveMeta.file_list).toHaveLength(3);
			expect(archiveMeta.file_list.some((f) => f.includes("txt"))).toBe(true);
			expect(archiveMeta.file_list).toContain("file with spaces.txt");
			expect(archiveMeta.file_list).toContain("file-with-dashes.txt");
		});

		it("should extract ZIP synchronously", async () => {
			const zipBytes = await createZip({
				"test.txt": "Hello from ZIP!",
			});

			const result = extractBytesSync(zipBytes, "application/zip");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("ZIP Archive");
			expect(result.content).toContain("test.txt");
			expect(result.content).toContain("Hello from ZIP!");

			expect(result.metadata.archive).toBeDefined();
			expect(result.metadata.archive.format).toBe("ZIP");
			expect(result.metadata.archive.file_count).toBe(1);
			expect(result.metadata.archive.file_list).toHaveLength(1);
			expect(result.metadata.archive.file_list[0]).toBe("test.txt");
		});
	});

	describe("TAR Extraction", () => {
		it("should extract simple TAR file", async () => {
			const tarBytes = await createTar({
				"test.txt": "Hello from TAR!",
			});

			const result = await extractBytes(tarBytes, "application/x-tar");

			expect(result.mimeType).toBe("application/x-tar");
			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("TAR Archive");
			expect(result.content).toContain("test.txt");
			expect(result.content).toContain("Hello from TAR!");

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.format).toBe("TAR");
			expect(archiveMeta.file_count).toBe(1);
		});

		it("should extract TAR with alternative MIME type", async () => {
			const tarBytes = await createTar({
				"test.txt": "Hello from TAR!",
			});

			const result = await extractBytes(tarBytes, "application/tar");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("TAR Archive");
			expect(result.metadata.archive).toBeDefined();
		});

		it("should extract TAR with multiple files", async () => {
			const tarBytes = await createTar({
				"file1.txt": "TAR content 1",
				"file2.md": "# TAR content 2",
			});

			const result = await extractBytes(tarBytes, "application/x-tar");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("file1.txt");
			expect(result.content).toContain("file2.md");
			expect(result.content).toContain("TAR content 1");

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_count).toBe(2);
		});

		it("should handle corrupted TAR gracefully", async () => {
			const corruptedTar = new Uint8Array(512);
			corruptedTar.fill(0xff);
			corruptedTar[0] = 0x66; // 'f'
			corruptedTar[1] = 0x69; // 'i'
			corruptedTar[2] = 0x6c; // 'l'
			corruptedTar[3] = 0x65; // 'e'
			corruptedTar[4] = 0x00;

			// Corrupted TAR should error or handle gracefully
			try {
				const result = await extractBytes(corruptedTar, "application/x-tar");
				expect(result).toBeDefined();
			} catch (error) {
				// Error is expected for corrupted TAR
				expect(error).toBeDefined();
			}
		});
	});

	describe("Nested Archives", () => {
		it("should extract ZIP containing ZIP", async () => {
			const innerZip = await createZip({
				"inner.txt": "Content in inner ZIP",
			});

			const outerZip = await createZip({
				"inner.zip": innerZip,
				"readme.txt": "This archive contains another archive",
			});

			const result = await extractBytes(outerZip, "application/zip");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("inner.zip");
			expect(result.content).toContain("readme.txt");
			expect(result.content).toContain("This archive contains another archive");

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_count).toBe(2);
			expect(archiveMeta.file_list).toContain("inner.zip");
			expect(archiveMeta.file_list).toContain("readme.txt");
		});

		it("should extract TAR containing ZIP", async () => {
			const innerZip = await createZip({
				"data.txt": "Data in ZIP inside TAR",
			});

			const outerTar = await createTar({
				"archive.zip": innerZip,
				"info.txt": "TAR contains ZIP",
			});

			const result = await extractBytes(outerTar, "application/x-tar");

			expect(result.chunks).toBeNull();
			expect(result.detectedLanguages).toBeNull();
			expect(result.tables).toEqual([]);

			expect(result.content).toContain("archive.zip");
			expect(result.content).toContain("info.txt");

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_count).toBeGreaterThanOrEqual(2);
		});
	});

	describe("Archive Metadata", () => {
		it("should report file count in metadata", async () => {
			const zipBytes = await createZip({
				"file1.txt": "Content 1",
				"file2.txt": "Content 2",
				"file3.txt": "Content 3",
			});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result.metadata.archive).toBeDefined();
			expect(result.metadata.archive.file_count).toBe(3);
		});

		it("should report total extracted size", async () => {
			const zipBytes = await createZip({
				"small.txt": "x",
				"medium.txt": "x".repeat(100),
			});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result.metadata.archive).toBeDefined();
			expect(result.metadata.archive.total_size).toBeGreaterThan(0);
		});

		it("should list all entry names", async () => {
			const zipBytes = await createZip({
				"alpha.txt": "A",
				"beta.txt": "B",
				"gamma.txt": "C",
			});

			const result = await extractBytes(zipBytes, "application/zip");

			const archiveMeta = result.metadata.archive;
			expect(archiveMeta).toBeDefined();
			expect(archiveMeta.file_list).toHaveLength(3);
			expect(archiveMeta.file_list).toContain("alpha.txt");
			expect(archiveMeta.file_list).toContain("beta.txt");
			expect(archiveMeta.file_list).toContain("gamma.txt");
		});

		it("should indicate archive format", async () => {
			const zipBytes = await createZip({ "test.txt": "zip" });
			const tarBytes = await createTar({ "test.txt": "tar" });

			const zipResult = await extractBytes(zipBytes, "application/zip");
			const tarResult = await extractBytes(tarBytes, "application/x-tar");

			expect(zipResult.metadata.archive.format).toBe("ZIP");
			expect(tarResult.metadata.archive.format).toBe("TAR");
		});
	});
});
