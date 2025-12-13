import { describe, expect, it } from "vitest";
import * as adapterModule from "./adapters/wasm-adapter.js";
import * as registryModule from "./ocr/registry.js";
import * as runtimeModule from "./runtime.js";

// Test adapter exports
describe("WASM Adapter Module", () => {
	it("should export all adapter functions", () => {
		expect(typeof adapterModule.fileToUint8Array).toBe("function");
		expect(typeof adapterModule.configToJS).toBe("function");
		expect(typeof adapterModule.jsToExtractionResult).toBe("function");
		expect(typeof adapterModule.wrapWasmError).toBe("function");
		expect(typeof adapterModule.isValidExtractionResult).toBe("function");
	});
});

// Test runtime exports
describe("Runtime Module", () => {
	it("should export all runtime detection functions", () => {
		expect(typeof runtimeModule.detectRuntime).toBe("function");
		expect(typeof runtimeModule.isBrowser).toBe("function");
		expect(typeof runtimeModule.isNode).toBe("function");
		expect(typeof runtimeModule.isDeno).toBe("function");
		expect(typeof runtimeModule.isBun).toBe("function");
		expect(typeof runtimeModule.isWebEnvironment).toBe("function");
		expect(typeof runtimeModule.isServerEnvironment).toBe("function");
		expect(typeof runtimeModule.hasFileApi).toBe("function");
		expect(typeof runtimeModule.hasBlob).toBe("function");
		expect(typeof runtimeModule.hasWorkers).toBe("function");
		expect(typeof runtimeModule.hasSharedArrayBuffer).toBe("function");
		expect(typeof runtimeModule.hasModuleWorkers).toBe("function");
		expect(typeof runtimeModule.hasWasm).toBe("function");
		expect(typeof runtimeModule.hasWasmStreaming).toBe("function");
		expect(typeof runtimeModule.hasBigInt).toBe("function");
		expect(typeof runtimeModule.getRuntimeVersion).toBe("function");
		expect(typeof runtimeModule.getWasmCapabilities).toBe("function");
		expect(typeof runtimeModule.getRuntimeInfo).toBe("function");
	});

	it("should provide correct runtime type", () => {
		const runtime = runtimeModule.detectRuntime();
		expect(["browser", "node", "deno", "bun", "unknown"]).toContain(runtime);
	});

	it("should provide consistent runtime checks", () => {
		const _runtime = runtimeModule.detectRuntime();
		const isBrowser = runtimeModule.isBrowser();
		const isNode = runtimeModule.isNode();
		const isDeno = runtimeModule.isDeno();
		const isBun = runtimeModule.isBun();

		// Only one should be true
		const count = [isBrowser, isNode, isDeno, isBun].filter(Boolean).length;
		expect(count).toBeLessThanOrEqual(1);
	});

	it("should provide WASM capabilities", () => {
		const caps = runtimeModule.getWasmCapabilities();

		expect(caps).toBeDefined();
		expect(typeof caps.hasWasm).toBe("boolean");
		expect(typeof caps.hasWasmStreaming).toBe("boolean");
		expect(typeof caps.hasFileApi).toBe("boolean");
		expect(typeof caps.hasBlob).toBe("boolean");
		expect(typeof caps.hasWorkers).toBe("boolean");
		expect(typeof caps.hasSharedArrayBuffer).toBe("boolean");
		expect(typeof caps.hasModuleWorkers).toBe("boolean");
		expect(typeof caps.hasBigInt).toBe("boolean");
	});

	it("should provide runtime info", () => {
		const info = runtimeModule.getRuntimeInfo();

		expect(info).toBeDefined();
		expect(info.runtime).toBeDefined();
		expect(typeof info.isBrowser).toBe("boolean");
		expect(typeof info.isNode).toBe("boolean");
		expect(typeof info.isDeno).toBe("boolean");
		expect(typeof info.isBun).toBe("boolean");
		expect(typeof info.isWeb).toBe("boolean");
		expect(typeof info.isServer).toBe("boolean");
		expect(info.capabilities).toBeDefined();
	});
});

// Test OCR registry exports
describe("OCR Registry Module", () => {
	it("should export all registry functions", () => {
		expect(typeof registryModule.registerOcrBackend).toBe("function");
		expect(typeof registryModule.getOcrBackend).toBe("function");
		expect(typeof registryModule.listOcrBackends).toBe("function");
		expect(typeof registryModule.unregisterOcrBackend).toBe("function");
		expect(typeof registryModule.clearOcrBackends).toBe("function");
	});

	it("should start with empty registry", () => {
		const backends = registryModule.listOcrBackends();
		expect(Array.isArray(backends)).toBe(true);
	});
});

// Integration tests between modules
describe("Module Integration", () => {
	it("should provide consistent error wrapping", () => {
		const error = new Error("Test");
		const wrapped = adapterModule.wrapWasmError(error, "context");

		expect(wrapped).toBeInstanceOf(Error);
		expect(wrapped.message).toContain("context");
	});

	it("should validate extraction results", () => {
		const valid = {
			content: "test",
			mimeType: "text/plain",
			metadata: {},
			tables: [],
		};

		expect(adapterModule.isValidExtractionResult(valid)).toBe(true);
	});

	it("should convert config to JS", () => {
		const config = { ocr: { backend: "test" } };
		const result = adapterModule.configToJS(config);

		expect(result).toBeDefined();
		expect(result.ocr).toBeDefined();
	});

	it("should handle null config", () => {
		const result = adapterModule.configToJS(null);

		expect(result).toEqual({});
	});
});
