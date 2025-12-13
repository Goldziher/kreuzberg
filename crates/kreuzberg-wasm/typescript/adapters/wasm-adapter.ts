/**
 * WASM Type Adapter
 *
 * This module provides type adapters for converting between JavaScript/TypeScript
 * types and WASM-compatible types, handling File/Blob conversions, config normalization,
 * and result parsing.
 *
 * @example File Conversion
 * ```typescript
 * import { fileToUint8Array } from '@kreuzberg/wasm/adapters/wasm-adapter';
 *
 * const file = event.target.files[0];
 * const bytes = await fileToUint8Array(file);
 * const result = await extractBytes(bytes, file.type);
 * ```
 *
 * @example Config Normalization
 * ```typescript
 * import { configToJS } from '@kreuzberg/wasm/adapters/wasm-adapter';
 *
 * const config = {
 *   ocr: { backend: 'tesseract', language: 'eng' },
 *   chunking: { maxChars: 1000 }
 * };
 * const normalized = configToJS(config);
 * ```
 */

import type { Chunk, ExtractedImage, ExtractionConfig, ExtractionResult, Table } from "@kreuzberg/core";

/**
 * Maximum file size for processing (512 MB)
 *
 * @internal
 */
const MAX_FILE_SIZE = 512 * 1024 * 1024;

/**
 * Type predicate to validate numeric value or null
 *
 * @internal
 */
function isNumberOrNull(value: unknown): value is number | null {
	return typeof value === "number" || value === null;
}

/**
 * Type predicate to validate string value or null
 *
 * @internal
 */
function isStringOrNull(value: unknown): value is string | null {
	return typeof value === "string" || value === null;
}

/**
 * Type predicate to validate boolean value
 *
 * @internal
 */
function isBoolean(value: unknown): value is boolean {
	return typeof value === "boolean";
}

/**
 * Convert a File or Blob to Uint8Array
 *
 * Handles both browser File API and server-side Blob-like objects,
 * providing a unified interface for reading binary data.
 *
 * @param file - The File or Blob to convert
 * @returns Promise resolving to the byte array
 * @throws {Error} If the file cannot be read or exceeds size limit
 *
 * @example
 * ```typescript
 * const file = document.getElementById('input').files[0];
 * const bytes = await fileToUint8Array(file);
 * const result = await extractBytes(bytes, 'application/pdf');
 * ```
 */
export async function fileToUint8Array(file: File | Blob): Promise<Uint8Array> {
	try {
		if (file.size > MAX_FILE_SIZE) {
			throw new Error(
				`File size (${file.size} bytes) exceeds maximum (${MAX_FILE_SIZE} bytes). Maximum file size is 512 MB.`,
			);
		}

		const arrayBuffer = await file.arrayBuffer();
		return new Uint8Array(arrayBuffer);
	} catch (error) {
		throw new Error(`Failed to read file: ${error instanceof Error ? error.message : String(error)}`);
	}
}

/**
 * Normalize ExtractionConfig for WASM processing
 *
 * Converts TypeScript configuration objects to a WASM-compatible format,
 * handling null values, undefined properties, and nested structures.
 *
 * @param config - The extraction configuration or null
 * @returns Normalized configuration object suitable for WASM
 *
 * @example
 * ```typescript
 * const config: ExtractionConfig = {
 *   ocr: { backend: 'tesseract' },
 *   chunking: { maxChars: 1000 }
 * };
 * const wasmConfig = configToJS(config);
 * ```
 */
export function configToJS(config: ExtractionConfig | null): Record<string, unknown> {
	if (!config) {
		return {};
	}

	const normalized: Record<string, unknown> = {};

	// Recursively normalize nested objects
	const normalizeValue = (value: unknown): unknown => {
		if (value === null || value === undefined) {
			return null;
		}
		if (typeof value === "object") {
			if (Array.isArray(value)) {
				return value.map(normalizeValue);
			}
			const obj = value as Record<string, unknown>;
			const normalized: Record<string, unknown> = {};
			for (const [key, val] of Object.entries(obj)) {
				const normalizedVal = normalizeValue(val);
				if (normalizedVal !== null && normalizedVal !== undefined) {
					normalized[key] = normalizedVal;
				}
			}
			return Object.keys(normalized).length > 0 ? normalized : null;
		}
		return value;
	};

	for (const [key, value] of Object.entries(config)) {
		const normalizedValue = normalizeValue(value);
		if (normalizedValue !== null && normalizedValue !== undefined) {
			normalized[key] = normalizedValue;
		}
	}

	return normalized;
}

