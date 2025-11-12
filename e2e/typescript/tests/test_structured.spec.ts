// Auto-generated tests for structured fixtures.

import { existsSync } from "node:fs";
import { describe, it } from "vitest";
import { extractFileSync } from "../../../packages/typescript/src/index.js";
import type { ExtractionResult } from "../../../packages/typescript/src/types.js";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers";

const TEST_TIMEOUT_MS = 60_000;

describe("structured fixtures", () => {
	it(
		"structured_json_basic",
		() => {
			const documentPath = resolveDocument("json/sample_document.json");
			if (!existsSync(documentPath)) {
				console.warn("Skipping structured_json_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "structured_json_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/json"]);
			assertions.assertMinContentLength(result, 20);
			assertions.assertContentContainsAny(result, ["Sample Document", "Test Author"]);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"structured_json_simple",
		() => {
			const documentPath = resolveDocument("data_formats/simple.json");
			if (!existsSync(documentPath)) {
				console.warn("Skipping structured_json_simple: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "structured_json_simple", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/json"]);
			assertions.assertMinContentLength(result, 10);
			assertions.assertContentContainsAny(result, ["{", "name"]);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"structured_yaml_simple",
		() => {
			const documentPath = resolveDocument("data_formats/simple.yaml");
			if (!existsSync(documentPath)) {
				console.warn("Skipping structured_yaml_simple: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "structured_yaml_simple", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/x-yaml"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);
});
