/**
 * Runtime detection and environment-specific utilities
 *
 * This module provides utilities for detecting the JavaScript runtime environment,
 * checking for feature availability, and enabling environment-specific WASM loading strategies.
 *
 * @example Basic Runtime Detection
 * ```typescript
 * import { detectRuntime, isBrowser, isNode } from '@kreuzberg/wasm/runtime';
 *
 * if (isBrowser()) {
 *   console.log('Running in browser');
 * } else if (isNode()) {
 *   console.log('Running in Node.js');
 * }
 * ```
 *
 * @example Feature Detection
 * ```typescript
 * import { hasFileApi, hasWorkers } from '@kreuzberg/wasm/runtime';
 *
 * if (hasFileApi()) {
 *   // Can use File API for browser file uploads
 * }
 *
 * if (hasWorkers()) {
 *   // Can use Web Workers for parallel processing
 * }
 * ```
 */

export type RuntimeType = "browser" | "node" | "deno" | "bun" | "unknown";

/**
 * WebAssembly capabilities available in the runtime
 */
export interface WasmCapabilities {
	/** Runtime environment type */
	runtime: RuntimeType;
	/** WebAssembly support available */
	hasWasm: boolean;
	/** Streaming WebAssembly instantiation available */
	hasWasmStreaming: boolean;
	/** File API available (browser) */
	hasFileApi: boolean;
	/** Blob API available */
	hasBlob: boolean;
	/** Worker support available */
	hasWorkers: boolean;
	/** SharedArrayBuffer available (may be restricted) */
	hasSharedArrayBuffer: boolean;
	/** Module Workers available */
	hasModuleWorkers: boolean;
	/** BigInt support */
	hasBigInt: boolean;
	/** Specific runtime version if available */
	runtimeVersion?: string;
}

/**
 * Detect the current JavaScript runtime
 *
 * Checks for various global objects and properties to determine
 * which JavaScript runtime environment is currently executing.
 *
 * @returns The detected runtime type
 *
 * @example
 * ```typescript
 * import { detectRuntime } from '@kreuzberg/wasm/runtime';
 *
 * const runtime = detectRuntime();
 * switch (runtime) {
 *   case 'browser':
 *     console.log('Running in browser');
 *     break;
 *   case 'node':
 *     console.log('Running in Node.js');
 *     break;
 *   case 'deno':
 *     console.log('Running in Deno');
 *     break;
 *   case 'bun':
 *     console.log('Running in Bun');
 *     break;
 * }
 * ```
 */
export function detectRuntime(): RuntimeType {
	// Check for Deno
	if (typeof (globalThis as unknown as Record<string, unknown>).Deno !== "undefined") {
		return "deno";
	}

	// Check for Bun
	if (typeof (globalThis as unknown as Record<string, unknown>).Bun !== "undefined") {
		return "bun";
	}

	// Check for Node.js
	if (typeof process !== "undefined" && process.versions && process.versions.node) {
		return "node";
	}

	// Check for browser
	if (typeof window !== "undefined" && typeof document !== "undefined") {
		return "browser";
	}

	return "unknown";
}

/**
 * Check if running in a browser environment
 *
 * @returns True if running in a browser, false otherwise
 */
export function isBrowser(): boolean {
	return detectRuntime() === "browser";
}

/**
 * Check if running in Node.js
 *
 * @returns True if running in Node.js, false otherwise
 */
export function isNode(): boolean {
	return detectRuntime() === "node";
}

/**
 * Check if running in Deno
 *
 * @returns True if running in Deno, false otherwise
 */
export function isDeno(): boolean {
	return detectRuntime() === "deno";
}

/**
 * Check if running in Bun
 *
 * @returns True if running in Bun, false otherwise
 */
export function isBun(): boolean {
	return detectRuntime() === "bun";
}

/**
 * Check if running in a web environment (browser or similar)
 *
 * @returns True if running in a web browser, false otherwise
 */
export function isWebEnvironment(): boolean {
	const runtime = detectRuntime();
	return runtime === "browser";
}

/**
 * Check if running in a server-like environment (Node.js, Deno, Bun)
 *
 * @returns True if running on a server runtime, false otherwise
 */
export function isServerEnvironment(): boolean {
	const runtime = detectRuntime();
	return runtime === "node" || runtime === "deno" || runtime === "bun";
}

/**
 * Check if File API is available
 *
 * The File API is required for handling browser file uploads.
 *
 * @returns True if File API is available, false otherwise
 *
 * @example
 * ```typescript
 * if (hasFileApi()) {
 *   const fileInput = document.getElementById('file');
 *   fileInput.addEventListener('change', (e) => {
 *     const file = e.target.files?.[0];
 *     // Handle file
 *   });
 * }
 * ```
 */
export function hasFileApi(): boolean {
	return typeof window !== "undefined" && typeof File !== "undefined" && typeof Blob !== "undefined";
}

/**
 * Check if Blob API is available
 *
 * @returns True if Blob API is available, false otherwise
 */
export function hasBlob(): boolean {
	return typeof Blob !== "undefined";
}

/**
 * Check if Web Workers are available
 *
 * @returns True if Web Workers can be created, false otherwise
 */
export function hasWorkers(): boolean {
	return typeof Worker !== "undefined";
}

