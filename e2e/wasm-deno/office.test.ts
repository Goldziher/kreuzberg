// Auto-generated tests for office fixtures.
// Run with: deno test --allow-read

import { extractBytes } from "npm:@kreuzberg/wasm@^4.0.0";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "npm:@kreuzberg/wasm@^4.0.0";

const TEST_TIMEOUT_MS = 60_000;

Deno.test("office_doc_legacy", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("legacy_office/unit_test_lists.doc");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (
			shouldSkipFixture(error, "office_doc_legacy", ["libreoffice"], "LibreOffice must be installed for conversion.")
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
});

Deno.test("office_docx_basic", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("office/document.docx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_docx_basic", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
	assertions.assertMinContentLength(result, 10);
});

Deno.test("office_docx_equations", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("documents/equations.docx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_docx_equations", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
	assertions.assertMinContentLength(result, 20);
});

Deno.test("office_docx_fake", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("documents/fake.docx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_docx_fake", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
	assertions.assertMinContentLength(result, 20);
});

Deno.test("office_docx_formatting", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("documents/unit_test_formatting.docx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_docx_formatting", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
	assertions.assertMinContentLength(result, 20);
});

Deno.test("office_docx_headers", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("documents/unit_test_headers.docx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_docx_headers", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
	assertions.assertMinContentLength(result, 20);
});

Deno.test("office_docx_lists", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("documents/unit_test_lists.docx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_docx_lists", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
	assertions.assertMinContentLength(result, 20);
});

Deno.test("office_docx_tables", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("documents/docx_tables.docx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_docx_tables", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
	assertions.assertMinContentLength(result, 50);
	assertions.assertContentContainsAll(result, ["Simple uniform table", "Nested Table", "merged cells", "Header Col"]);
	assertions.assertTableCount(result, 1, null);
});

Deno.test("office_ppt_legacy", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("legacy_office/simple.ppt");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (
			shouldSkipFixture(error, "office_ppt_legacy", ["libreoffice"], "Skip if LibreOffice conversion is unavailable.")
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
});

Deno.test("office_pptx_basic", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("presentations/simple.pptx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_pptx_basic", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.presentationml.presentation"]);
	assertions.assertMinContentLength(result, 50);
});

Deno.test("office_pptx_images", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("presentations/powerpoint_with_image.pptx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_pptx_images", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.presentationml.presentation"]);
	assertions.assertMinContentLength(result, 20);
});

Deno.test("office_pptx_pitch_deck", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("presentations/pitch_deck_presentation.pptx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "office_pptx_pitch_deck", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.presentationml.presentation"]);
	assertions.assertMinContentLength(result, 100);
});

Deno.test("office_xls_legacy", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("spreadsheets/test_excel.xls");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
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
});

Deno.test("office_xlsx_basic", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("spreadsheets/stanley_cups.xlsx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
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
	assertions.assertMetadataExpectation(result, "sheet_names", { contains: ["Stanley Cups"] });
});

Deno.test("office_xlsx_multi_sheet", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("spreadsheets/excel_multi_sheet.xlsx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
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
});

Deno.test("office_xlsx_office_example", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("office/excel.xlsx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
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
});
