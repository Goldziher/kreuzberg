/**
 * Kreuzberg - Multi-language document intelligence framework.
 *
 * This is a TypeScript SDK around a high-performance Rust core.
 * All extraction logic, chunking, quality processing, and language detection
 * are implemented in Rust for maximum performance.
 *
 * ## API Usage Recommendations
 *
 * **For processing multiple documents**, prefer batch APIs:
 * - Use `batchExtractFiles()` / `batchExtractFilesSync()` for multiple files
 * - Use `batchExtractBytes()` / `batchExtractBytesSync()` for multiple byte arrays
 *
 * **Batch APIs provide**:
 * - Better performance (parallel processing in Rust)
 * - More reliable memory management
 * - Recommended for all multi-document workflows
 *
 * **Single extraction APIs** (`extractFile`, `extractBytes`) are suitable for:
 * - One-off document processing
 * - Interactive applications processing documents on-demand
 * - Avoid calling these in tight loops - use batch APIs instead
 *
 * ## Supported Formats
 *
 * - **Documents**: PDF, DOCX, PPTX, XLSX, DOC, PPT (with LibreOffice)
 * - **Text**: Markdown, Plain Text, XML
 * - **Web**: HTML (converted to Markdown)
 * - **Data**: JSON, YAML, TOML
 * - **Email**: EML, MSG
 * - **Images**: PNG, JPEG, TIFF (with OCR support)
 *
 * @example
 * ```typescript
 * import { extractFile, batchExtractFiles } from 'kreuzberg';
 *
 * // Single file extraction
 * const result = await extractFile('document.pdf');
 * console.log(result.content);
 *
 * // Multiple files (recommended approach)
 * const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 * const results = await batchExtractFiles(files);
 * results.forEach(r => console.log(r.content));
 * ```
 */

import type {
	ExtractionConfig,
	ExtractionResult,
	OcrBackendProtocol,
	PostProcessorProtocol,
	ValidatorProtocol,
} from "./types.js";

export * from "./types.js";

// biome-ignore lint/suspicious/noExplicitAny: NAPI binding type is dynamically loaded
let binding: any = null;
let bindingInitialized = false;

// biome-ignore lint/suspicious/noExplicitAny: NAPI binding type is dynamically loaded
function getBinding(): any {
	if (bindingInitialized) {
		return binding;
	}

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
	} catch {}

	try {
		throw new Error("WASM binding not yet implemented");
	} catch {
		throw new Error(
			"Failed to load Kreuzberg bindings. Neither NAPI (Node.js) nor WASM (browsers/Deno) bindings are available. " +
				"Make sure you have installed the kreuzberg-node package for Node.js/Bun.",
		);
	}
}

// biome-ignore lint/suspicious/noExplicitAny: JSON.parse returns any
function parseMetadata(metadataStr: string): any {
	try {
		return JSON.parse(metadataStr);
	} catch {
		return {};
	}
}

// biome-ignore lint/suspicious/noExplicitAny: Raw NAPI result is untyped
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

