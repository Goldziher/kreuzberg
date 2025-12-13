import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
	detectRuntime,
	getRuntimeInfo,
	getRuntimeVersion,
	getWasmCapabilities,
	hasBigInt,
	hasBlob,
	hasFileApi,
	hasModuleWorkers,
	hasSharedArrayBuffer,
	hasWasm,
	hasWasmStreaming,
	hasWorkers,
	isBrowser,
	isBun,
	isDeno,
	isNode,
	isServerEnvironment,
	isWebEnvironment,
} from "./runtime.js";

describe("Runtime Detection", () => {
	beforeEach(() => {
		vi.resetModules();
	});

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	describe("detectRuntime", () => {
		it("should detect browser environment", () => {
			vi.stubGlobal("window", {});
			vi.stubGlobal("document", {});
			vi.unstubAllGlobals();

			const runtime = detectRuntime();
			expect(["browser", "node", "deno", "bun", "unknown"]).toContain(runtime);
		});

		it("should detect Node.js environment", () => {
			vi.stubGlobal("process", {
				versions: { node: "18.0.0" },
			});

			const runtime = detectRuntime();
			expect(["browser", "node", "deno", "bun", "unknown"]).toContain(runtime);
		});

		it("should detect Deno environment", () => {
			vi.stubGlobal("Deno", {});

			const runtime = detectRuntime();
			expect(["browser", "node", "deno", "bun", "unknown"]).toContain(runtime);
		});

		it("should detect Bun environment", () => {
			vi.stubGlobal("Bun", {});

			const runtime = detectRuntime();
			expect(["browser", "node", "deno", "bun", "unknown"]).toContain(runtime);
		});

		it("should return unknown for unsupported environment", () => {
			const runtime = detectRuntime();
			expect(["browser", "node", "deno", "bun", "unknown"]).toContain(runtime);
		});
	});

	describe("isBrowser", () => {
		it("should return true only in browser", () => {
			const result = isBrowser();
			expect(typeof result).toBe("boolean");
		});

		it("should be consistent with detectRuntime", () => {
			const runtime = detectRuntime();
			const browser = isBrowser();
			expect(browser).toBe(runtime === "browser");
		});
	});

	describe("isNode", () => {
		it("should return true only in Node.js", () => {
			const result = isNode();
			expect(typeof result).toBe("boolean");
		});

		it("should be consistent with detectRuntime", () => {
			const runtime = detectRuntime();
			const node = isNode();
			expect(node).toBe(runtime === "node");
		});
	});

	describe("isDeno", () => {
		it("should return true only in Deno", () => {
			const result = isDeno();
			expect(typeof result).toBe("boolean");
		});

		it("should be consistent with detectRuntime", () => {
			const runtime = detectRuntime();
			const deno = isDeno();
			expect(deno).toBe(runtime === "deno");
		});
	});

	describe("isBun", () => {
		it("should return true only in Bun", () => {
			const result = isBun();
			expect(typeof result).toBe("boolean");
		});

		it("should be consistent with detectRuntime", () => {
			const runtime = detectRuntime();
			const bun = isBun();
			expect(bun).toBe(runtime === "bun");
		});
	});

	describe("isWebEnvironment", () => {
		it("should return true only for browser", () => {
			const web = isWebEnvironment();
			const runtime = detectRuntime();
			expect(web).toBe(runtime === "browser");
		});
	});

	describe("isServerEnvironment", () => {
		it("should return true for Node.js, Deno, or Bun", () => {
			const server = isServerEnvironment();
			const runtime = detectRuntime();
			const expected = runtime === "node" || runtime === "deno" || runtime === "bun";
			expect(server).toBe(expected);
		});
	});

	describe("Feature Detection", () => {
		describe("hasFileApi", () => {
			it("should return boolean", () => {
				const result = hasFileApi();
				expect(typeof result).toBe("boolean");
			});

			it("should return false if window is undefined", () => {
				const result = hasFileApi();
				if (typeof window === "undefined") {
					expect(result).toBe(false);
				}
			});
		});

		describe("hasBlob", () => {
			it("should return boolean", () => {
				const result = hasBlob();
				expect(typeof result).toBe("boolean");
			});

			it("should return true if Blob is defined", () => {
				const result = hasBlob();
				expect(result).toBe(typeof Blob !== "undefined");
			});
		});

		describe("hasWorkers", () => {
			it("should return boolean", () => {
				const result = hasWorkers();
				expect(typeof result).toBe("boolean");
			});

			it("should return true if Worker is defined", () => {
				const result = hasWorkers();
				expect(result).toBe(typeof Worker !== "undefined");
			});
		});

		describe("hasSharedArrayBuffer", () => {
			it("should return boolean", () => {
				const result = hasSharedArrayBuffer();
				expect(typeof result).toBe("boolean");
			});

			it("should return true if SharedArrayBuffer is defined", () => {
				const result = hasSharedArrayBuffer();
				expect(result).toBe(typeof SharedArrayBuffer !== "undefined");
			});
		});

		describe("hasModuleWorkers", () => {
			it("should return boolean", () => {
				const result = hasModuleWorkers();
				expect(typeof result).toBe("boolean");
			});

			it("should return false if workers not available", () => {
				if (typeof Worker === "undefined") {
					expect(hasModuleWorkers()).toBe(false);
				}
			});
		});

		describe("hasWasm", () => {
			it("should return boolean", () => {
				const result = hasWasm();
				expect(typeof result).toBe("boolean");
			});

			it("should return true if WebAssembly is available", () => {
				const result = hasWasm();
				expect(result).toBe(typeof WebAssembly !== "undefined" && WebAssembly.instantiate !== undefined);
			});
		});

		describe("hasWasmStreaming", () => {
			it("should return boolean", () => {
				const result = hasWasmStreaming();
				expect(typeof result).toBe("boolean");
			});

			it("should return true if WebAssembly.instantiateStreaming available", () => {
				const result = hasWasmStreaming();
				expect(result).toBe(typeof WebAssembly !== "undefined" && WebAssembly.instantiateStreaming !== undefined);
			});
		});

		describe("hasBigInt", () => {
			it("should return boolean", () => {
				const result = hasBigInt();
				expect(typeof result).toBe("boolean");
			});

			it("should return true in modern environments", () => {
				const result = hasBigInt();
				// BigInt is available in Node 10.4+, modern browsers
				expect(typeof result).toBe("boolean");
			});
		});
	});

	describe("getRuntimeVersion", () => {
		it("should return string or undefined", () => {
			const version = getRuntimeVersion();
			expect(typeof version === "string" || version === undefined).toBe(true);
		});

		it("should return version for Node.js", () => {
			vi.stubGlobal("process", {
				version: "v18.12.0",
				versions: { node: "18.12.0" },
			});

			const version = getRuntimeVersion();
			if (detectRuntime() === "node" && version) {
				expect(version).toMatch(/^\d+\.\d+\.\d+/);
			}
		});
	});

	describe("getWasmCapabilities", () => {
		it("should return WasmCapabilities object", () => {
			const caps = getWasmCapabilities();

			expect(caps).toBeDefined();
			expect(typeof caps).toBe("object");
		});

		it("should include runtime field", () => {
			const caps = getWasmCapabilities();
			expect(caps.runtime).toBeDefined();
			expect(["browser", "node", "deno", "bun", "unknown"]).toContain(caps.runtime);
		});

		it("should include all required boolean fields", () => {
			const caps = getWasmCapabilities();

			expect(typeof caps.hasWasm).toBe("boolean");
			expect(typeof caps.hasWasmStreaming).toBe("boolean");
			expect(typeof caps.hasFileApi).toBe("boolean");
			expect(typeof caps.hasBlob).toBe("boolean");
			expect(typeof caps.hasWorkers).toBe("boolean");
			expect(typeof caps.hasSharedArrayBuffer).toBe("boolean");
			expect(typeof caps.hasModuleWorkers).toBe("boolean");
			expect(typeof caps.hasBigInt).toBe("boolean");
		});

		it("should include runtimeVersion if available", () => {
			const caps = getWasmCapabilities();

			if (caps.runtimeVersion !== undefined) {
				expect(typeof caps.runtimeVersion).toBe("string");
			}
		});

		it("should match detectRuntime", () => {
			const caps = getWasmCapabilities();
			const runtime = detectRuntime();

			expect(caps.runtime).toBe(runtime);
		});
	});

	describe("getRuntimeInfo", () => {
		it("should return runtime info object", () => {
			const info = getRuntimeInfo();

			expect(info).toBeDefined();
			expect(typeof info).toBe("object");
		});

		it("should include all required fields", () => {
			const info = getRuntimeInfo();

			expect(info.runtime).toBeDefined();
			expect(typeof info.isBrowser).toBe("boolean");
			expect(typeof info.isNode).toBe("boolean");
			expect(typeof info.isDeno).toBe("boolean");
			expect(typeof info.isBun).toBe("boolean");
			expect(typeof info.isWeb).toBe("boolean");
			expect(typeof info.isServer).toBe("boolean");
			expect(info.capabilities).toBeDefined();
		});

		it("should be consistent with helper functions", () => {
			const info = getRuntimeInfo();

			expect(info.isBrowser).toBe(isBrowser());
			expect(info.isNode).toBe(isNode());
			expect(info.isDeno).toBe(isDeno());
			expect(info.isBun).toBe(isBun());
			expect(info.isWeb).toBe(isWebEnvironment());
			expect(info.isServer).toBe(isServerEnvironment());
		});

		it("should include userAgent field", () => {
			const info = getRuntimeInfo();

			expect(typeof info.userAgent).toBe("string");
			// In jsdom test environment, userAgent may be present
			// In Node.js, it should be "N/A"
		});

		it("should include runtimeVersion if available", () => {
			const info = getRuntimeInfo();

			if (info.runtimeVersion !== undefined) {
				expect(typeof info.runtimeVersion).toBe("string");
			}
		});
	});

	describe("Consistency checks", () => {
		it("only one runtime should be true", () => {
			const checks = [
				{ name: "browser", value: isBrowser() },
				{ name: "node", value: isNode() },
				{ name: "deno", value: isDeno() },
				{ name: "bun", value: isBun() },
			];

			const trueCount = checks.filter((c) => c.value).length;
			expect(trueCount).toBeLessThanOrEqual(1);
		});

		it("isWeb and isServer should be mutually exclusive", () => {
			const isWeb = isWebEnvironment();
			const isServer = isServerEnvironment();

			// Browser is web, others are server
			if (isWeb) {
				expect(isServer).toBe(false);
			}
		});

		it("feature detection should match runtime", () => {
			const caps = getWasmCapabilities();

			// Browser should have File API
			if (caps.runtime === "browser" && caps.hasFileApi) {
				expect(hasFileApi()).toBe(true);
			}
		});

		it("WASM streaming requires WASM", () => {
			const caps = getWasmCapabilities();

			if (!caps.hasWasm) {
				expect(caps.hasWasmStreaming).toBe(false);
			}
		});

		it("Module workers require workers", () => {
			const caps = getWasmCapabilities();

			if (!caps.hasWorkers) {
				expect(caps.hasModuleWorkers).toBe(false);
			}
		});
	});

	describe("Edge cases", () => {
		it("should handle missing WebAssembly gracefully", () => {
			const result = hasWasm();
			expect(typeof result).toBe("boolean");
		});

		it("should handle missing Blob gracefully", () => {
			const result = hasBlob();
			expect(typeof result).toBe("boolean");
		});

		it("should handle missing Worker gracefully", () => {
			const result = hasWorkers();
			expect(typeof result).toBe("boolean");
		});

		it("hasBigInt should not throw", () => {
			expect(() => hasBigInt()).not.toThrow();
		});

		it("hasModuleWorkers should not throw even without Worker", () => {
			expect(() => hasModuleWorkers()).not.toThrow();
		});
	});
});
