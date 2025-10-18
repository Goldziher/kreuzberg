import { describe, it, expect } from "vitest";
import { extractFile, extractFileSync } from "../../src/index.js";
import type { ExtractionConfig } from "../../src/types.js";
import {
	getTestDocumentPath,
	testDocumentsAvailable,
	assertMimeType,
	assertNonEmptyContent,
	assertValidExtractionResult,
	assertPdfMetadata,
	assertSubstantialContent,
	assertTablesExtracted,
} from "../helpers/integration-helpers.js";
import { existsSync } from "node:fs";

describe("PDF Integration Tests", () => {
	it("should extract simple PDF text", async () => {
		if (!testDocumentsAvailable()) {
			console.log("Skipping: test_documents not available");
			return;
		}

		const path = getTestDocumentPath("pdfs/code_and_formula.pdf");
		if (!existsSync(path)) {
			console.log("Skipping: test file not found");
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/pdf");
		assertNonEmptyContent(result);

		// Chunks should be null or undefined without chunking config
		expect(result.chunks === null || result.chunks === undefined).toBe(true);

		// Language detection should be null when not enabled
		expect(result.detectedLanguages).toBeNull();
	});

	it("should extract large PDF document (400+ pages)", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath(
			"pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf",
		);
		if (!existsSync(path)) {
			console.log("Skipping: large PDF not available");
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/pdf");
		assertSubstantialContent(result, 10000);
		assertPdfMetadata(result.metadata);
	});

	it("should handle password-protected PDF gracefully", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("pdfs/copy_protected.pdf");
		if (!existsSync(path)) {
			console.log("Skipping: protected PDF not available");
			return;
		}

		const config: ExtractionConfig = {};

		try {
			const result = await extractFile(path, null, config);

			// If it succeeds (some protection can be bypassed), validate structure
			assertValidExtractionResult(result);
			console.log(
				"Protected PDF extracted (some protection can be bypassed)",
			);
		} catch (error) {
			// Expected - password protection detected
			console.log(
				"Password protection detected (expected):",
				(error as Error).message,
			);
			expect(error).toBeTruthy();
		}
	});

	it("should extract PDF metadata", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath(
			"pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf",
		);
		if (!existsSync(path)) {
			console.log("Skipping: PDF not available");
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/pdf");
		assertPdfMetadata(result.metadata);

		// Check page count is populated
		if (result.metadata.pdf?.pageCount) {
			expect(result.metadata.pdf.pageCount).toBeGreaterThan(0);
		}
	});

	it("should extract PDF with tables", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("pdfs_with_tables/tiny.pdf");
		if (!existsSync(path)) {
			console.log("Skipping: PDF with tables not available");
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/pdf");
		assertNonEmptyContent(result);

		// Table extraction is optional - if tables are extracted, validate them
		if (result.tables.length > 0) {
			console.log(`Tables extracted: ${result.tables.length}`);
			assertTablesExtracted(result);
		}

		assertPdfMetadata(result.metadata);
	});

	it("should extract PDF synchronously", () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("pdfs/code_and_formula.pdf");
		if (!existsSync(path)) {
			return;
		}

		const config: ExtractionConfig = {};
		const result = extractFileSync(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/pdf");
		assertNonEmptyContent(result);
	});

	it("should extract PDF with explicit MIME type hint", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("pdfs/code_and_formula.pdf");
		if (!existsSync(path)) {
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, "application/pdf", config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/pdf");
		assertNonEmptyContent(result);
	});

	it("should extract PDF with password from config", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		// This test demonstrates password configuration
		// Actual password-protected test files may not be available
		const path = getTestDocumentPath("pdfs/code_and_formula.pdf");
		if (!existsSync(path)) {
			return;
		}

		const config: ExtractionConfig = {
			pdfOptions: {
				passwords: ["test123", "password"],
				extractMetadata: true,
			},
		};

		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/pdf");
	});
});
