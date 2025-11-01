/**
 * Metadata type definition tests.
 *
 * These tests verify that:
 * 1. All metadata types are properly exported from both local and NAPI packages
 * 2. Type structure matches between local types and NAPI bindings
 * 3. Types are assignable/compatible across packages
 * 4. All expected metadata fields exist
 *
 * These tests will BREAK at compile time if:
 * - Metadata types are removed from index.d.ts
 * - metadata.d.ts is deleted
 * - Type structures diverge between packages
 * - Required fields are missing
 */

import { describe, expect, it } from "vitest";
// Import from NAPI bindings (crates/kreuzberg-node)
// This will break at compile time if the import fails
import type {
	ExcelMetadata as NapiExcelMetadata,
	HtmlMetadata as NapiHtmlMetadata,
	Metadata as NapiMetadata,
	PdfMetadata as NapiPdfMetadata,
} from "../../../../crates/kreuzberg-node/metadata";
// Import from local types (packages/typescript/src/types.ts)
import type {
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
} from "../../src/types";

// ============================================================================
// Type Compatibility Tests (Compile-Time)
// ============================================================================

// These type assertions will fail to compile if types are incompatible

// Test that local types are assignable to NAPI types (and vice versa)
type AssertMetadataCompatible = Metadata extends NapiMetadata ? true : never;
type AssertNapiMetadataCompatible = NapiMetadata extends Metadata ? true : never;

type AssertHtmlMetadataCompatible = HtmlMetadata extends NapiHtmlMetadata ? true : never;
type AssertNapiHtmlMetadataCompatible = NapiHtmlMetadata extends HtmlMetadata ? true : never;

type AssertPdfMetadataCompatible = PdfMetadata extends NapiPdfMetadata ? true : never;
type AssertNapiPdfMetadataCompatible = NapiPdfMetadata extends PdfMetadata ? true : never;

type AssertExcelMetadataCompatible = ExcelMetadata extends NapiExcelMetadata ? true : never;
type AssertNapiExcelMetadataCompatible = NapiExcelMetadata extends ExcelMetadata ? true : never;

// Verify all compatibility checks pass (will not compile if any fail)
const _compatibilityTests: [
	AssertMetadataCompatible,
	AssertNapiMetadataCompatible,
	AssertHtmlMetadataCompatible,
	AssertNapiHtmlMetadataCompatible,
	AssertPdfMetadataCompatible,
	AssertNapiPdfMetadataCompatible,
	AssertExcelMetadataCompatible,
	AssertNapiExcelMetadataCompatible,
] = [true, true, true, true, true, true, true, true];

// ============================================================================
// Type Structure Tests (Compile-Time)
// ============================================================================

// Test that Metadata has all required format-specific fields
type AssertMetadataHasPdf = "pdf" extends keyof Metadata ? true : never;
type AssertMetadataHasExcel = "excel" extends keyof Metadata ? true : never;
type AssertMetadataHasEmail = "email" extends keyof Metadata ? true : never;
type AssertMetadataHasPptx = "pptx" extends keyof Metadata ? true : never;
type AssertMetadataHasArchive = "archive" extends keyof Metadata ? true : never;
type AssertMetadataHasImage = "image" extends keyof Metadata ? true : never;
type AssertMetadataHasXml = "xml" extends keyof Metadata ? true : never;
type AssertMetadataHasText = "text" extends keyof Metadata ? true : never;
type AssertMetadataHasHtml = "html" extends keyof Metadata ? true : never;
type AssertMetadataHasOcr = "ocr" extends keyof Metadata ? true : never;
type AssertMetadataHasImagePreprocessing = "imagePreprocessing" extends keyof Metadata ? true : never;

const _metadataStructureTests: [
	AssertMetadataHasPdf,
	AssertMetadataHasExcel,
	AssertMetadataHasEmail,
	AssertMetadataHasPptx,
	AssertMetadataHasArchive,
	AssertMetadataHasImage,
	AssertMetadataHasXml,
	AssertMetadataHasText,
	AssertMetadataHasHtml,
	AssertMetadataHasOcr,
	AssertMetadataHasImagePreprocessing,
] = [true, true, true, true, true, true, true, true, true, true, true];

