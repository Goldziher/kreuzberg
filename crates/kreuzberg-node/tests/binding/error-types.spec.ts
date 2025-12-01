import { describe, expect, it } from "vitest";

/**
 * Test suite for error type exposure and handling.
 *
 * This test verifies that all Rust error types are properly exposed
 * to TypeScript consumers through the NAPI-RS bindings.
 *
 * Following TDD principles: these tests are written first and should fail
 * until the error types are properly exported from the TypeScript package.
 */
describe("Error Types", () => {
	describe("KreuzbergError", () => {
		it("should be importable from the package", async () => {
			const module = await import("../../dist/index.js");
			expect(module).toHaveProperty("KreuzbergError");
		});

		it("should be a proper Error subclass", async () => {
			const { KreuzbergError } = await import("../../dist/index.js");
			const error = new KreuzbergError("test error");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(KreuzbergError);
			expect(error.name).toBe("KreuzbergError");
			expect(error.message).toBe("test error");
		});

		it("should have a proper stack trace", async () => {
			const { KreuzbergError } = await import("../../dist/index.js");
			const error = new KreuzbergError("test error");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("KreuzbergError");
			expect(error.stack).toContain("test error");
		});

		it("should serialize to JSON with toJSON method", async () => {
			const { KreuzbergError } = await import("../../dist/index.js");
			const error = new KreuzbergError("test error");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("test error");
			expect(parsed.name).toBe("KreuzbergError");
			expect(parsed.stack).toBeDefined();
		});
	});

	describe("ValidationError", () => {
		it("should be importable from the package", async () => {
			const module = await import("../../dist/index.js");
			expect(module).toHaveProperty("ValidationError");
		});

		it("should be a proper Error subclass", async () => {
			const { ValidationError } = await import("../../dist/index.js");
			const error = new ValidationError("test validation error");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(ValidationError);
			expect(error.name).toBe("ValidationError");
			expect(error.message).toBe("test validation error");
		});

		it("should extend KreuzbergError", async () => {
			const { ValidationError, KreuzbergError } = await import("../../dist/index.js");
			const error = new ValidationError("test message");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should have a proper stack trace", async () => {
			const { ValidationError } = await import("../../dist/index.js");
			const error = new ValidationError("test validation error");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("ValidationError");
			expect(error.stack).toContain("test validation error");
		});
	});

	describe("ParsingError", () => {
		it("should be importable from the package", async () => {
			const module = await import("../../dist/index.js");
			expect(module).toHaveProperty("ParsingError");
		});

		it("should be a proper Error subclass", async () => {
			const { ParsingError } = await import("../../dist/index.js");
			const error = new ParsingError("test parsing error");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(ParsingError);
			expect(error.name).toBe("ParsingError");
			expect(error.message).toBe("test parsing error");
		});

		it("should extend KreuzbergError", async () => {
			const { ParsingError, KreuzbergError } = await import("../../dist/index.js");
			const error = new ParsingError("test message");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should have a proper stack trace", async () => {
			const { ParsingError } = await import("../../dist/index.js");
			const error = new ParsingError("test parsing error");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("ParsingError");
			expect(error.stack).toContain("test parsing error");
		});
	});

	describe("OcrError", () => {
		it("should be importable from the package", async () => {
			const module = await import("../../dist/index.js");
			expect(module).toHaveProperty("OcrError");
		});

		it("should be a proper Error subclass", async () => {
			const { OcrError } = await import("../../dist/index.js");
			const error = new OcrError("test ocr error");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(OcrError);
			expect(error.name).toBe("OcrError");
			expect(error.message).toBe("test ocr error");
		});

		it("should extend KreuzbergError", async () => {
			const { OcrError, KreuzbergError } = await import("../../dist/index.js");
			const error = new OcrError("test message");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should have a proper stack trace", async () => {
			const { OcrError } = await import("../../dist/index.js");
			const error = new OcrError("test ocr error");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("OcrError");
			expect(error.stack).toContain("test ocr error");
		});
	});

	describe("CacheError", () => {
		it("should be importable from the package", async () => {
			// This test verifies that CacheError is exported
			const module = await import("../../dist/index.js");
			expect(module).toHaveProperty("CacheError");
		});

		it("should be a proper Error subclass", async () => {
			const { CacheError } = await import("../../dist/index.js");
			const error = new CacheError("test cache error");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(CacheError);
			expect(error.name).toBe("CacheError");
			expect(error.message).toBe("test cache error");
		});

		it("should extend KreuzbergError", async () => {
			const { CacheError, KreuzbergError } = await import("../../dist/index.js");
			const error = new CacheError("test message");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should have a proper stack trace", async () => {
			const { CacheError } = await import("../../dist/index.js");
			const error = new CacheError("test cache error");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("CacheError");
			expect(error.stack).toContain("test cache error");
		});
	});

	describe("ImageProcessingError", () => {
		it("should be importable from the package", async () => {
			const module = await import("../../dist/index.js");
			expect(module).toHaveProperty("ImageProcessingError");
		});

		it("should be a proper Error subclass", async () => {
			const { ImageProcessingError } = await import("../../dist/index.js");
			const error = new ImageProcessingError("test image processing error");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(ImageProcessingError);
			expect(error.name).toBe("ImageProcessingError");
			expect(error.message).toBe("test image processing error");
		});

		it("should extend KreuzbergError", async () => {
			const { ImageProcessingError, KreuzbergError } = await import("../../dist/index.js");
			const error = new ImageProcessingError("test message");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should have a proper stack trace", async () => {
			const { ImageProcessingError } = await import("../../dist/index.js");
			const error = new ImageProcessingError("test image processing error");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("ImageProcessingError");
			expect(error.stack).toContain("test image processing error");
		});
	});

	describe("PluginError", () => {
		it("should be importable from the package", async () => {
			const module = await import("../../dist/index.js");
			expect(module).toHaveProperty("PluginError");
		});

		it("should be a proper Error subclass", async () => {
			const { PluginError } = await import("../../dist/index.js");
			const error = new PluginError("test plugin error", "test-plugin");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(PluginError);
			expect(error.name).toBe("PluginError");
			expect(error.message).toContain("test plugin error");
		});

		it("should extend KreuzbergError", async () => {
			const { PluginError, KreuzbergError } = await import("../../dist/index.js");
			const error = new PluginError("test message", "test-plugin");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should include plugin name in message", async () => {
			const { PluginError } = await import("../../dist/index.js");
			const error = new PluginError("operation failed", "my-custom-plugin");

			expect(error.message).toContain("my-custom-plugin");
			expect(error.message).toContain("operation failed");
		});

		it("should store plugin name as property", async () => {
			const { PluginError } = await import("../../dist/index.js");
			const error = new PluginError("test error", "test-plugin");

			expect(error.pluginName).toBe("test-plugin");
		});

		it("should have a proper stack trace", async () => {
			const { PluginError } = await import("../../dist/index.js");
			const error = new PluginError("test plugin error", "test-plugin");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("PluginError");
		});
	});

	describe("MissingDependencyError", () => {
		it("should be importable from the package", async () => {
			const module = await import("../../dist/index.js");
			expect(module).toHaveProperty("MissingDependencyError");
		});

		it("should be a proper Error subclass", async () => {
			const { MissingDependencyError } = await import("../../dist/index.js");
			const error = new MissingDependencyError("test dependency error");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(MissingDependencyError);
			expect(error.name).toBe("MissingDependencyError");
			expect(error.message).toBe("test dependency error");
		});

		it("should extend KreuzbergError", async () => {
			const { MissingDependencyError, KreuzbergError } = await import("../../dist/index.js");
			const error = new MissingDependencyError("test message");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should have a proper stack trace", async () => {
			const { MissingDependencyError } = await import("../../dist/index.js");
			const error = new MissingDependencyError("test dependency error");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("MissingDependencyError");
			expect(error.stack).toContain("test dependency error");
		});
	});

	describe("Error hierarchy", () => {
		it("should have KreuzbergError as base class for all errors", async () => {
			const {
				KreuzbergError,
				ValidationError,
				ParsingError,
				OcrError,
				CacheError,
				ImageProcessingError,
				PluginError,
				MissingDependencyError,
			} = await import("../../dist/index.js");

			const baseError = new KreuzbergError("base error");
			const validationError = new ValidationError("validation error");
			const parsingError = new ParsingError("parsing error");
			const ocrError = new OcrError("ocr error");
			const cacheError = new CacheError("cache error");
			const imageError = new ImageProcessingError("image error");
			const pluginError = new PluginError("plugin error", "plugin");
			const dependencyError = new MissingDependencyError("dependency error");

			expect(baseError).toBeInstanceOf(Error);
			expect(baseError).toBeInstanceOf(KreuzbergError);

			expect(validationError).toBeInstanceOf(Error);
			expect(validationError).toBeInstanceOf(KreuzbergError);

			expect(parsingError).toBeInstanceOf(Error);
			expect(parsingError).toBeInstanceOf(KreuzbergError);

			expect(ocrError).toBeInstanceOf(Error);
			expect(ocrError).toBeInstanceOf(KreuzbergError);

			expect(cacheError).toBeInstanceOf(Error);
			expect(cacheError).toBeInstanceOf(KreuzbergError);

			expect(imageError).toBeInstanceOf(Error);
			expect(imageError).toBeInstanceOf(KreuzbergError);

			expect(pluginError).toBeInstanceOf(Error);
			expect(pluginError).toBeInstanceOf(KreuzbergError);

			expect(dependencyError).toBeInstanceOf(Error);
			expect(dependencyError).toBeInstanceOf(KreuzbergError);
		});
	});

	describe("Error serialization", () => {
		it("should serialize KreuzbergError to JSON with relevant fields", async () => {
			const { KreuzbergError } = await import("../../dist/index.js");
			const error = new KreuzbergError("base error");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("base error");
			expect(parsed.name).toBe("KreuzbergError");
			expect(parsed.stack).toBeDefined();
		});

		it("should serialize ValidationError to JSON with relevant fields", async () => {
			const { ValidationError } = await import("../../dist/index.js");
			const error = new ValidationError("validation failed");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("validation failed");
			expect(parsed.name).toBe("ValidationError");
		});

		it("should serialize ParsingError to JSON with relevant fields", async () => {
			const { ParsingError } = await import("../../dist/index.js");
			const error = new ParsingError("parsing failed");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("parsing failed");
			expect(parsed.name).toBe("ParsingError");
		});

		it("should serialize OcrError to JSON with relevant fields", async () => {
			const { OcrError } = await import("../../dist/index.js");
			const error = new OcrError("ocr failed");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("ocr failed");
			expect(parsed.name).toBe("OcrError");
		});

		it("should serialize CacheError to JSON with relevant fields", async () => {
			const { CacheError } = await import("../../dist/index.js");
			const error = new CacheError("cache write failed");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("cache write failed");
			expect(parsed.name).toBe("CacheError");
		});

		it("should serialize ImageProcessingError to JSON with relevant fields", async () => {
			const { ImageProcessingError } = await import("../../dist/index.js");
			const error = new ImageProcessingError("failed to resize image");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("failed to resize image");
			expect(parsed.name).toBe("ImageProcessingError");
		});

		it("should serialize PluginError to JSON with plugin name", async () => {
			const { PluginError } = await import("../../dist/index.js");
			const error = new PluginError("plugin crashed", "my-plugin");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.pluginName).toBe("my-plugin");
			expect(parsed.name).toBe("PluginError");
		});

		it("should serialize MissingDependencyError to JSON with relevant fields", async () => {
			const { MissingDependencyError } = await import("../../dist/index.js");
			const error = new MissingDependencyError("dependency not found");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("dependency not found");
			expect(parsed.name).toBe("MissingDependencyError");
		});
	});
});
