import fs from "node:fs";
import { describe, expect, it } from "vitest";
import { batchExtractBytesSync, batchExtractFiles, batchExtractFilesSync } from "../../src/index.js";
import { getTestDocumentPath } from "../helpers/integration-helpers.js";

describe("Batch API Stress Tests", () => {
	it("should handle batch extractions without NAPI crash", async () => {
		const files = [
			getTestDocumentPath("json/simple.json"),
			getTestDocumentPath("pandoc/simple_metadata.md"),
			getTestDocumentPath("text/contract.txt"),
		].filter((p) => fs.existsSync(p));

		if (files.length === 0) {
			console.log("Skipping: No test files available");
			return;
		}

		const results = await batchExtractFiles(files);
		expect(results.length).toBe(files.length);

		console.log(`✅ batchExtractFiles: ${files.length} files extracted`);
	});

	it("should handle batchExtractFilesSync", () => {
		const files = [getTestDocumentPath("json/simple.json"), getTestDocumentPath("text/contract.txt")].filter((p) =>
			fs.existsSync(p),
		);

		if (files.length === 0) {
			console.log("Skipping: No test files available");
			return;
		}

		const results = batchExtractFilesSync(files);
		expect(results.length).toBe(files.length);

		console.log(`✅ batchExtractFilesSync: ${files.length} files extracted`);
	});

	it("should handle batchExtractBytesSync", () => {
		const files = [getTestDocumentPath("json/simple.json"), getTestDocumentPath("text/contract.txt")].filter((p) =>
			fs.existsSync(p),
		);

		if (files.length === 0) {
			console.log("Skipping: No test files available");
			return;
		}

		const dataList = files.map((file) => fs.readFileSync(file));
		const mimeTypes = files.map(() => "application/json");

		const results = batchExtractBytesSync(dataList, mimeTypes);
		expect(results.length).toBe(files.length);

		console.log(`✅ batchExtractBytesSync: ${files.length} items extracted`);
	});
});
