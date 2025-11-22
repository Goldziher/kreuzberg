// Auto-generated tests for ocr fixtures.

import { existsSync } from "node:fs";
import { describe, it } from "vitest";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.js";
import { extractFileSync } from "kreuzberg-node";
import type { ExtractionResult } from "kreuzberg-node";

const TEST_TIMEOUT_MS = 60_000;

describe("ocr fixtures", () => {
  it("ocr_image_hello_world", () => {
    const documentPath = resolveDocument("images/test_hello_world.png");
    if (!existsSync(documentPath)) {
      console.warn("Skipping ocr_image_hello_world: missing document at", documentPath);
      console.warn("Notes: Requires Tesseract OCR for image text extraction.");
      return;
    }
    const config = buildConfig({"force_ocr":true,"ocr":{"backend":"tesseract","language":"eng"}});
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
    } catch (error) {
      if (shouldSkipFixture(error, "ocr_image_hello_world", ["tesseract"], "Requires Tesseract OCR for image text extraction.")) {
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
  }, TEST_TIMEOUT_MS);

  it("ocr_image_no_text", () => {
    const documentPath = resolveDocument("images/flower_no_text.jpg");
    if (!existsSync(documentPath)) {
      console.warn("Skipping ocr_image_no_text: missing document at", documentPath);
      console.warn("Notes: Skip when Tesseract is unavailable.");
      return;
    }
    const config = buildConfig({"force_ocr":true,"ocr":{"backend":"tesseract","language":"eng"}});
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
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
  }, TEST_TIMEOUT_MS);

  it("ocr_pdf_image_only_german", () => {
    const documentPath = resolveDocument("pdfs/image_only_german_pdf.pdf");
    if (!existsSync(documentPath)) {
      console.warn("Skipping ocr_pdf_image_only_german: missing document at", documentPath);
      console.warn("Notes: Skip if OCR backend unavailable.");
      return;
    }
    const config = buildConfig({"force_ocr":true,"ocr":{"backend":"tesseract","language":"eng"}});
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
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
    assertions.assertMetadataExpectation(result, "format_type", {"eq":"pdf"});
  }, TEST_TIMEOUT_MS);

  it("ocr_pdf_rotated_90", () => {
    const documentPath = resolveDocument("pdfs/ocr_test_rotated_90.pdf");
    if (!existsSync(documentPath)) {
      console.warn("Skipping ocr_pdf_rotated_90: missing document at", documentPath);
      console.warn("Notes: Skip automatically when OCR backend is missing.");
      return;
    }
    const config = buildConfig({"force_ocr":true,"ocr":{"backend":"tesseract","language":"eng"}});
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
    } catch (error) {
      if (shouldSkipFixture(error, "ocr_pdf_rotated_90", ["tesseract"], "Skip automatically when OCR backend is missing.")) {
        return;
      }
      throw error;
    }
    if (result === null) {
      return;
    }
    assertions.assertExpectedMime(result, ["application/pdf"]);
    assertions.assertMinContentLength(result, 10);
  }, TEST_TIMEOUT_MS);

  it("ocr_pdf_tesseract", () => {
    const documentPath = resolveDocument("pdfs/ocr_test.pdf");
    if (!existsSync(documentPath)) {
      console.warn("Skipping ocr_pdf_tesseract: missing document at", documentPath);
      console.warn("Notes: Skip automatically if OCR backend is unavailable.");
      return;
    }
    const config = buildConfig({"force_ocr":true,"ocr":{"backend":"tesseract","language":"eng"}});
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
    } catch (error) {
      if (shouldSkipFixture(error, "ocr_pdf_tesseract", ["tesseract"], "Skip automatically if OCR backend is unavailable.")) {
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
  }, TEST_TIMEOUT_MS);

});