/**
 * Parse WASM extraction result and convert to TypeScript type
 *
 * Handles conversion of WASM-returned objects to proper ExtractionResult types,
 * including proper array conversions and type assertions for tables, chunks, and images.
 *
 * @param jsValue - The raw WASM result value
 * @returns Properly typed ExtractionResult
 * @throws {Error} If the result structure is invalid
 *
 * @example
 * ```typescript
 * const wasmResult = await wasmExtract(bytes, mimeType, config);
 * const result = jsToExtractionResult(wasmResult);
 * console.log(result.content);
 * ```
 */
export function jsToExtractionResult(jsValue: unknown): ExtractionResult {
	if (!jsValue || typeof jsValue !== "object") {
		throw new Error("Invalid extraction result: value is not an object");
	}

	const result = jsValue as Record<string, unknown>;

	// Validate required fields
	if (typeof result.content !== "string") {
		throw new Error("Invalid extraction result: missing or invalid content");
	}
	if (typeof result.mimeType !== "string") {
		throw new Error("Invalid extraction result: missing or invalid mimeType");
	}
	if (!result.metadata || typeof result.metadata !== "object") {
		throw new Error("Invalid extraction result: missing or invalid metadata");
	}

	// Parse tables
	const tables: Table[] = [];
	if (Array.isArray(result.tables)) {
		for (const table of result.tables) {
			if (table && typeof table === "object") {
				const t = table as Record<string, unknown>;
				// Validate table structure before type casting
				if (
					Array.isArray(t.cells) &&
					t.cells.every((row) => Array.isArray(row) && row.every((cell) => typeof cell === "string")) &&
					typeof t.markdown === "string" &&
					typeof t.pageNumber === "number"
				) {
					tables.push({
						cells: t.cells as string[][],
						markdown: t.markdown,
						pageNumber: t.pageNumber,
					});
				}
			}
		}
	}

	// Parse chunks
	const chunks: Chunk[] | null = Array.isArray(result.chunks)
		? result.chunks.map((chunk) => {
				if (!chunk || typeof chunk !== "object") {
					throw new Error("Invalid chunk structure");
				}
				const c = chunk as Record<string, unknown>;
				if (typeof c.content !== "string") {
					throw new Error("Invalid chunk: missing content");
				}
				if (!c.metadata || typeof c.metadata !== "object") {
					throw new Error("Invalid chunk: missing metadata");
				}
				const metadata = c.metadata as Record<string, unknown>;

				// Validate embedding array contains only numbers
				let embedding: number[] | null = null;
				if (Array.isArray(c.embedding)) {
					if (!c.embedding.every((item) => typeof item === "number")) {
						throw new Error("Invalid chunk: embedding must contain only numbers");
					}
					embedding = c.embedding;
				}

				// Validate metadata fields
				if (typeof metadata.charStart !== "number") {
					throw new Error("Invalid chunk metadata: charStart must be a number");
				}
				if (typeof metadata.charEnd !== "number") {
					throw new Error("Invalid chunk metadata: charEnd must be a number");
				}
				if (!isNumberOrNull(metadata.tokenCount)) {
					throw new Error("Invalid chunk metadata: tokenCount must be a number or null");
				}
				if (typeof metadata.chunkIndex !== "number") {
					throw new Error("Invalid chunk metadata: chunkIndex must be a number");
				}
				if (typeof metadata.totalChunks !== "number") {
					throw new Error("Invalid chunk metadata: totalChunks must be a number");
				}

				return {
					content: c.content,
					embedding,
					metadata: {
						charStart: metadata.charStart,
						charEnd: metadata.charEnd,
						tokenCount: metadata.tokenCount,
						chunkIndex: metadata.chunkIndex,
						totalChunks: metadata.totalChunks,
					},
				};
			})
		: null;

	// Parse images
	const images: ExtractedImage[] | null = Array.isArray(result.images)
		? result.images.map((image) => {
				if (!image || typeof image !== "object") {
					throw new Error("Invalid image structure");
				}
				const img = image as Record<string, unknown>;
				if (!(img.data instanceof Uint8Array)) {
					throw new Error("Invalid image: data must be Uint8Array");
				}
				if (typeof img.format !== "string") {
					throw new Error("Invalid image: missing format");
				}

				// Validate numeric fields
				if (typeof img.imageIndex !== "number") {
					throw new Error("Invalid image: imageIndex must be a number");
				}
				if (!isNumberOrNull(img.pageNumber)) {
					throw new Error("Invalid image: pageNumber must be a number or null");
				}
				if (!isNumberOrNull(img.width)) {
					throw new Error("Invalid image: width must be a number or null");
				}
				if (!isNumberOrNull(img.height)) {
					throw new Error("Invalid image: height must be a number or null");
				}
				if (!isNumberOrNull(img.bitsPerComponent)) {
					throw new Error("Invalid image: bitsPerComponent must be a number or null");
				}

				// Validate boolean field
				if (!isBoolean(img.isMask)) {
					throw new Error("Invalid image: isMask must be a boolean");
				}

				// Validate string fields
				if (!isStringOrNull(img.colorspace)) {
					throw new Error("Invalid image: colorspace must be a string or null");
				}
				if (!isStringOrNull(img.description)) {
					throw new Error("Invalid image: description must be a string or null");
				}

				return {
					data: img.data,
					format: img.format,
					imageIndex: img.imageIndex,
					pageNumber: img.pageNumber,
					width: img.width,
					height: img.height,
					colorspace: img.colorspace,
					bitsPerComponent: img.bitsPerComponent,
					isMask: img.isMask,
					description: img.description,
					ocrResult: img.ocrResult ? jsToExtractionResult(img.ocrResult) : null,
				};
			})
		: null;

	// Validate detectedLanguages array
	let detectedLanguages: string[] | null = null;
	if (Array.isArray(result.detectedLanguages)) {
		if (!result.detectedLanguages.every((lang) => typeof lang === "string")) {
			throw new Error("Invalid result: detectedLanguages must contain only strings");
		}
		detectedLanguages = result.detectedLanguages;
	}

	return {
		content: result.content,
		mimeType: result.mimeType,
		metadata: result.metadata,
		tables,
		detectedLanguages,
		chunks,
		images,
	};
}

