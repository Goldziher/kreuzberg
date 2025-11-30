// Auto-generated tests for pdf fixtures.

import { existsSync } from "node:fs";
import type { ExtractionResult } from "kreuzberg";
import { extractFileSync } from "kreuzberg";
import { describe, it } from "vitest";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.js";

const TEST_TIMEOUT_MS = 60_000;

describe("pdf fixtures", () => {
	it(
		"pdf_assembly_technical",
		() => {
			const documentPath = resolveDocument("pdfs/assembly_language_for_beginners_al4_b_en.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_assembly_technical: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_bayesian_data_analysis",
		() => {
			const documentPath = resolveDocument("pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_bayesian_data_analysis: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_code_and_formula",
		() => {
			const documentPath = resolveDocument("pdfs/code_and_formula.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_code_and_formula: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_deep_learning",
		() => {
			const documentPath = resolveDocument("pdfs/fundamentals_of_deep_learning_2014.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_deep_learning: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_embedded_images",
		() => {
			const documentPath = resolveDocument("pdfs/embedded_images_tables.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_embedded_images: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_google_doc",
		() => {
			const documentPath = resolveDocument("pdfs/google_doc_document.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_google_doc: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_large_ciml",
		() => {
			const documentPath = resolveDocument("pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_large_ciml: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_non_english_german",
		() => {
			const documentPath = resolveDocument("pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_non_english_german: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_right_to_left",
		() => {
			const documentPath = resolveDocument("pdfs/right_to_left_01.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_right_to_left: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_simple_text",
		() => {
			const documentPath = resolveDocument("pdfs/fake_memo.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_simple_text: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_tables_large",
		() => {
			const documentPath = resolveDocument("pdfs_with_tables/large.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_tables_large: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_tables_medium",
		() => {
			const documentPath = resolveDocument("pdfs_with_tables/medium.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_tables_medium: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_tables_small",
		() => {
			const documentPath = resolveDocument("pdfs_with_tables/tiny.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_tables_small: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"pdf_technical_stat_learning",
		() => {
			const documentPath = resolveDocument(
				"pdfs/an_introduction_to_statistical_learning_with_applications_in_r_islr_sixth_printing.pdf",
			);
			if (!existsSync(documentPath)) {
				console.warn("Skipping pdf_technical_stat_learning: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);
});
