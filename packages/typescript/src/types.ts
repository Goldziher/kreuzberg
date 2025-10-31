/**
 * Type definitions for Kreuzberg extraction results.
 *
 * These types mirror the strongly-typed Rust metadata structures,
 * providing type safety for TypeScript users.
 */

// ============================================================================
// ============================================================================

export interface TesseractConfig {
	psm?: number;
	enableTableDetection?: boolean;
	tesseditCharWhitelist?: string;
}

export interface OcrConfig {
	backend: string;
	language?: string;
	tesseractConfig?: TesseractConfig;
}

export interface ChunkingConfig {
	maxChars?: number;
	maxOverlap?: number;
}

export interface LanguageDetectionConfig {
	enabled?: boolean;
	minConfidence?: number;
	detectMultiple?: boolean;
}

export interface TokenReductionConfig {
	mode?: string;
	preserveImportantWords?: boolean;
}

export interface PdfConfig {
	extractImages?: boolean;
	passwords?: string[];
	extractMetadata?: boolean;
}

export interface ImageExtractionConfig {
	extractImages?: boolean;
	targetDpi?: number;
	maxImageDimension?: number;
	autoAdjustDpi?: boolean;
	minDpi?: number;
	maxDpi?: number;
}

export interface PostProcessorConfig {
	enabled?: boolean;
	enabledProcessors?: string[];
	disabledProcessors?: string[];
}

export interface ExtractionConfig {
	useCache?: boolean;
	enableQualityProcessing?: boolean;
	ocr?: OcrConfig;
	forceOcr?: boolean;
	chunking?: ChunkingConfig;
	images?: ImageExtractionConfig;
	pdfOptions?: PdfConfig;
	tokenReduction?: TokenReductionConfig;
	languageDetection?: LanguageDetectionConfig;
	postprocessor?: PostProcessorConfig;
	maxConcurrentExtractions?: number;
}

export interface Table {
	cells: string[][];
	markdown: string;
	pageNumber: number;
}

export interface ExcelMetadata {
	sheetCount?: number;
	sheetNames?: string[];
}

export interface EmailMetadata {
	fromEmail?: string | null;
	fromName?: string | null;
	toEmails?: string[];
	ccEmails?: string[];
	bccEmails?: string[];
	messageId?: string | null;
	attachments?: string[];
}

export interface ArchiveMetadata {
	format?: string;
	fileCount?: number;
	fileList?: string[];
	totalSize?: number;
	compressedSize?: number | null;
}

export interface ImageMetadata {
	width?: number;
	height?: number;
	format?: string;
	exif?: Record<string, string>;
}

export interface XmlMetadata {
	elementCount?: number;
	uniqueElements?: string[];
}

export interface TextMetadata {
	lineCount?: number;
	wordCount?: number;
	characterCount?: number;
	headers?: string[] | null;
	links?: [string, string][] | null;
	codeBlocks?: [string, string][] | null;
}

export interface HtmlMetadata {
	title?: string | null;
	description?: string | null;
	keywords?: string | null;
	author?: string | null;
	canonical?: string | null;
	baseHref?: string | null;
	ogTitle?: string | null;
	ogDescription?: string | null;
	ogImage?: string | null;
	ogUrl?: string | null;
	ogType?: string | null;
	ogSiteName?: string | null;
	twitterCard?: string | null;
	twitterTitle?: string | null;
	twitterDescription?: string | null;
	twitterImage?: string | null;
	twitterSite?: string | null;
	twitterCreator?: string | null;
	linkAuthor?: string | null;
	linkLicense?: string | null;
	linkAlternate?: string | null;
}

export interface PdfMetadata {
	title?: string | null;
	author?: string | null;
	subject?: string | null;
	keywords?: string | null;
	creator?: string | null;
	producer?: string | null;
	creationDate?: string | null;
	modificationDate?: string | null;
	pageCount?: number;
}

