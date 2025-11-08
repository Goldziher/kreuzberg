/**
 * Metadata type definitions for Kreuzberg extraction results.
 *
 * These types mirror the Rust metadata structures and are referenced by
 * the auto-generated index.d.ts file.
 */

// ===== Enums =====

/**
 * Page Segmentation Mode for Tesseract OCR.
 * Maps to Rust's PSMMode enum.
 */
export enum PSMMode {
	/** Orientation and script detection only */
	OsdOnly = 0,
	/** Automatic page segmentation with OSD */
	AutoOsd = 1,
	/** Automatic page segmentation, but no OSD, or OCR */
	AutoOnly = 2,
	/** Fully automatic page segmentation, but no OSD (default) */
	Auto = 3,
	/** Assume a single column of text of variable sizes */
	SingleColumn = 4,
	/** Assume a single uniform block of vertically aligned text */
	SingleBlockVertical = 5,
	/** Assume a single uniform block of text */
	SingleBlock = 6,
	/** Treat the image as a single text line */
	SingleLine = 7,
	/** Treat the image as a single word */
	SingleWord = 8,
	/** Treat the image as a single word in a circle */
	CircleWord = 9,
	/** Treat the image as a single character */
	SingleChar = 10,
}

/**
 * Output format for OCR extraction.
 */
export type OcrOutputFormat = "text" | "markdown" | "hocr" | "tsv";

/**
 * Token reduction level.
 */
export type TokenReductionLevel = "off" | "light" | "moderate" | "aggressive" | "maximum";

/**
 * Image binarization method for preprocessing.
 */
export type BinarizationMethod = "otsu" | "adaptive" | "sauvola";

/**
 * Processing stage for postprocessors.
 */
export type ProcessingStage = "early" | "middle" | "late";

/**
 * Common EXIF metadata fields from images.
 * Based on standard EXIF 2.3 specification.
 */
export interface ExifMetadata {
	/** Image width in pixels */
	ImageWidth?: number;
	/** Image height in pixels */
	ImageHeight?: number;
	/** Camera make/manufacturer */
	Make?: string;
	/** Camera model */
	Model?: string;
	/** Image orientation (1-8) */
	Orientation?: number;
	/** Horizontal resolution (DPI) */
	XResolution?: number;
	/** Vertical resolution (DPI) */
	YResolution?: number;
	/** Resolution unit (2=inches, 3=cm) */
	ResolutionUnit?: number;
	/** Software used to create/edit image */
	Software?: string;
	/** Date and time of image creation */
	DateTime?: string;
	/** Date and time original image was taken */
	DateTimeOriginal?: string;
	/** Date and time image was digitized */
	DateTimeDigitized?: string;
	/** Copyright information */
	Copyright?: string;
	/** Image description */
	ImageDescription?: string;
	/** Artist/creator */
	Artist?: string;
	/** Color space (1=sRGB, 2=Adobe RGB) */
	ColorSpace?: number;
	/** Bits per sample */
	BitsPerSample?: number;
	/** Compression type */
	Compression?: number;
	/** Photometric interpretation */
	PhotometricInterpretation?: number;
	/** GPS latitude */
	GPSLatitude?: string;
	/** GPS longitude */
	GPSLongitude?: string;
	/** GPS altitude */
	GPSAltitude?: string;
	/** Exposure time (seconds) */
	ExposureTime?: string;
	/** F-number */
	FNumber?: string;
	/** ISO speed rating */
	ISOSpeedRatings?: number;
	/** Focal length (mm) */
	FocalLength?: string;
	/** Flash mode */
	Flash?: number;
	/** White balance mode */
	WhiteBalance?: number;
	/** Metering mode */
	MeteringMode?: number;
	/** Exposure program */
	ExposureProgram?: number;
}

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
	pdfVersion?: string;
	isEncrypted?: boolean;
	width?: number;
	height?: number;
	summary?: string;
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
	exif: ExifMetadata;
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
	psm: PSMMode;
	outputFormat: OcrOutputFormat;
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

/**
 * Discriminated union type for format-specific metadata.
 * The `formatType` field indicates which format-specific fields are present.
 * All format-specific fields are flattened into the metadata object.
 *
 * Example for PDF:
 * ```typescript
 * {
 *   "language": "en",
 *   "formatType": "pdf",
 *   "title": "My Document",
 *   "pageCount": 5
 * }
 * ```
 *
 * Example for Excel:
 * ```typescript
 * {
 *   "formatType": "excel",
 *   "sheetCount": 3,
 *   "sheetNames": ["Sheet1", "Sheet2", "Sheet3"]
 * }
 * ```
 */
export type FormatType = "pdf" | "excel" | "email" | "pptx" | "archive" | "image" | "xml" | "text" | "html" | "ocr";

/**
 * Base metadata interface with common fields.
 * Use the generic `Metadata<T>` type to extend with custom fields.
 */
export interface BaseMetadata {
	language?: string;
	date?: string;
	subject?: string;

	formatType?: FormatType;

	title?: string;
	author?: string;
	keywords?: string;
	creator?: string;
	producer?: string;
	creationDate?: string;
	modificationDate?: string;
	pageCount?: number;
	pdfVersion?: string;
	isEncrypted?: boolean;
	width?: number;
	height?: number;
	summary?: string;

	sheetCount?: number;
	sheetNames?: string[];

