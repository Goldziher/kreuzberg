// Auto-generated tests for smoke fixtures.
// Run with: deno test --allow-read

import { extractBytes } from "npm:@kreuzberg/wasm@^4.0.0";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "npm:@kreuzberg/wasm@^4.0.0";

const TEST_TIMEOUT_MS = 60_000;

Deno.test("smoke_docx_basic", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("documents/fake.docx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "smoke_docx_basic", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
	assertions.assertMinContentLength(result, 20);
	assertions.assertContentContainsAny(result, ["Lorem", "ipsum", "document", "text"]);
});

Deno.test("smoke_html_basic", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("web/simple_table.html");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "smoke_html_basic", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["text/html"]);
	assertions.assertMinContentLength(result, 10);
	assertions.assertContentContainsAny(result, ["#", "**", "simple", "HTML"]);
});

Deno.test("smoke_image_png", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("images/sample.png");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "smoke_image_png", [], "Image extraction requires image processing dependencies")) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["image/png"]);
	assertions.assertMetadataExpectation(result, "format", "PNG");
});

Deno.test("smoke_json_basic", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("data_formats/simple.json");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "smoke_json_basic", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/json"]);
	assertions.assertMinContentLength(result, 5);
});

Deno.test("smoke_pdf_basic", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/fake_memo.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "smoke_pdf_basic", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 50);
	assertions.assertContentContainsAny(result, ["May 5, 2023", "To Whom it May Concern"]);
});

Deno.test("smoke_txt_basic", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("text/report.txt");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "smoke_txt_basic", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["text/plain"]);
	assertions.assertMinContentLength(result, 5);
});

Deno.test("smoke_xlsx_basic", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("spreadsheets/stanley_cups.xlsx");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "smoke_xlsx_basic", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"]);
	assertions.assertMinContentLength(result, 100);
	assertions.assertContentContainsAll(result, [
		"Team",
		"Location",
		"Stanley Cups",
		"Blues",
		"Flyers",
		"Maple Leafs",
		"STL",
		"PHI",
		"TOR",
	]);
	assertions.assertTableCount(result, 1, null);
	assertions.assertMetadataExpectation(result, "sheet_count", { gte: 2 });
	assertions.assertMetadataExpectation(result, "sheet_names", { contains: ["Stanley Cups"] });
});