/**
 * Extract content from a single file (synchronous).
 *
 * **Usage Note**: For processing multiple files, prefer `batchExtractFilesSync()` which
 * provides better performance and memory management.
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
 * console.log(result.content);
 *
 * // With OCR configuration
 * const config = {
 *   ocr: {
 *     backend: 'tesseract',
 *     language: 'eng',
 *     tesseractConfig: {
 *       psm: 6,
 *       enableTableDetection: true,
 *     },
 *   },
 * };
 * const result2 = extractFileSync('scanned.pdf', null, config);
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
 * Extract content from a single file (asynchronous).
 *
 * **Usage Note**: For processing multiple files, prefer `batchExtractFiles()` which
 * provides better performance and memory management.
 *
 * @param filePath - Path to the file (string)
 * @param mimeType - Optional MIME type hint (auto-detected if null)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Promise<ExtractionResult> with content, metadata, and tables
 *
 * @example
 * ```typescript
 * import { extractFile } from 'kreuzberg';
 *
 * // Basic usage
 * const result = await extractFile('document.pdf');
 * console.log(result.content);
 *
 * // With chunking enabled
 * const config = {
 *   chunking: {
 *     maxChars: 1000,
 *     maxOverlap: 200,
 *   },
 * };
 * const result2 = await extractFile('long_document.pdf', null, config);
 * console.log(result2.chunks); // Array of text chunks
 * ```
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
 * Extract content from raw bytes (synchronous).
 *
 * **Usage Note**: For processing multiple byte arrays, prefer `batchExtractBytesSync()`
 * which provides better performance and memory management.
 *
 * @param data - File content as Uint8Array
 * @param mimeType - MIME type of the data (required for format detection)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns ExtractionResult with content, metadata, and tables
 *
 * @example
 * ```typescript
 * import { extractBytesSync } from 'kreuzberg';
 * import { readFileSync } from 'fs';
 *
 * const data = readFileSync('document.pdf');
 * const result = extractBytesSync(data, 'application/pdf');
 * console.log(result.content);
 * ```
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
 * Extract content from raw bytes (asynchronous).
 *
 * **Usage Note**: For processing multiple byte arrays, prefer `batchExtractBytes()`
 * which provides better performance and memory management.
 *
 * @param data - File content as Uint8Array
 * @param mimeType - MIME type of the data (required for format detection)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Promise<ExtractionResult> with content, metadata, and tables
 *
 * @example
 * ```typescript
 * import { extractBytes } from 'kreuzberg';
 * import { readFile } from 'fs/promises';
 *
 * const data = await readFile('document.pdf');
 * const result = await extractBytes(data, 'application/pdf');
 * console.log(result.content);
 * ```
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
 * **Recommended for**: Processing multiple documents efficiently with better
 * performance and memory management compared to individual `extractFileSync()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param paths - List of file paths to extract
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Array of ExtractionResults (one per file, in same order as input)
 *
 * @example
 * ```typescript
 * import { batchExtractFilesSync } from 'kreuzberg';
 *
 * const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 * const results = batchExtractFilesSync(files);
 *
 * results.forEach((result, i) => {
 *   console.log(`File ${files[i]}: ${result.content.substring(0, 100)}...`);
 * });
 * ```
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
 * **Recommended for**: Processing multiple documents efficiently with better
 * performance and memory management compared to individual `extractFile()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param paths - List of file paths to extract
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Promise resolving to array of ExtractionResults (one per file, in same order as input)
 *
 * @example
 * ```typescript
 * import { batchExtractFiles } from 'kreuzberg';
 *
 * const files = ['invoice1.pdf', 'invoice2.pdf', 'invoice3.pdf'];
 * const results = await batchExtractFiles(files, {
 *   ocr: { backend: 'tesseract', language: 'eng' }
 * });
 *
 * // Process all results
 * const totalAmount = results
 *   .map(r => extractAmount(r.content))
 *   .reduce((a, b) => a + b, 0);
 * ```
 */
