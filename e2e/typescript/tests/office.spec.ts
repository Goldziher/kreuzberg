// Auto-generated tests for office fixtures.

import { existsSync } from "node:fs";
import type { ExtractionResult } from '@kreuzberg/node';
import { extractFileSync } from '@kreuzberg/node';
import { describe, it } from "vitest";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.js";

const TEST_TIMEOUT_MS = 60_000;

describe("office fixtures", () => {
	it(
		"office_doc_legacy",
		() => {
			const documentPath = resolveDocument("legacy_office/unit_test_lists.doc");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_doc_legacy: missing document at", documentPath);
				console.warn("Notes: LibreOffice must be installed for conversion.");
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"office_doc_legacy",
						["libreoffice"],
						"LibreOffice must be installed for conversion.",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/msword"]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_basic",
		() => {
			const documentPath = resolveDocument("office/document.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_equations",
		() => {
			const documentPath = resolveDocument("documents/equations.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_equations: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_equations", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_fake",
		() => {
			const documentPath = resolveDocument("documents/fake.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_fake: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_fake", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_formatting",
		() => {
			const documentPath = resolveDocument("documents/unit_test_formatting.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_formatting: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_formatting", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_headers",
		() => {
			const documentPath = resolveDocument("documents/unit_test_headers.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_headers: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_headers", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_lists",
		() => {
			const documentPath = resolveDocument("documents/unit_test_lists.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_lists: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_lists", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_tables",
		() => {
			const documentPath = resolveDocument("documents/docx_tables.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_tables: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_tables", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 50);
			assertions.assertContentContainsAll(result, [
				"Simple uniform table",
				"Nested Table",
				"merged cells",
				"Header Col",
			]);
			assertions.assertTableCount(result, 1, null);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_ppt_legacy",
		() => {
			const documentPath = resolveDocument("legacy_office/simple.ppt");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_ppt_legacy: missing document at", documentPath);
				console.warn("Notes: Skip if LibreOffice conversion is unavailable.");
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"office_ppt_legacy",
						["libreoffice"],
						"Skip if LibreOffice conversion is unavailable.",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.ms-powerpoint"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_pptx_basic",
		() => {
			const documentPath = resolveDocument("presentations/simple.pptx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_pptx_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_pptx_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.presentationml.presentation",
			]);
			assertions.assertMinContentLength(result, 50);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_pptx_images",
		() => {
			const documentPath = resolveDocument("presentations/powerpoint_with_image.pptx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_pptx_images: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_pptx_images", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.presentationml.presentation",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_pptx_pitch_deck",
		() => {
			const documentPath = resolveDocument("presentations/pitch_deck_presentation.pptx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_pptx_pitch_deck: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_pptx_pitch_deck", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.presentationml.presentation",
			]);
			assertions.assertMinContentLength(result, 100);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_xls_legacy",
		() => {
			const documentPath = resolveDocument("spreadsheets/test_excel.xls");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_xls_legacy: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_xls_legacy", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.ms-excel"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_xlsx_basic",
		() => {
			const documentPath = resolveDocument("spreadsheets/stanley_cups.xlsx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_xlsx_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_xlsx_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"]);
			assertions.assertMinContentLength(result, 100);
			assertions.assertContentContainsAll(result, ["Team", "Location", "Stanley Cups"]);
			assertions.assertTableCount(result, 1, null);
			assertions.assertMetadataExpectation(result, "sheet_count", { gte: 2 });
			assertions.assertMetadataExpectation(result, "sheet_names", { contains: "Stanley Cups" });
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_xlsx_multi_sheet",
		() => {
			const documentPath = resolveDocument("spreadsheets/excel_multi_sheet.xlsx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_xlsx_multi_sheet: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_xlsx_multi_sheet", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"]);
			assertions.assertMinContentLength(result, 20);
			assertions.assertMetadataExpectation(result, "sheet_count", { gte: 2 });
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_xlsx_office_example",
		() => {
			const documentPath = resolveDocument("office/excel.xlsx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_xlsx_office_example: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_xlsx_office_example", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);
});
