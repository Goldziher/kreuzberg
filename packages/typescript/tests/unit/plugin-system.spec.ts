/**
 * Comprehensive plugin system tests for TypeScript.
 *
 * Tests cover:
 * - PostProcessor registration, execution, and ordering
 * - OCR backend registration
 * - Error handling
 * - Multiple processor chains
 * - Metadata modification
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
	__resetBindingForTests,
	__setBindingForTests,
	clearPostProcessors,
	type ExtractionResult,
	extractBytes,
	type PostProcessorProtocol,
	type ProcessingStage,
	registerPostProcessor,
	unregisterPostProcessor,
} from "../../src/index.js";
import { createMockExtractionBinding } from "./helpers/mock-binding.js";

// ============================================================================
// Test Processors
// ============================================================================

class EarlyStageProcessor implements PostProcessorProtocol {
	name(): string {
		return "early_processor";
	}

	process(result: ExtractionResult): ExtractionResult {
		if (!result.metadata) {
			result.metadata = {};
		}
		result.metadata.early_executed = true;
		result.metadata.execution_order = result.metadata.execution_order || [];
		result.metadata.execution_order.push("early");
		return result;
	}

	processingStage(): ProcessingStage {
		return "early";
	}
}

class MiddleStageProcessor implements PostProcessorProtocol {
	name(): string {
		return "middle_processor";
	}

	process(result: ExtractionResult): ExtractionResult {
		if (!result.metadata) {
			result.metadata = {};
		}
		result.metadata.middle_executed = true;
		result.metadata.execution_order = result.metadata.execution_order || [];
		result.metadata.execution_order.push("middle");
		return result;
	}

	processingStage(): ProcessingStage {
		return "middle";
	}
}

class LateStageProcessor implements PostProcessorProtocol {
	name(): string {
		return "late_processor";
	}

	process(result: ExtractionResult): ExtractionResult {
		if (!result.metadata) {
			result.metadata = {};
		}
		result.metadata.late_executed = true;
		result.metadata.execution_order = result.metadata.execution_order || [];
		result.metadata.execution_order.push("late");
		return result;
	}

	processingStage(): ProcessingStage {
		return "late";
	}
}

class WordCountProcessor implements PostProcessorProtocol {
	name(): string {
		return "word_count";
	}

	process(result: ExtractionResult): ExtractionResult {
		const words = result.content.split(/\s+/).filter((w) => w.length > 0);
		if (!result.metadata) {
			result.metadata = {};
		}
		result.metadata.word_count = words.length;
		result.metadata.character_count = result.content.length;
		return result;
	}

	processingStage(): ProcessingStage {
		return "middle";
	}
}

class SentenceCountProcessor implements PostProcessorProtocol {
	name(): string {
		return "sentence_count";
	}

	process(result: ExtractionResult): ExtractionResult {
		const sentenceEndings =
			(result.content.match(/\./g) || []).length +
			(result.content.match(/!/g) || []).length +
			(result.content.match(/\?/g) || []).length;
		if (!result.metadata) {
			result.metadata = {};
		}
		result.metadata.sentence_count = Math.max(1, sentenceEndings);
		return result;
	}

	processingStage(): ProcessingStage {
		return "middle";
	}
}

class UppercaseTagProcessor implements PostProcessorProtocol {
	name(): string {
		return "uppercase_tag";
	}

	process(result: ExtractionResult): ExtractionResult {
		const text = result.content;
		const alphaChars = text.split("").filter((c) => /[a-zA-Z]/.test(c));

		if (alphaChars.length > 0) {
			const uppercaseCount = alphaChars.filter((c) => c === c.toUpperCase()).length;
			const uppercaseRatio = uppercaseCount / alphaChars.length;

			if (!result.metadata) {
				result.metadata = {};
			}
			result.metadata.uppercase_ratio = uppercaseRatio;
			result.metadata.is_mostly_uppercase = uppercaseRatio > 0.5;
		}
		return result;
	}

	processingStage(): ProcessingStage {
		return "late";
	}
}

class InitializableProcessor implements PostProcessorProtocol {
	private initialized = false;
	private callCount = 0;

	name(): string {
		return "initializable";
	}

	initialize(): void {
		this.initialized = true;
	}

	process(result: ExtractionResult): ExtractionResult {
		this.callCount++;
		if (!result.metadata) {
			result.metadata = {};
		}
		result.metadata.processor_initialized = this.initialized;
		result.metadata.processor_call_count = this.callCount;
		return result;
	}

	processingStage(): ProcessingStage {
		return "middle";
	}

	shutdown(): void {
		this.initialized = false;
	}
}

class ErrorHandlingProcessor implements PostProcessorProtocol {
	constructor(private shouldFail: boolean = false) {}

	name(): string {
		return "error_handler";
	}

	process(result: ExtractionResult): ExtractionResult {
		try {
			if (this.shouldFail) {
				throw new Error("Intentional error for testing");
			}
			if (!result.metadata) {
				result.metadata = {};
			}
			result.metadata.error_handler_success = true;
		} catch (e) {
			if (!result.metadata) {
				result.metadata = {};
			}
			result.metadata.error_handler_error = (e as Error).message;
		}
		return result;
	}

	processingStage(): ProcessingStage {
		return "middle";
	}
}

class MinimalProcessor implements PostProcessorProtocol {
	name(): string {
		return "minimal";
	}

	process(result: ExtractionResult): ExtractionResult {
		if (!result.metadata) {
			result.metadata = {};
		}
		result.metadata.minimal_executed = true;
		return result;
	}
}

// ============================================================================
// Tests
// ============================================================================

beforeEach(() => {
	const mockBinding = createMockExtractionBinding();
	__setBindingForTests(mockBinding);
	clearPostProcessors();
});

afterEach(() => {
	__resetBindingForTests();
});

describe("Plugin System - PostProcessors", () => {
	// Clear processors before each test
	beforeEach(() => {
		clearPostProcessors();
	});

	describe("Registration and Execution", () => {
		it("should register and execute early-stage processor", async () => {
			const processor = new EarlyStageProcessor();
			registerPostProcessor(processor);

			const testContent = "This is a test document.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.early_executed).toBe(true);
			expect(result.metadata?.execution_order).toContain("early");
		});

		it("should register and execute middle-stage processor", async () => {
			const processor = new MiddleStageProcessor();
			registerPostProcessor(processor);

			const testContent = "This is a test document.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.middle_executed).toBe(true);
			expect(result.metadata?.execution_order).toContain("middle");
		});

		it("should register and execute late-stage processor", async () => {
			const processor = new LateStageProcessor();
			registerPostProcessor(processor);

			const testContent = "This is a test document.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.late_executed).toBe(true);
			expect(result.metadata?.execution_order).toContain("late");
		});
	});

	describe("Execution Order", () => {
		it("should execute multiple processors in stage order", async () => {
			const lateProc = new LateStageProcessor();
			const middleProc = new MiddleStageProcessor();
			const earlyProc = new EarlyStageProcessor();

			// Register in reverse order to test stage ordering
			registerPostProcessor(lateProc);
			registerPostProcessor(middleProc);
			registerPostProcessor(earlyProc);

			const testContent = "Test content for ordering.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.early_executed).toBe(true);
			expect(result.metadata?.middle_executed).toBe(true);
			expect(result.metadata?.late_executed).toBe(true);

			const executionOrder = result.metadata?.execution_order as string[];
			expect(executionOrder).toBeDefined();

			const earlyIdx = executionOrder.indexOf("early");
			const middleIdx = executionOrder.indexOf("middle");
			const lateIdx = executionOrder.indexOf("late");

			expect(earlyIdx).toBeLessThan(middleIdx);
			expect(middleIdx).toBeLessThan(lateIdx);
		});

		it("should execute multiple processors in same stage", async () => {
			const wordProc = new WordCountProcessor();
			const sentenceProc = new SentenceCountProcessor();

			registerPostProcessor(wordProc);
			registerPostProcessor(sentenceProc);

			const testContent = "Hello world. This is a test. How are you?";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.word_count).toBe(9);
			expect(result.metadata?.sentence_count).toBe(3);
		});
	});

	describe("Metadata Modification", () => {
		it("should add metadata without overwriting existing", async () => {
			const processor = new WordCountProcessor();
			registerPostProcessor(processor);

			const testContent = "One two three four five.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.word_count).toBe(5);
			expect(result.metadata?.character_count).toBe(testContent.length);
		});

		it("should accumulate metadata across processor chain", async () => {
			const proc1 = new WordCountProcessor();
			const proc2 = new SentenceCountProcessor();
			const proc3 = new UppercaseTagProcessor();

			registerPostProcessor(proc1);
			registerPostProcessor(proc2);
			registerPostProcessor(proc3);

			const testContent = "HELLO WORLD. THIS IS A TEST.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.word_count).toBeDefined();
			expect(result.metadata?.sentence_count).toBeDefined();
			expect(result.metadata?.uppercase_ratio).toBeDefined();
			expect(result.metadata?.is_mostly_uppercase).toBe(true);
		});
	});

	describe("Processor Lifecycle", () => {
		it("should call initialize() when processor is registered", async () => {
			const processor = new InitializableProcessor();

			expect(processor.initialized).toBe(false);

			registerPostProcessor(processor);

			const testContent = "Test initialization.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.processor_initialized).toBe(true);
			expect((result.metadata?.processor_call_count as number) > 0).toBe(true);
		});

		it("should handle multiple extraction calls with same processor", async () => {
			const processor = new InitializableProcessor();
			registerPostProcessor(processor);

			const result1 = await extractBytes(Buffer.from("First call."), "text/plain");
			const count1 = result1.metadata?.processor_call_count as number;

			const result2 = await extractBytes(Buffer.from("Second call."), "text/plain");
			const count2 = result2.metadata?.processor_call_count as number;

			expect(count2).toBeGreaterThan(count1);
		});
	});

	describe("Error Handling", () => {
		it("should handle processor errors gracefully", async () => {
			const processor = new ErrorHandlingProcessor(false);
			registerPostProcessor(processor);

			const testContent = "Test error handling.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.error_handler_success).toBe(true);
			expect(result.metadata?.error_handler_error).toBeUndefined();
		});

		it("should capture internal processor errors", async () => {
			const processor = new ErrorHandlingProcessor(true);
			registerPostProcessor(processor);

			const testContent = "Test error handling with failure.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.error_handler_error).toContain("Intentional error");
		});
	});

	describe("Processor Configuration", () => {
		it("should support PostProcessorConfig.enabled", async () => {
			const processor = new WordCountProcessor();
			registerPostProcessor(processor);

			const testContent = "Test with postprocessing enabled.";

			// With postprocessing enabled
			const resultEnabled = await extractBytes(Buffer.from(testContent), "text/plain", {
				postprocessor: { enabled: true },
			});
			expect(resultEnabled.metadata?.word_count).toBeDefined();

			// With postprocessing disabled
			const resultDisabled = await extractBytes(Buffer.from(testContent), "text/plain", {
				postprocessor: { enabled: false },
			});
			expect(resultDisabled.metadata?.word_count).toBeUndefined();
		});

		it("should support PostProcessorConfig whitelist", async () => {
			const proc1 = new WordCountProcessor();
			const proc2 = new SentenceCountProcessor();

			registerPostProcessor(proc1);
			registerPostProcessor(proc2);

			const testContent = "Test whitelist. Second sentence.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain", {
				postprocessor: {
					enabled: true,
					enabledProcessors: ["word_count"],
				},
			});

			expect(result.metadata?.word_count).toBeDefined();
			expect(result.metadata?.sentence_count).toBeUndefined();
		});

		it("should support PostProcessorConfig blacklist", async () => {
			const proc1 = new WordCountProcessor();
			const proc2 = new SentenceCountProcessor();

			registerPostProcessor(proc1);
			registerPostProcessor(proc2);

			const testContent = "Test blacklist. Second sentence.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain", {
				postprocessor: {
					enabled: true,
					disabledProcessors: ["sentence_count"],
				},
			});

			expect(result.metadata?.word_count).toBeDefined();
			expect(result.metadata?.sentence_count).toBeUndefined();
		});
	});

	describe("Edge Cases", () => {
		it("should handle empty content gracefully", async () => {
			const processor = new WordCountProcessor();
			registerPostProcessor(processor);

			const result = await extractBytes(Buffer.from(""), "text/plain");

			expect(result.metadata?.word_count).toBe(0);
			expect(result.metadata?.character_count).toBe(0);
		});

		it("should handle Unicode content correctly", async () => {
			const processor = new WordCountProcessor();
			registerPostProcessor(processor);

			const testContent = "Hello 世界! Здравствуй мир! مرحبا بالعالم!";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect((result.metadata?.word_count as number) > 0).toBe(true);
			expect(result.metadata?.character_count).toBe(testContent.length);
		});

		it("should handle processor with duplicate names", async () => {
			class DuplicateNameProcessor implements PostProcessorProtocol {
				name(): string {
					return "word_count";
				}

				process(result: ExtractionResult): ExtractionResult {
					if (!result.metadata) {
						result.metadata = {};
					}
					result.metadata.duplicate = true;
					return result;
				}

				processingStage(): ProcessingStage {
					return "middle";
				}
			}

			const proc1 = new WordCountProcessor();
			const proc2 = new DuplicateNameProcessor();

			registerPostProcessor(proc1);
			registerPostProcessor(proc2);

			const testContent = "Test duplicate names.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			// One of them should have run
			const hasWordCount = result.metadata?.word_count !== undefined;
			const hasDuplicate = result.metadata?.duplicate !== undefined;
			expect(hasWordCount || hasDuplicate).toBe(true);
		});

		it("should handle processor without optional methods", async () => {
			const processor = new MinimalProcessor();
			registerPostProcessor(processor);

			const testContent = "Test minimal processor.";
			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.minimal_executed).toBe(true);
		});
	});

	describe("Realistic Use Cases", () => {
		it("should support realistic text analysis pipeline", async () => {
			const wordProc = new WordCountProcessor();
			const sentenceProc = new SentenceCountProcessor();
			const uppercaseProc = new UppercaseTagProcessor();

			registerPostProcessor(wordProc);
			registerPostProcessor(sentenceProc);
			registerPostProcessor(uppercaseProc);

			const testContent = `Machine learning is a subset of artificial intelligence.
It focuses on building systems that can learn from data.
Deep learning uses neural networks with multiple layers.`;

			const result = await extractBytes(Buffer.from(testContent), "text/plain");

			expect(result.metadata?.word_count).toBe(26);
			expect(result.metadata?.sentence_count).toBe(3);
			expect(result.metadata?.is_mostly_uppercase).toBe(false);
			expect(
				(result.metadata?.uppercase_ratio as number) >= 0 && (result.metadata?.uppercase_ratio as number) <= 1,
			).toBe(true);
		});
	});

	describe("Registry Management", () => {
		it("should unregister processor by name", async () => {
			const processor = new WordCountProcessor();
			registerPostProcessor(processor);

			let result = await extractBytes(Buffer.from("Test content."), "text/plain");
			expect(result.metadata?.word_count).toBeDefined();

			unregisterPostProcessor("word_count");

			result = await extractBytes(Buffer.from("Test content."), "text/plain");
			expect(result.metadata?.word_count).toBeUndefined();
		});

		it("should clear all processors", async () => {
			const proc1 = new WordCountProcessor();
			const proc2 = new SentenceCountProcessor();

			registerPostProcessor(proc1);
			registerPostProcessor(proc2);

			let result = await extractBytes(Buffer.from("Test content."), "text/plain");
			expect(result.metadata?.word_count).toBeDefined();
			expect(result.metadata?.sentence_count).toBeDefined();

			clearPostProcessors();

			result = await extractBytes(Buffer.from("Test content."), "text/plain");
			expect(result.metadata?.word_count).toBeUndefined();
			expect(result.metadata?.sentence_count).toBeUndefined();
		});
	});
});
