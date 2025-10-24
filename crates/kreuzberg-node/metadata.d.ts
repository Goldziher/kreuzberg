/**
 * Metadata type definitions for Kreuzberg extraction results.
 *
 * These types mirror the Rust metadata structures and are referenced by
 * the auto-generated index.d.ts file.
 */

export interface PdfMetadata {
	title?: string;
	author?: string;
	subject?: string;
	keywords?: string;
	creator?: string;
	producer?: string;
	creationDate?: string;
	modificationDate?: string;
	pageCount?: number;
}

export interface ExcelMetadata {
	sheetCount: number;
	sheetNames: string[];
}

export interface EmailMetadata {
	fromEmail?: string;
	fromName?: string;
	toEmails: string[];
	ccEmails: string[];
	bccEmails: string[];
	messageId?: string;
	attachments: string[];
}

export interface PptxMetadata {
	title?: string;
	author?: string;
	description?: string;
	summary?: string;
	fonts: string[];
}

export interface ArchiveMetadata {
	format: string;
	fileCount: number;
	fileList: string[];
	totalSize: number;
	compressedSize?: number;
}

export interface ImageMetadata {
	width: number;
	height: number;
	format: string;
	exif: Record<string, string>;
}

export interface XmlMetadata {
	elementCount: number;
	uniqueElements: string[];
}

export interface TextMetadata {
	lineCount: number;
	wordCount: number;
	characterCount: number;
	headers?: string[];
	links?: Array<[string, string]>;
	codeBlocks?: Array<[string, string]>;
}

export interface HtmlMetadata {
	title?: string;
	description?: string;
	keywords?: string;
	author?: string;
	canonical?: string;
	baseHref?: string;
	ogTitle?: string;
	ogDescription?: string;
	ogImage?: string;
	ogUrl?: string;
	ogType?: string;
	ogSiteName?: string;
	twitterCard?: string;
	twitterTitle?: string;
	twitterDescription?: string;
	twitterImage?: string;
	twitterSite?: string;
	twitterCreator?: string;
	linkAuthor?: string;
	linkLicense?: string;
	linkAlternate?: string;
}

export interface OcrMetadata {
	language: string;
	psm: number;
	outputFormat: string;
	tableCount: number;
	tableRows?: number;
	tableCols?: number;
}

export interface ImagePreprocessingMetadata {
	originalDimensions: [number, number];
	originalDpi: [number, number];
	targetDpi: number;
	scaleFactor: number;
	autoAdjusted: boolean;
	finalDpi: number;
	newDimensions?: [number, number];
	resampleMethod: string;
	dimensionClamped: boolean;
	calculatedDpi?: number;
	skippedResize: boolean;
	resizeError?: string;
}

export interface ErrorMetadata {
	errorType: string;
	message: string;
}

export interface Metadata {
	language?: string;
	date?: string;
	subject?: string;
	format?: string;
	pdf?: PdfMetadata;
	excel?: ExcelMetadata;
	email?: EmailMetadata;
	pptx?: PptxMetadata;
	archive?: ArchiveMetadata;
	image?: ImageMetadata;
	xml?: XmlMetadata;
	text?: TextMetadata;
	html?: HtmlMetadata;
	ocr?: OcrMetadata;
	imagePreprocessing?: ImagePreprocessingMetadata;
	jsonSchema?: any;
	error?: ErrorMetadata;
	[key: string]: any;
}
