import { afterEach, describe, expect, it, vi } from "vitest";

type MockRawResult = {
	content: string;
	mimeType: string;
	metadata: unknown;
	tables?: unknown;
	detectedLanguages?: unknown;
	chunks?: unknown;
};

function mockBinding(metadata: unknown): MockRawResult {
	return {
		content: "mock-content",
		mimeType: "text/plain",
		metadata,
		tables: undefined,
		detectedLanguages: undefined,
		chunks: undefined,
	};
}

describe("Binding conversion", () => {
	afterEach(async () => {
		const module = await import("../../dist/index.js");
		module.__resetBindingForTests();
		vi.resetModules();
		vi.clearAllMocks();
	});

	it("parses JSON metadata returned as a string", async () => {
		const module = await import("../../dist/index.js");
		module.__setBindingForTests({
			extractFileSync: vi.fn().mockReturnValue(mockBinding('{"key":"value"}')),
		});

		const result = module.extractFileSync("dummy.txt", null, null);

		expect(result.metadata).toStrictEqual({ key: "value" });
	});

	it("returns empty object for invalid JSON metadata", async () => {
		const module = await import("../../dist/index.js");
		module.__setBindingForTests({
			extractFileSync: vi.fn().mockReturnValue(mockBinding("{invalid")),
		});

		const result = module.extractFileSync("dummy.txt", null, null);

		expect(result.metadata).toStrictEqual({});
	});
});
