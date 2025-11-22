// Auto-generated tests for xml fixtures.

import { existsSync } from "node:fs";
import { describe, it } from "vitest";
import { extractFileSync } from "./index.js";
import type { ExtractionResult } from "./types.js";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.js";

const TEST_TIMEOUT_MS = 60_000;

describe("xml fixtures", () => {
  it("xml_plant_catalog", () => {
    const documentPath = resolveDocument("xml/plant_catalog.xml");
    if (!existsSync(documentPath)) {
      console.warn("Skipping xml_plant_catalog: missing document at", documentPath);
      return;
    }
    const config = buildConfig(undefined);
    let result: ExtractionResult | null = null;
    try {
      result = extractFileSync(documentPath, null, config);
    } catch (error) {
      if (shouldSkipFixture(error, "xml_plant_catalog", [], undefined)) {
        return;
      }
      throw error;
    }
    if (result === null) {
      return;
    }
    assertions.assertExpectedMime(result, ["application/xml"]);
    assertions.assertMinContentLength(result, 100);
    assertions.assertMetadataExpectation(result, "element_count", {"gte":1});
  }, TEST_TIMEOUT_MS);

});

