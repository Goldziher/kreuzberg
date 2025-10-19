/**
 * Type definitions for Kreuzberg extraction results.
 *
 * These types mirror the strongly-typed Rust metadata structures,
 * providing type safety for TypeScript users.
 */

// ============================================================================
// Configuration Types
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

// ============================================================================
// Result Types
// ============================================================================

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
	// Common fields
	language?: string | null;
	date?: string | null;
	subject?: string | null;
	format?: string | null;

	// Format-specific metadata
	pdf?: PdfMetadata | null;
	excel?: ExcelMetadata | null;
	email?: EmailMetadata | null;
	pptx?: PptxMetadata | null;
	archive?: ArchiveMetadata | null;
	image?: ImageMetadata | null;
	xml?: XmlMetadata | null;
	text?: TextMetadata | null;

	// Processing metadata
	ocr?: OcrMetadata | null;
	imagePreprocessing?: ImagePreprocessingMetadata | null;

	// Structured data
	// biome-ignore lint/suspicious/noExplicitAny: JSON schema can be any valid JSON structure
	jsonSchema?: any | null;

	// Error metadata
	error?: ErrorMetadata | null;

	// Custom fields from postprocessors
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

// ============================================================================
// PostProcessor Protocol
// ============================================================================

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
	process(
		result: ExtractionResult,
	): ExtractionResult | Promise<ExtractionResult>;

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
// OCR Backend Protocol
// ============================================================================

export interface OcrBackendProtocol {
	/**
	 * Return the unique name of this OCR backend.
	 */
	name(): string;

	/**
	 * Perform OCR on image bytes.
	 *
	 * @param imageBytes - Image data as Uint8Array
	 * @param language - Language code (e.g., "eng", "deu")
	 * @returns Extracted text
	 */
	extractText(
		imageBytes: Uint8Array,
		language: string,
	): string | Promise<string>;
}
