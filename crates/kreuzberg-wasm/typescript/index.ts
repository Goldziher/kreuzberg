/**
 * Kreuzberg - WebAssembly Bindings for Browser and Runtime Environments
 *
 * This module provides WebAssembly bindings for Kreuzberg document intelligence,
 * enabling high-performance document extraction in browser and JavaScript runtime environments.
 *
 * ## Features
 *
 * - Extract text, metadata, and tables from documents
 * - Support for multiple document formats (PDF, Office, images, etc.)
 * - Browser and runtime-compatible WASM bindings
 * - Type-safe TypeScript interfaces
 * - Runtime detection and feature capability checking
 * - Automatic type conversion and error handling
 *
 * ## Installation
 *
 * ```bash
 * npm install @kreuzberg/wasm
 * ```
 *
 * ## Basic Usage
 *
 * ```typescript
 * import { extractBytes, initWasm } from '@kreuzberg/wasm';
 *
 * // Initialize WASM module once at app startup
 * await initWasm();
 *
 * // Extract from bytes
 * const bytes = new Uint8Array(buffer);
 * const result = await extractBytes(bytes, 'application/pdf');
 * console.log(result.content);
 * ```
 *
 * ## Browser Usage with File Input
 *
 * ```typescript
 * import { extractBytes, initWasm } from '@kreuzberg/wasm';
 * import { fileToUint8Array } from '@kreuzberg/wasm/adapters/wasm-adapter';
 *
 * // Initialize once at app startup
 * await initWasm();
 *
 * // Handle file input
 * const fileInput = document.getElementById('file');
 * fileInput.addEventListener('change', async (e) => {
 *   const file = e.target.files?.[0];
 *   if (file) {
 *     const bytes = await fileToUint8Array(file);
 *     const result = await extractBytes(bytes, file.type);
 *     console.log(result.content);
 *   }
 * });
 * ```
 *
 * ## Runtime Detection
 *
 * ```typescript
 * import { detectRuntime, getWasmCapabilities } from '@kreuzberg/wasm/runtime';
 *
 * const runtime = detectRuntime();
 * const caps = getWasmCapabilities();
 *
 * if (caps.hasWorkers) {
 *   // Can use Web Workers for parallel processing
 * }
 * ```
 *
 * ## Configuration
 *
 * ```typescript
 * import { extractBytes, initWasm } from '@kreuzberg/wasm';
 * import type { ExtractionConfig } from '@kreuzberg/wasm';
 *
 * await initWasm();
 *
 * const config: ExtractionConfig = {
 *   ocr: {
 *     backend: 'tesseract',
 *     language: 'eng'
 *   },
 *   chunking: {
 *     maxChars: 1000,
 *     chunkOverlap: 100
 *   },
 *   images: {
 *     extractImages: true,
 *     targetDpi: 150
 *   }
 * };
 *
 * const result = await extractBytes(bytes, 'application/pdf', config);
 * ```
 */

import type { ExtractionConfig as ExtractionConfigType, ExtractionResult } from "./types.js";

import { configToJS, fileToUint8Array, jsToExtractionResult, wrapWasmError } from "./adapters/wasm-adapter.js";
import { registerOcrBackend } from "./ocr/registry.js";
import { TesseractWasmBackend } from "./ocr/tesseract-wasm-backend.js";
import { detectRuntime, hasWasm, isBrowser } from "./runtime.js";

export type {
	Chunk,
	ChunkingConfig,
	ExtractedImage,
	ExtractionConfig,
	ExtractionResult,
	Table,
} from "./types.js";

export type * from "./types.js";

// Re-export adapter utilities for convenient access
export {
	configToJS,
	fileToUint8Array,
	isValidExtractionResult,
	jsToExtractionResult,
	wrapWasmError,
} from "./adapters/wasm-adapter.js";

// Re-export OCR registry
export {
	clearOcrBackends,
	getOcrBackend,
	listOcrBackends,
	registerOcrBackend,
	unregisterOcrBackend,
} from "./ocr/registry.js";

// Re-export OCR backends
export { TesseractWasmBackend } from "./ocr/tesseract-wasm-backend.js";