export interface PptxMetadata {
	title?: string | null;
	author?: string | null;
	description?: string | null;
	summary?: string | null;
	fonts?: string[];
}

export interface OcrMetadata {
	language?: string;
	psm?: number;
	outputFormat?: string;
	tableCount?: number;
	tableRows?: number | null;
	tableCols?: number | null;
}

export interface ImagePreprocessingMetadata {
	originalDimensions?: [number, number];
	originalDpi?: [number, number];
	targetDpi?: number;
	scaleFactor?: number;
	autoAdjusted?: boolean;
	finalDpi?: number;
	newDimensions?: [number, number] | null;
	resampleMethod?: string;
	dimensionClamped?: boolean;
	calculatedDpi?: number | null;
	skippedResize?: boolean;
	resizeError?: string | null;
}

export interface ErrorMetadata {
	errorType?: string;
	message?: string;
}

export interface Metadata {
	language?: string | null;
	date?: string | null;
	subject?: string | null;
	format?: string | null;

	pdf?: PdfMetadata | null;
	excel?: ExcelMetadata | null;
	email?: EmailMetadata | null;
	pptx?: PptxMetadata | null;
	archive?: ArchiveMetadata | null;
	image?: ImageMetadata | null;
	xml?: XmlMetadata | null;
	text?: TextMetadata | null;
	html?: HtmlMetadata | null;

	ocr?: OcrMetadata | null;
	imagePreprocessing?: ImagePreprocessingMetadata | null;

	// biome-ignore lint/suspicious/noExplicitAny: JSON schema can be any valid JSON structure
	jsonSchema?: any | null;

	error?: ErrorMetadata | null;

	// biome-ignore lint/suspicious/noExplicitAny: Postprocessors can add arbitrary metadata fields
	[key: string]: any;
}

export interface ExtractionResult {
	content: string;
	mimeType: string;
	metadata: Metadata;
	tables: Table[];
	detectedLanguages: string[] | null;
	chunks?: string[] | null;
}

export type ProcessingStage = "early" | "middle" | "late";

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
 * import { GutenOcrBackend } from '@goldziher/kreuzberg/ocr/guten-ocr';
 * import { registerOcrBackend, extractFile } from '@goldziher/kreuzberg';
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
	 * @param imageBytes - Raw image data (PNG, JPEG, TIFF, etc.)
	 * @param language - Language code from supportedLanguages()
	 * @returns Promise resolving to extraction result
	 *
	 * @example
	 * ```typescript
	 * async processImage(imageBytes: Uint8Array, language: string): Promise<{
	 *   content: string;
	 *   mime_type: string;
	 *   metadata: Record<string, unknown>;
	 *   tables: unknown[];
	 * }> {
	 *   const text = await myOcrEngine.recognize(imageBytes, language);
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
		imageBytes: Uint8Array,
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

export interface ValidatorProtocol {
	/**
	 * Return the unique name of this validator.
	 */
	name(): string;

	/**
	 * Validate an extraction result.
	 *
	 * Validators are fail-fast: if validation fails, throw an error with
	 * "ValidationError" in the message to stop the extraction process.
	 *
	 * @param result - ExtractionResult to validate
	 * @throws Error with "ValidationError" in message if validation fails
	 *
	 * @example
	 * ```typescript
	 * validate(result: ExtractionResult): void | Promise<void> {
	 *   if (result.content.length < 10) {
	 *     throw new Error("ValidationError: Content too short");
	 *   }
	 * }
	 * ```
	 */
	validate(result: ExtractionResult): void | Promise<void>;

	/**
	 * Return the priority for this validator (optional).
	 *
	 * Validators with higher priority run first.
	 * Default: 50
	 *
	 * @returns Priority number (higher runs first)
	 */
	priority?(): number;

	/**
	 * Determine if this validator should run for a given result (optional).
	 *
	 * @param result - ExtractionResult to check
	 * @returns true if validation should run, false to skip
	 */
	shouldValidate?(result: ExtractionResult): boolean;
}
