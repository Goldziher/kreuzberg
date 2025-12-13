/**
 * E2E tests for plugin/config/utility APIs.
 *
 * Generated from plugin API fixtures.
 * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang typescript
 */

import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { describe, expect, it } from "vitest";
import * as kreuzberg from "@kreuzberg/node";

describe("Configuration", () => {
	it("Discover configuration from current or parent directories", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "kreuzberg-test-"));
		const configPath = path.join(tmpDir, "kreuzberg.toml");
		fs.writeFileSync(configPath, "[chunking]\nmax_chars = 50\n");

		const subDir = path.join(tmpDir, "subdir");
		fs.mkdirSync(subDir);

		const originalCwd = process.cwd();
		try {
			process.chdir(subDir);

			const config = kreuzberg.ExtractionConfig.discover();

			expect(config.chunking).toBeDefined();
			expect(config.chunking?.maxChars).toBe(50);
		} finally {
			process.chdir(originalCwd);
		}
		fs.rmSync(tmpDir, { recursive: true, force: true });
	});

	it("Load configuration from a TOML file", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "kreuzberg-test-"));
		const configPath = path.join(tmpDir, "test_config.toml");
		fs.writeFileSync(
			configPath,
			"[chunking]\nmax_chars = 100\nmax_overlap = 20\n\n[language_detection]\nenabled = false\n",
		);

		const config = kreuzberg.ExtractionConfig.fromFile(configPath);

		expect(config.chunking).toBeDefined();
		expect(config.chunking?.maxChars).toBe(100);
		expect(config.chunking?.maxOverlap).toBe(20);
		expect(config.languageDetection).toBeDefined();
		expect(config.languageDetection?.enabled).toBe(false);
		fs.rmSync(tmpDir, { recursive: true, force: true });
	});
});

describe("Document Extractor Management", () => {
	it("Clear all document extractors and verify list is empty", () => {
		kreuzberg.clearDocumentExtractors();
		const result = kreuzberg.listDocumentExtractors();
		expect(result).toHaveLength(0);
	});

	it("List all registered document extractors", () => {
		const result = kreuzberg.listDocumentExtractors();
		expect(Array.isArray(result)).toBe(true);
		expect(result.every((item) => typeof item === "string")).toBe(true);
	});

	it("Unregister nonexistent document extractor gracefully", () => {
		expect(() => kreuzberg.unregisterDocumentExtractor("nonexistent-extractor-xyz")).not.toThrow();
	});
});

describe("Mime Utilities", () => {
	it("Detect MIME type from file bytes", () => {
		const testData = Buffer.from("%PDF-1.4\\n");
		const result = kreuzberg.detectMimeType(testData);
		expect(result.toLowerCase()).toContain("pdf");
	});

	it("Detect MIME type from file path", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "kreuzberg-test-"));
		const filePath = path.join(tmpDir, "test.txt");
		fs.writeFileSync(filePath, "Hello, world!");

		const result = kreuzberg.detectMimeTypeFromPath(filePath);
		expect(result.toLowerCase()).toContain("text");
		fs.rmSync(tmpDir, { recursive: true, force: true });
	});

	it("Get file extensions for a MIME type", () => {
		const result = kreuzberg.getExtensionsForMime("application/pdf");
		expect(Array.isArray(result)).toBe(true);
		expect(result).toContain("pdf");
	});
});

describe("Ocr Backend Management", () => {
	it("Clear all OCR backends and verify list is empty", () => {
		kreuzberg.clearOcrBackends();
		const result = kreuzberg.listOcrBackends();
		expect(result).toHaveLength(0);
	});

	it("List all registered OCR backends", () => {
		const result = kreuzberg.listOcrBackends();
		expect(Array.isArray(result)).toBe(true);
		expect(result.every((item) => typeof item === "string")).toBe(true);
	});

	it("Unregister nonexistent OCR backend gracefully", () => {
		expect(() => kreuzberg.unregisterOcrBackend("nonexistent-backend-xyz")).not.toThrow();
	});
});

describe("Post Processor Management", () => {
	it("Clear all post-processors and verify list is empty", () => {
		kreuzberg.clearPostProcessors();
		const result = kreuzberg.listPostProcessors();
		expect(result).toHaveLength(0);
	});

	it("List all registered post-processors", () => {
		const result = kreuzberg.listPostProcessors();
		expect(Array.isArray(result)).toBe(true);
		expect(result.every((item) => typeof item === "string")).toBe(true);
	});
});

describe("Validator Management", () => {
	it("Clear all validators and verify list is empty", () => {
		kreuzberg.clearValidators();
		const result = kreuzberg.listValidators();
		expect(result).toHaveLength(0);
	});

	it("List all registered validators", () => {
		const result = kreuzberg.listValidators();
		expect(Array.isArray(result)).toBe(true);
		expect(result.every((item) => typeof item === "string")).toBe(true);
	});
});
