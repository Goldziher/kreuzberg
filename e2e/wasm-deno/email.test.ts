// Auto-generated tests for email fixtures.
// Run with: deno test --allow-read

import { extractBytes } from "npm:@kreuzberg/wasm@^4.0.0";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.ts";
import type { ExtractionResult } from "npm:@kreuzberg/wasm@^4.0.0";

const TEST_TIMEOUT_MS = 60_000;

Deno.test("email_sample_eml", { permissions: { read: true }, timeout: TEST_TIMEOUT_MS }, async () => {
	const documentBytes = await resolveDocument("email/sample_email.eml");
	const config = buildConfig(undefined);
	let result: ExtractionResult | null = null;
	try {
		result = await extractBytes(documentBytes, "application/pdf", config);
	} catch (error) {
		if (shouldSkipFixture(error, "email_sample_eml", [], undefined)) {
			return;
		}
		throw error;
	}
	if (result === null) {
		return;
	}
	assertions.assertExpectedMime(result, ["message/rfc822"]);
	assertions.assertMinContentLength(result, 20);
});
