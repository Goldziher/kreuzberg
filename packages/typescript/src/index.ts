/**
 * Kreuzberg - Multi-language document intelligence framework.
 *
 * This is a TypeScript SDK around a high-performance Rust core.
 * All extraction logic, chunking, quality processing, and language detection
 * are implemented in Rust for maximum performance.
 *
 * TypeScript-specific features:
 * - Runtime detection: Automatically uses NAPI (Node.js) or WASM (browsers/Deno)
 * - OCR backends: @gutenye/ocr (TypeScript-based OCR engine)
 * - Custom PostProcessors: Register your own TypeScript processing logic
 * - CLI proxy: Execute kreuzberg-cli binary for API and MCP server
 */

import type {
	ExtractionConfig,
	ExtractionResult,
	OcrBackendProtocol,
	PostProcessorProtocol,
} from "./types.js";

export * from "./types.js";

// ============================================================================
// Runtime Detection
// ============================================================================

let binding: any = null;
let bindingInitialized = false;

function getBinding(): any {
	if (bindingInitialized) {
		return binding;
	}

	// Try to load NAPI binding (Node.js/Bun)
	try {
		if (
			typeof process !== "undefined" &&
			process.versions &&
			process.versions.node
		) {
			binding = require("kreuzberg-node");
			bindingInitialized = true;
			return binding;
		}
	} catch {
		// NAPI binding not available, will try WASM
	}

	// If NAPI failed, try WASM (browsers/Deno)
	try {
		// WASM binding would be loaded here
		// binding = require('kreuzberg-wasm');
		throw new Error("WASM binding not yet implemented");
	} catch {
		throw new Error(
			"Failed to load Kreuzberg bindings. Neither NAPI (Node.js) nor WASM (browsers/Deno) bindings are available. " +
				"Make sure you have installed the kreuzberg-node package for Node.js/Bun.",
		);
	}
}

// ============================================================================
// Helper Functions
// ============================================================================

function parseMetadata(metadataStr: string): any {
	try {
		return JSON.parse(metadataStr);
	} catch {
		return {};
	}
}

function convertResult(rawResult: any): ExtractionResult {
	return {
		content: rawResult.content,
		mimeType: rawResult.mimeType,
		metadata:
			typeof rawResult.metadata === "string"
				? parseMetadata(rawResult.metadata)
				: rawResult.metadata,
		tables: rawResult.tables || [],
		detectedLanguages: rawResult.detectedLanguages || null,
		chunks: rawResult.chunks || null,
	};
}

// ============================================================================
// Extraction Functions
// ============================================================================

/**
 * Extract content from a file (synchronous).
 *
 * @param filePath - Path to the file (string)
 * @param mimeType - Optional MIME type hint (auto-detected if null)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns ExtractionResult with content, metadata, and tables
 *
 * @example
 * ```typescript
 * import { extractFileSync } from 'kreuzberg';
 *
 * // Basic usage
 * const result = extractFileSync('document.pdf');
 *
 * // With Tesseract configuration
 * const config = {
 *   ocr: {
 *     backend: 'tesseract',
 *     language: 'eng',
 *     tesseractConfig: {
 *       psm: 6,
 *       enableTableDetection: true,
 *       tesseditCharWhitelist: '0123456789',
 *     },
 *   },
 * };
 * const result2 = extractFileSync('invoice.pdf', null, config);
 * ```
 */
export function extractFileSync(
	filePath: string,
	mimeType: string | null = null,
	config: ExtractionConfig | null = null,
): ExtractionResult {
	const rawResult = getBinding().extractFileSync(filePath, mimeType, config);
	return convertResult(rawResult);
}

/**
 * Extract content from a file (asynchronous).
 *
 * @param filePath - Path to the file (string)
 * @param mimeType - Optional MIME type hint (auto-detected if null)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Promise<ExtractionResult> with content, metadata, and tables
 */
export async function extractFile(
	filePath: string,
	mimeType: string | null = null,
	config: ExtractionConfig | null = null,
): Promise<ExtractionResult> {
	const rawResult = await getBinding().extractFile(filePath, mimeType, config);
	return convertResult(rawResult);
}

