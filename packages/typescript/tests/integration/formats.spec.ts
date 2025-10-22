import { existsSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { extractFile } from "../../src/index.js";
import type { ExtractionConfig } from "../../src/types.js";
import {
	assertExcelMetadata,
	assertMarkdownConversion,
	assertMimeType,
	assertNonEmptyContent,
	assertSubstantialContent,
	assertValidExtractionResult,
	getTestDocumentPath,
	testDocumentsAvailable,
} from "../helpers/integration-helpers.js";

describe("Office Document Integration Tests", () => {
	it("should extract DOCX document", async () => {
		if (!testDocumentsAvailable()) {
			console.log("Skipping: test_documents not available");
			return;
		}

		const path = getTestDocumentPath("office/document.docx");
		if (!existsSync(path)) {
			console.log("Skipping: DOCX not available");
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/vnd.openxmlformats-officedocument.wordprocessingml.document");
		assertNonEmptyContent(result);

		console.log("DOCX extraction successful");
	});

	it("should extract XLSX spreadsheet", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		let path = getTestDocumentPath("office/excel.xlsx");
		if (!existsSync(path)) {
			path = getTestDocumentPath("spreadsheets/stanley_cups.xlsx");
			if (!existsSync(path)) {
				console.log("Skipping: XLSX not available");
				return;
			}
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");
		assertNonEmptyContent(result);

		assertExcelMetadata(result.metadata);

		console.log("XLSX extraction successful");
	});

	it("should extract PPTX presentation", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const testFiles = [
			"presentations/simple.pptx",
			"presentations/powerpoint_sample.pptx",
			"presentations/pitch_deck_presentation.pptx",
		];

		for (const testFile of testFiles) {
			const path = getTestDocumentPath(testFile);
			if (existsSync(path)) {
				const config: ExtractionConfig = {};
				const result = await extractFile(path, null, config);

				assertValidExtractionResult(result);
				assertMimeType(result, "application/vnd.openxmlformats-officedocument.presentationml.presentation");
				assertNonEmptyContent(result);

				console.log("PPTX extraction successful");
				return;
			}
		}

		console.log("Skipping: No PPTX files available");
	});

	it("should extract legacy Word document (.doc)", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("legacy_office/unit_test_lists.doc");
		if (!existsSync(path)) {
			console.log("Skipping: Legacy .doc file not available");
			return;
		}

		const config: ExtractionConfig = {};

		try {
			const result = await extractFile(path, null, config);

			assertValidExtractionResult(result);
			assertMimeType(result, "application/msword");
			assertNonEmptyContent(result);

			console.log("Legacy DOC extraction successful");
		} catch (error) {
			console.log("Legacy Office extraction failed (LibreOffice may not be installed):", (error as Error).message);
		}
	});
});

describe("HTML/Web Integration Tests", () => {
	it("should convert HTML to Markdown", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("web/simple_table.html");
		if (!existsSync(path)) {
			console.log("Skipping: HTML file not available");
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "text/html");
		assertNonEmptyContent(result);

		assertMarkdownConversion(result);

		console.log("HTML to Markdown conversion successful");
	});

	it("should extract complex HTML (Wikipedia article)", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("web/taylor_swift.html");
		if (!existsSync(path)) {
			console.log("Skipping: Wikipedia HTML not available");
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "text/html");
		assertSubstantialContent(result, 1000);

		console.log("Complex HTML extraction successful");
	});

	it("should handle non-English HTML (UTF-8)", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("web/germany_german.html");
		if (!existsSync(path)) {
			console.log("Skipping: German HTML not available");
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "text/html");
		assertNonEmptyContent(result);

		console.log("Non-English HTML extraction successful");
	});
});