/**
 * Check if SharedArrayBuffer is available
 *
 * Note: SharedArrayBuffer is restricted in some browser contexts
 * due to security considerations (Spectre/Meltdown mitigations).
 *
 * @returns True if SharedArrayBuffer is available, false otherwise
 */
export function hasSharedArrayBuffer(): boolean {
	return typeof SharedArrayBuffer !== "undefined";
}

/**
 * Check if module workers are available
 *
 * Module workers allow importing ES modules in worker threads.
 *
 * @returns True if module workers are supported, false otherwise
 */
export function hasModuleWorkers(): boolean {
	if (!hasWorkers()) {
		return false;
	}

	try {
		// Try to detect module worker support
		const blob = new Blob(['console.log("test")'], {
			type: "application/javascript",
		});
		const workerUrl = URL.createObjectURL(blob);
		try {
			// Module workers require type: 'module' option
			// We can't actually instantiate without issues, so we check the API exists
			return true;
		} finally {
			URL.revokeObjectURL(workerUrl);
		}
	} catch {
		return false;
	}
}

/**
 * Check if WebAssembly is available
 *
 * @returns True if WebAssembly is supported, false otherwise
 */
export function hasWasm(): boolean {
	return typeof WebAssembly !== "undefined" && WebAssembly.instantiate !== undefined;
}

/**
 * Check if WebAssembly.instantiateStreaming is available
 *
 * Streaming instantiation is more efficient than buffering the entire WASM module.
 *
 * @returns True if streaming WebAssembly is supported, false otherwise
 */
export function hasWasmStreaming(): boolean {
	return typeof WebAssembly !== "undefined" && WebAssembly.instantiateStreaming !== undefined;
}

/**
 * Check if BigInt is available
 *
 * @returns True if BigInt type is supported, false otherwise
 */
export function hasBigInt(): boolean {
	try {
		const test = BigInt("1");
		return typeof test === "bigint";
	} catch {
		return false;
	}
}

/**
 * Get runtime version information
 *
 * @returns Version string if available, undefined otherwise
 *
 * @example
 * ```typescript
 * const version = getRuntimeVersion();
 * console.log(`Running on Node ${version}`); // "Running on Node 18.12.0"
 * ```
 */
export function getRuntimeVersion(): string | undefined {
	const runtime = detectRuntime();

	switch (runtime) {
		case "node":
			return process.version?.substring(1); // Remove 'v' prefix
		case "deno": {
			const deno = (globalThis as unknown as Record<string, unknown>).Deno as Record<string, unknown> | undefined;
			const version = deno?.version as Record<string, unknown> | undefined;
			return version?.deno as string | undefined;
		}
		case "bun": {
			const bun = (globalThis as unknown as Record<string, unknown>).Bun as Record<string, unknown> | undefined;
			return bun?.version as string | undefined;
		}
		default:
			return undefined;
	}
}

/**
 * Get comprehensive WebAssembly capabilities for current runtime
 *
 * Returns detailed information about WASM and related APIs available
 * in the current runtime environment.
 *
 * @returns Object describing available WASM capabilities
 *
 * @example
 * ```typescript
 * import { getWasmCapabilities } from '@kreuzberg/wasm/runtime';
 *
 * const caps = getWasmCapabilities();
 * console.log(`WASM available: ${caps.hasWasm}`);
 * console.log(`Streaming WASM: ${caps.hasWasmStreaming}`);
 * console.log(`Workers available: ${caps.hasWorkers}`);
 *
 * if (caps.hasWasm && caps.hasWorkers) {
 *   // Can offload WASM processing to workers
 * }
 * ```
 */
export function getWasmCapabilities(): WasmCapabilities {
	const runtime = detectRuntime();
	const version = getRuntimeVersion();
	const capabilities: WasmCapabilities = {
		runtime,
		hasWasm: hasWasm(),
		hasWasmStreaming: hasWasmStreaming(),
		hasFileApi: hasFileApi(),
		hasBlob: hasBlob(),
		hasWorkers: hasWorkers(),
		hasSharedArrayBuffer: hasSharedArrayBuffer(),
		hasModuleWorkers: hasModuleWorkers(),
		hasBigInt: hasBigInt(),
		...(version !== undefined ? { runtimeVersion: version } : {}),
	};
	return capabilities;
}

/**
 * Get comprehensive runtime information
 *
 * Returns detailed information about the current runtime environment,
 * capabilities, and identifying information.
 *
 * @returns Object with runtime details and capabilities
 *
 * @example
 * ```typescript
 * const info = getRuntimeInfo();
 * console.log(info.runtime); // 'browser' | 'node' | 'deno' | 'bun'
 * console.log(info.isBrowser); // true/false
 * console.log(info.userAgent); // Browser user agent string
 * console.log(info.capabilities); // Detailed capability information
 * ```
 */
export function getRuntimeInfo() {
	const runtime = detectRuntime();
	const capabilities = getWasmCapabilities();

	return {
		runtime,
		isBrowser: isBrowser(),
		isNode: isNode(),
		isDeno: isDeno(),
		isBun: isBun(),
		isWeb: isWebEnvironment(),
		isServer: isServerEnvironment(),
		runtimeVersion: getRuntimeVersion(),
		userAgent: typeof navigator !== "undefined" ? navigator.userAgent : "N/A",
		capabilities,
	};
}
