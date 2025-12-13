/**
 * Type definitions for Kreuzberg WASM bindings
 *
 * These types are generated from the Rust core library and define
 * the interface for extraction, configuration, and results.
 */

/**
 * Configuration for document extraction
 */
export interface ExtractionConfig {
	/** OCR configuration */
	ocr?: OcrConfig;
	/** Chunking configuration */
	chunking?: ChunkingConfig;
	/** Image extraction configuration */
	images?: ImageExtractionConfig;
	/** Page extraction configuration */
	pages?: PageExtractionConfig;
	/** Language detection configuration */
	languageDetection?: LanguageDetectionConfig;
}

/**
 * OCR configuration
 */
export interface OcrConfig {
	/** OCR backend to use */
	backend?: string;
	/** Language codes (ISO 639) */
	languages?: string[];
	/** Whether to perform OCR */
	enabled?: boolean;
}

/**
 * Chunking configuration
 */
export interface ChunkingConfig {
	/** Maximum characters per chunk */
	maxChars?: number;
	/** Overlap between chunks */
	maxOverlap?: number;
}

/**
 * Image extraction configuration
 */
export interface ImageExtractionConfig {
	/** Whether to extract images */
	enabled?: boolean;
}

/**
 * Page extraction configuration
 */
export interface PageExtractionConfig {
	/** Whether to extract per-page content */
	enabled?: boolean;
}

/**
 * Language detection configuration
 */
export interface LanguageDetectionConfig {
	/** Whether to detect languages */
	enabled?: boolean;
}

/**
 * Result of document extraction
 */
export interface ExtractionResult {
	/** Extracted text content */
	content: string;
	/** MIME type of the document */
	mimeType: string;
	/** Document metadata */
	metadata: Metadata;
	/** Extracted tables */
	tables: Table[];
	/** Detected languages (ISO 639 codes) */
	detectedLanguages?: string[] | null;
	/** Text chunks when chunking is enabled */
	chunks?: Chunk[] | null;
	/** Extracted images */
	images?: ExtractedImage[] | null;
	/** Per-page content */
	pages?: PageContent[] | null;
}

/**
 * Document metadata
 */
export interface Metadata {
	/** Document title */
	title?: string;
	/** Document subject or description */
	subject?: string;
	/** Document author(s) */
	authors?: string[];
	/** Keywords/tags */
	keywords?: string[];
	/** Primary language (ISO 639 code) */
	language?: string;
	/** Creation timestamp (ISO 8601 format) */
	createdAt?: string;
	/** Last modification timestamp (ISO 8601 format) */
	modifiedAt?: string;
	/** User who created the document */
	creator?: string;
	/** User who last modified the document */
	lastModifiedBy?: string;
	/** Number of pages/slides */
	pageCount?: number;
	/** Format-specific metadata */
	formatMetadata?: unknown;
	/** Custom additional fields */
	additional?: Record<string, unknown>;
}

/**
 * Extracted table
 */
export interface Table {
	/** Table cells/rows */
	cells?: string[][];
	/** Table markdown representation */
	markdown?: string;
	/** Page number if available */
	pageNumber?: number;
	/** Table headers */
	headers?: string[];
	/** Table rows */
	rows?: string[][];
}

/**
 * Chunk metadata
 */
export interface ChunkMetadata {
	/** Character start position in original content */
	charStart: number;
	/** Character end position in original content */
	charEnd: number;
	/** Token count if available */
	tokenCount: number | null;
	/** Index of this chunk */
	chunkIndex: number;
	/** Total number of chunks */
	totalChunks: number;
}

/**
 * Text chunk from chunked content
 */
export interface Chunk {
	/** Chunk text content */
	content: string;
	/** Chunk metadata */
	metadata?: ChunkMetadata;
	/** Character position in original content (legacy) */
	charIndex?: number;
	/** Token count if available (legacy) */
	tokenCount?: number;
	/** Embedding vector if computed */
	embedding?: number[] | null;
}

/**
 * Extracted image from document
 */
export interface ExtractedImage {
	/** Image data as Uint8Array or base64 string */
	data: Uint8Array | string;
	/** Image format/MIME type */
	format?: string;
	/** MIME type of the image */
	mimeType?: string;
	/** Image index in document */
	imageIndex?: number;
	/** Page number if available */
	pageNumber?: number | null;
	/** Image width in pixels */
	width?: number | null;
	/** Image height in pixels */
	height?: number | null;
	/** Color space of the image */
	colorspace?: string | null;
	/** Bits per color component */
	bitsPerComponent?: number | null;
	/** Whether this is a mask image */
	isMask?: boolean;
	/** Image description */
	description?: string | null;
	/** Optional OCR result from the image */
	ocrResult?: ExtractionResult | string | null;
}

/**
 * Per-page content
 */
export interface PageContent {
	/** Page number (1-indexed) */
	pageNumber: number;
	/** Text content of the page */
	content: string;
	/** Tables on this page */
	tables?: Table[];
	/** Images on this page */
	images?: ExtractedImage[];
}

/**
 * OCR backend protocol/interface
 */
export interface OcrBackendProtocol {
	/** Get the backend name */
	name(): string;
	/** Get supported language codes */
	supportedLanguages?(): string[];
	/** Initialize the backend */
	initialize(options?: Record<string, unknown>): void | Promise<void>;
	/** Shutdown the backend */
	shutdown?(): void | Promise<void>;
	/** Process an image with OCR */
	processImage(
		imageData: Uint8Array | string,
		language?: string,
	): Promise<
		| {
				content: string;
				mime_type: string;
				metadata?: Record<string, unknown>;
				tables?: unknown[];
		  }
		| string
	>;
}
