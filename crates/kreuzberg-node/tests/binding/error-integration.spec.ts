import { describe, expect, it } from "vitest";
import { CacheError, extractFile, ImageProcessingError, KreuzbergError, PluginError } from "../../dist/index.js";

/**
 * Integration tests for error handling in real-world scenarios.
 *
 * These tests demonstrate how to catch and handle different error types
 * in production code.
 */
describe("Error Integration", () => {
	describe("Error catching patterns", () => {
		it("should catch specific error types", async () => {
			try {
				await extractFile("/nonexistent/file.pdf");
				expect.fail("Should have thrown an error");
			} catch (error) {
				if (error instanceof CacheError) {
					console.log("Cache error - continuing without cache");
				} else if (error instanceof ImageProcessingError) {
					console.log("Image processing error - skipping images");
				} else if (error instanceof PluginError) {
					console.log(`Plugin error in ${error.pluginName}`);
				} else if (error instanceof KreuzbergError) {
					console.log("Kreuzberg error:", error.message);
				}

				expect(error instanceof Error).toBe(true);
			}
		});

		it("should demonstrate error hierarchy", () => {
			const errors = [
				new CacheError("cache error"),
				new ImageProcessingError("image error"),
				new PluginError("plugin error", "test-plugin"),
			];

			for (const error of errors) {
				expect(error instanceof Error).toBe(true);

				expect(error instanceof KreuzbergError).toBe(true);

				if (error instanceof CacheError) {
					expect(error.name).toBe("CacheError");
				} else if (error instanceof ImageProcessingError) {
					expect(error.name).toBe("ImageProcessingError");
				} else if (error instanceof PluginError) {
					expect(error.name).toBe("PluginError");
					expect(error.pluginName).toBe("test-plugin");
				}
			}
		});

		it("should preserve error information across async boundaries", async () => {
			const testError = async () => {
				throw new CacheError("async cache error");
			};

			try {
				await testError();
				expect.fail("Should have thrown an error");
			} catch (error) {
				expect(error instanceof CacheError).toBe(true);
				expect(error instanceof KreuzbergError).toBe(true);
				if (error instanceof CacheError) {
					expect(error.message).toBe("async cache error");
					expect(error.name).toBe("CacheError");
					expect(error.stack).toBeDefined();
				}
			}
		});

		it("should support error serialization for logging", () => {
			const errors = [
				new CacheError("cache write failed"),
				new ImageProcessingError("resize failed"),
				new PluginError("processing failed", "custom-plugin"),
			];

			for (const error of errors) {
				const serialized = JSON.stringify(error);
				const parsed = JSON.parse(serialized);

				expect(parsed.name).toBe(error.name);
				expect(parsed.message).toBe(error.message);
				expect(parsed.stack).toBeDefined();

				if (error instanceof PluginError) {
					expect(parsed.pluginName).toBe(error.pluginName);
				}
			}
		});
	});

	describe("Production error handling patterns", () => {
		it("should demonstrate graceful degradation for cache errors", async () => {
			const handleExtraction = async (filePath: string) => {
				try {
					return await extractFile(filePath);
				} catch (error) {
					if (error instanceof CacheError) {
						console.warn("Cache unavailable, continuing without cache");
						return await extractFile(filePath, null, { useCache: false });
					}
					throw error;
				}
			};

			try {
				await handleExtraction("/nonexistent/file.pdf");
			} catch (error) {
				expect(error instanceof CacheError).toBe(false);
			}
		});

		it("should demonstrate retry logic for image processing errors", async () => {
			let attempts = 0;
			const maxAttempts = 3;

			const extractWithRetry = async (filePath: string) => {
				while (attempts < maxAttempts) {
					try {
						attempts++;
						return await extractFile(filePath);
					} catch (error) {
						if (error instanceof ImageProcessingError && attempts < maxAttempts) {
							console.log(`Image processing failed, retrying (${attempts}/${maxAttempts})`);
							continue;
						}
						throw error;
					}
				}
				throw new Error("Max retries exceeded");
			};

			try {
				await extractWithRetry("/nonexistent/file.pdf");
			} catch (_error) {
				expect(attempts).toBeGreaterThan(0);
			}
		});

		it("should demonstrate plugin error reporting", () => {
			const reportPluginError = (error: unknown) => {
				if (error instanceof PluginError) {
					return {
						errorType: "plugin",
						pluginName: error.pluginName,
						message: error.message,
						stack: error.stack,
					};
				}
				return null;
			};

			const pluginError = new PluginError("initialization failed", "my-custom-plugin");
			const report = reportPluginError(pluginError);

			expect(report).not.toBeNull();
			expect(report?.errorType).toBe("plugin");
			expect(report?.pluginName).toBe("my-custom-plugin");
			expect(report?.message).toContain("initialization failed");
			expect(report?.message).toContain("my-custom-plugin");
		});
	});
});
