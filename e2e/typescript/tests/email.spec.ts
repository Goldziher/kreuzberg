import { existsSync } from "node:fs";
import { describe, it } from "vitest";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.js";
import { extractFileSync } from "@kreuzberg/node";
import type { ExtractionResult } from "@kreuzberg/node";

const TEST_TIMEOUT_MS = 60_000;

describe("email fixtures", () => {
	it(
		"email_sample_eml",
		() => {
			const documentPath = resolveDocument("email/sample_email.eml");
			if (!existsSync(documentPath)) {
				console.warn("Skipping email_sample_eml: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
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
		},
		TEST_TIMEOUT_MS,
	);
});