// Test that HtmlMetadata has all required fields
type AssertHtmlHasTitle = "title" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasDescription = "description" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasKeywords = "keywords" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasAuthor = "author" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasCanonical = "canonical" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasBaseHref = "baseHref" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasOgTitle = "ogTitle" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasOgDescription = "ogDescription" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasOgImage = "ogImage" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasTwitterCard = "twitterCard" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasTwitterTitle = "twitterTitle" extends keyof HtmlMetadata ? true : never;

const _htmlMetadataStructureTests: [
	AssertHtmlHasTitle,
	AssertHtmlHasDescription,
	AssertHtmlHasKeywords,
	AssertHtmlHasAuthor,
	AssertHtmlHasCanonical,
	AssertHtmlHasBaseHref,
	AssertHtmlHasOgTitle,
	AssertHtmlHasOgDescription,
	AssertHtmlHasOgImage,
	AssertHtmlHasTwitterCard,
	AssertHtmlHasTwitterTitle,
] = [true, true, true, true, true, true, true, true, true, true, true];

// ============================================================================
// Runtime Tests
// ============================================================================

describe("Metadata Types", () => {
	describe("Type Exports", () => {
		it("should export Metadata type from local package", () => {
			// This will fail at compile time if Metadata is not exported
			const metadata: Metadata = {
				language: "en",
				date: "2025-01-01",
			};
			expect(metadata).toBeDefined();
		});

		it("should export HtmlMetadata type from local package", () => {
			// This will fail at compile time if HtmlMetadata is not exported
			const htmlMetadata: HtmlMetadata = {
				title: "Test Page",
				description: "Test description",
				keywords: "test, keywords",
				author: "Test Author",
				ogTitle: "OG Title",
				twitterCard: "summary",
			};
			expect(htmlMetadata).toBeDefined();
		});

		it("should export all metadata types from local package", () => {
			// This will fail at compile time if any type is missing
			const pdf: PdfMetadata = { title: "Test", pageCount: 10 };
			const excel: ExcelMetadata = { sheetCount: 1, sheetNames: ["Sheet1"] };
			const email: EmailMetadata = { toEmails: [], ccEmails: [], bccEmails: [], attachments: [] };
			const pptx: PptxMetadata = { fonts: [] };
			const archive: ArchiveMetadata = { format: "zip", fileCount: 1, fileList: [], totalSize: 0 };
			const image: ImageMetadata = { width: 100, height: 100, format: "png", exif: {} };
			const xml: XmlMetadata = { elementCount: 10, uniqueElements: [] };
			const text: TextMetadata = { lineCount: 10, wordCount: 100, characterCount: 500 };
			const ocr: OcrMetadata = { language: "eng", psm: 3, outputFormat: "text", tableCount: 0 };
			const imgPreproc: ImagePreprocessingMetadata = {
				originalDimensions: [100, 100],
				originalDpi: [72, 72],
				targetDpi: 300,
				scaleFactor: 1.0,
				autoAdjusted: false,
				finalDpi: 300,
				resampleMethod: "lanczos",
				dimensionClamped: false,
				skippedResize: false,
			};
			const error: ErrorMetadata = { errorType: "test", message: "test error" };

			expect(pdf).toBeDefined();
			expect(excel).toBeDefined();
			expect(email).toBeDefined();
			expect(pptx).toBeDefined();
			expect(archive).toBeDefined();
			expect(image).toBeDefined();
			expect(xml).toBeDefined();
			expect(text).toBeDefined();
			expect(ocr).toBeDefined();
			expect(imgPreproc).toBeDefined();
			expect(error).toBeDefined();
		});
	});

	describe("Type Structure", () => {
		it("should have html field in Metadata type", () => {
			const metadata: Metadata = {
				html: {
					title: "Test",
					description: "Test description",
				},
			};

			// TypeScript will ensure html field exists and has correct type
			expect(metadata.html).toBeDefined();
			expect(metadata.html?.title).toBe("Test");
		});

		it("should have all expected fields in HtmlMetadata", () => {
			const html: HtmlMetadata = {
				title: "Title",
				description: "Description",
				keywords: "keywords",
				author: "Author",
				canonical: "https://example.com",
				baseHref: "https://example.com/",
				ogTitle: "OG Title",
				ogDescription: "OG Description",
				ogImage: "https://example.com/image.png",
				ogUrl: "https://example.com",
				ogType: "website",
				ogSiteName: "Example Site",
				twitterCard: "summary",
				twitterTitle: "Twitter Title",
				twitterDescription: "Twitter Description",
				twitterImage: "https://example.com/twitter.png",
				twitterSite: "@example",
				twitterCreator: "@author",
				linkAuthor: "https://example.com/author",
				linkLicense: "https://example.com/license",
				linkAlternate: "https://example.com/alt",
			};

			// Verify all fields are accessible
			expect(html.title).toBe("Title");
			expect(html.description).toBe("Description");
			expect(html.keywords).toBe("keywords");
			expect(html.author).toBe("Author");
			expect(html.canonical).toBe("https://example.com");
			expect(html.baseHref).toBe("https://example.com/");
			expect(html.ogTitle).toBe("OG Title");
			expect(html.ogDescription).toBe("OG Description");
			expect(html.ogImage).toBe("https://example.com/image.png");
			expect(html.ogUrl).toBe("https://example.com");
			expect(html.ogType).toBe("website");
			expect(html.ogSiteName).toBe("Example Site");
			expect(html.twitterCard).toBe("summary");
			expect(html.twitterTitle).toBe("Twitter Title");
			expect(html.twitterDescription).toBe("Twitter Description");
			expect(html.twitterImage).toBe("https://example.com/twitter.png");
			expect(html.twitterSite).toBe("@example");
			expect(html.twitterCreator).toBe("@author");
			expect(html.linkAuthor).toBe("https://example.com/author");
			expect(html.linkLicense).toBe("https://example.com/license");
			expect(html.linkAlternate).toBe("https://example.com/alt");
		});

		it("should allow Metadata to have all format-specific fields", () => {
			const metadata: Metadata = {
				language: "en",
				pdf: { title: "PDF", pageCount: 10 },
				excel: { sheetCount: 1, sheetNames: ["Sheet1"] },
				email: { toEmails: [], ccEmails: [], bccEmails: [], attachments: [] },
				pptx: { fonts: [] },
				archive: { format: "zip", fileCount: 1, fileList: [], totalSize: 0 },
				image: { width: 100, height: 100, format: "png", exif: {} },
				xml: { elementCount: 10, uniqueElements: [] },
				text: { lineCount: 10, wordCount: 100, characterCount: 500 },
				html: { title: "HTML Page" },
				ocr: { language: "eng", psm: 3, outputFormat: "text", tableCount: 0 },
			};

			expect(metadata.pdf).toBeDefined();
			expect(metadata.excel).toBeDefined();
			expect(metadata.email).toBeDefined();
			expect(metadata.pptx).toBeDefined();
			expect(metadata.archive).toBeDefined();
			expect(metadata.image).toBeDefined();
			expect(metadata.xml).toBeDefined();
			expect(metadata.text).toBeDefined();
			expect(metadata.html).toBeDefined();
			expect(metadata.ocr).toBeDefined();
		});
	});

	describe("NAPI Bindings Compatibility", () => {
		it("should import Metadata from NAPI bindings", () => {
			// This will fail at compile time if NAPI types are not exported
			const napiMetadata: NapiMetadata = {
				language: "en",
			};
			expect(napiMetadata).toBeDefined();
		});

		it("should import HtmlMetadata from NAPI bindings", () => {
			// This will fail at compile time if NAPI HtmlMetadata is not exported
			const napiHtml: NapiHtmlMetadata = {
				title: "Test",
			};
			expect(napiHtml).toBeDefined();
		});

		it("should allow local Metadata to be assigned to NAPI Metadata", () => {
			const localMetadata: Metadata = {
				language: "en",
				html: { title: "Test" },
			};

			// This will fail at compile time if types are incompatible
			const napiMetadata: NapiMetadata = localMetadata;
			expect(napiMetadata).toBeDefined();
		});

		it("should allow NAPI Metadata to be assigned to local Metadata", () => {
			const napiMetadata: NapiMetadata = {
				language: "en",
				html: { title: "Test" },
			};

			// This will fail at compile time if types are incompatible
			const localMetadata: Metadata = napiMetadata;
			expect(localMetadata).toBeDefined();
		});
	});
});
