/**
 * Metadata type definitions for extracted documents.
 *
 * Includes format-specific metadata for various document types
 * (PDF, Excel, Email, Images, etc.) and the unified Metadata interface.
 */

// ============================================================================
// Format-Specific Metadata Interfaces
// ============================================================================

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

// ============================================================================
// Image Preprocessing Metadata
// ============================================================================

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

// ============================================================================
// Error Metadata
// ============================================================================

export interface ErrorMetadata {
	errorType?: string;
	message?: string;
}

// ============================================================================
// Unified Metadata Interface
// ============================================================================

/**
 * Extraction result metadata.
 *
 * Uses a flattened discriminated union approach with format_type as the discriminator.
 * When format_type is set (e.g., "archive"), the corresponding format-specific fields
 * are available at the root level of the metadata object.
 *
 * This structure matches the Rust serialization with serde's tagged enum flattening.
 */
export interface Metadata {
	language?: string | null;
	date?: string | null;
	subject?: string | null;

	format_type?: "pdf" | "excel" | "email" | "pptx" | "archive" | "image" | "xml" | "text" | "html" | "ocr";

	// Common PDF/Document metadata
	title?: string | null;
	author?: string | null;
	keywords?: string | null;
	creator?: string | null;
	producer?: string | null;
	creation_date?: string | null;
	modification_date?: string | null;
	page_count?: number;

	// Excel-specific metadata
	sheet_count?: number;
	sheet_names?: string[];

	// Email-specific metadata
	from_email?: string | null;
	from_name?: string | null;
	to_emails?: string[];
	cc_emails?: string[];
	bcc_emails?: string[];
	message_id?: string | null;
	attachments?: string[];

	// PowerPoint-specific metadata
	description?: string | null;
	summary?: string | null;
	fonts?: string[];

	// Archive-specific metadata
	format?: string;
	file_count?: number;
	file_list?: string[];
	total_size?: number;
	compressed_size?: number | null;

	// Image-specific metadata
	width?: number;
	height?: number;
	exif?: Record<string, string>;

	// XML-specific metadata
	element_count?: number;
	unique_elements?: string[];

	// Text-specific metadata
	line_count?: number;
	word_count?: number;
	character_count?: number;
	headers?: string[] | null;
	links?: [string, string][] | null;
	code_blocks?: [string, string][] | null;

	// HTML-specific metadata
	canonical?: string | null;
	base_href?: string | null;
	og_title?: string | null;
	og_description?: string | null;
	og_image?: string | null;
	og_url?: string | null;
	og_type?: string | null;
	og_site_name?: string | null;
	twitter_card?: string | null;
	twitter_title?: string | null;
	twitter_description?: string | null;
	twitter_image?: string | null;
	twitter_site?: string | null;
	twitter_creator?: string | null;
	link_author?: string | null;
	link_license?: string | null;
	link_alternate?: string | null;

	// OCR-specific metadata
	psm?: number;
	output_format?: string;
	table_count?: number;
	table_rows?: number | null;
	table_cols?: number | null;

	// Image preprocessing metadata
	image_preprocessing?: ImagePreprocessingMetadata | null;

	// JSON schema
	json_schema?: Record<string, unknown> | null;

	// Error information
	error?: ErrorMetadata | null;

	// biome-ignore lint/suspicious/noExplicitAny: Postprocessors can add arbitrary metadata fields
	[key: string]: any;
}
