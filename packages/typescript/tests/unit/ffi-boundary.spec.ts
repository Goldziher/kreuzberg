import { describe, expect, it } from "vitest";
import {
	batchExtractBytes,
	batchExtractBytesSync,
	extractBytes,
	extractBytesSync,
	extractFile,
	extractFileSync,
} from "../../src/index.js";

describe("FFI Boundary Tests", () => {
	describe("Large data handling", () => {
		it("should handle large byte arrays (sync)", () => {
			const data = new Uint8Array(1024 * 1024); // 1MB of zeros
			expect(() => {
				extractBytesSync(data, "application/pdf", null);
			}).toThrow(); // Should fail because data is not a valid PDF
		});

		it("should handle large byte arrays (async)", async () => {
			const data = new Uint8Array(1024 * 1024); // 1MB of zeros
			await expect(extractBytes(data, "application/pdf", null)).rejects.toThrow();
		});

		it("should handle empty byte arrays (sync)", () => {
			const data = new Uint8Array(0);
			expect(() => {
				extractBytesSync(data, "text/plain", null);
			}).not.toThrow(); // Empty text file is valid
		});

		it("should handle empty byte arrays (async)", async () => {
			const data = new Uint8Array(0);
			const result = await extractBytes(data, "text/plain", null);
			expect(result.content).toBe("");
		});
	});

	describe("Unicode handling across FFI", () => {
		it("should handle Unicode file paths (sync)", () => {
			const unicodePath = "/nonexistent/文件/файл.pdf";
			expect(() => {
				extractFileSync(unicodePath, null, null);
			}).toThrow(); // File doesn't exist, but path should be handled correctly
		});

		it("should handle Unicode file paths (async)", async () => {
			const unicodePath = "/nonexistent/文件/файл.pdf";
			await expect(extractFile(unicodePath, null, null)).rejects.toThrow();
		});

		it("should handle Unicode in text extraction (sync)", () => {
			const text = "Hello 世界 мир 🌍";
			const encoder = new TextEncoder();
			const data = encoder.encode(text);

			const result = extractBytesSync(data, "text/plain", null);
			expect(result.content).toContain("Hello");
			expect(result.content).toContain("世界");
			expect(result.content).toContain("мир");
		});

		it("should handle Unicode in text extraction (async)", async () => {
			const text = "Hello 世界 мир 🌍";
			const encoder = new TextEncoder();
			const data = encoder.encode(text);

			const result = await extractBytes(data, "text/plain", null);
			expect(result.content).toContain("Hello");
			expect(result.content).toContain("世界");
			expect(result.content).toContain("мир");
		});

		it("should handle emoji in MIME types gracefully", async () => {
			const data = new Uint8Array([1, 2, 3]);
			await expect(extractBytes(data, "application/🎉", null)).rejects.toThrow();
		});
	});

	describe("Null and undefined handling", () => {
		it("should handle null config (sync)", () => {
			const data = new Uint8Array([1, 2, 3]);
			expect(() => {
				extractBytesSync(data, "application/pdf", null);
			}).toThrow(); // Invalid PDF, but null config should be handled
		});

		it("should handle null config (async)", async () => {
			const data = new Uint8Array([1, 2, 3]);
			await expect(extractBytes(data, "application/pdf", null)).rejects.toThrow();
		});

		it("should handle undefined MIME type detection (sync)", () => {
			const text = "Hello World";
			const encoder = new TextEncoder();
			const data = encoder.encode(text);

			expect(() => {
				extractFileSync("test.txt", data as any, null);
			}).toThrow();
		});

		it("should reject null file paths (sync)", () => {
			expect(() => {
				extractFileSync(null as any, null, null);
			}).toThrow();
		});

		it("should reject null file paths (async)", async () => {
			await expect(extractFile(null as any, null, null)).rejects.toThrow();
		});

		it("should reject null byte arrays (sync)", () => {
			expect(() => {
				extractBytesSync(null as any, "text/plain", null);
			}).toThrow();
		});

		it("should reject null byte arrays (async)", async () => {
			await expect(extractBytes(null as any, "text/plain", null)).rejects.toThrow();
		});
	});

	describe("Buffer lifecycle and cleanup", () => {
		it("should not retain references to buffers after extraction (sync)", () => {
			const text = "Test content";
			const encoder = new TextEncoder();
			const data = encoder.encode(text);

			const result = extractBytesSync(data, "text/plain", null);

			// Modify original buffer
			data[0] = 0;

			// Result should not be affected
			expect(result.content).toBe("Test content");
		});

		it("should not retain references to buffers after extraction (async)", async () => {
			const text = "Test content";
			const encoder = new TextEncoder();
			const data = encoder.encode(text);

			const result = await extractBytes(data, "text/plain", null);

			// Modify original buffer
			data[0] = 0;

			// Result should not be affected
			expect(result.content).toBe("Test content");
		});

		it("should handle buffer reuse safely (sync)", () => {
			const text1 = "First content";
			const text2 = "Second content that is longer";
			const encoder = new TextEncoder();
			const data = encoder.encode(text1);

			const result1 = extractBytesSync(data, "text/plain", null);

			// Reuse the same buffer reference with different content
			const data2 = encoder.encode(text2);

			const result2 = extractBytesSync(data2, "text/plain", null);

			expect(result1.content).toBe("First content");
			expect(result2.content).toBe("Second content that is longer");
		});

		it("should handle buffer reuse safely (async)", async () => {
			const text1 = "First content";
			const text2 = "Second content that is longer";
			const encoder = new TextEncoder();
			const data = encoder.encode(text1);

			const result1 = await extractBytes(data, "text/plain", null);

			// Reuse the same buffer reference with different content
			const data2 = encoder.encode(text2);

			const result2 = await extractBytes(data2, "text/plain", null);

			expect(result1.content).toBe("First content");
			expect(result2.content).toBe("Second content that is longer");
		});
	});

	describe("Type conversion errors", () => {
		it("should reject invalid buffer types (sync)", () => {
			expect(() => {
				extractBytesSync("not a buffer" as any, "text/plain", null);
			}).toThrow();
		});

		it("should reject invalid buffer types (async)", async () => {
			await expect(extractBytes("not a buffer" as any, "text/plain", null)).rejects.toThrow();
		});

		it("should reject array instead of Uint8Array (sync)", () => {
			expect(() => {
				extractBytesSync([1, 2, 3] as any, "text/plain", null);
			}).toThrow();
		});

		it("should reject array instead of Uint8Array (async)", async () => {
			await expect(extractBytes([1, 2, 3] as any, "text/plain", null)).rejects.toThrow();
		});

		it("should reject invalid MIME type format (sync)", () => {
			const data = new Uint8Array([1, 2, 3]);
			expect(() => {
				extractBytesSync(data, 123 as any, null);
			}).toThrow();
		});

		it("should reject invalid MIME type format (async)", async () => {
			const data = new Uint8Array([1, 2, 3]);
			await expect(extractBytes(data, 123 as any, null)).rejects.toThrow();
		});
	});

	describe("Batch operations boundary", () => {
		it("should handle empty batch arrays (sync)", () => {
			const results = batchExtractBytesSync([], [], null);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(0);
		});

		it("should handle empty batch arrays (async)", async () => {
			const results = await batchExtractBytes([], [], null);
			expect(Array.isArray(results)).toBe(true);
			expect(results.length).toBe(0);
		});

		it("should handle mismatched array lengths (sync)", () => {
			const data = [new Uint8Array([1, 2, 3])];
			const mimeTypes = ["text/plain", "application/pdf"];

			expect(() => {
				batchExtractBytesSync(data, mimeTypes, null);
			}).toThrow();
		});

		it("should handle mismatched array lengths (async)", async () => {
			const data = [new Uint8Array([1, 2, 3])];
			const mimeTypes = ["text/plain", "application/pdf"];

			await expect(batchExtractBytes(data, mimeTypes, null)).rejects.toThrow();
		});

		it("should handle large batch sizes (sync)", () => {
			const text = "Test content";
			const encoder = new TextEncoder();
			const data = Array(100)
				.fill(null)
				.map(() => encoder.encode(text));
			const mimeTypes = Array(100).fill("text/plain");

			const results = batchExtractBytesSync(data, mimeTypes, null);
			expect(results.length).toBe(100);
			results.forEach((result) => {
				expect(result.content).toBe("Test content");
			});
		});

		it("should handle large batch sizes (async)", async () => {
			const text = "Test content";
			const encoder = new TextEncoder();
			const data = Array(100)
				.fill(null)
				.map(() => encoder.encode(text));
			const mimeTypes = Array(100).fill("text/plain");

			const results = await batchExtractBytes(data, mimeTypes, null);
			expect(results.length).toBe(100);
			results.forEach((result) => {
				expect(result.content).toBe("Test content");
			});
		});
	});
});