/**
 * Extract content from bytes (synchronous).
 *
 * @param data - File content as Uint8Array
 * @param mimeType - MIME type of the data (required for format detection)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns ExtractionResult with content, metadata, and tables
 */
export function extractBytesSync(
	data: Uint8Array,
	mimeType: string,
	config: ExtractionConfig | null = null,
): ExtractionResult {
	const rawResult = getBinding().extractBytesSync(
		Buffer.from(data),
		mimeType,
		config,
	);
	return convertResult(rawResult);
}

/**
 * Extract content from bytes (asynchronous).
 *
 * @param data - File content as Uint8Array
 * @param mimeType - MIME type of the data (required for format detection)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Promise<ExtractionResult> with content, metadata, and tables
 */
export async function extractBytes(
	data: Uint8Array,
	mimeType: string,
	config: ExtractionConfig | null = null,
): Promise<ExtractionResult> {
	const rawResult = await getBinding().extractBytes(
		Buffer.from(data),
		mimeType,
		config,
	);
	return convertResult(rawResult);
}

/**
 * Extract content from multiple files in parallel (synchronous).
 *
 * @param paths - List of file paths
 * @param config - Extraction configuration (uses defaults if null)
 * @returns List of ExtractionResults (one per file)
 */
export function batchExtractFilesSync(
	paths: string[],
	config: ExtractionConfig | null = null,
): ExtractionResult[] {
	const rawResults = getBinding().batchExtractFilesSync(paths, config);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple files in parallel (asynchronous).
 *
 * @param paths - List of file paths
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Promise<ExtractionResult[]> (one per file)
 */
export async function batchExtractFiles(
	paths: string[],
	config: ExtractionConfig | null = null,
): Promise<ExtractionResult[]> {
	const rawResults = await getBinding().batchExtractFiles(paths, config);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple byte arrays in parallel (asynchronous).
 *
 * @param dataList - List of file contents as Uint8Arrays
 * @param mimeTypes - List of MIME types (one per data item)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Promise<ExtractionResult[]> (one per data item)
 */
export async function batchExtractBytes(
	dataList: Uint8Array[],
	mimeTypes: string[],
	config: ExtractionConfig | null = null,
): Promise<ExtractionResult[]> {
	const buffers = dataList.map((data) => Buffer.from(data));
	const rawResults = await getBinding().batchExtractBytes(
		buffers,
		mimeTypes,
		config,
	);
	return rawResults.map(convertResult);
}

// ============================================================================
// Plugin System (Stub - To be implemented)
// ============================================================================

/**
 * Register a custom postprocessor.
 *
 * @param processor - PostProcessorProtocol implementation
 *
 * @example
 * ```typescript
 * import { registerPostProcessor, ExtractionResult } from 'kreuzberg';
 *
 * class MyProcessor implements PostProcessorProtocol {
 *   name(): string {
 *     return 'my_processor';
 *   }
 *
 *   process(result: ExtractionResult): ExtractionResult {
 *     result.metadata.customField = 'custom_value';
 *     return result;
 *   }
 *
 *   processingStage(): 'early' | 'middle' | 'late' {
 *     return 'middle';
 *   }
 * }
 *
 * registerPostProcessor(new MyProcessor());
 * ```
 */
export function registerPostProcessor(_processor: PostProcessorProtocol): void {
	// TODO: Implement FFI bridge to Rust core
	throw new Error("registerPostProcessor not yet implemented");
}

/**
 * Unregister a postprocessor by name.
 *
 * @param name - Name of the processor to unregister
 */
export function unregisterPostProcessor(_name: string): void {
	// TODO: Implement FFI bridge to Rust core
	throw new Error("unregisterPostProcessor not yet implemented");
}

/**
 * Clear all registered postprocessors.
 */
export function clearPostProcessors(): void {
	// TODO: Implement FFI bridge to Rust core
	throw new Error("clearPostProcessors not yet implemented");
}

/**
 * Register a custom OCR backend.
 *
 * @param backend - OcrBackendProtocol implementation
 */
export function registerOcrBackend(_backend: OcrBackendProtocol): void {
	// TODO: Implement FFI bridge to Rust core
	throw new Error("registerOcrBackend not yet implemented");
}

// ============================================================================
// Version
// ============================================================================

export const __version__ = "4.0.0";
