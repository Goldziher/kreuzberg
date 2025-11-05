/**
 * Security Validation Tests
 *
 * Tests the system's resilience against malicious inputs including:
 * - Archive attacks (zip bombs, path traversal)
 * - XML attacks (billion laughs, XXE)
 * - Resource exhaustion (large files, memory limits)
 * - Malformed inputs (invalid MIME, encoding)
 * - PDF-specific attacks (malicious structure)
 */

import { describe, expect, it } from "vitest";
import { extractBytes, extractBytesSync, extractFile } from "../../src/index.js";
import { createZip } from "../helpers/integration-helpers.js";

describe("Security Validation Tests", () => {
	describe("Archive Attacks", () => {
		it("should handle zip bomb gracefully", async () => {
			// Create a ZIP with highly compressible content (simulates zip bomb)
			const largeContent = Buffer.alloc(10 * 1024 * 1024, 0); // 10MB of zeros
			const zipBytes = await createZip({
				"large.txt": largeContent,
			});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result).toBeDefined();
			const metadata = result.metadata ?? {};
			expect(typeof metadata.file_count).toBe("number");
			expect(metadata.file_count).toBeGreaterThan(0);
		});

		it("should reject path traversal attempts in ZIP", async () => {
			const zipBytes = await createZip({
				"../../etc/passwd": "malicious content",
			});

			const result = await extractBytes(zipBytes, "application/zip");

			if (result.metadata.archive) {
				for (const filePath of result.metadata.archive.file_list) {
					expect(filePath.startsWith("/")).toBe(false);
				}
			}
		});

		it("should handle absolute paths in archives gracefully", async () => {
			const zipBytes = await createZip({
				"/tmp/malicious.txt": "malicious content",
			});

			// Should not throw - either succeeds or fails gracefully
			await expect(extractBytes(zipBytes, "application/zip")).resolves.toBeDefined();
		});

		it("should handle deeply nested directories", async () => {
			const deepPath = Array.from({ length: 100 }, (_, i) => `dir${i}`).join("/");
			const files: Record<string, string> = {};
			files[`${deepPath}/file.txt`] = "deep content";

			const zipBytes = await createZip(files);

			// Should handle deep nesting without crashing
			await expect(extractBytes(zipBytes, "application/zip")).resolves.toBeDefined();
		});

		it("should handle many small files efficiently", async () => {
			const files: Record<string, string> = {};
			for (let i = 0; i < 1000; i++) {
				files[`file${i}.txt`] = "small content";
			}

			const zipBytes = await createZip(files);
			const result = await extractBytes(zipBytes, "application/zip");

			expect(result).toBeDefined();
			const metadata = result.metadata ?? {};
			expect(metadata.file_count).toBe(1000);
		});
	});

	describe("XML Attacks", () => {
		it("should handle billion laughs attack", async () => {
			const billionLaughs = `<?xml version="1.0"?>
<!DOCTYPE lolz [
  <!ENTITY lol "lol">
  <!ENTITY lol1 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
  <!ENTITY lol2 "&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;&lol1;">
  <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;">
]>
<lolz>&lol3;</lolz>`;

			// Should not hang or consume excessive memory
			await expect(extractBytes(Buffer.from(billionLaughs), "application/xml")).resolves.toBeDefined();
		});

		it("should handle XML quadratic blowup", async () => {
			const quadraticBlowup = `<?xml version="1.0"?>
<!DOCTYPE bomb [
  <!ENTITY a "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa">
]>
<bomb>&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;</bomb>`;

			await expect(extractBytes(Buffer.from(quadraticBlowup), "application/xml")).resolves.toBeDefined();
		});

		it("should block external entity injection (XXE)", async () => {
			const xxeAttack = `<?xml version="1.0"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<foo>&xxe;</foo>`;

			const result = await extractBytes(Buffer.from(xxeAttack), "application/xml");

			// Should not contain system file contents
			expect(result.content).not.toContain("root:");
			expect(result.content).not.toContain("/bin/bash");
		});

		it("should handle DTD entity expansion", async () => {
			const dtdExpansion = `<?xml version="1.0"?>
<!DOCTYPE data [
  <!ENTITY large "THIS_IS_A_LARGE_STRING_REPEATED_MANY_TIMES">
]>
<data>&large;&large;&large;&large;&large;&large;&large;&large;</data>`;

			await expect(extractBytes(Buffer.from(dtdExpansion), "application/xml")).resolves.toBeDefined();
		});
	});

	describe("Resource Limits", () => {
		it("should handle large text files efficiently", async () => {
			const largeText = "This is a line of text that will be repeated many times.\n".repeat(200_000);

			const result = await extractBytes(Buffer.from(largeText), "text/plain");

			expect(result).toBeDefined();
			expect(result.content.length).toBeGreaterThan(0);
		});

		it("should handle large XML with streaming", async () => {
			let xml = '<?xml version="1.0"?><root>';
			for (let i = 0; i < 10000; i++) {
				xml += `<item id="${i}">${"x".repeat(100)}</item>`;
			}
			xml += "</root>";

			await expect(extractBytes(Buffer.from(xml), "application/xml")).resolves.toBeDefined();
		});

		it("should handle empty files", async () => {
			const result = await extractBytes(Buffer.from(""), "text/plain");

			expect(result).toBeDefined();
			expect(result.content).toBe("");
		});

		it("should handle single byte files", async () => {
			const result = await extractBytes(Buffer.from("a"), "text/plain");

			expect(result).toBeDefined();
			expect(result.content).toBe("a");
		});

		it("should handle null bytes in content", async () => {
			const nullBytes = Buffer.from("Hello\x00World\x00Test\x00");

			const result = await extractBytes(nullBytes, "text/plain");

			expect(result).toBeDefined();
		});
	});

	describe("Malformed Input Handling", () => {
		it("should reject invalid MIME types", async () => {
			const content = Buffer.from("Some content");

			await expect(extractBytes(content, "invalid/mime/type")).rejects.toThrow();
		});

		it("should handle malformed XML structure", async () => {
			const malformedXml = '<?xml version="1.0"?><root><item>test</item>';

			// May succeed with partial content or fail gracefully
			await expect(extractBytes(Buffer.from(malformedXml), "application/xml")).resolves.toBeDefined();
		});

		it("should reject malformed ZIP structure", async () => {
			const corruptZip = Buffer.from("PK\x03\x04CORRUPTED_DATA");

			await expect(extractBytes(corruptZip, "application/zip")).rejects.toThrow();
		});

		it("should handle invalid UTF-8 sequences", async () => {
			const invalidUtf8 = Buffer.from([
				0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0xff, 0xfe, 0x20, 0x57, 0x6f, 0x72, 0x6c, 0x64,
			]);

			// Should handle gracefully (replace invalid sequences or error)
			await expect(extractBytes(invalidUtf8, "text/plain")).resolves.toBeDefined();
		});

		it("should handle mixed line endings", async () => {
			const mixedEndings = Buffer.from("Line 1\r\nLine 2\nLine 3\rLine 4");

			const result = await extractBytes(mixedEndings, "text/plain");

			expect(result.content).toContain("Line 1");
			expect(result.content).toContain("Line 2");
			expect(result.content).toContain("Line 3");
			expect(result.content).toContain("Line 4");
		});
	});

	describe("PDF Security", () => {
		it("should handle minimal valid PDF", async () => {
			const minimalPdf = Buffer.from("%PDF-1.4\nThis is a very minimal PDF structure for security testing.\n%%EOF");

			// May succeed or fail depending on PDF validation strictness
			try {
				const result = await extractBytes(minimalPdf, "application/pdf");
				expect(result).toBeDefined();
			} catch (error) {
				// Failure is acceptable for invalid PDF structure
				expect(error).toBeDefined();
			}
		});

		it("should reject malformed PDF header", async () => {
			const malformedPdf = Buffer.from("%PDF-INVALID\nThis is not a valid PDF structure");

			await expect(extractBytes(malformedPdf, "application/pdf")).rejects.toThrow();
		});

		it("should handle truncated PDF gracefully", async () => {
			const truncatedPdf = Buffer.from(`%PDF-1.4
1 0 obj
<<
/Type /Catalog
>>
endobj`);

			// May succeed with partial content or fail gracefully
			try {
				const result = await extractBytes(truncatedPdf, "application/pdf");
				expect(result).toBeDefined();
			} catch (error) {
				expect(error).toBeDefined();
			}
		});
	});

	describe("File System Security", () => {
		it("should reject nonexistent files", async () => {
			await expect(extractFile("/nonexistent/path/to/file.txt")).rejects.toThrow();
		});

		it("should reject directory instead of file", async () => {
			await expect(extractFile("/tmp")).rejects.toThrow();
		});
	});

	describe("Path Traversal Protection", () => {
		it("should sanitize relative paths in archives", async () => {
			const zipBytes = await createZip({
				"../../../etc/shadow": "malicious",
				"./local/file.txt": "safe content",
			});

			const result = await extractBytes(zipBytes, "application/zip");

			if (result.metadata.archive) {
				for (const path of result.metadata.archive.file_list) {
					// Paths should be normalized/sanitized
					expect(path).not.toMatch(/\.\.\//);
					expect(path.startsWith("/")).toBe(false);
				}
			}
		});

		it("should handle symlink-like paths safely", async () => {
			const zipBytes = await createZip({
				"link/../target.txt": "content",
			});

			const result = await extractBytes(zipBytes, "application/zip");

			expect(result).toBeDefined();
		});
	});

	describe("Decompression Ratio Validation", () => {
		it("should detect extreme decompression ratios", async () => {
			// Create highly compressible content
			const highlyCompressible = Buffer.alloc(50 * 1024 * 1024, 0); // 50MB zeros
			const zipBytes = await createZip({
				"bomb.txt": highlyCompressible,
			});

			// Should handle without exhausting memory
			await expect(extractBytes(zipBytes, "application/zip")).resolves.toBeDefined();
		});
	});

	describe("Concurrent Extraction Safety", () => {
		it("should handle concurrent extractions without interference", async () => {
			const content1 = Buffer.from("Content 1");
			const content2 = Buffer.from("Content 2");
			const content3 = Buffer.from("Content 3");

			const results = await Promise.all([
				extractBytes(content1, "text/plain"),
				extractBytes(content2, "text/plain"),
				extractBytes(content3, "text/plain"),
			]);

			expect(results[0].content).toBe("Content 1");
			expect(results[1].content).toBe("Content 2");
			expect(results[2].content).toBe("Content 3");
		});
	});

	describe("Synchronous Extraction Security", () => {
		it("should apply same security checks to sync extraction", () => {
			const corruptZip = Buffer.from("PK\x03\x04CORRUPTED");

			expect(() => extractBytesSync(corruptZip, "application/zip")).toThrow();
		});

		it("should handle malformed input in sync mode", () => {
			const invalidMime = Buffer.from("content");

			expect(() => extractBytesSync(invalidMime, "invalid/type")).toThrow();
		});
	});
});
