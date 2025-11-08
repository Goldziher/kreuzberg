import { existsSync } from "node:fs";
import { describe, it } from "vitest";
import { extractFileSync } from "../../../packages/typescript/src/index.js";
import type { ExtractionResult } from "../../../packages/typescript/src/types.js";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers";

const TEST_TIMEOUT_MS = 60_000;

describe("html fixtures", () => {
	it(
		"html_complex_layout",
		() => {
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
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"html_simple_table",
		() => {
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
			assertions.assertMinContentLength(result, 20);
			assertions.assertContentContainsAll(result, ["|"]);
		},
		TEST_TIMEOUT_MS,
	);
});