// Re-export runtime utilities
export {
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
	type RuntimeType,
	type WasmCapabilities,
} from "./runtime.js";

// WASM binding imports (to be populated during build)
// Type definition for the WASM module interface
type WasmModule = {
	// Extraction functions
	extractBytes: (data: Uint8Array, mimeType: string, config: Record<string, unknown> | null) => Promise<unknown>;
	extractBytesSync: (data: Uint8Array, mimeType: string, config: Record<string, unknown> | null) => unknown;
	batchExtractBytes: (
		dataList: Uint8Array[],
		mimeTypes: string[],
		config: Record<string, unknown> | null,
	) => Promise<unknown>;
	batchExtractBytesSync: (
		dataList: Uint8Array[],
		mimeTypes: string[],
		config: Record<string, unknown> | null,
	) => unknown;
	extractFile: (file: File, mimeType: string | null, config: Record<string, unknown> | null) => Promise<unknown>;
	batchExtractFiles: (files: File[], config: Record<string, unknown> | null) => Promise<unknown>;

	// MIME and file type utilities
	detectMimeFromBytes: (data: Uint8Array) => string;
	normalizeMimeType: (mimeType: string) => string;
	getMimeFromExtension: (extension: string) => string | null;
	getExtensionsForMime: (mimeType: string) => string[];

	// Configuration
	loadConfigFromString: (content: string, format: string) => Record<string, unknown>;
	discoverConfig: () => Record<string, unknown>;

	// Module information
	version: () => string;
	get_module_info: () => ModuleInfo;

	// OCR backend management
	register_ocr_backend: (backend: unknown) => void;
	unregister_ocr_backend: (name: string) => void;
	list_ocr_backends: () => string[];
	clear_ocr_backends: () => void;

	// Post-processor management
	register_post_processor: (processor: unknown) => void;
	unregister_post_processor: (name: string) => void;
	list_post_processors: () => string[];
	clear_post_processors: () => void;

	// Validator management
	register_validator: (validator: unknown) => void;
	unregister_validator: (name: string) => void;
	list_validators: () => string[];
	clear_validators: () => void;

	// PDF/WASM utilities
	initialize_pdfium_render: (pdfiumWasmModule: unknown, localWasmModule: unknown, debug: boolean) => boolean;
	read_block_from_callback_wasm: (param: number, position: number, pBuf: number, size: number) => number;
	write_block_from_callback_wasm: (param: number, buf: number, size: number) => number;

	// Initialization
	default?: () => Promise<void>;
};

// ModuleInfo type definition
type ModuleInfo = {
	name: () => string;
	version: () => string;
	free: () => void;
};

let wasm: WasmModule | null = null;

/** Initialize flag */
let initialized = false;

/** Initialization error (if any) */
let initializationError: Error | null = null;

/** Initialization promise for handling concurrent init calls */
let initializationPromise: Promise<void> | null = null;

/**
 * Initialize the WASM module
 *
 * This function must be called once before using any extraction functions.
 * It loads and initializes the WASM module in the current runtime environment,
 * automatically selecting the appropriate WASM variant for the detected runtime.
 *
 * Multiple calls to initWasm() are safe and will return immediately if already initialized.
 *
 * @throws {Error} If WASM module fails to load or is not supported in the current environment
 *
 * @example Basic Usage
 * ```typescript
 * import { initWasm } from '@kreuzberg/wasm';
 *
 * async function main() {
 *   await initWasm();
 *   // Now you can use extraction functions
 * }
 *
 * main().catch(console.error);
 * ```
 *
 * @example With Error Handling
 * ```typescript
 * import { initWasm, getWasmCapabilities } from '@kreuzberg/wasm';
 *
 * async function initializeKreuzberg() {
 *   const caps = getWasmCapabilities();
 *   if (!caps.hasWasm) {
 *     throw new Error('WebAssembly is not supported in this environment');
 *   }
 *
 *   try {
 *     await initWasm();
 *     console.log('Kreuzberg initialized successfully');
 *   } catch (error) {
 *     console.error('Failed to initialize Kreuzberg:', error);
 *     throw error;
 *   }
 * }
 * ```
 */