/**
 * Wrap and format WASM errors with context
 *
 * Converts WASM error messages to JavaScript Error objects with proper context
 * and stack trace information when available.
 *
 * @param error - The error from WASM
 * @param context - Additional context about what operation failed
 * @returns A formatted Error object
 *
 * @internal
 *
 * @example
 * ```typescript
 * try {
 *   await wasmExtract(bytes, mimeType);
 * } catch (error) {
 *   throw wrapWasmError(error, 'extracting document');
 * }
 * ```
 */
export function wrapWasmError(error: unknown, context: string): Error {
	if (error instanceof Error) {
		return new Error(`Error ${context}: ${error.message}`, {
			cause: error,
		});
	}

	const message = String(error);
	return new Error(`Error ${context}: ${message}`);
}

/**
 * Validate that a WASM-returned value conforms to ExtractionResult structure
 *
 * Performs structural validation without full type checking,
 * useful for runtime validation of WASM output.
 *
 * @param value - The value to validate
 * @returns True if value appears to be a valid ExtractionResult
 *
 * @internal
 */
export function isValidExtractionResult(value: unknown): value is ExtractionResult {
	if (!value || typeof value !== "object") {
		return false;
	}

	const obj = value as Record<string, unknown>;
	return (
		typeof obj.content === "string" &&
		typeof obj.mimeType === "string" &&
		obj.metadata !== null &&
		typeof obj.metadata === "object" &&
		Array.isArray(obj.tables)
	);
}
