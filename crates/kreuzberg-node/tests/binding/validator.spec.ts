/**
 * @file Validator plugin tests
 * Tests custom validator registration, execution, and error handling
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
	__resetBindingForTests,
	__setBindingForTests,
	clearValidators,
	type ExtractionResult,
	extractBytes,
	listValidators,
	registerValidator,
	unregisterValidator,
	type ValidatorProtocol,
} from "../../dist/index.js";

describe("Validator Plugin System", () => {
	beforeEach(() => {
		clearValidators();
	});

	afterEach(() => {
		clearValidators();
	});

	it("should register and execute validator that passes", async () => {
		class AlwaysPassValidator implements ValidatorProtocol {
			name(): string {
				return "always_pass";
			}

			validate(_result: ExtractionResult): void {}
		}

		registerValidator(new AlwaysPassValidator());

		const result = await extractBytes(Buffer.from("Test content"), "text/plain", null);
		expect(result.content).toBe("Test content");
	});

	it("should fail when validator throws ValidationError", async () => {
		class ContentLengthValidator implements ValidatorProtocol {
			name(): string {
				return "content_length";
			}

			validate(result: ExtractionResult): void {
				if (result.content.length < 10) {
					throw new Error("ValidationError: Content too short");
				}
			}
		}

		registerValidator(new ContentLengthValidator());

		await expect(extractBytes(Buffer.from("Short"), "text/plain", null)).rejects.toThrow(
			/ValidationError|Content too short/,
		);
	});

	it("should execute validators in priority order", async () => {
		const executionOrder: string[] = [];

		class HighPriorityValidator implements ValidatorProtocol {
			name(): string {
				return "high_priority";
			}

			priority(): number {
				return 100;
			}

			validate(_result: ExtractionResult): void {
				executionOrder.push("high");
			}
		}

		class LowPriorityValidator implements ValidatorProtocol {
			name(): string {
				return "low_priority";
			}

			priority(): number {
				return 10;
			}

			validate(_result: ExtractionResult): void {
				executionOrder.push("low");
			}
		}

		registerValidator(new LowPriorityValidator());
		registerValidator(new HighPriorityValidator());

		await extractBytes(Buffer.from("Test content"), "text/plain", null);

		expect(executionOrder).toEqual(["high", "low"]);
	});

	it("should unregister validator by name", async () => {
		class FailValidator implements ValidatorProtocol {
			name(): string {
				return "fail_validator";
			}

			validate(_result: ExtractionResult): void {
				throw new Error("ValidationError: Should not be called");
			}
		}

		registerValidator(new FailValidator());
		unregisterValidator("fail_validator");

		const result = await extractBytes(Buffer.from("Test"), "text/plain", null);
		expect(result.content).toBe("Test");
	});

	it("should clear all validators", async () => {
		class FailValidator implements ValidatorProtocol {
			name(): string {
				return "fail";
			}

			validate(_result: ExtractionResult): void {
				throw new Error("ValidationError: Should not be called");
			}
		}

		registerValidator(new FailValidator());
		clearValidators();

		const result = await extractBytes(Buffer.from("Test"), "text/plain", null);
		expect(result.content).toBe("Test");
	});

	it("should validate with multiple validators (all pass)", async () => {
		class LengthValidator implements ValidatorProtocol {
			name(): string {
				return "length";
			}

			validate(result: ExtractionResult): void {
				if (result.content.length < 5) {
					throw new Error("ValidationError: Too short");
				}
			}
		}

		class WordCountValidator implements ValidatorProtocol {
			name(): string {
				return "word_count";
			}

			validate(result: ExtractionResult): void {
				const words = result.content.split(/\s+/);
				if (words.length < 2) {
					throw new Error("ValidationError: Need more words");
				}
			}
		}

		registerValidator(new LengthValidator());
		registerValidator(new WordCountValidator());

		const result = await extractBytes(Buffer.from("Hello world"), "text/plain", null);
		expect(result.content).toBe("Hello world");
	});

	it("should stop on first validation failure (fail-fast)", async () => {
		let secondCalled = false;

		class FirstValidator implements ValidatorProtocol {
			name(): string {
				return "first";
			}

			priority(): number {
				return 100;
			}

			validate(_result: ExtractionResult): void {
				throw new Error("ValidationError: First failed");
			}
		}

		class SecondValidator implements ValidatorProtocol {
			name(): string {
				return "second";
			}

			priority(): number {
				return 50;
			}

			validate(_result: ExtractionResult): void {
				secondCalled = true;
			}
		}

		registerValidator(new FirstValidator());
		registerValidator(new SecondValidator());

		await expect(extractBytes(Buffer.from("Test"), "text/plain", null)).rejects.toThrow();

		expect(secondCalled).toBe(false);
	});

	it("should handle invalid JSON in validator wrapper gracefully", async () => {
		class TestValidator implements ValidatorProtocol {
			name(): string {
				return "test_validator";
			}

			validate(result: ExtractionResult): void {
				expect(result).toBeDefined();
				expect(result.content).toBeDefined();
			}
		}

		registerValidator(new TestValidator());

		const result = await extractBytes(Buffer.from("Test content"), "text/plain", null);
		expect(result.content).toBe("Test content");
	});
});

describe("Validator bridge wiring", () => {
	const createValidatorBinding = () => ({
		registerValidator: vi.fn(),
		unregisterValidator: vi.fn(),
		clearValidators: vi.fn(),
		listValidators: vi.fn().mockReturnValue(["alpha", "beta"]),
	});

	afterEach(() => {
		__resetBindingForTests();
	});

	it("should convert wire payloads before invoking validator", async () => {
		const mockBinding = createValidatorBinding();
		__setBindingForTests(mockBinding);

		const validateSpy = vi.fn();
		const validator: ValidatorProtocol = {
			name: () => "wire-validator",
			validate: validateSpy,
		};

		registerValidator(validator);
		expect(mockBinding.registerValidator).toHaveBeenCalledTimes(1);

		const wrapped = mockBinding.registerValidator.mock.calls[0][0];
		const wirePayload = JSON.stringify({
			content: "hello world",
			mime_type: "text/plain",
			metadata: JSON.stringify({ score: 0.99, nested: { value: 42 } }),
			tables: [{ cells: [["a"]] }],
			detected_languages: ["en"],
			chunks: [
				{
					content: "chunk",
					embedding: null,
					metadata: {
						charStart: 0,
						charEnd: 5,
						tokenCount: null,
						chunkIndex: 0,
						totalChunks: 1,
					},
				},
			],
		});

		await wrapped.validate(wirePayload);

		expect(validateSpy).toHaveBeenCalledWith(
			expect.objectContaining({
				content: "hello world",
				mimeType: "text/plain",
				metadata: { score: 0.99, nested: { value: 42 } },
				tables: [{ cells: [["a"]] }],
				detectedLanguages: ["en"],
				chunks: [
					{
						content: "chunk",
						embedding: null,
						metadata: {
							charStart: 0,
							charEnd: 5,
							tokenCount: null,
							chunkIndex: 0,
							totalChunks: 1,
						},
					},
				],
			}),
		);
	});

	it("should reject invalid wire payloads", async () => {
		const mockBinding = createValidatorBinding();
		__setBindingForTests(mockBinding);

		const validator: ValidatorProtocol = {
			name: () => "wire-validator",
			validate: () => {},
		};

		registerValidator(validator);
		const wrapped = mockBinding.registerValidator.mock.calls[0][0];

		await expect(wrapped.validate("undefined")).rejects.toThrow(/invalid JSON string/i);
	});

	it("should forward registry helpers to the native binding", () => {
		const mockBinding = createValidatorBinding();
		__setBindingForTests(mockBinding);

		unregisterValidator("foo");
		clearValidators();
		const names = listValidators();

		expect(mockBinding.unregisterValidator).toHaveBeenCalledWith("foo");
		expect(mockBinding.clearValidators).toHaveBeenCalledTimes(1);
		expect(mockBinding.listValidators).toHaveBeenCalledTimes(1);
		expect(names).toEqual(["alpha", "beta"]);
	});
});
