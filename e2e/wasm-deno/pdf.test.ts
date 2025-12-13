// Auto-generated tests for pdf fixtures.
// Run with: deno test --allow-read

import { extractBytes } from "npm:@kreuzberg/wasm@^4.0.0";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "npm:@kreuzberg/wasm@^4.0.0";

const TEST_TIMEOUT_MS = 60_000;

Deno.test("pdf_assembly_technical", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/assembly_language_for_beginners_al4_b_en.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_assembly_technical", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 5000);
	assertions.assertContentContainsAny(result, ["assembly", "register", "instruction"]);
	assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
});

Deno.test("pdf_bayesian_data_analysis", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_bayesian_data_analysis", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 10000);
	assertions.assertContentContainsAny(result, ["Bayesian", "probability", "distribution"]);
	assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
});

Deno.test("pdf_code_and_formula", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/code_and_formula.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_code_and_formula", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 100);
});

Deno.test("pdf_deep_learning", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/fundamentals_of_deep_learning_2014.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_deep_learning", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 1000);
	assertions.assertContentContainsAny(result, ["neural", "network", "deep learning"]);
	assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
});

Deno.test("pdf_embedded_images", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/embedded_images_tables.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_embedded_images", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 50);
	assertions.assertTableCount(result, 0, null);
});

Deno.test("pdf_google_doc", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/google_doc_document.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_google_doc", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 50);
	assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
});

Deno.test("pdf_large_ciml", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_large_ciml", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 10000);
	assertions.assertContentContainsAny(result, ["machine learning", "algorithm", "training"]);
	assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
});

Deno.test("pdf_non_english_german", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_non_english_german", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 100);
	assertions.assertContentContainsAny(result, ["Intel", "paging"]);
	assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
});

Deno.test("pdf_right_to_left", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/right_to_left_01.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_right_to_left", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 50);
	assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
});

Deno.test("pdf_simple_text", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/fake_memo.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_simple_text", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 50);
	assertions.assertContentContainsAny(result, ["May 5, 2023", "To Whom it May Concern", "Mallori"]);
});

Deno.test("pdf_tables_large", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs_with_tables/large.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_tables_large", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 500);
	assertions.assertTableCount(result, 1, null);
});

Deno.test("pdf_tables_medium", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs_with_tables/medium.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_tables_medium", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 100);
	assertions.assertTableCount(result, 1, null);
});

Deno.test("pdf_tables_small", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs_with_tables/tiny.pdf");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_tables_small", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 50);
	assertions.assertContentContainsAll(result, [
		"Table 1",
		"Selected Numbers",
		"Celsius",
		"Fahrenheit",
		"Water Freezing Point",
		"Water Boiling Point",
	]);
	assertions.assertTableCount(result, 1, null);
});

Deno.test("pdf_technical_stat_learning", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument(
		"pdfs/an_introduction_to_statistical_learning_with_applications_in_r_islr_sixth_printing.pdf",
	);
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "pdf_technical_stat_learning", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 10000);
	assertions.assertContentContainsAny(result, ["statistical", "regression", "learning"]);
	assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
});
