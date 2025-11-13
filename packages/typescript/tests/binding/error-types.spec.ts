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
	describe("CacheError", () => {
		it("should be importable from the package", async () => {
			// This test verifies that CacheError is exported
			const module = await import("../../src/index.js");
			expect(module).toHaveProperty("CacheError");
		});

		it("should be a proper Error subclass", async () => {
			const { CacheError } = await import("../../src/index.js");
			const error = new CacheError("test cache error");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(CacheError);
			expect(error.name).toBe("CacheError");
			expect(error.message).toBe("test cache error");
		});

		it("should extend KreuzbergError", async () => {
			const { CacheError, KreuzbergError } = await import("../../src/index.js");
			const error = new CacheError("test message");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should have a proper stack trace", async () => {
			const { CacheError } = await import("../../src/index.js");
			const error = new CacheError("test cache error");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("CacheError");
			expect(error.stack).toContain("test cache error");
		});
	});

	describe("ImageProcessingError", () => {
		it("should be importable from the package", async () => {
			const module = await import("../../src/index.js");
			expect(module).toHaveProperty("ImageProcessingError");
		});

		it("should be a proper Error subclass", async () => {
			const { ImageProcessingError } = await import("../../src/index.js");
			const error = new ImageProcessingError("test image processing error");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(ImageProcessingError);
			expect(error.name).toBe("ImageProcessingError");
			expect(error.message).toBe("test image processing error");
		});

		it("should extend KreuzbergError", async () => {
			const { ImageProcessingError, KreuzbergError } = await import("../../src/index.js");
			const error = new ImageProcessingError("test message");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should have a proper stack trace", async () => {
			const { ImageProcessingError } = await import("../../src/index.js");
			const error = new ImageProcessingError("test image processing error");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("ImageProcessingError");
			expect(error.stack).toContain("test image processing error");
		});
	});

	describe("PluginError", () => {
		it("should be importable from the package", async () => {
			const module = await import("../../src/index.js");
			expect(module).toHaveProperty("PluginError");
		});

		it("should be a proper Error subclass", async () => {
			const { PluginError } = await import("../../src/index.js");
			const error = new PluginError("test plugin error", "test-plugin");

			expect(error).toBeInstanceOf(Error);
			expect(error).toBeInstanceOf(PluginError);
			expect(error.name).toBe("PluginError");
			expect(error.message).toContain("test plugin error");
		});

		it("should extend KreuzbergError", async () => {
			const { PluginError, KreuzbergError } = await import("../../src/index.js");
			const error = new PluginError("test message", "test-plugin");

			expect(error).toBeInstanceOf(KreuzbergError);
		});

		it("should include plugin name in message", async () => {
			const { PluginError } = await import("../../src/index.js");
			const error = new PluginError("operation failed", "my-custom-plugin");

			expect(error.message).toContain("my-custom-plugin");
			expect(error.message).toContain("operation failed");
		});

		it("should store plugin name as property", async () => {
			const { PluginError } = await import("../../src/index.js");
			const error = new PluginError("test error", "test-plugin");

			expect(error.pluginName).toBe("test-plugin");
		});

		it("should have a proper stack trace", async () => {
			const { PluginError } = await import("../../src/index.js");
			const error = new PluginError("test plugin error", "test-plugin");

			expect(error.stack).toBeDefined();
			expect(error.stack).toContain("PluginError");
		});
	});

	describe("Error hierarchy", () => {
		it("should have KreuzbergError as base class for all errors", async () => {
			const { KreuzbergError, CacheError, ImageProcessingError, PluginError } = await import("../../src/index.js");

			const baseError = new KreuzbergError("base error");
			const cacheError = new CacheError("cache error");
			const imageError = new ImageProcessingError("image error");
			const pluginError = new PluginError("plugin error", "plugin");

			expect(baseError).toBeInstanceOf(Error);
			expect(baseError).toBeInstanceOf(KreuzbergError);

			expect(cacheError).toBeInstanceOf(Error);
			expect(cacheError).toBeInstanceOf(KreuzbergError);

			expect(imageError).toBeInstanceOf(Error);
			expect(imageError).toBeInstanceOf(KreuzbergError);

			expect(pluginError).toBeInstanceOf(Error);
			expect(pluginError).toBeInstanceOf(KreuzbergError);
		});
	});

	describe("Error serialization", () => {
		it("should serialize CacheError to JSON with relevant fields", async () => {
			const { CacheError } = await import("../../src/index.js");
			const error = new CacheError("cache write failed");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("cache write failed");
			expect(parsed.name).toBe("CacheError");
		});

		it("should serialize ImageProcessingError to JSON with relevant fields", async () => {
			const { ImageProcessingError } = await import("../../src/index.js");
			const error = new ImageProcessingError("failed to resize image");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.message).toBe("failed to resize image");
			expect(parsed.name).toBe("ImageProcessingError");
		});

		it("should serialize PluginError to JSON with plugin name", async () => {
			const { PluginError } = await import("../../src/index.js");
			const error = new PluginError("plugin crashed", "my-plugin");

			const serialized = JSON.stringify(error);
			const parsed = JSON.parse(serialized);

			expect(parsed.pluginName).toBe("my-plugin");
			expect(parsed.name).toBe("PluginError");
		});
	});
});
