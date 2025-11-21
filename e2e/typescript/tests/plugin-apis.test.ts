// Auto-generated plugin API tests for TypeScript binding.
/**
 * E2E tests for plugin registration and management APIs.
 *
 * Tests all plugin types:
 * - Validators
 * - Post-processors
 * - OCR backends
 * - Document extractors
 *
 * Tests all management operations:
 * - Registration
 * - Unregistration
 * - Listing
 * - Clearing
 */

import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { beforeEach, describe, expect, it } from "vitest";
import * as kreuzberg from "../../../packages/typescript/src/index.js";

describe("Validator APIs", () => {
	it("should list all registered validators", () => {
		const validators = kreuzberg.listValidators();
		expect(Array.isArray(validators)).toBe(true);
		expect(validators.every((v) => typeof v === "string")).toBe(true);
	});

	it("should clear all validators", () => {
		kreuzberg.clearValidators();
		const validators = kreuzberg.listValidators();
		expect(validators).toHaveLength(0);
	});
});

describe("Post-processor APIs", () => {
	it("should list all registered post-processors", () => {
		const processors = kreuzberg.listPostProcessors();
		expect(Array.isArray(processors)).toBe(true);
		expect(processors.every((p) => typeof p === "string")).toBe(true);
	});

	it("should clear all post-processors", () => {
		kreuzberg.clearPostProcessors();
		const processors = kreuzberg.listPostProcessors();
		expect(processors).toHaveLength(0);
	});
});

describe("OCR Backend APIs", () => {
	it("should list all registered OCR backends", () => {
		const backends = kreuzberg.listOcrBackends();
		expect(Array.isArray(backends)).toBe(true);
		expect(backends.every((b) => typeof b === "string")).toBe(true);
		// Should include built-in backends
		expect(backends).toContain("tesseract");
	});

	it("should unregister an OCR backend", () => {
		// Should handle nonexistent backend gracefully
		expect(() => kreuzberg.unregisterOcrBackend("nonexistent-backend-xyz")).not.toThrow();
	});

	it("should clear all OCR backends", () => {
		kreuzberg.clearOcrBackends();
		const backends = kreuzberg.listOcrBackends();
		expect(backends).toHaveLength(0);
	});
});

describe("Document Extractor APIs", () => {
	it("should list all registered document extractors", () => {
		const extractors = kreuzberg.listDocumentExtractors();
		expect(Array.isArray(extractors)).toBe(true);
		expect(extractors.every((e) => typeof e === "string")).toBe(true);
		// Note: Unlike OCR backends, document extractors are not auto-registered
		// They are only registered when actually used by the extraction pipeline
	});

	it("should unregister a document extractor", () => {
		// Should handle nonexistent extractor gracefully
		expect(() => kreuzberg.unregisterDocumentExtractor("nonexistent-extractor-xyz")).not.toThrow();
	});

	it("should clear all document extractors", () => {
		kreuzberg.clearDocumentExtractors();
		const extractors = kreuzberg.listDocumentExtractors();
		expect(extractors).toHaveLength(0);
	});
});

describe("Configuration APIs", () => {
	it("should load configuration from a TOML file", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "kreuzberg-test-"));
		const configPath = path.join(tmpDir, "test_config.toml");

		fs.writeFileSync(
			configPath,
			`
[chunking]
max_chars = 100
max_overlap = 20

[language_detection]
enabled = false
`,
		);

		const config = kreuzberg.ExtractionConfig.fromFile(configPath);
		expect(config.chunking).toBeDefined();
		expect(config.chunking?.maxChars).toBe(100);
		expect(config.chunking?.maxOverlap).toBe(20);
		expect(config.languageDetection).toBeDefined();
		expect(config.languageDetection?.enabled).toBe(false);

		// Cleanup
		fs.rmSync(tmpDir, { recursive: true });
	});

	it("should discover configuration from current or parent directories", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "kreuzberg-test-"));
		const configPath = path.join(tmpDir, "kreuzberg.toml");

		fs.writeFileSync(
			configPath,
			`
[chunking]
max_chars = 50
`,
		);

		// Create subdirectory
		const subDir = path.join(tmpDir, "subdir");
		fs.mkdirSync(subDir);

		const originalDir = process.cwd();
		try {
			process.chdir(subDir);
			const config = kreuzberg.ExtractionConfig.discover();
			expect(config).toBeDefined();
			expect(config?.chunking).toBeDefined();
			expect(config?.chunking?.maxChars).toBe(50);
		} finally {
			process.chdir(originalDir);
			fs.rmSync(tmpDir, { recursive: true });
		}
	});
});

describe("MIME Utilities", () => {
	it("should detect MIME type from bytes", () => {
		// PDF magic bytes
		const pdfBytes = Buffer.from("%PDF-1.4\n", "utf-8");
		const mimeType = kreuzberg.detectMimeType(pdfBytes);
		expect(mimeType.toLowerCase()).toContain("pdf");
	});

	it("should detect MIME type from file path", () => {
		const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "kreuzberg-test-"));
		const testFile = path.join(tmpDir, "test.txt");
		fs.writeFileSync(testFile, "Hello, world!");

		const mimeType = kreuzberg.detectMimeTypeFromPath(testFile);
		expect(mimeType.toLowerCase()).toContain("text");

		// Cleanup
		fs.rmSync(tmpDir, { recursive: true });
	});

	it("should get file extensions for a MIME type", () => {
		const extensions = kreuzberg.getExtensionsForMime("application/pdf");
		expect(Array.isArray(extensions)).toBe(true);
		expect(extensions).toContain("pdf");
	});
});