export async function initWasm(): Promise<void> {
	if (initialized) {
		return;
	}

	// Handle concurrent init calls
	if (initializationPromise) {
		return initializationPromise;
	}

	initializationPromise = (async () => {
		try {
			// Check WASM support
			if (!hasWasm()) {
				throw new Error("WebAssembly is not supported in this environment");
			}

			// Dynamic WASM import and initialization
			let wasmModule: unknown;
			try {
				// Try importing from pkg directory (development/monorepo layout)
				// @ts-expect-error - Path may not exist at type-check time
				wasmModule = await import("../../pkg/kreuzberg_wasm");
			} catch {
				// Fallback to dist-relative path (published package layout)
				// @ts-expect-error - Dynamic import path
				wasmModule = await import("./kreuzberg_wasm");
			}
			wasm = wasmModule as unknown as WasmModule;

			// Call default initialization if available (for some wasm-pack targets)
			if (wasm && typeof wasm.default === "function") {
				await wasm.default();
			}

			initialized = true;
			initializationError = null;
		} catch (error) {
			initializationError = error instanceof Error ? error : new Error(String(error));
			throw wrapWasmError(error, "initializing Kreuzberg WASM module");
		}
	})();

	return initializationPromise;
}

/**
 * Check if WASM module is initialized
 *
 * @returns True if WASM module is initialized, false otherwise
 *
 * @example
 * ```typescript
 * if (!isInitialized()) {
 *   await initWasm();
 * }
 * ```
 */
export function isInitialized(): boolean {
	return initialized;
}

/**
 * Get WASM module version
 *
 * @throws {Error} If WASM module is not initialized
 * @returns The version string of the WASM module
 *
 * @example
 * ```typescript
 * const version = getVersion();
 * console.log(`Using Kreuzberg ${version}`);
 * ```
 */
export function getVersion(): string {
	if (!initialized) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	if (!wasm) {
		throw new Error("WASM module not loaded. Call initWasm() first.");
	}

	return wasm.version();
}

/**
 * Get initialization error if module failed to load
 *
 * @returns The error that occurred during initialization, or null if no error
 *
 * @internal
 */
export function getInitializationError(): Error | null {
	return initializationError;
}

/**
 * Extract content from bytes (document data)
 *
 * Extracts text, metadata, tables, images, and other content from document bytes.
 * Automatically detects document type from MIME type and applies appropriate extraction logic.
 *
 * @param data - The document bytes to extract from
 * @param mimeType - MIME type of the document (e.g., 'application/pdf', 'image/jpeg')
 * @param config - Optional extraction configuration
 * @returns Promise resolving to the extraction result
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example Extract PDF
 * ```typescript
 * const bytes = new Uint8Array(buffer);
 * const result = await extractBytes(bytes, 'application/pdf');
 * console.log(result.content);
 * console.log(result.tables);
 * ```
 *
 * @example Extract with Configuration
 * ```typescript
 * const result = await extractBytes(bytes, 'application/pdf', {
 *   ocr: {
 *     backend: 'tesseract',
 *     language: 'deu' // German
 *   },
 *   images: {
 *     extractImages: true,
 *     targetDpi: 200
 *   }
 * });
 * ```
 *
 * @example Extract from File
 * ```typescript
 * const file = inputEvent.target.files[0];
 * const bytes = await fileToUint8Array(file);
 * const result = await extractBytes(bytes, file.type);
 * ```
 */
