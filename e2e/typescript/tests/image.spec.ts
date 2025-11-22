// Auto-generated tests for image fixtures.

import { existsSync } from "node:fs";
import { describe, it } from "vitest";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.js";
import { extractFileSync } from "kreuzberg-node";
import type { ExtractionResult } from "kreuzberg-node";

const TEST_TIMEOUT_MS = 60_000;

describe("image fixtures", () => {
  it("image_metadata_only", () => {
    const documentPath = resolveDocument("images/example.jpg");
    if (!existsSync(documentPath)) {
      console.warn("Skipping image_metadata_only: missing document at", documentPath);
      return;
    }
    const config = buildConfig({"ocr":null});
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
    } catch (error) {
      if (shouldSkipFixture(error, "image_metadata_only", [], undefined)) {
        return;
      }
      throw error;
    }
    if (result === null) {
      return;
    }
    assertions.assertExpectedMime(result, ["image/jpeg"]);
    assertions.assertMaxContentLength(result, 100);
  }, TEST_TIMEOUT_MS);

});

