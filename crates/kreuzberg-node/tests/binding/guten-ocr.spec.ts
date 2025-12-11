import { describe, expect, it, afterAll } from "vitest";

/**
 * Integration tests for GutenOcrBackend.
 *
 * These tests verify the GutenOcrBackend implementation with real @gutenye/ocr-node
 * if available. Tests skip gracefully when the optional dependency is not installed.
 *
 * To run these tests with the optional dependency:
 * npm install @gutenye/ocr-node
 * pnpm test:binding -- guten-ocr.spec.ts
 */

// Global cleanup after all tests complete
afterAll(async () => {
	try {
		const { clearOcrBackends } = await import("../../dist/index.js");
		clearOcrBackends();
	} catch {
		// Ignore errors during cleanup
	}
});

/**
 * Check if a module is available without throwing
 */
async function isModuleAvailable(moduleName: string): Promise<boolean> {
	try {
		await import(moduleName);
		return true;
	} catch {
		return false;
	}
}

describe("GutenOcrBackend - Static Properties & Construction", () => {
	it("should create instance without options", async () => {
		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();
		expect(backend).toBeInstanceOf(GutenOcrBackend);
	});

	it("should create instance with custom options", async () => {
		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend({
			models: {
				detectionPath: "/custom/detection.onnx",
				recognitionPath: "/custom/recognition.onnx",
				dictionaryPath: "/custom/dict.txt",
			},
			isDebug: true,
			debugOutputDir: "/debug",
		});
		expect(backend).toBeInstanceOf(GutenOcrBackend);
	});

	it("should return correct backend name", async () => {
		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();
		expect(backend.name()).toBe("guten-ocr");
	});

	it("should return supported languages", async () => {
		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();
		const languages = backend.supportedLanguages();
		expect(languages).toContain("en");
		expect(languages).toContain("eng");
		expect(languages).toContain("ch_sim");
		expect(languages).toContain("ch_tra");
		expect(languages).toContain("chinese");
	});
});

