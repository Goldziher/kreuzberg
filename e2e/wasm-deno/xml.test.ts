// Auto-generated tests for xml fixtures.
// Run with: deno test --allow-read

import { extractBytes } from "npm:@kreuzberg/wasm@^4.0.0";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "npm:@kreuzberg/wasm@^4.0.0";

const TEST_TIMEOUT_MS = 60_000;

Deno.test("xml_plant_catalog", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("xml/plant_catalog.xml");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
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
	assertions.assertMetadataExpectation(result, "element_count", { gte: 1 });
});
