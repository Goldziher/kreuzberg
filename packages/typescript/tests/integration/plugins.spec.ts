/**
 * Integration tests for plugin registration and execution.
 *
 * Tests the full plugin lifecycle for:
 * - OCR backends
 * - Post-processors
 * - Validators
 *
 * These tests verify that plugins work correctly with the Rust core through
 * the NAPI bridge, including proper state management, error handling, and
 * concurrent execution.
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
	clearPostProcessors,
	clearValidators,
	type ExtractionResult,
	extractBytes,
	type OcrBackendProtocol,
	type PostProcessorProtocol,
	registerOcrBackend,
	registerPostProcessor,
	registerValidator,
	unregisterPostProcessor,
	unregisterValidator,
	type ValidatorProtocol,
} from "../../src/index.js";

describe("Plugin Integration Tests - OCR Backends", () => {
	describe("OCR Backend Registration", () => {
		it("should register and use custom OCR backend", async () => {
			class MockOcrBackend implements OcrBackendProtocol {
				name(): string {
					return "mock-ocr-test";
				}

				supportedLanguages(): string[] {
					return ["en", "de", "fr"];
				}

				async processImage(imageBytes: Uint8Array, language: string) {
					expect(imageBytes).toBeInstanceOf(Uint8Array);
					expect(imageBytes.length).toBeGreaterThan(0);
					expect(["en", "de", "fr"]).toContain(language);

					return {
						content: `Mock OCR extracted text in ${language}`,
						mime_type: "text/plain",
						metadata: {
							language,
							confidence: 0.95,
							backend: "mock-ocr-test",
						},
						tables: [],
					};
				}
			}

			const backend = new MockOcrBackend();
			registerOcrBackend(backend);

			console.log("✓ Mock OCR backend registered successfully");
		});

		it("should reject OCR backend with empty name", () => {
			class InvalidBackend implements OcrBackendProtocol {
				name(): string {
					return "";
				}

				supportedLanguages(): string[] {
					return ["en"];
				}

				async processImage() {
					return {
						content: "test",
						mime_type: "text/plain",
						metadata: {},
						tables: [],
					};
				}
			}

			expect(() => registerOcrBackend(new InvalidBackend())).toThrow();
		});

		it("should reject OCR backend with no supported languages", () => {
			class InvalidBackend implements OcrBackendProtocol {
				name(): string {
					return "invalid-ocr";
				}

				supportedLanguages(): string[] {
					return [];
				}

				async processImage() {
					return {
						content: "test",
						mime_type: "text/plain",
						metadata: {},
						tables: [],
					};
				}
			}

			expect(() => registerOcrBackend(new InvalidBackend())).toThrow();
		});

		it("should handle OCR backend with initialization", async () => {
			let initCalled = false;
			let shutdownCalled = false;

			class InitializableOcrBackend implements OcrBackendProtocol {
				name(): string {
					return "init-ocr-test";
				}

				supportedLanguages(): string[] {
					return ["en"];
				}

				async initialize(): Promise<void> {
					initCalled = true;
				}

				async shutdown(): Promise<void> {
					shutdownCalled = true;
				}

				async processImage() {
					return {
						content: "Initialized OCR result",
						mime_type: "text/plain",
						metadata: { initialized: initCalled },
						tables: [],
					};
				}
			}

			const backend = new InitializableOcrBackend();
			await backend.initialize();
			registerOcrBackend(backend);

			expect(initCalled).toBe(true);

			await backend.shutdown();
			expect(shutdownCalled).toBe(true);
		});
	});
});

describe("Plugin Integration Tests - Post-Processors", () => {
	beforeEach(() => {
		clearPostProcessors();
	});

	afterEach(() => {
		clearPostProcessors();
	});

	describe("Post-Processor Registration and Execution", () => {
		it("should register and execute post-processor with async extraction", async () => {
			class TestPostProcessor implements PostProcessorProtocol {
				name(): string {
					return "test-processor";
				}

				process(result: ExtractionResult): ExtractionResult {
					result.metadata.test_processor_executed = true;
					result.metadata.processed_at = new Date().toISOString();
					return result;
				}

				processingStage() {
					return "middle" as const;
				}
			}

			registerPostProcessor(new TestPostProcessor());

			const testContent = "Test document content for post-processor";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata.test_processor_executed).toBe(true);
			expect(result.metadata.processed_at).toBeDefined();
		});

		it("should execute multiple post-processors in stage order", async () => {
			const executionOrder: string[] = [];

			class EarlyProcessor implements PostProcessorProtocol {
				name() {
					return "early-proc";
				}
				process(result: ExtractionResult): ExtractionResult {
					executionOrder.push("early");
					result.metadata.early_stage = true;
					return result;
				}
				processingStage() {
					return "early" as const;
				}
			}

			class MiddleProcessor implements PostProcessorProtocol {
				name() {
					return "middle-proc";
				}
				process(result: ExtractionResult): ExtractionResult {
					executionOrder.push("middle");
					result.metadata.middle_stage = true;
					return result;
				}
				processingStage() {
					return "middle" as const;
				}
			}

			class LateProcessor implements PostProcessorProtocol {
				name() {
					return "late-proc";
				}
				process(result: ExtractionResult): ExtractionResult {
					executionOrder.push("late");
					result.metadata.late_stage = true;
					return result;
				}
				processingStage() {
					return "late" as const;
				}
			}

			registerPostProcessor(new LateProcessor());
			registerPostProcessor(new MiddleProcessor());
			registerPostProcessor(new EarlyProcessor());

			const result = await extractBytes(Buffer.from("Test content"), "text/plain");

			expect(executionOrder).toEqual(["early", "middle", "late"]);
			expect(result.metadata.early_stage).toBe(true);
			expect(result.metadata.middle_stage).toBe(true);
			expect(result.metadata.late_stage).toBe(true);
		});

		it("should handle async post-processors", async () => {
			class AsyncProcessor implements PostProcessorProtocol {
				name() {
					return "async-proc";
				}

				async process(result: ExtractionResult): Promise<ExtractionResult> {
					await new Promise((resolve) => setTimeout(resolve, 10));
					result.metadata.async_processed = true;
					result.metadata.timestamp = Date.now();
					return result;
				}

				processingStage() {
					return "middle" as const;
				}
			}

			registerPostProcessor(new AsyncProcessor());

			const result = await extractBytes(Buffer.from("Async test"), "text/plain");

			expect(result.metadata.async_processed).toBe(true);
			expect(result.metadata.timestamp).toBeDefined();
		});

		it("should maintain state across multiple extractions", async () => {
			class StatefulProcessor implements PostProcessorProtocol {
				private counter = 0;

				name() {
					return "stateful-proc";
				}

				process(result: ExtractionResult): ExtractionResult {
					this.counter++;
					result.metadata.call_count = this.counter;
					return result;
				}

				processingStage() {
					return "middle" as const;
				}
			}

			const processor = new StatefulProcessor();
			registerPostProcessor(processor);

			const result1 = await extractBytes(Buffer.from("First call"), "text/plain");
			expect(result1.metadata.call_count).toBe(1);

			const result2 = await extractBytes(Buffer.from("Second call"), "text/plain");
			expect(result2.metadata.call_count).toBe(2);

			const result3 = await extractBytes(Buffer.from("Third call"), "text/plain");
			expect(result3.metadata.call_count).toBe(3);
		});

		it("should handle post-processor that modifies content", async () => {
			class ContentModifier implements PostProcessorProtocol {
				name() {
					return "content-modifier";
				}

				process(result: ExtractionResult): ExtractionResult {
					result.content = result.content.toUpperCase();
					result.metadata.content_modified = true;
					return result;
				}

				processingStage() {
					return "late" as const;
				}
			}

			registerPostProcessor(new ContentModifier());

			const testContent = "lowercase content";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.content).toBe("LOWERCASE CONTENT");
			expect(result.metadata.content_modified).toBe(true);
		});

		it("should handle post-processor errors gracefully", async () => {
			class ErrorProcessor implements PostProcessorProtocol {
				name() {
					return "error-proc";
				}

				process(result: ExtractionResult): ExtractionResult {
					throw new Error("Intentional error for testing");
				}

				processingStage() {
					return "middle" as const;
				}
			}

			registerPostProcessor(new ErrorProcessor());

			const result = await extractBytes(Buffer.from("Test error handling"), "text/plain");

			const errorKey = Object.keys(result.metadata).find((k) => k.includes("error"));
			expect(errorKey).toBeDefined();
		});
	});

	describe("Post-Processor Registry Management", () => {
		it("should unregister post-processor by name", async () => {
			class UnregisterTestProcessor implements PostProcessorProtocol {
				name() {
					return "unreg-proc";
				}

				process(result: ExtractionResult): ExtractionResult {
					result.metadata.unreg_executed = true;
					return result;
				}
			}

			registerPostProcessor(new UnregisterTestProcessor());

			let result = await extractBytes(Buffer.from("Test"), "text/plain");
			expect(result.metadata.unreg_executed).toBe(true);

			unregisterPostProcessor("unreg-proc");

			result = await extractBytes(Buffer.from("Test"), "text/plain");
			expect(result.metadata.unreg_executed).toBeUndefined();
		});

		it("should clear all post-processors", async () => {
			class Proc1 implements PostProcessorProtocol {
				name() {
					return "proc1";
				}
				process(result: ExtractionResult): ExtractionResult {
					result.metadata.proc1 = true;
					return result;
				}
			}

			class Proc2 implements PostProcessorProtocol {
				name() {
					return "proc2";
				}
				process(result: ExtractionResult): ExtractionResult {
					result.metadata.proc2 = true;
					return result;
				}
			}

			registerPostProcessor(new Proc1());
			registerPostProcessor(new Proc2());

			let result = await extractBytes(Buffer.from("Test"), "text/plain");
			expect(result.metadata.proc1).toBe(true);
			expect(result.metadata.proc2).toBe(true);

			clearPostProcessors();

			result = await extractBytes(Buffer.from("Test"), "text/plain");
			expect(result.metadata.proc1).toBeUndefined();
			expect(result.metadata.proc2).toBeUndefined();
		});

		it("should handle duplicate processor names", async () => {
			class Proc1 implements PostProcessorProtocol {
				name() {
					return "duplicate";
				}
				process(result: ExtractionResult): ExtractionResult {
					result.metadata.proc_version = 1;
					return result;
				}
			}

			class Proc2 implements PostProcessorProtocol {
				name() {
					return "duplicate";
				}
				process(result: ExtractionResult): ExtractionResult {
					result.metadata.proc_version = 2;
					return result;
				}
			}

			registerPostProcessor(new Proc1());
			registerPostProcessor(new Proc2());

			const result = await extractBytes(Buffer.from("Test"), "text/plain");
			expect(result.metadata.proc_version).toBeDefined();
		});
	});

	describe("Post-Processor with Configuration", () => {
		it("should respect postprocessor.enabled = false", async () => {
			class SkippableProcessor implements PostProcessorProtocol {
				name() {
					return "skippable";
				}
				process(result: ExtractionResult): ExtractionResult {
					result.metadata.skippable_executed = true;
					return result;
				}
			}

			registerPostProcessor(new SkippableProcessor());

			const resultEnabled = await extractBytes(Buffer.from("Test"), "text/plain", {
				postprocessor: { enabled: true },
			});
			expect(resultEnabled.metadata.skippable_executed).toBe(true);

			const resultDisabled = await extractBytes(Buffer.from("Test"), "text/plain", {
				postprocessor: { enabled: false },
			});
			expect(resultDisabled.metadata.skippable_executed).toBeUndefined();
		});

		it("should respect processor whitelist", async () => {
			class Proc1 implements PostProcessorProtocol {
				name() {
					return "whitelisted";
				}
				process(result: ExtractionResult): ExtractionResult {
					result.metadata.whitelisted = true;
					return result;
				}
			}

			class Proc2 implements PostProcessorProtocol {
				name() {
					return "not-whitelisted";
				}
				process(result: ExtractionResult): ExtractionResult {
					result.metadata.not_whitelisted = true;
					return result;
				}
			}

			registerPostProcessor(new Proc1());
			registerPostProcessor(new Proc2());

			const result = await extractBytes(Buffer.from("Test"), "text/plain", {
				postprocessor: {
					enabled: true,
					enabledProcessors: ["whitelisted"],
				},
			});

			expect(result.metadata.whitelisted).toBe(true);
			expect(result.metadata.not_whitelisted).toBeUndefined();
		});

		it("should respect processor blacklist", async () => {
			class Proc1 implements PostProcessorProtocol {
				name() {
					return "allowed";
				}
				process(result: ExtractionResult): ExtractionResult {
					result.metadata.allowed = true;
					return result;
				}
			}

			class Proc2 implements PostProcessorProtocol {
				name() {
					return "blocked";
				}
				process(result: ExtractionResult): ExtractionResult {
					result.metadata.blocked = true;
					return result;
				}
			}

			registerPostProcessor(new Proc1());
			registerPostProcessor(new Proc2());

			const result = await extractBytes(Buffer.from("Test"), "text/plain", {
				postprocessor: {
					enabled: true,
					disabledProcessors: ["blocked"],
				},
			});

			expect(result.metadata.allowed).toBe(true);
			expect(result.metadata.blocked).toBeUndefined();
		});
	});
});

describe("Plugin Integration Tests - Validators", () => {
	beforeEach(() => {
		clearValidators();
	});

	afterEach(() => {
		clearValidators();
	});

	describe("Validator Registration and Execution", () => {
		it("should register and execute validator that passes", async () => {
			class PassValidator implements ValidatorProtocol {
				name(): string {
					return "pass-validator";
				}

				validate(result: ExtractionResult): void {
					expect(result.content).toBeDefined();
				}
			}

			registerValidator(new PassValidator());

			const result = await extractBytes(Buffer.from("Valid content"), "text/plain");
			expect(result.content).toBe("Valid content");
		});

		it("should fail extraction when validator throws error", async () => {
			class FailValidator implements ValidatorProtocol {
				name(): string {
					return "fail-validator";
				}

				validate(result: ExtractionResult): void {
					if (result.content.length < 20) {
						throw new Error("ValidationError: Content too short");
					}
				}
			}

			registerValidator(new FailValidator());

			await expect(extractBytes(Buffer.from("Short"), "text/plain")).rejects.toThrow(
				/ValidationError|Content too short/,
			);
		});

		it("should execute validators in priority order", async () => {
			const executionOrder: string[] = [];

			class HighPriorityValidator implements ValidatorProtocol {
				name() {
					return "high-priority";
				}

				priority() {
					return 100;
				}

				validate(_result: ExtractionResult): void {
					executionOrder.push("high");
				}
			}

			class MediumPriorityValidator implements ValidatorProtocol {
				name() {
					return "medium-priority";
				}

				priority() {
					return 50;
				}

				validate(_result: ExtractionResult): void {
					executionOrder.push("medium");
				}
			}

			class LowPriorityValidator implements ValidatorProtocol {
				name() {
					return "low-priority";
				}

				priority() {
					return 10;
				}

				validate(_result: ExtractionResult): void {
					executionOrder.push("low");
				}
			}

			registerValidator(new LowPriorityValidator());
			registerValidator(new HighPriorityValidator());
			registerValidator(new MediumPriorityValidator());

			await extractBytes(Buffer.from("Test content"), "text/plain");

			expect(executionOrder).toEqual(["high", "medium", "low"]);
		});

		it("should stop on first validation failure (fail-fast)", async () => {
			let secondCalled = false;

			class FirstValidator implements ValidatorProtocol {
				name() {
					return "first";
				}

				priority() {
					return 100;
				}

				validate(_result: ExtractionResult): void {
					throw new Error("ValidationError: First validator failed");
				}
			}

			class SecondValidator implements ValidatorProtocol {
				name() {
					return "second";
				}

				priority() {
					return 50;
				}

				validate(_result: ExtractionResult): void {
					secondCalled = true;
				}
			}

			registerValidator(new FirstValidator());
			registerValidator(new SecondValidator());

			await expect(extractBytes(Buffer.from("Test"), "text/plain")).rejects.toThrow();

			expect(secondCalled).toBe(false);
		});

		it("should handle async validators", async () => {
			class AsyncValidator implements ValidatorProtocol {
				name() {
					return "async-validator";
				}

				async validate(result: ExtractionResult): Promise<void> {
					await new Promise((resolve) => setTimeout(resolve, 10));

					if (result.content.length < 5) {
						throw new Error("ValidationError: Content too short after async check");
					}
				}
			}

			registerValidator(new AsyncValidator());

			await expect(extractBytes(Buffer.from("Sh"), "text/plain")).rejects.toThrow(/ValidationError/);

			const validResult = await extractBytes(Buffer.from("Long enough content"), "text/plain");
			expect(validResult.content).toBe("Long enough content");
		});

		it("should validate content requirements", async () => {
			class ContentValidator implements ValidatorProtocol {
				name() {
					return "content-validator";
				}

				validate(result: ExtractionResult): void {
					if (!result.content.includes("required")) {
						throw new Error("ValidationError: Missing required keyword");
					}
				}
			}

			registerValidator(new ContentValidator());

			await expect(extractBytes(Buffer.from("This is invalid"), "text/plain")).rejects.toThrow(/required keyword/);

			const validResult = await extractBytes(Buffer.from("This has the required keyword"), "text/plain");
			expect(validResult.content).toContain("required");
		});

		it("should validate with multiple validators (all pass)", async () => {
			class LengthValidator implements ValidatorProtocol {
				name() {
					return "length-val";
				}

				validate(result: ExtractionResult): void {
					if (result.content.length < 10) {
						throw new Error("ValidationError: Too short");
					}
				}
			}

			class WordCountValidator implements ValidatorProtocol {
				name() {
					return "word-count-val";
				}

				validate(result: ExtractionResult): void {
					const words = result.content.split(/\s+/);
					if (words.length < 3) {
						throw new Error("ValidationError: Need more words");
					}
				}
			}

			registerValidator(new LengthValidator());
			registerValidator(new WordCountValidator());

			const result = await extractBytes(Buffer.from("This is valid content"), "text/plain");
			expect(result.content).toBe("This is valid content");
		});
	});

	describe("Validator Registry Management", () => {
		it("should unregister validator by name", async () => {
			class UnregValidator implements ValidatorProtocol {
				name() {
					return "unreg-val";
				}

				validate(_result: ExtractionResult): void {
					throw new Error("ValidationError: Should not be called after unregister");
				}
			}

			registerValidator(new UnregValidator());
			unregisterValidator("unreg-val");

			const result = await extractBytes(Buffer.from("Test"), "text/plain");
			expect(result.content).toBe("Test");
		});

		it("should clear all validators", async () => {
			class Val1 implements ValidatorProtocol {
				name() {
					return "val1";
				}
				validate(_result: ExtractionResult): void {
					throw new Error("ValidationError: Val1 should not be called");
				}
			}

			class Val2 implements ValidatorProtocol {
				name() {
					return "val2";
				}
				validate(_result: ExtractionResult): void {
					throw new Error("ValidationError: Val2 should not be called");
				}
			}

			registerValidator(new Val1());
			registerValidator(new Val2());
			clearValidators();

			const result = await extractBytes(Buffer.from("Test"), "text/plain");
			expect(result.content).toBe("Test");
		});
	});
});

describe("Plugin Integration Tests - Multiple Plugins Working Together", () => {
	beforeEach(() => {
		clearPostProcessors();
		clearValidators();
	});

	afterEach(() => {
		clearPostProcessors();
		clearValidators();
	});

	it("should execute post-processors before validators", async () => {
		const executionOrder: string[] = [];

		class TestPostProcessor implements PostProcessorProtocol {
			name() {
				return "test-proc";
			}

			process(result: ExtractionResult): ExtractionResult {
				executionOrder.push("postprocessor");
				result.metadata.processed = true;
				return result;
			}
		}

		class TestValidator implements ValidatorProtocol {
			name() {
				return "test-val";
			}

			validate(result: ExtractionResult): void {
				executionOrder.push("validator");
				expect(result.metadata.processed).toBe(true);
			}
		}

		registerPostProcessor(new TestPostProcessor());
		registerValidator(new TestValidator());

		await extractBytes(Buffer.from("Test content"), "text/plain");

		expect(executionOrder).toEqual(["postprocessor", "validator"]);
	});

	it("should allow validators to check post-processor results", async () => {
		class EnrichmentProcessor implements PostProcessorProtocol {
			name() {
				return "enrichment";
			}

			process(result: ExtractionResult): ExtractionResult {
				const wordCount = result.content.split(/\s+/).filter((w) => w).length;
				result.metadata.word_count = wordCount;
				return result;
			}
		}

		class MetadataValidator implements ValidatorProtocol {
			name() {
				return "metadata-val";
			}

			validate(result: ExtractionResult): void {
				if (!result.metadata.word_count || (result.metadata.word_count as number) < 3) {
					throw new Error("ValidationError: Word count too low");
				}
			}
		}

		registerPostProcessor(new EnrichmentProcessor());
		registerValidator(new MetadataValidator());

		await expect(extractBytes(Buffer.from("Too short"), "text/plain")).rejects.toThrow(/Word count too low/);

		const validResult = await extractBytes(Buffer.from("This is long enough"), "text/plain");
		expect(validResult.metadata.word_count).toBeGreaterThanOrEqual(3);
	});

	it("should handle complex multi-plugin pipeline", async () => {
		class StatsProcessor implements PostProcessorProtocol {
			name() {
				return "stats";
			}

			process(result: ExtractionResult): ExtractionResult {
				const words = result.content.split(/\s+/).filter((w) => w);
				result.metadata.word_count = words.length;
				result.metadata.char_count = result.content.length;
				result.metadata.avg_word_length = result.content.length / words.length;
				return result;
			}

			processingStage() {
				return "early" as const;
			}
		}

		class TaggingProcessor implements PostProcessorProtocol {
			name() {
				return "tagging";
			}

			process(result: ExtractionResult): ExtractionResult {
				const wordCount = result.metadata.word_count as number;
				result.metadata.document_size = wordCount < 50 ? "small" : wordCount < 200 ? "medium" : "large";
				return result;
			}

			processingStage() {
				return "middle" as const;
			}
		}

		class QualityValidator implements ValidatorProtocol {
			name() {
				return "quality";
			}

			priority() {
				return 100;
			}

			validate(result: ExtractionResult): void {
				if ((result.metadata.word_count as number) < 5) {
					throw new Error("ValidationError: Document too short for processing");
				}
			}
		}

		class MetadataValidator implements ValidatorProtocol {
			name() {
				return "metadata-check";
			}

			priority() {
				return 50;
			}

			validate(result: ExtractionResult): void {
				if (!result.metadata.document_size) {
					throw new Error("ValidationError: Missing document_size tag");
				}
			}
		}

		registerPostProcessor(new StatsProcessor());
		registerPostProcessor(new TaggingProcessor());
		registerValidator(new QualityValidator());
		registerValidator(new MetadataValidator());

		const result = await extractBytes(
			Buffer.from("This is a test document with enough words to pass validation"),
			"text/plain",
		);

		expect(result.metadata.word_count).toBeDefined();
		expect(result.metadata.char_count).toBeDefined();
		expect(result.metadata.avg_word_length).toBeDefined();
		expect(result.metadata.document_size).toBeDefined();
		expect(["small", "medium", "large"]).toContain(result.metadata.document_size);
	});
});

describe("Plugin Integration Tests - Concurrent Processing", () => {
	beforeEach(() => {
		clearPostProcessors();
	});

	afterEach(() => {
		clearPostProcessors();
	});

	it("should handle concurrent extractions with stateful processor", async () => {
		class ConcurrentProcessor implements PostProcessorProtocol {
			private callCount = 0;

			name() {
				return "concurrent-proc";
			}

			async process(result: ExtractionResult): Promise<ExtractionResult> {
				this.callCount++;
				const currentCall = this.callCount;

				await new Promise((resolve) => setTimeout(resolve, 10));

				result.metadata.call_number = currentCall;
				result.metadata.total_calls = this.callCount;
				return result;
			}
		}

		const processor = new ConcurrentProcessor();
		registerPostProcessor(processor);

		const promises = [
			extractBytes(Buffer.from("Doc 1"), "text/plain"),
			extractBytes(Buffer.from("Doc 2"), "text/plain"),
			extractBytes(Buffer.from("Doc 3"), "text/plain"),
			extractBytes(Buffer.from("Doc 4"), "text/plain"),
			extractBytes(Buffer.from("Doc 5"), "text/plain"),
		];

		const results = await Promise.all(promises);

		expect(results.length).toBe(5);

		const callNumbers = results.map((r) => r.metadata.call_number as number);
		expect(callNumbers.sort()).toEqual([1, 2, 3, 4, 5]);
	});

	it("should handle multiple processors with concurrent extractions", async () => {
		class Proc1 implements PostProcessorProtocol {
			name() {
				return "proc1";
			}
			async process(result: ExtractionResult): Promise<ExtractionResult> {
				await new Promise((resolve) => setTimeout(resolve, 5));
				result.metadata.proc1_executed = true;
				return result;
			}
		}

		class Proc2 implements PostProcessorProtocol {
			name() {
				return "proc2";
			}
			async process(result: ExtractionResult): Promise<ExtractionResult> {
				await new Promise((resolve) => setTimeout(resolve, 5));
				result.metadata.proc2_executed = true;
				return result;
			}
		}

		registerPostProcessor(new Proc1());
		registerPostProcessor(new Proc2());

		const results = await Promise.all([
			extractBytes(Buffer.from("Test 1"), "text/plain"),
			extractBytes(Buffer.from("Test 2"), "text/plain"),
			extractBytes(Buffer.from("Test 3"), "text/plain"),
		]);

		for (const result of results) {
			expect(result.metadata.proc1_executed).toBe(true);
			expect(result.metadata.proc2_executed).toBe(true);
		}
	});
});

describe("Plugin Integration Tests - Edge Cases", () => {
	beforeEach(() => {
		clearPostProcessors();
		clearValidators();
	});

	afterEach(() => {
		clearPostProcessors();
		clearValidators();
	});

	it("should handle empty content", async () => {
		class EmptyContentProcessor implements PostProcessorProtocol {
			name() {
				return "empty-proc";
			}

			process(result: ExtractionResult): ExtractionResult {
				result.metadata.was_empty = result.content.length === 0;
				return result;
			}
		}

		registerPostProcessor(new EmptyContentProcessor());

		const result = await extractBytes(Buffer.from(""), "text/plain");
		expect(result.metadata.was_empty).toBe(true);
	});

	it("should handle Unicode content in processors", async () => {
		class UnicodeProcessor implements PostProcessorProtocol {
			name() {
				return "unicode-proc";
			}

			process(result: ExtractionResult): ExtractionResult {
				result.metadata.has_emoji = /[\u{1F600}-\u{1F64F}]/u.test(result.content);
				result.metadata.has_cjk = /[\u4E00-\u9FFF\u3040-\u309F\u30A0-\u30FF]/.test(result.content);
				return result;
			}
		}

		registerPostProcessor(new UnicodeProcessor());

		const result = await extractBytes(Buffer.from("Hello 世界 😀"), "text/plain");
		expect(result.metadata.has_emoji).toBe(true);
		expect(result.metadata.has_cjk).toBe(true);
	});

	it("should handle large metadata objects", async () => {
		class LargeMetadataProcessor implements PostProcessorProtocol {
			name() {
				return "large-metadata";
			}

			process(result: ExtractionResult): ExtractionResult {
				result.metadata.large_array = Array.from({ length: 1000 }, (_, i) => i);
				result.metadata.nested = {
					level1: {
						level2: {
							level3: {
								data: "deeply nested",
							},
						},
					},
				};
				return result;
			}
		}

		registerPostProcessor(new LargeMetadataProcessor());

		const result = await extractBytes(Buffer.from("Test"), "text/plain");
		expect((result.metadata.large_array as number[]).length).toBe(1000);
		expect((result.metadata.nested as any).level1.level2.level3.data).toBe("deeply nested");
	});

	it("should handle processor that returns same result", async () => {
		class NoOpProcessor implements PostProcessorProtocol {
			name() {
				return "noop";
			}

			process(result: ExtractionResult): ExtractionResult {
				return result;
			}
		}

		registerPostProcessor(new NoOpProcessor());

		const testContent = "Unchanged content";
		const result = await extractBytes(Buffer.from(testContent), "text/plain");
		expect(result.content).toBe(testContent);
	});
});
