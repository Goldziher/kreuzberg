/**
 * @file Validator plugin tests
 * Tests custom validator registration, execution, and error handling
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
	type ExtractionResult,
	type ValidatorProtocol,
	clearValidators,
	extractBytes,
	registerValidator,
	unregisterValidator,
} from "../../src/index.js";

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

			validate(_result: ExtractionResult): void {
				// Validation passes
			}
		}

		registerValidator(new AlwaysPassValidator());

		const result = await extractBytes(
			Buffer.from("Test content"),
			"text/plain",
			null,
		);
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

		await expect(
			extractBytes(Buffer.from("Short"), "text/plain", null),
		).rejects.toThrow(/ValidationError|Content too short/);
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

		// Register in reverse order
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

		// Should not throw because validator was unregistered
		const result = await extractBytes(
			Buffer.from("Test"),
			"text/plain",
			null,
		);
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

		// Should not throw because validators were cleared
		const result = await extractBytes(
			Buffer.from("Test"),
			"text/plain",
			null,
		);
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

		const result = await extractBytes(
			Buffer.from("Hello world"),
			"text/plain",
			null,
		);
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

		await expect(
			extractBytes(Buffer.from("Test"), "text/plain", null),
		).rejects.toThrow();

		expect(secondCalled).toBe(false);
	});
});
