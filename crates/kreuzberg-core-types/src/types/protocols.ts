/**
 * Plugin protocol interfaces for extending Kreuzberg functionality.
 *
 * These interfaces define the contracts for custom plugins:
 * - PostProcessorProtocol: Enriches extraction results
 * - ValidatorProtocol: Validates extraction results
 * - OcrBackendProtocol: Implements custom OCR engines
 */

import type { ExtractionResult } from "./results.js";

// ============================================================================
// Processing Stage Type
// ============================================================================

export type ProcessingStage = "early" | "middle" | "late";

// ============================================================================
// Post-Processor Protocol
// ============================================================================

export interface PostProcessorProtocol {
	/**
	 * Return the unique name of this postprocessor.
	 */
	name(): string;

	/**
	 * Process and enrich an extraction result.
	 *
	 * @param result - ExtractionResult with extracted content, metadata, and tables
	 * @returns Modified result with enriched metadata
	 */
	process(result: ExtractionResult): ExtractionResult | Promise<ExtractionResult>;

	/**
	 * Return the processing stage for this processor.
	 *
	 * @returns One of "early", "middle", or "late" (default: "middle")
	 */
	processingStage?(): ProcessingStage;

	/**
	 * Initialize the processor (e.g., load ML models).
	 *
	 * Called once when the processor is registered.
	 */
	initialize?(): void | Promise<void>;

	/**
	 * Shutdown the processor and release resources.
	 *
	 * Called when the processor is unregistered.
	 */
	shutdown?(): void | Promise<void>;
}

// ============================================================================
// Validator Protocol
// ============================================================================

export interface ValidatorProtocol {
	/**
	 * Return the unique name of this validator.
	 */
	name(): string;

	/**
	 * Validate an extraction result.
	 *
	 * Throw an error if validation fails. The error message should explain why validation failed.
	 * If validation passes, return without throwing.
	 *
	 * @param result - ExtractionResult to validate
	 * @throws Error if validation fails (extraction will fail)
	 */
	validate(result: ExtractionResult): void | Promise<void>;

	/**
	 * Return the validation priority.
	 *
	 * Higher priority validators run first. Useful for running cheap validations before expensive ones.
	 *
	 * @returns Priority value (higher = runs earlier, default: 50)
	 */
	priority?(): number;

	/**
	 * Check if this validator should run for a given result.
	 *
	 * Allows conditional validation based on MIME type, metadata, or content.
	 *
	 * @param result - ExtractionResult to check
	 * @returns true if validator should run, false to skip (default: true)
	 */
	shouldValidate?(result: ExtractionResult): boolean;

	/**
	 * Initialize the validator.
	 *
	 * Called once when the validator is registered.
	 */
	initialize?(): void | Promise<void>;

	/**
	 * Shutdown the validator and release resources.
	 *
	 * Called when the validator is unregistered.
	 */
	shutdown?(): void | Promise<void>;
}

// ============================================================================
// OCR Backend Protocol
// ============================================================================

/**
 * OCR backend protocol for implementing custom OCR engines.
 *
 * This interface defines the contract for OCR backends that can be registered
 * with Kreuzberg's extraction pipeline.
 *
 * ## Implementation Requirements
 *
 * OCR backends must implement:
 * - `name()`: Return a unique backend identifier
 * - `supportedLanguages()`: Return list of supported ISO 639-1/2/3 language codes
 * - `processImage()`: Process image bytes and return extraction result
 *
 * ## Optional Methods
 *
 * - `initialize()`: Called when backend is registered (load models, etc.)
 * - `shutdown()`: Called when backend is unregistered (cleanup resources)
 *
 * @example
 * ```typescript
 * import { GutenOcrBackend } from '@kreuzberg/node/ocr/guten-ocr';
 * import { registerOcrBackend, extractFile } from '@kreuzberg/node';
 *
 * // Create and register the backend
 * const backend = new GutenOcrBackend();
 * await backend.initialize();
 * registerOcrBackend(backend);
 *
 * // Use with extraction
 * const result = await extractFile('scanned.pdf', null, {
 *   ocr: { backend: 'guten-ocr', language: 'en' }
 * });
 * ```
 */
export interface OcrBackendProtocol {
	/**
	 * Return the unique name of this OCR backend.
	 *
	 * This name is used in ExtractionConfig to select the backend:
	 * ```typescript
	 * { ocr: { backend: 'guten-ocr', language: 'en' } }
	 * ```
	 *
	 * @returns Unique backend identifier (e.g., "guten-ocr", "tesseract")
	 */
	name(): string;

	/**
	 * Return list of supported language codes.
	 *
	 * Language codes should follow ISO 639-1 (2-letter) or ISO 639-2 (3-letter) standards.
	 * Common codes: "en", "eng" (English), "de", "deu" (German), "fr", "fra" (French).
	 *
	 * @returns Array of supported language codes
	 *
	 * @example
	 * ```typescript
	 * supportedLanguages(): string[] {
	 *   return ["en", "eng", "de", "deu", "fr", "fra"];
	 * }
	 * ```
	 */
	supportedLanguages(): string[];

	/**
	 * Process image bytes and extract text via OCR.
	 *
	 * This method receives raw image data and must return a result object with:
	 * - `content`: Extracted text content
	 * - `mime_type`: MIME type (usually "text/plain")
	 * - `metadata`: Additional information (confidence, dimensions, etc.)
	 * - `tables`: Optional array of detected tables
	 *
	 * @param imageBytes - Raw image data (Uint8Array) or Base64-encoded string (when called from Rust bindings)
	 * @param language - Language code from supportedLanguages()
	 * @returns Promise resolving to extraction result
	 *
	 * @example
	 * ```typescript
	 * async processImage(imageBytes: Uint8Array | string, language: string): Promise<{
	 *   content: string;
	 *   mime_type: string;
	 *   metadata: Record<string, unknown>;
	 *   tables: unknown[];
	 * }> {
	 *   const buffer = typeof imageBytes === "string" ? Buffer.from(imageBytes, "base64") : Buffer.from(imageBytes);
	 *   const text = await myOcrEngine.recognize(buffer, language);
	 *   return {
	 *     content: text,
	 *     mime_type: "text/plain",
	 *     metadata: { confidence: 0.95, language },
	 *     tables: []
	 *   };
	 * }
	 * ```
	 */
	processImage(
		imageBytes: Uint8Array | string,
		language: string,
	): Promise<{
		content: string;
		mime_type: string;
		metadata: Record<string, unknown>;
		tables: unknown[];
	}>;

	/**
	 * Initialize the OCR backend (optional).
	 *
	 * Called once when the backend is registered. Use this to:
	 * - Load ML models
	 * - Initialize libraries
	 * - Validate dependencies
	 *
	 * @example
	 * ```typescript
	 * async initialize(): Promise<void> {
	 *   this.model = await loadModel('./path/to/model');
	 * }
	 * ```
	 */
	initialize?(): void | Promise<void>;

	/**
	 * Shutdown the OCR backend and release resources (optional).
	 *
	 * Called when the backend is unregistered. Use this to:
	 * - Free model memory
	 * - Close file handles
	 * - Cleanup temporary files
	 *
	 * @example
	 * ```typescript
	 * async shutdown(): Promise<void> {
	 *   await this.model.dispose();
	 *   this.model = null;
	 * }
	 * ```
	 */
	shutdown?(): void | Promise<void>;
}
