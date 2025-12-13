/**
 * Kreuzberg Core Types Package
 *
 * This package exports all shared type definitions used by Kreuzberg bindings
 * (Node.js, WASM, etc.). It provides:
 *
 * - Configuration types for extraction options
 * - Result types for extraction outputs
 * - Metadata types for document information
 * - Protocol interfaces for custom plugins
 * - Error types and error codes
 */

// Re-export configuration types
export type {
	ChunkingConfig,
	ExtractionConfig,
	HtmlConversionOptions,
	HtmlPreprocessingOptions,
	ImageExtractionConfig,
	KeywordAlgorithm,
	KeywordConfig,
	LanguageDetectionConfig,
	OcrConfig,
	PdfConfig,
	PostProcessorConfig,
	RakeParams,
	TesseractConfig,
	TokenReductionConfig,
	YakeParams,
} from "./config.js";

// Re-export result types
export type {
	Chunk,
	ChunkMetadata,
	ExtractionResult,
	ExtractedImage,
	Table,
} from "./results.js";

// Re-export metadata types
export type {
	ArchiveMetadata,
	EmailMetadata,
	ErrorMetadata,
	ExcelMetadata,
	HtmlMetadata,
	ImageMetadata,
	ImagePreprocessingMetadata,
	Metadata,
	OcrMetadata,
	PdfMetadata,
	PptxMetadata,
	TextMetadata,
	XmlMetadata,
} from "./metadata.js";

// Re-export protocol types
export type {
	OcrBackendProtocol,
	PostProcessorProtocol,
	ValidatorProtocol,
} from "./protocols.js";

export type { ProcessingStage } from "./protocols.js";

// Re-export error types
export {
	CacheError,
	ErrorCode,
	ImageProcessingError,
	KreuzbergError,
	MissingDependencyError,
	OcrError,
	type PanicContext,
	ParsingError,
	PluginError,
	ValidationError,
} from "./errors.js";
