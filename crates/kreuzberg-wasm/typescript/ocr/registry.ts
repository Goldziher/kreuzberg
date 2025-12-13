/**
 * OCR Backend Registry
 *
 * Provides a registry for OCR backends in the WASM environment.
 * This enables auto-registration and management of OCR backends.
 *
 * Note: The WASM package provides a lightweight registry in the browser.
 * For more advanced features like Rust integration, use @kreuzberg/node or @kreuzberg/deno.
 *
 * @example
 * ```typescript
 * import { TesseractWasmBackend } from '@kreuzberg/wasm/ocr/tesseract-wasm-backend';
 * import { enableOcr } from '@kreuzberg/wasm';
 *
 * // Simple auto-registration
 * await enableOcr();
 * ```
 */

import type { OcrBackendProtocol } from "@kreuzberg/core";

/** Global registry of OCR backends */
const ocrBackendRegistry = new Map<string, OcrBackendProtocol>();

/**
 * Register an OCR backend
 *
 * Registers an OCR backend with the WASM extraction pipeline.
 * If a backend with the same name is already registered, it will be replaced.
 *
 * @param backend - OCR backend implementing OcrBackendProtocol
 * @throws {Error} If backend validation fails
 *
 * @example
 * ```typescript
 * import { TesseractWasmBackend } from '@kreuzberg/wasm/ocr/tesseract-wasm-backend';
 * import { registerOcrBackend } from '@kreuzberg/wasm/ocr/registry';
 *
 * const backend = new TesseractWasmBackend();
 * await backend.initialize();
 * registerOcrBackend(backend);
 * ```
 */
export function registerOcrBackend(backend: OcrBackendProtocol): void {
	// Validate backend
	if (!backend) {
		throw new Error("Backend cannot be null or undefined");
	}

	if (typeof backend.name !== "function") {
		throw new Error("Backend must implement name() method");
	}

	if (typeof backend.supportedLanguages !== "function") {
		throw new Error("Backend must implement supportedLanguages() method");
	}

	if (typeof backend.processImage !== "function") {
		throw new Error("Backend must implement processImage() method");
	}

	const backendName = backend.name();

	if (!backendName || typeof backendName !== "string") {
		throw new Error("Backend name must be a non-empty string");
	}

	// Check for duplicate registration (allow overwriting with warning)
	if (ocrBackendRegistry.has(backendName)) {
		console.warn(`OCR backend "${backendName}" is already registered and will be replaced`);
	}

	// Register the backend
	ocrBackendRegistry.set(backendName, backend);
}

/**
 * Get a registered OCR backend by name
 *
 * @param name - Backend name
 * @returns The OCR backend or undefined if not found
 *
 * @example
 * ```typescript
 * import { getOcrBackend } from '@kreuzberg/wasm/ocr/registry';
 *
 * const backend = getOcrBackend('tesseract-wasm');
 * if (backend) {
 *   console.log('Available languages:', backend.supportedLanguages());
 * }
 * ```
 */
export function getOcrBackend(name: string): OcrBackendProtocol | undefined {
	return ocrBackendRegistry.get(name);
}

/**
 * List all registered OCR backends
 *
 * @returns Array of registered backend names
 *
 * @example
 * ```typescript
 * import { listOcrBackends } from '@kreuzberg/wasm/ocr/registry';
 *
 * const backends = listOcrBackends();
 * console.log('Available OCR backends:', backends);
 * ```
 */
export function listOcrBackends(): string[] {
	return Array.from(ocrBackendRegistry.keys());
}

/**
 * Unregister an OCR backend
 *
 * @param name - Backend name to unregister
 * @throws {Error} If backend is not found
 *
 * @example
 * ```typescript
 * import { unregisterOcrBackend } from '@kreuzberg/wasm/ocr/registry';
 *
 * unregisterOcrBackend('tesseract-wasm');
 * ```
 */
export async function unregisterOcrBackend(name: string): Promise<void> {
	const backend = ocrBackendRegistry.get(name);

	if (!backend) {
		throw new Error(
			`OCR backend "${name}" is not registered. Available backends: ${Array.from(ocrBackendRegistry.keys()).join(", ")}`,
		);
	}

	// Call shutdown if available
	if (typeof backend.shutdown === "function") {
		try {
			await backend.shutdown();
		} catch (error) {
			console.warn(
				`Error shutting down OCR backend "${name}": ${error instanceof Error ? error.message : String(error)}`,
			);
		}
	}

	ocrBackendRegistry.delete(name);
}

/**
 * Clear all registered OCR backends
 *
 * Unregisters all OCR backends and calls their shutdown methods.
 *
 * @example
 * ```typescript
 * import { clearOcrBackends } from '@kreuzberg/wasm/ocr/registry';
 *
 * // Clean up all backends when shutting down
 * await clearOcrBackends();
 * ```
 */
export async function clearOcrBackends(): Promise<void> {
	const backends = Array.from(ocrBackendRegistry.entries());

	for (const [name, backend] of backends) {
		if (typeof backend.shutdown === "function") {
			try {
				await backend.shutdown();
			} catch (error) {
				console.warn(
					`Error shutting down OCR backend "${name}": ${error instanceof Error ? error.message : String(error)}`,
				);
			}
		}
	}

	ocrBackendRegistry.clear();
}
