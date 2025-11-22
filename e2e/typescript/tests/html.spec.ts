// Auto-generated tests for html fixtures.

import { existsSync } from "node:fs";
import { describe, it } from "vitest";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.js";
import { extractFileSync } from "kreuzberg-node";
import type { ExtractionResult } from "kreuzberg-node";

const TEST_TIMEOUT_MS = 60_000;

describe("html fixtures", () => {
  it("html_complex_layout", () => {
    const documentPath = resolveDocument("web/taylor_swift.html");
    if (!existsSync(documentPath)) {
      console.warn("Skipping html_complex_layout: missing document at", documentPath);
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
    } catch (error) {
      if (shouldSkipFixture(error, "html_complex_layout", [], undefined)) {
        return;
      }
      throw error;
    }
    if (result === null) {
      return;
    }
    assertions.assertExpectedMime(result, ["text/html"]);
    assertions.assertMinContentLength(result, 1000);
  }, TEST_TIMEOUT_MS);

  it("html_simple_table", () => {
    const documentPath = resolveDocument("web/simple_table.html");
    if (!existsSync(documentPath)) {
      console.warn("Skipping html_simple_table: missing document at", documentPath);
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
    } catch (error) {
      if (shouldSkipFixture(error, "html_simple_table", [], undefined)) {
        return;
      }
      throw error;
    }
    if (result === null) {
      return;
    }
    assertions.assertExpectedMime(result, ["text/html"]);
    assertions.assertMinContentLength(result, 100);
    assertions.assertContentContainsAll(result, ["Product", "Category", "Price", "Stock", "Laptop", "Electronics", "Sample Data Table"]);
    assertions.assertTableCount(result, 1, null);
  }, TEST_TIMEOUT_MS);

});

