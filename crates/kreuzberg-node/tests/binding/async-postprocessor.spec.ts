/**
 * Async PostProcessor Tests
 *
 * Tests the new async PostProcessor wrapper that was implemented to support:
 * - Async processors with Promise return types
 * - Clean TypeScript API (no manual JSON serialization)
 * - Automatic metadata parsing (string → object)
 * - Automatic case conversion (snake_case ↔ camelCase)
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
	__resetBindingForTests,
	__setBindingForTests,
	clearPostProcessors,
	type ExtractionResult,
	extractBytes,
	type PostProcessorProtocol,
	registerPostProcessor,
} from "../../dist/index.js";
import { createMockExtractionBinding } from "./helpers/mock-binding.js";

class AsyncWordCountProcessor implements PostProcessorProtocol {
	name(): string {
		return "async_word_counter";
	}

	async process(result: ExtractionResult): Promise<ExtractionResult> {
		await new Promise((resolve) => setTimeout(resolve, 10));

		const wordCount = result.content.split(/\s+/).filter((w) => w).length;
		result.metadata.async_word_count = wordCount;
		result.metadata.processed_async = true;

		return result;
	}

	processingStage() {
		return "middle" as const;
	}
}

class SyncWordCountProcessor implements PostProcessorProtocol {
	name(): string {
		return "sync_word_counter";
	}

	process(result: ExtractionResult): ExtractionResult {
		const wordCount = result.content.split(/\s+/).filter((w) => w).length;
		result.metadata.sync_word_count = wordCount;
		result.metadata.processed_sync = true;

		return result;
	}

	processingStage() {
		return "middle" as const;
	}
}

class MetadataManipulationProcessor implements PostProcessorProtocol {
	name(): string {
		return "metadata_manipulator";
	}

	process(result: ExtractionResult): ExtractionResult {
		expect(typeof result.metadata).toBe("object");

		result.metadata.custom = {
			nested: {
				value: "test",
				count: 42,
			},
			array: [1, 2, 3],
		};

		return result;
	}
}

class CaseConversionTestProcessor implements PostProcessorProtocol {
	name(): string {
		return "case_converter";
	}

	process(result: ExtractionResult): ExtractionResult {
		expect(result.mimeType).toBeDefined();
		expect(typeof result.mimeType).toBe("string");

		if (result.detectedLanguages !== null && result.detectedLanguages !== undefined) {
			expect(Array.isArray(result.detectedLanguages)).toBe(true);
		}

		result.metadata.case_test_passed = true;

		return result;
	}
}

class AsyncErrorProcessor implements PostProcessorProtocol {
	name(): string {
		return "async_error";
	}

	async process(_result: ExtractionResult): Promise<ExtractionResult> {
		await new Promise((resolve) => setTimeout(resolve, 5));
		throw new Error("Async processor error");
	}
}

class ChainedAsyncProcessor implements PostProcessorProtocol {
	private counter: number = 0;

	name(): string {
		return "chained_async";
	}

	async process(result: ExtractionResult): Promise<ExtractionResult> {
		await new Promise((resolve) => setTimeout(resolve, 5));

		this.counter++;
		result.metadata.chain_count = this.counter;
		result.metadata.previous_metadata_keys = Object.keys(result.metadata);

		return result;
	}

	processingStage() {
		return "late" as const;
	}
}

describe("Async PostProcessor Support", () => {
	beforeEach(() => {
		const mockBinding = createMockExtractionBinding();
		__setBindingForTests(mockBinding);
		clearPostProcessors();
	});

	afterEach(() => {
		__resetBindingForTests();
	});

	describe("Async Processor Execution", () => {
		it("should execute async processor with Promise return", async () => {
			const processor = new AsyncWordCountProcessor();
			registerPostProcessor(processor);

			const testContent = "Hello world from async processor";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata.async_word_count).toBe(5);
			expect(result.metadata.processed_async).toBe(true);
		});

		it("should execute sync processor normally", async () => {
			const processor = new SyncWordCountProcessor();
			registerPostProcessor(processor);

			const testContent = "Hello world from sync processor";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata.sync_word_count).toBe(5);
			expect(result.metadata.processed_sync).toBe(true);
		});

		it("should support mixing async and sync processors", async () => {
			const asyncProc = new AsyncWordCountProcessor();
			const syncProc = new SyncWordCountProcessor();

			registerPostProcessor(asyncProc);
			registerPostProcessor(syncProc);

			const testContent = "Test mixed processors";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata.async_word_count).toBe(3);
			expect(result.metadata.sync_word_count).toBe(3);
			expect(result.metadata.processed_async).toBe(true);
			expect(result.metadata.processed_sync).toBe(true);
		});
	});

	describe("Clean API - Metadata Handling", () => {
		it("should automatically parse metadata from string to object", async () => {
			const processor = new MetadataManipulationProcessor();
			registerPostProcessor(processor);

			const result = await extractBytes(Buffer.from("Test metadata parsing"), "text/plain");

			expect(result.metadata.custom).toBeDefined();
			expect(result.metadata.custom.nested.value).toBe("test");
			expect(result.metadata.custom.nested.count).toBe(42);
			expect(result.metadata.custom.array).toEqual([1, 2, 3]);
		});

		it("should preserve existing metadata when adding new fields", async () => {
			const metaProc = new MetadataManipulationProcessor();
			const wordProc = new SyncWordCountProcessor();

			registerPostProcessor(wordProc);
			registerPostProcessor(metaProc);

			const result = await extractBytes(Buffer.from("Test metadata preservation"), "text/plain");

			expect(result.metadata.sync_word_count).toBeDefined();
			expect(result.metadata.custom).toBeDefined();
		});
	});

	describe("Case Conversion", () => {
		it("should provide camelCase properties to processors", async () => {
			const processor = new CaseConversionTestProcessor();
			registerPostProcessor(processor);

			const result = await extractBytes(Buffer.from("Test case conversion"), "text/plain");

			expect(result.metadata.case_test_passed).toBe(true);
		});

		it("should handle all ExtractionResult properties correctly", async () => {
			class PropertyTestProcessor implements PostProcessorProtocol {
				name() {
					return "prop_test";
				}

				process(result: ExtractionResult): ExtractionResult {
					expect(result.content).toBeDefined();
					expect(result.mimeType).toBeDefined();
					expect(result.metadata).toBeDefined();
					expect(result.tables).toBeDefined();
					expect(Array.isArray(result.tables)).toBe(true);

					expect(result.detectedLanguages === null || Array.isArray(result.detectedLanguages)).toBe(true);
					expect(result.chunks === null || Array.isArray(result.chunks)).toBe(true);
					expect(result.images === null || Array.isArray(result.images)).toBe(true);

					result.metadata.all_props_valid = true;
					return result;
				}
			}

			const processor = new PropertyTestProcessor();
			registerPostProcessor(processor);

			const result = await extractBytes(Buffer.from("Test all properties"), "text/plain");

			expect(result.metadata.all_props_valid).toBe(true);
		});
	});

	describe("Error Handling", () => {
		it("should handle async processor errors gracefully", async () => {
			const errorProc = new AsyncErrorProcessor();
			registerPostProcessor(errorProc);

			const result = await extractBytes(Buffer.from("Test async error handling"), "text/plain");

			const errorKey = Object.keys(result.metadata).find((k) => k.includes("error"));
			expect(errorKey).toBeDefined();
		});

		it("should continue processing other processors after error", async () => {
			const errorProc = new AsyncErrorProcessor();
			const wordProc = new AsyncWordCountProcessor();

			registerPostProcessor(errorProc);
			registerPostProcessor(wordProc);

			const result = await extractBytes(Buffer.from("Test error recovery one two three"), "text/plain");

			expect(result.metadata.async_word_count).toBeDefined();
		});
	});

	describe("Processor Chaining", () => {
		it("should chain multiple async processors correctly", async () => {
			const proc1 = new AsyncWordCountProcessor();
			const proc2 = new ChainedAsyncProcessor();

			registerPostProcessor(proc1);
			registerPostProcessor(proc2);

			const result = await extractBytes(Buffer.from("Test chaining one two three"), "text/plain");

			expect(result.metadata.async_word_count).toBeDefined();
			expect(result.metadata.chain_count).toBeDefined();
			expect(result.metadata.previous_metadata_keys).toBeDefined();
			expect((result.metadata.previous_metadata_keys as string[]).includes("async_word_count")).toBe(true);
		});

		it("should maintain processor state across multiple extractions", async () => {
			const processor = new ChainedAsyncProcessor();
			registerPostProcessor(processor);

			const result1 = await extractBytes(Buffer.from("First"), "text/plain");
			const count1 = result1.metadata.chain_count as number;

			const result2 = await extractBytes(Buffer.from("Second"), "text/plain");
			const count2 = result2.metadata.chain_count as number;

			expect(count2).toBeGreaterThan(count1);
		});
	});

	describe("Performance", () => {
		it("should handle concurrent async processors efficiently", async () => {
			const processors = Array.from(
				{ length: 5 },
				(_, i) =>
					new (class implements PostProcessorProtocol {
						name() {
							return `async_proc_${i}`;
						}
						async process(result: ExtractionResult) {
							await new Promise((resolve) => setTimeout(resolve, 10));
							result.metadata[`proc_${i}_executed`] = true;
							return result;
						}
					})(),
			);

			for (const p of processors) {
				registerPostProcessor(p);
			}

			const start = Date.now();
			const result = await extractBytes(Buffer.from("Concurrent test"), "text/plain");
			const duration = Date.now() - start;

			for (let i = 0; i < 5; i++) {
				expect(result.metadata[`proc_${i}_executed`]).toBe(true);
			}

			console.log(`Concurrent execution took ${duration}ms`);
		});
	});

	describe("Integration with extraction API", () => {
		it("should preserve sync processor behaviour when used via async extraction", async () => {
			const processor = new SyncWordCountProcessor();
			registerPostProcessor(processor);

			const result = await extractBytes(Buffer.from("Test sync extraction five words"), "text/plain");

			expect(result.metadata.sync_word_count).toBe(5);
		});
	});
});