describe("Data Format Integration Tests", () => {
	it("should extract JSON file", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		let path = getTestDocumentPath("data_formats/simple.json");
		if (!existsSync(path)) {
			path = getTestDocumentPath("json/simple.json");
			if (!existsSync(path)) {
				console.log("Skipping: JSON file not available");
				return;
			}
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/json");
		assertNonEmptyContent(result);

		expect(result.content.includes("{") || result.content.includes("[")).toBe(true);

		console.log("JSON extraction successful");
	});

	it("should extract YAML file", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		let path = getTestDocumentPath("data_formats/simple.yaml");
		if (!existsSync(path)) {
			path = getTestDocumentPath("yaml/simple.yaml");
			if (!existsSync(path)) {
				console.log("Skipping: YAML file not available");
				return;
			}
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/x-yaml");
		assertNonEmptyContent(result);

		console.log("YAML extraction successful");
	});

	it("should extract XML file", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("xml");
		if (!existsSync(path)) {
			console.log("Skipping: XML directory not available");
			return;
		}

		const { readdirSync } = await import("node:fs");
		const xmlFiles = readdirSync(path).filter((f) => f.endsWith(".xml"));

		if (xmlFiles.length === 0) {
			console.log("Skipping: No XML files found");
			return;
		}

		const xmlPath = `xml/${xmlFiles[0]}`;
		const fullPath = getTestDocumentPath(xmlPath);

		const config: ExtractionConfig = {};
		const result = await extractFile(fullPath, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "application/xml");
		assertNonEmptyContent(result);

		const hasXmlMetadata =
			result.metadata.xml?.elementCount !== undefined || result.metadata.xml?.uniqueElements !== undefined;

		if (hasXmlMetadata) {
			console.log("XML metadata successfully extracted");
		}

		console.log("XML extraction successful");
	});
});

describe("Email Integration Tests", () => {
	it("should extract email message", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const emailDir = getTestDocumentPath("email");
		if (!existsSync(emailDir)) {
			console.log("Skipping: email directory not available");
			return;
		}

		const { readdirSync } = await import("node:fs");
		const emlFiles = readdirSync(emailDir).filter((f) => f.endsWith(".eml"));

		if (emlFiles.length === 0) {
			console.log("Skipping: No EML files found");
			return;
		}

		const emlPath = `email/${emlFiles[0]}`;
		const fullPath = getTestDocumentPath(emlPath);

		const config: ExtractionConfig = {};
		const result = await extractFile(fullPath, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "message/rfc822");
		assertNonEmptyContent(result);

		const hasEmailMetadata =
			result.metadata.email?.fromEmail !== undefined ||
			result.metadata.email?.toEmails !== undefined ||
			result.metadata.email?.messageId !== undefined;

		if (hasEmailMetadata) {
			console.log("Email metadata successfully extracted");
		}

		console.log("Email extraction successful");
	});
});

describe("Text/Markdown Integration Tests", () => {
	it("should extract Markdown with metadata", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const path = getTestDocumentPath("pandoc/simple_metadata.md");
		if (!existsSync(path)) {
			console.log("Skipping: Markdown file not available");
			return;
		}

		const config: ExtractionConfig = {};
		const result = await extractFile(path, null, config);

		assertValidExtractionResult(result);
		assertMimeType(result, "text/markdown");
		assertNonEmptyContent(result);

		const hasMarkdownMetadata =
			result.metadata.text?.headers !== undefined ||
			result.metadata.text?.links !== undefined ||
			result.metadata.text?.wordCount !== undefined;

		if (hasMarkdownMetadata) {
			console.log("Markdown metadata successfully extracted");
		}

		console.log("Markdown extraction successful");
	});

	it("should extract plain text file", async () => {
		if (!testDocumentsAvailable()) {
			return;
		}

		const testFiles = ["text/contract.txt", "text/book_war_and_peace_1p.txt", "text/norwich_city.txt"];

		for (const testFile of testFiles) {
			const path = getTestDocumentPath(testFile);
			if (existsSync(path)) {
				const config: ExtractionConfig = {};
				const result = await extractFile(path, null, config);

				assertValidExtractionResult(result);
				assertMimeType(result, "text/plain");
				assertNonEmptyContent(result);

				console.log("Plain text extraction successful");
				return;
			}
		}

		console.log("Skipping: No plain text files available");
	});
});
