// Auto-generated tests for ocr fixtures.
// Designed for Cloudflare Workers with Vitest + Miniflare

import { describe, it, expect } from "vitest";
import { extractBytes } from "@kreuzberg/wasm";
import { assertions, buildConfig, getFixture, shouldSkipFixture } from "./helpers.js";
import type { ExtractionResult } from "@kreuzberg/wasm";

describe("ocr", () => {
	it("ocr_image_hello_world", async () => {
		const documentBytes = getFixture("images/test_hello_world.png");
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

	it("ocr_image_no_text", async () => {
		const documentBytes = getFixture("images/flower_no_text.jpg");
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

	it("ocr_pdf_image_only_german", async () => {
		const documentBytes = getFixture("pdfs/image_only_german_pdf.pdf");
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

	it("ocr_pdf_rotated_90", async () => {
		const documentBytes = getFixture("pdfs/ocr_test_rotated_90.pdf");
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

	it("ocr_pdf_tesseract", async () => {
		const documentBytes = getFixture("pdfs/ocr_test.pdf");
		const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
		let result: ExtractionResult | null = null;
		try {
			result = await extractBytes(documentBytes, "application/pdf", config);
		} catch (error) {
			if (
				shouldSkipFixture(
					error,
					"ocr_pdf_tesseract",
					["tesseract"],
					"Skip automatically if OCR backend is unavailable.",
				)
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
});
