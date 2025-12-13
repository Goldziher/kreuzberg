/**
 * Configuration interfaces for Kreuzberg extraction options.
 *
 * These types define all configurable parameters for document extraction,
 * including OCR, chunking, image processing, and post-processing options.
 */

// ============================================================================
// OCR Configuration
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

// ============================================================================
// Chunking Configuration
// ============================================================================

export interface ChunkingConfig {
	maxChars?: number;
	maxOverlap?: number;
	chunkSize?: number;
	chunkOverlap?: number;
	preset?: string;
	embedding?: Record<string, unknown>;
	enabled?: boolean;
}

// ============================================================================
// Language and Token Configuration
// ============================================================================

export interface LanguageDetectionConfig {
	enabled?: boolean;
	minConfidence?: number;
	detectMultiple?: boolean;
}

export interface TokenReductionConfig {
	mode?: string;
	preserveImportantWords?: boolean;
}

// ============================================================================
// Document Format Configuration
// ============================================================================

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

// ============================================================================
// Post-Processing Configuration
// ============================================================================

export interface PostProcessorConfig {
	enabled?: boolean;
	enabledProcessors?: string[];
	disabledProcessors?: string[];
}

// ============================================================================
// HTML Processing Configuration
// ============================================================================

export interface HtmlPreprocessingOptions {
	enabled?: boolean;
	preset?: "minimal" | "standard" | "aggressive";
	removeNavigation?: boolean;
	removeForms?: boolean;
}

export interface HtmlConversionOptions {
	headingStyle?: "atx" | "underlined" | "atx_closed";
	listIndentType?: "spaces" | "tabs";
	listIndentWidth?: number;
	bullets?: string;
	strongEmSymbol?: string;
	escapeAsterisks?: boolean;
	escapeUnderscores?: boolean;
	escapeMisc?: boolean;
	escapeAscii?: boolean;
	codeLanguage?: string;
	autolinks?: boolean;
	defaultTitle?: boolean;
	brInTables?: boolean;
	hocrSpatialTables?: boolean;
	highlightStyle?: "double_equal" | "html" | "bold" | "none";
	extractMetadata?: boolean;
	whitespaceMode?: "normalized" | "strict";
	stripNewlines?: boolean;
	wrap?: boolean;
	wrapWidth?: number;
	convertAsInline?: boolean;
	subSymbol?: string;
	supSymbol?: string;
	newlineStyle?: "spaces" | "backslash";
	codeBlockStyle?: "indented" | "backticks" | "tildes";
	keepInlineImagesIn?: string[];
	encoding?: string;
	debug?: boolean;
	stripTags?: string[];
	preserveTags?: string[];
	preprocessing?: HtmlPreprocessingOptions;
}

// ============================================================================
// Keyword Extraction Configuration
// ============================================================================

export type KeywordAlgorithm = "yake" | "rake";

export interface YakeParams {
	windowSize?: number;
}

export interface RakeParams {
	minWordLength?: number;
	maxWordsPerPhrase?: number;
}

export interface KeywordConfig {
	algorithm?: KeywordAlgorithm;
	maxKeywords?: number;
	minScore?: number;
	ngramRange?: [number, number];
	language?: string;
	yakeParams?: YakeParams;
	rakeParams?: RakeParams;
}

// ============================================================================
// Main Extraction Configuration
// ============================================================================

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
	htmlOptions?: HtmlConversionOptions;
	keywords?: KeywordConfig;
	maxConcurrentExtractions?: number;
}