	fromEmail?: string;
	fromName?: string;
	toEmails?: string[];
	ccEmails?: string[];
	bccEmails?: string[];
	messageId?: string;
	attachments?: string[];

	description?: string;
	fonts?: string[];

	format?: string;
	fileCount?: number;
	fileList?: string[];
	totalSize?: number;
	compressedSize?: number;

	exif?: ExifMetadata;

	elementCount?: number;
	uniqueElements?: string[];

	lineCount?: number;
	wordCount?: number;
	characterCount?: number;
	headers?: string[];
	links?: Array<[string, string]>;
	codeBlocks?: Array<[string, string]>;

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

	psm?: PSMMode;
	outputFormat?: OcrOutputFormat;
	tableCount?: number;
	tableRows?: number;
	tableCols?: number;

	imagePreprocessing?: ImagePreprocessingMetadata;
	jsonSchema?: Record<string, unknown>;
	error?: ErrorMetadata;
}

/**
 * Generic metadata type that allows extension with custom fields.
 *
 * Example usage:
 * ```typescript
 * // Extend with custom fields
 * interface MyCustomMetadata {
 *   customField1: string;
 *   customField2: number;
 * }
 *
 * type ExtendedMetadata = Metadata<MyCustomMetadata>;
 *
 * const metadata: ExtendedMetadata = {
 *   formatType: 'pdf',
 *   pageCount: 5,
 *   customField1: 'value',
 *   customField2: 42
 * };
 * ```
 */
export type Metadata<T extends Record<string, unknown> = Record<string, never>> = BaseMetadata & T;

/**
 * Extracted image with metadata and optional nested OCR result.
 */
export interface ExtractedImage {
	/** Raw image bytes */
	data: Buffer;
	/** Image format (e.g., "jpeg", "png", "tiff") */
	format: string;
	/** Zero-based image index within the document */
	imageIndex: number;
	/** Page number where image was found (1-indexed) */
	pageNumber?: number;
	/** Image width in pixels */
	width?: number;
	/** Image height in pixels */
	height?: number;
	/** Colorspace (e.g., "DeviceRGB", "DeviceGray") */
	colorspace?: string;
	/** Bits per color component */
	bitsPerComponent?: number;
	/** Whether this is a mask image */
	isMask: boolean;
	/** Optional description/alt text */
	description?: string;
	/** Nested OCR result if image was processed with OCR */
	ocrResult?: ExtractionResult;
}

/**
 * Generic extraction result type that allows extension with custom metadata.
 *
 * Example usage:
 * ```typescript
 * interface MyMetadata {
 *   processingTime: number;
 *   confidence: number;
 * }
 *
 * type MyResult = ExtractionResult<MyMetadata>;
 * ```
 */
export interface ExtractionResult<T extends Record<string, unknown> = Record<string, never>> {
	content: string;
	mimeType: string;
	metadata: Metadata<T>;
	tables: ExtractedTable[];
	detectedLanguages?: string[];
	chunks?: string[];
	images?: ExtractedImage[];
}

/**
 * Extracted table structure.
 */
export interface ExtractedTable {
	/** 2D array of cell values */
	cells: string[][];
	/** Markdown representation of the table */
	markdown: string;
	/** Page number where table was found (1-indexed) */
	pageNumber: number;
}

/**
 * Type guard to check if metadata is for a specific format type.
 */
export declare function isFormatType<T extends FormatType>(
	metadata: BaseMetadata,
	formatType: T,
): metadata is BaseMetadata & { formatType: T };

/**
 * Type guard to check if metadata contains PDF-specific fields.
 */
export declare function isPdfMetadata(metadata: BaseMetadata): metadata is BaseMetadata & PdfMetadata;

/**
 * Type guard to check if metadata contains Excel-specific fields.
 */
export declare function isExcelMetadata(metadata: BaseMetadata): metadata is BaseMetadata & ExcelMetadata;

/**
 * Type guard to check if metadata contains Email-specific fields.
 */
export declare function isEmailMetadata(metadata: BaseMetadata): metadata is BaseMetadata & EmailMetadata;

/**
 * Type guard to check if metadata contains PPTX-specific fields.
 */
export declare function isPptxMetadata(metadata: BaseMetadata): metadata is BaseMetadata & PptxMetadata;

/**
 * Type guard to check if metadata contains Archive-specific fields.
 */
export declare function isArchiveMetadata(metadata: BaseMetadata): metadata is BaseMetadata & ArchiveMetadata;

/**
 * Type guard to check if metadata contains Image-specific fields.
 */
export declare function isImageMetadata(metadata: BaseMetadata): metadata is BaseMetadata & ImageMetadata;

/**
 * Type guard to check if metadata contains XML-specific fields.
 */
export declare function isXmlMetadata(metadata: BaseMetadata): metadata is BaseMetadata & XmlMetadata;

/**
 * Type guard to check if metadata contains Text-specific fields.
 */
export declare function isTextMetadata(metadata: BaseMetadata): metadata is BaseMetadata & TextMetadata;

/**
 * Type guard to check if metadata contains HTML-specific fields.
 */
export declare function isHtmlMetadata(metadata: BaseMetadata): metadata is BaseMetadata & HtmlMetadata;

/**
 * Type guard to check if metadata contains OCR-specific fields.
 */
export declare function isOcrMetadata(metadata: BaseMetadata): metadata is BaseMetadata & OcrMetadata;