export async function batchExtractFiles(
	paths: string[],
	config: ExtractionConfig | null = null,
): Promise<ExtractionResult[]> {
	const rawResults = await getBinding().batchExtractFiles(paths, config);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple byte arrays in parallel (synchronous).
 *
 * **Recommended for**: Processing multiple documents from memory efficiently with better
 * performance and memory management compared to individual `extractBytesSync()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param dataList - List of file contents as Uint8Arrays
 * @param mimeTypes - List of MIME types (one per data item, required for format detection)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Array of ExtractionResults (one per data item, in same order as input)
 *
 * @example
 * ```typescript
 * import { batchExtractBytesSync } from 'kreuzberg';
 * import { readFileSync } from 'fs';
 *
 * const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 * const dataList = files.map(f => readFileSync(f));
 * const mimeTypes = ['application/pdf', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'];
 *
 * const results = batchExtractBytesSync(dataList, mimeTypes);
 * results.forEach((result, i) => {
 *   console.log(`File ${files[i]}: ${result.content.substring(0, 100)}...`);
 * });
 * ```
 */
export function batchExtractBytesSync(
	dataList: Uint8Array[],
	mimeTypes: string[],
	config: ExtractionConfig | null = null,
): ExtractionResult[] {
	const buffers = dataList.map((data) => Buffer.from(data));
	const rawResults = getBinding().batchExtractBytesSync(
		buffers,
		mimeTypes,
		config,
	);
	return rawResults.map(convertResult);
}

/**
 * Extract content from multiple byte arrays in parallel (asynchronous).
 *
 * **Recommended for**: Processing multiple documents from memory efficiently with better
 * performance and memory management compared to individual `extractBytes()` calls.
 *
 * **Benefits**:
 * - Parallel processing in Rust for maximum performance
 * - Optimized memory usage across all extractions
 * - More reliable for batch document processing
 *
 * @param dataList - List of file contents as Uint8Arrays
 * @param mimeTypes - List of MIME types (one per data item, required for format detection)
 * @param config - Extraction configuration (uses defaults if null)
 * @returns Promise resolving to array of ExtractionResults (one per data item, in same order as input)
 *
 * @example
 * ```typescript
 * import { batchExtractBytes } from 'kreuzberg';
 * import { readFile } from 'fs/promises';
 *
 * const files = ['invoice1.pdf', 'invoice2.pdf', 'invoice3.pdf'];
 * const dataList = await Promise.all(files.map(f => readFile(f)));
 * const mimeTypes = files.map(() => 'application/pdf');
 *
 * const results = await batchExtractBytes(dataList, mimeTypes, {
 *   ocr: { backend: 'tesseract', language: 'eng' }
 * });
 *
 * // Process all results
 * const totalAmount = results
 *   .map(r => extractAmount(r.content))
 *   .reduce((a, b) => a + b, 0);
 * ```
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

/**
 * Register a custom postprocessor.
 *
 * **IMPORTANT**: Custom processors only work with **async extraction functions**:
 * - ✅ `extractFile()`, `extractBytes()`, `batchExtractFiles()`, `batchExtractBytes()`
 * - ❌ `extractFileSync()`, `extractBytesSync()`, etc. (will skip custom processors)
 *
 * This limitation exists because sync extraction blocks the Node.js event loop,
 * preventing JavaScript callbacks from executing. For v4.0, use async extraction
 * when you need custom processors.
 *
 * @param processor - PostProcessorProtocol implementation
 *
 * @example
 * ```typescript
 * import { registerPostProcessor, extractFile, ExtractionResult } from 'kreuzberg';
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
 *
 * // Use async extraction (required for custom processors)
 * const result = await extractFile('document.pdf');
 * console.log(result.metadata.customField); // 'custom_value'
 * ```
 */
export function registerPostProcessor(processor: PostProcessorProtocol): void {
	const binding = getBinding();

	// Wrap the processor to handle JSON serialization required by NAPI
	const wrappedProcessor = {
		name: processor.name.bind(processor),
		processingStage: processor.processingStage?.bind(processor),
		async process(...args: unknown[]): Promise<string> {
			// With build_callback returning vec![value], NAPI passes args as [null, value]
			// The first element is the error (null if no error), second is the value
			const jsonString = args[1] as string;
			const wireResult = JSON.parse(jsonString) as any;

			// Convert from snake_case (Rust) to camelCase (TypeScript) and parse metadata
			const result: ExtractionResult = {
				content: wireResult.content,
				mimeType: wireResult.mime_type,
				metadata:
					typeof wireResult.metadata === "string"
						? JSON.parse(wireResult.metadata)
						: wireResult.metadata,
				tables: wireResult.tables || [],
				detectedLanguages: wireResult.detected_languages,
				chunks: wireResult.chunks,
			};

			// Call user's processor with parsed object (may be sync or async)
			const updated = await processor.process(result);

			// Convert from camelCase (TypeScript) back to snake_case (Rust) and stringify metadata
			const wireUpdated = {
				content: updated.content,
				mime_type: updated.mimeType,
				metadata: JSON.stringify(updated.metadata),
				tables: updated.tables,
				detected_languages: updated.detectedLanguages,
				chunks: updated.chunks,
			};

			// Return as JSON string
			return JSON.stringify(wireUpdated);
		},
	};

	binding.registerPostProcessor(wrappedProcessor);
}

/**
 * Unregister a postprocessor by name.
 *
 * Removes a previously registered postprocessor from the registry.
 *
 * @param name - Name of the processor to unregister
 *
 * @example
 * ```typescript
 * import { unregisterPostProcessor } from 'kreuzberg';
 *
 * unregisterPostProcessor('my_processor');
 * ```
 */
export function unregisterPostProcessor(name: string): void {
	const binding = getBinding();
	binding.unregisterPostProcessor(name);
}

/**
 * Clear all registered postprocessors.
 *
 * Removes all postprocessors from the registry.
 *
 * @example
 * ```typescript
 * import { clearPostProcessors } from 'kreuzberg';
 *
 * clearPostProcessors();
 * ```
 */
export function clearPostProcessors(): void {
	const binding = getBinding();
	binding.clearPostProcessors();
}

/**
 * Register a custom validator.
 *
 * Validators check extraction results for quality, completeness, or correctness.
 * Unlike post-processors, validator errors **fail fast** - if a validator throws an error,
 * the extraction fails immediately.
 *
 * @param validator - ValidatorProtocol implementation
 *
 * @example
 * ```typescript
 * import { registerValidator } from 'kreuzberg';
 *
 * class MinLengthValidator implements ValidatorProtocol {
 *   name(): string {
 *     return 'min_length_validator';
 *   }
 *
 *   priority(): number {
 *     return 100; // Run early
 *   }
 *
 *   validate(result: ExtractionResult): void {
 *     if (result.content.length < 100) {
 *       throw new Error('Content too short: minimum 100 characters required');
 *     }
 *   }
 * }
 *
 * registerValidator(new MinLengthValidator());
 * ```
 */
export function registerValidator(validator: ValidatorProtocol): void {
	const binding = getBinding();

	const wrappedValidator = {
		name: validator.name.bind(validator),
		priority: validator.priority?.bind(validator),
		async validate(...args: string[]): Promise<string> {
			const jsonString = args[0];
			const wireResult = JSON.parse(jsonString);
			const result: ExtractionResult = {
				content: wireResult.content,
				mimeType: wireResult.mime_type,
				metadata:
					typeof wireResult.metadata === "string"
						? JSON.parse(wireResult.metadata)
						: wireResult.metadata,
				tables: wireResult.tables || [],
				detectedLanguages: wireResult.detected_languages,
				chunks: wireResult.chunks,
			};

			await Promise.resolve(validator.validate(result));
			return ""; // Return empty string on success
		},
	};

	binding.registerValidator(wrappedValidator);
}

/**
 * Unregister a validator by name.
 *
 * Removes a previously registered validator from the global registry.
 *
 * @param name - Validator name to unregister
 *
 * @example
 * ```typescript
 * import { unregisterValidator } from 'kreuzberg';
 *
 * unregisterValidator('min_length_validator');
 * ```
 */
export function unregisterValidator(name: string): void {
	const binding = getBinding();
	binding.unregisterValidator(name);
}

/**
 * Clear all registered validators.
 *
 * Removes all validators from the global registry. Useful for test cleanup
 * or resetting state.
 *
 * @example
 * ```typescript
 * import { clearValidators } from 'kreuzberg';
 *
 * clearValidators();
 * ```
 */
export function clearValidators(): void {
	const binding = getBinding();
	binding.clearValidators();
}

/**
 * Register a custom OCR backend.
 *
 * **Status**: Not yet implemented. JavaScript callback support for OCR backends
 * is planned for a future release.
 *
 * @param backend - OcrBackendProtocol implementation
 *
 * @example
 * ```typescript
 * import { registerOcrBackend } from 'kreuzberg';
 *
 * class MyOcrBackend implements OcrBackendProtocol {
 *   name(): string {
 *     return 'my_ocr';
 *   }
 *
 *   async extractText(imageBytes: Uint8Array, language: string): Promise<string> {
 *     // Custom OCR logic
 *     return 'extracted text';
 *   }
 * }
 *
 * registerOcrBackend(new MyOcrBackend());
 * ```
 */
export function registerOcrBackend(backend: OcrBackendProtocol): void {
	const binding = getBinding();
	binding.registerOcrBackend(backend);
}

export const __version__ = "4.0.0";
