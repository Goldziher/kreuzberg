// Auto-generated tests for smoke fixtures.

import { existsSync } from "node:fs";
import { describe, it } from "vitest";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.js";
import { extractFileSync } from "kreuzberg-node";
import type { ExtractionResult } from "kreuzberg-node";

const TEST_TIMEOUT_MS = 60_000;

describe("smoke fixtures", () => {
  it("smoke_docx_basic", () => {
    const documentPath = resolveDocument("documents/fake.docx");
    if (!existsSync(documentPath)) {
      console.warn("Skipping smoke_docx_basic: missing document at", documentPath);
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
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
  }, TEST_TIMEOUT_MS);

  it("smoke_html_basic", () => {
    const documentPath = resolveDocument("web/simple_table.html");
    if (!existsSync(documentPath)) {
      console.warn("Skipping smoke_html_basic: missing document at", documentPath);
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
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
  }, TEST_TIMEOUT_MS);

  it("smoke_image_png", () => {
    const documentPath = resolveDocument("images/sample.png");
    if (!existsSync(documentPath)) {
      console.warn("Skipping smoke_image_png: missing document at", documentPath);
      console.warn("Notes: Image extraction requires image processing dependencies");
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
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
  }, TEST_TIMEOUT_MS);

  it("smoke_json_basic", () => {
    const documentPath = resolveDocument("data_formats/simple.json");
    if (!existsSync(documentPath)) {
      console.warn("Skipping smoke_json_basic: missing document at", documentPath);
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
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
  }, TEST_TIMEOUT_MS);

  it("smoke_pdf_basic", () => {
    const documentPath = resolveDocument("pdfs/fake_memo.pdf");
    if (!existsSync(documentPath)) {
      console.warn("Skipping smoke_pdf_basic: missing document at", documentPath);
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
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
  }, TEST_TIMEOUT_MS);

  it("smoke_txt_basic", () => {
    const documentPath = resolveDocument("text/report.txt");
    if (!existsSync(documentPath)) {
      console.warn("Skipping smoke_txt_basic: missing document at", documentPath);
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
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
  }, TEST_TIMEOUT_MS);

  it("smoke_xlsx_basic", () => {
    const documentPath = resolveDocument("spreadsheets/stanley_cups.xlsx");
    if (!existsSync(documentPath)) {
      console.warn("Skipping smoke_xlsx_basic: missing document at", documentPath);
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
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
    assertions.assertContentContainsAll(result, ["Team", "Location", "Stanley Cups", "Blues", "Flyers", "Maple Leafs", "STL", "PHI", "TOR"]);
    assertions.assertTableCount(result, 1, null);
    assertions.assertMetadataExpectation(result, "sheet_count", {"gte":2});
    assertions.assertMetadataExpectation(result, "sheet_names", {"contains":"Stanley Cups"});
  }, TEST_TIMEOUT_MS);

});