describe("GutenOcrBackend - Integration Tests", () => {
	it("should skip gracefully if @gutenye/ocr-node is not installed", async () => {
		const gutenAvailable = await isModuleAvailable("@gutenye/ocr-node");

		if (!gutenAvailable) {
			console.log(
				"Skipping GutenOcrBackend integration tests: @gutenye/ocr-node not installed. " +
					"Install with: npm install @gutenye/ocr-node",
			);
			// Test passes but is skipped
			expect(true).toBe(true);
			return;
		}

		expect(gutenAvailable).toBe(true);
	});

	it("should initialize successfully when @gutenye/ocr-node is available", async () => {
		const gutenAvailable = await isModuleAvailable("@gutenye/ocr-node");

		if (!gutenAvailable) {
			console.log("Skipping: @gutenye/ocr-node not installed. " + "Install with: npm install @gutenye/ocr-node");
			expect(true).toBe(true);
			return;
		}

		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();

		// Should not throw
		await backend.initialize();

		// Verify backend is ready
		expect(backend.name()).toBe("guten-ocr");

		// Clean up - explicitly release resources
		await backend.shutdown();
	});

	it("should throw when @gutenye/ocr-node is not installed", async () => {
		const gutenAvailable = await isModuleAvailable("@gutenye/ocr-node");

		if (gutenAvailable) {
			console.log("Skipping error case: @gutenye/ocr-node IS installed, " + "so cannot test missing dependency error");
			expect(true).toBe(true);
			return;
		}

		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();

		// Should throw when trying to initialize without the dependency
		await expect(backend.initialize()).rejects.toThrow(/requires the '@gutenye\/ocr-node' package/i);
	});

	it("should shutdown gracefully", async () => {
		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();
		await backend.shutdown();
		// Should not throw
		expect(true).toBe(true);
	});

	it("should not reinitialize if already initialized", async () => {
		const gutenAvailable = await isModuleAvailable("@gutenye/ocr-node");

		if (!gutenAvailable) {
			console.log("Skipping: @gutenye/ocr-node not installed. " + "Install with: npm install @gutenye/ocr-node");
			expect(true).toBe(true);
			return;
		}

		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();

		try {
			// First initialization
			await backend.initialize();

			// Second initialization should be no-op
			await backend.initialize();

			// Backend should still work
			expect(backend.name()).toBe("guten-ocr");
		} finally {
			await backend.shutdown();
		}
	});

	it("should handle processImage when initialized", async () => {
		const gutenAvailable = await isModuleAvailable("@gutenye/ocr-node");

		if (!gutenAvailable) {
			console.log("Skipping: @gutenye/ocr-node not installed. " + "Install with: npm install @gutenye/ocr-node");
			expect(true).toBe(true);
			return;
		}

		// Valid minimal 1x1 PNG image for testing
		// This is a complete, valid PNG file that can be processed by image libraries
		const validMinimalPng = new Uint8Array([
			0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
			0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49,
			0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00,
			0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
		]);

		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();
		await backend.initialize();

		try {
			const result = await backend.processImage(validMinimalPng, "en");

			// Verify result structure
			expect(result).toHaveProperty("content");
			expect(result).toHaveProperty("mime_type");
			expect(result).toHaveProperty("metadata");
			expect(result).toHaveProperty("tables");

			expect(typeof result.content).toBe("string");
			expect(result.mime_type).toBe("text/plain");
			expect(Array.isArray(result.tables)).toBe(true);

			// Verify metadata structure
			expect(result.metadata).toHaveProperty("confidence");
			expect(result.metadata).toHaveProperty("text_regions");
			expect(result.metadata).toHaveProperty("language", "en");
		} finally {
			await backend.shutdown();
		}
	});

	it("should auto-initialize on processImage if not initialized", async () => {
		const gutenAvailable = await isModuleAvailable("@gutenye/ocr-node");

		if (!gutenAvailable) {
			console.log("Skipping: @gutenye/ocr-node not installed. " + "Install with: npm install @gutenye/ocr-node");
			expect(true).toBe(true);
			return;
		}

		const validMinimalPng = new Uint8Array([
			0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
			0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49,
			0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00,
			0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
		]);

		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();

		// Do NOT call initialize - should auto-initialize
		try {
			const result = await backend.processImage(validMinimalPng, "en");
			expect(result).toBeDefined();
			expect(typeof result.content).toBe("string");
		} finally {
			await backend.shutdown();
		}
	});

	it("should handle empty OCR results", async () => {
		const gutenAvailable = await isModuleAvailable("@gutenye/ocr-node");

		if (!gutenAvailable) {
			console.log("Skipping: @gutenye/ocr-node not installed. " + "Install with: npm install @gutenye/ocr-node");
			expect(true).toBe(true);
			return;
		}

		// Very small or blank image - may result in no detections
		const blankImage = new Uint8Array([
			0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
			0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49,
			0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00,
			0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
		]);

		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();
		await backend.initialize();

		try {
			const result = await backend.processImage(blankImage, "en");

			// Empty content is valid (blank image)
			expect(typeof result.content).toBe("string");
			expect(result.metadata.confidence).toBeGreaterThanOrEqual(0);
			expect(result.metadata.text_regions).toBeGreaterThanOrEqual(0);
		} finally {
			await backend.shutdown();
		}
	});

	it("should support buffer input in addition to Uint8Array", async () => {
		const gutenAvailable = await isModuleAvailable("@gutenye/ocr-node");

		if (!gutenAvailable) {
			console.log("Skipping: @gutenye/ocr-node not installed. " + "Install with: npm install @gutenye/ocr-node");
			expect(true).toBe(true);
			return;
		}

		const validMinimalPng = Buffer.from([
			0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
			0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49,
			0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00,
			0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
		]);

		const { GutenOcrBackend } = await import("../../dist/index.js");
		const backend = new GutenOcrBackend();
		await backend.initialize();

		try {
			const result = await backend.processImage(validMinimalPng, "en");
			expect(result).toBeDefined();
			expect(typeof result.content).toBe("string");
		} finally {
			await backend.shutdown();
		}
	});
});