export async function extractBytes(
	data: Uint8Array,
	mimeType: string,
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult> {
	if (!initialized) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	if (!wasm) {
		throw new Error("WASM module not loaded. Call initWasm() first.");
	}

	try {
		// Validate input
		if (!data || data.length === 0) {
			throw new Error("Document data cannot be empty");
		}

		if (!mimeType) {
			throw new Error("MIME type is required");
		}

		// Normalize config for WASM
		const normalizedConfig = configToJS(config ?? null);

		// Call WASM function
		const result = await wasm.extractBytes(data, mimeType, normalizedConfig);

		// Validate result
		if (!result) {
			throw new Error("Invalid extraction result: no result from WASM module");
		}

		// Convert and return result
		return jsToExtractionResult(result);
	} catch (error) {
		throw wrapWasmError(error, "extracting from bytes");
	}
}

/**
 * Extract content from a file on the file system
 *
 * Node.js and Deno specific function that reads a file from the file system
 * and extracts content from it. Automatically detects MIME type if not provided.
 *
 * @param path - Path to the file to extract from
 * @param mimeType - Optional MIME type of the file. If not provided, will attempt to detect
 * @param config - Optional extraction configuration
 * @returns Promise resolving to the extraction result
 * @throws {Error} If WASM module is not initialized, file doesn't exist, or extraction fails
 *
 * @example Extract with auto-detection
 * ```typescript
 * const result = await extractFile('./document.pdf');
 * console.log(result.content);
 * ```
 *
 * @example Extract with explicit MIME type
 * ```typescript
 * const result = await extractFile('./document.docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document');
 * ```
 *
 * @example Extract from Node.js with config
 * ```typescript
 * import { extractFile } from '@kreuzberg/wasm';
 * import { readFile } from 'fs/promises';
 *
 * const result = await extractFile('./report.xlsx', null, {
 *   chunking: {
 *     maxChars: 1000
 *   }
 * });
 * ```
 */
export async function extractFile(
	path: string,
	mimeType?: string | null,
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult> {
	if (!initialized) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	if (!wasm) {
		throw new Error("WASM module not loaded. Call initWasm() first.");
	}

	try {
		if (!path) {
			throw new Error("File path is required");
		}

		// This function is only suitable for Node.js/Deno/Bun environments
		// Browser environments should use extractBytes with fileToUint8Array
		const runtime = detectRuntime();
		if (runtime === "browser") {
			throw new Error("Use extractBytes with fileToUint8Array for browser environments");
		}

		// Read file based on runtime
		let fileData: Uint8Array;

		if (runtime === "node") {
			// Node.js: use dynamic import to avoid issues in non-Node.js environments
			const { readFile } = await import("node:fs/promises");
			const buffer = await readFile(path);
			fileData = new Uint8Array(buffer);
		} else if (runtime === "deno") {
			// Deno: use Deno.readFile
			const deno = (globalThis as Record<string, unknown>).Deno as {
				readFile: (path: string) => Promise<Uint8Array>;
			};
			fileData = await deno.readFile(path);
		} else if (runtime === "bun") {
			// Bun: use dynamic import for fs/promises (compatible with Node.js API)
			const { readFile } = await import("node:fs/promises");
			const buffer = await readFile(path);
			fileData = new Uint8Array(buffer);
		} else {
			throw new Error(`Unsupported runtime for file extraction: ${runtime}`);
		}

		// Detect MIME type if not provided
		let detectedMimeType = mimeType;
		if (!detectedMimeType) {
			detectedMimeType = wasm.detectMimeFromBytes(fileData);
		}

		if (!detectedMimeType) {
			throw new Error("Could not detect MIME type for file. Please provide mimeType parameter.");
		}

		// Normalize MIME type
		detectedMimeType = wasm.normalizeMimeType(detectedMimeType);

		// Call extractBytes with the file contents
		return await extractBytes(fileData, detectedMimeType, config);
	} catch (error) {
		throw wrapWasmError(error, `extracting from file: ${path}`);
	}
}

/**
 * Extract content from a File or Blob (browser-friendly wrapper)
 *
 * Convenience function that wraps fileToUint8Array and extractBytes,
 * providing a streamlined API for browser applications handling file inputs.
 *
 * @param file - The File or Blob to extract from
 * @param mimeType - Optional MIME type. If not provided, uses file.type if available
 * @param config - Optional extraction configuration
 * @returns Promise resolving to the extraction result
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example Simple file extraction
 * ```typescript
 * const fileInput = document.getElementById('file');
 * fileInput.addEventListener('change', async (e) => {
 *   const file = e.target.files?.[0];
 *   if (file) {
 *     const result = await extractFromFile(file);
 *     console.log(result.content);
 *   }
 * });
 * ```
 *
 * @example With configuration
 * ```typescript
 * const result = await extractFromFile(file, file.type, {
 *   chunking: { maxChars: 1000 },
 *   images: { extractImages: true }
 * });
 * ```
 */
export async function extractFromFile(
	file: File | Blob,
	mimeType?: string | null,
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult> {
	if (!initialized) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	if (!wasm) {
		throw new Error("WASM module not loaded. Call initWasm() first.");
	}

	try {
		const bytes = await fileToUint8Array(file);
		let type = mimeType ?? (file instanceof File ? file.type : "application/octet-stream");

		// Normalize MIME type
		type = wasm.normalizeMimeType(type);

		return await extractBytes(bytes, type, config);
	} catch (error) {
		throw wrapWasmError(error, `extracting from ${file instanceof File ? "file" : "blob"}`);
	}
}

/**
 * Extract content from bytes synchronously
 *
 * Synchronous version of extractBytes. Performs extraction without async operations.
 * Note: Some extraction features may still be async internally, but the wrapper is synchronous.
 *
 * @param data - The document bytes to extract from
 * @param mimeType - MIME type of the document
 * @param config - Optional extraction configuration
 * @returns The extraction result
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example
 * ```typescript
 * const bytes = new Uint8Array(buffer);
 * const result = extractBytesSync(bytes, 'application/pdf');
 * console.log(result.content);
 * ```
 */
export function extractBytesSync(
	data: Uint8Array,
	mimeType: string,
	config?: ExtractionConfigType | null,
): ExtractionResult {
	if (!initialized) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	if (!wasm) {
		throw new Error("WASM module not loaded. Call initWasm() first.");
	}

	try {
		// Validate input
		if (!data || data.length === 0) {
			throw new Error("Document data cannot be empty");
		}

		if (!mimeType) {
			throw new Error("MIME type is required");
		}

		// Normalize config for WASM
		const normalizedConfig = configToJS(config ?? null);

		// Call WASM function
		const result = wasm.extractBytesSync(data, mimeType, normalizedConfig);

		// Validate result
		if (!result) {
			throw new Error("Invalid extraction result: no result from WASM module");
		}

		// Convert and return result
		return jsToExtractionResult(result);
	} catch (error) {
		throw wrapWasmError(error, "extracting from bytes (sync)");
	}
}

/**
 * Batch extract content from multiple byte arrays asynchronously
 *
 * Extracts content from multiple documents in a single batch operation,
 * allowing for more efficient processing of multiple files.
 *
 * @param files - Array of objects containing data (Uint8Array) and mimeType (string)
 * @param config - Optional extraction configuration applied to all files
 * @returns Promise resolving to array of extraction results
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example
 * ```typescript
 * const files = [
 *   { data: pdfBytes, mimeType: 'application/pdf' },
 *   { data: docxBytes, mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' }
 * ];
 * const results = await batchExtractBytes(files);
 * results.forEach((result) => console.log(result.content));
 * ```
 */
export async function batchExtractBytes(
	files: Array<{ data: Uint8Array; mimeType: string }>,
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult[]> {
	if (!initialized) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	if (!wasm) {
		throw new Error("WASM module not loaded. Call initWasm() first.");
	}

	try {
		// Validate input
		if (!Array.isArray(files)) {
			throw new Error("Files parameter must be an array");
		}

		if (files.length === 0) {
			throw new Error("Files array cannot be empty");
		}

		// Extract data arrays and MIME types
		const dataList: Uint8Array[] = [];
		const mimeTypes: string[] = [];

		for (let i = 0; i < files.length; i += 1) {
			const file = files[i];
			if (!file || typeof file !== "object") {
				throw new Error(`Invalid file at index ${i}: must be an object with data and mimeType`);
			}

			const f = file as Record<string, unknown>;

			if (!(f.data instanceof Uint8Array)) {
				throw new Error(`Invalid file at index ${i}: data must be Uint8Array`);
			}

			if (typeof f.mimeType !== "string") {
				throw new Error(`Invalid file at index ${i}: mimeType must be a string`);
			}

			if (f.data.length === 0) {
				throw new Error(`Invalid file at index ${i}: data cannot be empty`);
			}

			dataList.push(f.data);
			mimeTypes.push(f.mimeType);
		}

		// Normalize config for WASM
		const normalizedConfig = configToJS(config ?? null);

		// Call WASM function
		const results = await wasm.batchExtractBytes(dataList, mimeTypes, normalizedConfig);

		// Validate results
		if (!Array.isArray(results)) {
			throw new Error("Invalid batch extraction result: expected array");
		}

		// Convert each result
		return results.map((result, index) => {
			if (!result) {
				throw new Error(`Invalid extraction result at index ${index}: no result from WASM module`);
			}

			return jsToExtractionResult(result);
		});
	} catch (error) {
		throw wrapWasmError(error, "batch extracting from bytes");
	}
}

/**
 * Batch extract content from multiple byte arrays synchronously
 *
 * Synchronous version of batchExtractBytes. Extracts content from multiple documents
 * in a single batch operation without async operations.
 *
 * @param files - Array of objects containing data (Uint8Array) and mimeType (string)
 * @param config - Optional extraction configuration applied to all files
 * @returns Array of extraction results
 * @throws {Error} If WASM module is not initialized or extraction fails
 *
 * @example
 * ```typescript
 * const files = [
 *   { data: pdfBytes, mimeType: 'application/pdf' },
 *   { data: docxBytes, mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' }
 * ];
 * const results = batchExtractBytesSync(files);
 * results.forEach((result) => console.log(result.content));
 * ```
 */
export function batchExtractBytesSync(
	files: Array<{ data: Uint8Array; mimeType: string }>,
	config?: ExtractionConfigType | null,
): ExtractionResult[] {
	if (!initialized) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	if (!wasm) {
		throw new Error("WASM module not loaded. Call initWasm() first.");
	}

	try {
		// Validate input
		if (!Array.isArray(files)) {
			throw new Error("Files parameter must be an array");
		}

		if (files.length === 0) {
			throw new Error("Files array cannot be empty");
		}

		// Extract data arrays and MIME types
		const dataList: Uint8Array[] = [];
		const mimeTypes: string[] = [];

		for (let i = 0; i < files.length; i += 1) {
			const file = files[i];
			if (!file || typeof file !== "object") {
				throw new Error(`Invalid file at index ${i}: must be an object with data and mimeType`);
			}

			const f = file as Record<string, unknown>;

			if (!(f.data instanceof Uint8Array)) {
				throw new Error(`Invalid file at index ${i}: data must be Uint8Array`);
			}

			if (typeof f.mimeType !== "string") {
				throw new Error(`Invalid file at index ${i}: mimeType must be a string`);
			}

			if (f.data.length === 0) {
				throw new Error(`Invalid file at index ${i}: data cannot be empty`);
			}

			dataList.push(f.data);
			mimeTypes.push(f.mimeType);
		}

		// Normalize config for WASM
		const normalizedConfig = configToJS(config ?? null);

		// Call WASM function
		const results = wasm.batchExtractBytesSync(dataList, mimeTypes, normalizedConfig);

		// Validate results
		if (!Array.isArray(results)) {
			throw new Error("Invalid batch extraction result: expected array");
		}

		// Convert each result
		return results.map((result, index) => {
			if (!result) {
				throw new Error(`Invalid extraction result at index ${index}: no result from WASM module`);
			}

			return jsToExtractionResult(result);
		});
	} catch (error) {
		throw wrapWasmError(error, "batch extracting from bytes (sync)");
	}
}

/**
 * Batch extract content from multiple File objects asynchronously
 *
 * Convenience function that converts File objects to Uint8Array and calls batchExtractBytes.
 * Automatically uses the file.type as MIME type if available.
 *
 * @param files - Array of File objects to extract from
 * @param config - Optional extraction configuration applied to all files
 * @returns Promise resolving to array of extraction results
 * @throws {Error} If WASM module is not initialized, files cannot be read, or extraction fails
 *
 * @example
 * ```typescript
 * const fileInput = document.getElementById('files');
 * const files = Array.from(fileInput.files ?? []);
 * const results = await batchExtractFiles(files);
 * results.forEach((result, index) => {
 *   console.log(`File ${index}: ${result.content.substring(0, 50)}...`);
 * });
 * ```
 */
export async function batchExtractFiles(
	files: File[],
	config?: ExtractionConfigType | null,
): Promise<ExtractionResult[]> {
	if (!initialized) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	try {
		// Validate input
		if (!Array.isArray(files)) {
			throw new Error("Files parameter must be an array");
		}

		if (files.length === 0) {
			throw new Error("Files array cannot be empty");
		}

		// Convert all files to Uint8Array and collect MIME types
		const byteFiles: Array<{ data: Uint8Array; mimeType: string }> = [];

		for (let i = 0; i < files.length; i += 1) {
			const file = files[i];
			if (!(file instanceof File)) {
				throw new Error(`Invalid file at index ${i}: must be a File object`);
			}

			const bytes = await fileToUint8Array(file);
			byteFiles.push({
				data: bytes,
				mimeType: file.type || "application/octet-stream",
			});
		}

		// Call batchExtractBytes with converted files
		return await batchExtractBytes(byteFiles, config);
	} catch (error) {
		throw wrapWasmError(error, "batch extracting from files");
	}
}

/**
 * Enable OCR functionality with tesseract-wasm backend
 *
 * Convenience function that automatically initializes and registers the Tesseract WASM backend.
 * This is the recommended approach for enabling OCR in WASM-based applications.
 *
 * ## Browser Requirement
 *
 * This function requires a browser environment with support for:
 * - WebWorkers (for Tesseract processing)
 * - createImageBitmap (for image conversion)
 * - Blob API
 *
 * ## Network Requirement
 *
 * Training data will be loaded from jsDelivr CDN on first use of each language.
 * Ensure network access to cdn.jsdelivr.net is available.
 *
 * @throws {Error} If not in browser environment or tesseract-wasm is not available
 *
 * @example Basic Usage
 * ```typescript
 * import { enableOcr, extractBytes, initWasm } from '@kreuzberg/wasm';
 *
 * async function main() {
 *   // Initialize WASM module
 *   await initWasm();
 *
 *   // Enable OCR with tesseract-wasm
 *   await enableOcr();
 *
 *   // Now you can use OCR in extraction
 *   const imageBytes = new Uint8Array(buffer);
 *   const result = await extractBytes(imageBytes, 'image/png', {
 *     ocr: { backend: 'tesseract-wasm', language: 'eng' }
 *   });
 *
 *   console.log(result.content); // Extracted text
 * }
 *
 * main().catch(console.error);
 * ```
 *
 * @example With Progress Tracking
 * ```typescript
 * import { enableOcr, TesseractWasmBackend } from '@kreuzberg/wasm';
 *
 * async function setupOcrWithProgress() {
 *   const backend = new TesseractWasmBackend();
 *   backend.setProgressCallback((progress) => {
 *     console.log(`OCR Progress: ${progress}%`);
 *     updateProgressBar(progress);
 *   });
 *
 *   await backend.initialize();
 *   registerOcrBackend(backend);
 * }
 *
 * setupOcrWithProgress().catch(console.error);
 * ```
 *
 * @example Multiple Languages
 * ```typescript
 * import { enableOcr, extractBytes, initWasm } from '@kreuzberg/wasm';
 *
 * await initWasm();
 * await enableOcr();
 *
 * // Extract English text
 * const englishResult = await extractBytes(engImageBytes, 'image/png', {
 *   ocr: { backend: 'tesseract-wasm', language: 'eng' }
 * });
 *
 * // Extract German text - model is cached after first use
 * const germanResult = await extractBytes(deImageBytes, 'image/png', {
 *   ocr: { backend: 'tesseract-wasm', language: 'deu' }
 * });
 * ```
 */
export async function enableOcr(): Promise<void> {
	// Check if WASM module is initialized
	if (!initialized) {
		throw new Error("WASM module not initialized. Call initWasm() first.");
	}

	// Check if in browser environment
	if (!isBrowser()) {
		throw new Error(
			"OCR is only available in browser environments. TesseractWasmBackend requires Web Workers and createImageBitmap.",
		);
	}

	try {
		// Create and initialize backend
		const backend = new TesseractWasmBackend();
		await backend.initialize();

		// Register the backend
		registerOcrBackend(backend);
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		throw new Error(`Failed to enable OCR: ${message}`);
	}
}
