// Auto-generated tests for ocr fixtures.
// Run with: deno test --allow-read

import { extractBytes } from "npm:@kreuzberg/wasm@^4.0.0";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "npm:@kreuzberg/wasm@^4.0.0";

const TEST_TIMEOUT_MS = 60_000;

Deno.test("ocr_image_hello_world", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("images/test_hello_world.png");
	const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (
			shouldSkipFixture(
				error,
				"ocr_image_hello_world",
				["tesseract"],
				"Requires Tesseract OCR for image text extraction.",
			)
		) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["image/png"]);
	assertions.assertMinContentLength(result, 5);
	assertions.assertContentContainsAny(result, ["hello", "world"]);
});

Deno.test("ocr_image_no_text", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("images/flower_no_text.jpg");
	const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "ocr_image_no_text", ["tesseract"], "Skip when Tesseract is unavailable.")) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["image/jpeg"]);
	assertions.assertMaxContentLength(result, 200);
});

Deno.test("ocr_pdf_image_only_german", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/image_only_german_pdf.pdf");
	const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "ocr_pdf_image_only_german", ["tesseract"], "Skip if OCR backend unavailable.")) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 20);
	assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
});

Deno.test("ocr_pdf_rotated_90", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/ocr_test_rotated_90.pdf");
	const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (
			shouldSkipFixture(error, "ocr_pdf_rotated_90", ["tesseract"], "Skip automatically when OCR backend is missing.")
		) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 10);
});

Deno.test("ocr_pdf_tesseract", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("pdfs/ocr_test.pdf");
	const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (
			shouldSkipFixture(error, "ocr_pdf_tesseract", ["tesseract"], "Skip automatically if OCR backend is unavailable.")
		) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["application/pdf"]);
	assertions.assertMinContentLength(result, 20);
	assertions.assertContentContainsAny(result, ["Docling", "Markdown", "JSON"]);
});
