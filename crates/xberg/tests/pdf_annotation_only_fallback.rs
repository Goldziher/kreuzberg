#![cfg(feature = "pdf")]

use xberg::core::config::{ExtractInput, OcrConfig, PdfConfig};
use xberg::{ExtractionConfig, ResultFormat, extract};

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use async_trait::async_trait;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use std::borrow::Cow;
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

const EXPECTED_TEXT: &str = "VISIBLE ANNOTATION TEXT";
const SECOND_PAGE_TEXT: &str = "PAGE TWO ANNOTATION";
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const HEADER_TEXT: &str = "HEADER";
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const TOTAL_TEXT: &str = "TOTAL";
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const LONG_ANNOTATION_TEXT: &str = concat!(
    "This annotation contains enough ordinary words to look substantive to the native text quality gate ",
    "even though the PDF page itself is only a scanned image and still requires optical character recognition ",
    "to recover its underlying document content."
);
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const OCR_TEXT: &str = "OCR RECOVERED SCANNED PAGE";
const BODY_TEXT: &str = "BODY TEXT";
const PDF_MIME: &str = "application/pdf";
const FALLBACK_WARNING_SOURCE: &str = "pdf_annotations";
const FALLBACK_WARNING_MESSAGE: &str =
    "native PDF page text was empty; recovered 1 text-bearing annotation(s) as document content";
const INVISIBLE_ANNOTATION_FLAG: u32 = 1;
const HIDDEN_ANNOTATION_FLAG: u32 = 2;
const NO_VIEW_ANNOTATION_FLAG: u32 = 32;

struct AnnotationOptions<'a> {
    content: &'a str,
    include_appearance: bool,
    appearance_content: Option<&'a str>,
    appearance_hidden_optional_content: bool,
    subtype: &'a str,
    flags: u32,
    opacity: Option<f64>,
    rect: [f64; 4],
    hidden_optional_content: bool,
}

impl Default for AnnotationOptions<'_> {
    fn default() -> Self {
        Self {
            content: EXPECTED_TEXT,
            include_appearance: true,
            appearance_content: None,
            appearance_hidden_optional_content: false,
            subtype: "FreeText",
            flags: 0,
            opacity: None,
            rect: [100.0, 100.0, 320.0, 140.0],
            hidden_optional_content: false,
        }
    }
}

fn rotated_pdf(body_text: Option<&str>, annotation: AnnotationOptions<'_>) -> Vec<u8> {
    let appearance_text = annotation.appearance_content.unwrap_or(annotation.content);
    let appearance_body = format!("BT\n/Helv 14 Tf\n10 20 Td\n({appearance_text}) Tj\nET\n");
    let appearance = if annotation.appearance_hidden_optional_content {
        format!("/OC /Hidden BDC\n{appearance_body}EMC\n")
    } else {
        appearance_body
    };
    let page_content = body_text
        .map(|text| format!("BT\n/Helv 14 Tf\n72 700 Td\n({text}) Tj\nET\n"))
        .unwrap_or_default();
    let opacity = annotation
        .opacity
        .map(|value| format!(" /CA {value}"))
        .unwrap_or_default();
    let optional_content = if annotation.hidden_optional_content {
        " /OC 8 0 R"
    } else {
        ""
    };
    let appearance_entry = if annotation.include_appearance {
        " /AP << /N 6 0 R >>"
    } else {
        ""
    };
    let catalog_optional_content =
        if annotation.hidden_optional_content || annotation.appearance_hidden_optional_content {
            " /OCProperties << /OCGs [8 0 R] /D << /OFF [8 0 R] >> >>"
        } else {
            ""
        };
    let [x0, y0, x1, y1] = annotation.rect;
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R{catalog_optional_content} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate 90 \
         /Resources << /Font << /Helv 7 0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{page_content}endstream", page_content.len()),
        format!(
            "<< /Type /Annot /Subtype /{} /Rect [{x0} {y0} {x1} {y1}] \
             /Contents ({}) /DA (0 0 0 rg /Helv 14 Tf) /Rotate 90 \
             /F {}{opacity}{optional_content}{appearance_entry} >>",
            annotation.subtype, annotation.content, annotation.flags
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 220 40] \
             /Resources << /Font << /Helv 7 0 R >> \
             /Properties << /Hidden 8 0 R >> >> /Length {} >>\nstream\n{appearance}endstream",
            appearance.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /Encoding << /Type /Encoding /Differences \
         [32 /space 65 /A 66 /B 68 /D /E 73 /I 76 /L 78 /N /O /P 82 /R \
          84 /T 86 /V 88 /X /Y] >> >>"
            .to_string(),
        "<< /Type /OCG /Name (Hidden annotation layer) >>".to_string(),
    ];

    assemble_pdf(&objects)
}

fn two_page_pdf_with_second_page_annotation(rect: [f64; 4], page_one_text: Option<&str>) -> Vec<u8> {
    let appearance = format!("BT\n/Helv 14 Tf\n10 20 Td\n({SECOND_PAGE_TEXT}) Tj\nET\n");
    let first_page_content = page_one_text
        .map(|text| format!("BT\n/Helv 14 Tf\n72 700 Td\n({text}) Tj\nET\n"))
        .unwrap_or_default();
    let [x0, y0, x1, y1] = rect;
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /CropBox [0 0 300 300] \
         /Resources << /Font << /Helv 9 0 R >> >> /Contents 5 0 R >>"
            .to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /CropBox [0 0 300 300] \
         /Resources << /Font << /Helv 9 0 R >> >> /Contents 6 0 R /Annots [7 0 R] >>"
            .to_string(),
        format!(
            "<< /Length {} >>\nstream\n{first_page_content}endstream",
            first_page_content.len()
        ),
        "<< /Length 0 >>\nstream\nendstream".to_string(),
        format!(
            "<< /Type /Annot /Subtype /FreeText /Rect [{x0} {y0} {x1} {y1}] \
             /Contents ({SECOND_PAGE_TEXT}) /DA (0 0 0 rg /Helv 14 Tf) /AP << /N 8 0 R >> >>"
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 220 40] \
             /Resources << /Font << /Helv 9 0 R >> >> /Length {} >>\nstream\n{appearance}endstream",
            appearance.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];

    assemble_pdf(&objects)
}

fn two_page_pdf_with_identical_annotations() -> Vec<u8> {
    let appearance = format!("BT\n/Helv 14 Tf\n10 20 Td\n({EXPECTED_TEXT}) Tj\nET\n");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 5 0 R /Annots [7 0 R] >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 6 0 R /Annots [8 0 R] >>".to_string(),
        "<< /Length 0 >>\nstream\nendstream".to_string(),
        "<< /Length 0 >>\nstream\nendstream".to_string(),
        format!(
            "<< /Type /Annot /Subtype /FreeText /Rect [100 100 320 140] \
             /Contents ({EXPECTED_TEXT}) /AP << /N 9 0 R >> >>"
        ),
        format!(
            "<< /Type /Annot /Subtype /FreeText /Rect [100 100 320 140] \
             /Contents ({EXPECTED_TEXT}) /AP << /N 9 0 R >> >>"
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 220 40] \
             /Resources << /Font << /Helv 10 0 R >> >> /Length {} >>\nstream\n{appearance}endstream",
            appearance.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    assemble_pdf(&objects)
}

fn assemble_pdf(objects: &[String]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }

    let xref_offset = bytes.len();
    let size = objects.len() + 1;
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );
    bytes
}

fn extraction_config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        result_format: ResultFormat::ElementBased,
        ocr: Some(OcrConfig {
            enabled: false,
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            extract_annotations: false,
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[tokio::test]
async fn should_recover_visible_annotation_text_when_native_page_body_is_empty() {
    let config = extraction_config();
    let result = extract(
        ExtractInput::from_bytes(rotated_pdf(None, AnnotationOptions::default()), PDF_MIME, None),
        &config,
    )
    .await
    .expect("annotation-only PDF extraction must succeed");
    let document = result
        .results
        .first()
        .expect("one input must yield one extracted document");

    assert_eq!(document.content, EXPECTED_TEXT);
    assert!(document.annotations.is_none());
    let fallback_warnings: Vec<_> = document
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == FALLBACK_WARNING_SOURCE)
        .collect();
    assert_eq!(fallback_warnings.len(), 1);
    assert_eq!(fallback_warnings[0].message, FALLBACK_WARNING_MESSAGE);
    assert!(
        document
            .elements
            .as_deref()
            .is_some_and(|elements| elements.iter().any(|element| element.text == EXPECTED_TEXT)),
        "recovered annotation text must be represented in document elements: {:?}",
        document.elements
    );
}

#[tokio::test]
async fn should_not_include_free_text_annotation_when_native_body_is_non_empty() {
    let config = extraction_config();
    let result = extract(
        ExtractInput::from_bytes(
            rotated_pdf(Some(BODY_TEXT), AnnotationOptions::default()),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await
    .expect("PDF extraction with a native body must succeed");
    let document = result
        .results
        .first()
        .expect("one input must yield one extracted document");

    assert_eq!(document.content, BODY_TEXT);
    assert!(document.annotations.is_none());
    assert!(
        !document
            .processing_warnings
            .iter()
            .any(|warning| warning.source == FALLBACK_WARNING_SOURCE),
        "the empty-body fallback warning must not fire for a non-empty native body"
    );
}

#[tokio::test]
async fn should_not_recover_hidden_free_text_annotation_as_document_content() {
    for flags in [
        INVISIBLE_ANNOTATION_FLAG,
        HIDDEN_ANNOTATION_FLAG,
        NO_VIEW_ANNOTATION_FLAG,
    ] {
        assert_annotation_is_excluded(AnnotationOptions {
            flags,
            ..Default::default()
        })
        .await;
    }
}

#[tokio::test]
async fn should_not_recover_transparent_free_text_annotation_as_document_content() {
    assert_annotation_is_excluded(AnnotationOptions {
        opacity: Some(0.0),
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_not_recover_free_text_annotation_without_visible_page_area() {
    for rect in [[700.0, 100.0, 800.0, 140.0], [100.0, 100.0, 100.0, 140.0]] {
        assert_annotation_is_excluded(AnnotationOptions {
            rect,
            ..Default::default()
        })
        .await;
    }
}

#[tokio::test]
async fn should_not_recover_non_free_text_annotation_as_document_content() {
    assert_annotation_is_excluded(AnnotationOptions {
        subtype: "Text",
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_not_recover_free_text_annotation_on_hidden_optional_content_layer() {
    assert_annotation_is_excluded(AnnotationOptions {
        hidden_optional_content: true,
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_not_recover_contents_when_normal_appearance_is_blank() {
    assert_annotation_is_excluded(AnnotationOptions {
        appearance_content: Some(""),
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_not_recover_contents_when_normal_appearance_text_differs() {
    assert_annotation_is_excluded(AnnotationOptions {
        appearance_content: Some("DIFFERENT APPEARANCE"),
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_not_recover_contents_hidden_inside_normal_appearance() {
    assert_annotation_is_excluded(AnnotationOptions {
        appearance_hidden_optional_content: true,
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn should_recover_visible_contents_when_normal_appearance_is_absent() {
    let config = extraction_config();
    let result = extract(
        ExtractInput::from_bytes(
            rotated_pdf(
                None,
                AnnotationOptions {
                    include_appearance: false,
                    ..Default::default()
                },
            ),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await
    .expect("FreeText without an appearance stream must succeed");

    assert_eq!(result.results[0].content, EXPECTED_TEXT);
    assert!(result.results[0].annotations.is_none());
}

#[tokio::test]
async fn should_recover_annotation_into_its_exact_page_and_element() {
    let config = extraction_config();
    let result = extract(
        ExtractInput::from_bytes(
            two_page_pdf_with_second_page_annotation([100.0, 100.0, 250.0, 140.0], None),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await
    .expect("two-page annotation-only PDF extraction must succeed");
    let document = result.results.first().expect("one document must be returned");
    let pages = document
        .pages
        .as_deref()
        .expect("element-based extraction must return pages");
    let elements = document
        .elements
        .as_deref()
        .expect("element-based extraction must return elements");

    assert_eq!(document.content, SECOND_PAGE_TEXT);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].page_number, 1);
    assert_eq!(pages[0].content, "");
    assert_eq!(pages[0].is_blank, Some(true));
    assert_eq!(pages[1].page_number, 2);
    assert_eq!(pages[1].content, SECOND_PAGE_TEXT);
    assert_eq!(pages[1].is_blank, Some(false));
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].text, SECOND_PAGE_TEXT);
    assert_eq!(elements[0].metadata.page_number, Some(2));
    let page_structure = document
        .metadata
        .pages
        .as_ref()
        .expect("page structure must be retained");
    let boundaries = page_structure
        .boundaries
        .as_deref()
        .expect("page boundaries must be rebuilt");
    assert_eq!(boundaries.len(), 2);
    assert_eq!(boundaries[0].page_number, 1);
    assert_eq!((boundaries[0].byte_start, boundaries[0].byte_end), (0, 0));
    assert_eq!(boundaries[1].page_number, 2);
    assert_eq!(
        (boundaries[1].byte_start, boundaries[1].byte_end),
        (0, SECOND_PAGE_TEXT.len())
    );
    let metadata_pages = page_structure
        .pages
        .as_deref()
        .expect("per-page metadata must be retained");
    assert_eq!(metadata_pages[0].is_blank, Some(true));
    assert_eq!(metadata_pages[1].is_blank, Some(false));
    assert!(document.annotations.is_none());
}

#[tokio::test]
async fn should_not_recover_annotation_outside_crop_box() {
    let config = extraction_config();
    let result = extract(
        ExtractInput::from_bytes(
            two_page_pdf_with_second_page_annotation([400.0, 100.0, 500.0, 140.0], None),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await
    .expect("PDF with an annotation outside CropBox must succeed");
    let document = result.results.first().expect("one document must be returned");

    assert_eq!(document.content, "");
    assert!(document.annotations.is_none());
    assert!(
        document
            .processing_warnings
            .iter()
            .all(|warning| warning.source != FALLBACK_WARNING_SOURCE)
    );
}

#[tokio::test]
async fn should_recover_reversed_free_text_rectangle() {
    let config = extraction_config();
    let result = extract(
        ExtractInput::from_bytes(
            rotated_pdf(
                None,
                AnnotationOptions {
                    rect: [320.0, 140.0, 100.0, 100.0],
                    ..Default::default()
                },
            ),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await
    .expect("reversed annotation rectangle must be normalized");

    assert_eq!(result.results[0].content, EXPECTED_TEXT);
}

#[tokio::test]
async fn should_recover_blank_page_annotation_in_mixed_native_document() {
    let mut config = extraction_config();
    config.result_format = ResultFormat::Unified;
    config.ocr = None;
    let result = extract(
        ExtractInput::from_bytes(
            two_page_pdf_with_second_page_annotation([100.0, 100.0, 250.0, 140.0], Some(BODY_TEXT)),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await
    .expect("mixed native and annotation-only pages must succeed");
    let document = &result.results[0];

    assert!(document.content.contains(BODY_TEXT));
    assert!(document.content.contains(SECOND_PAGE_TEXT));
    assert!(document.elements.is_none());
}

#[tokio::test]
async fn should_preserve_explicit_annotation_extraction_on_blank_body() {
    let mut config = extraction_config();
    config
        .pdf_options
        .as_mut()
        .expect("PDF options must exist")
        .extract_annotations = true;
    let result = extract(
        ExtractInput::from_bytes(rotated_pdf(None, AnnotationOptions::default()), PDF_MIME, None),
        &config,
    )
    .await
    .expect("explicit annotation extraction must succeed");
    let document = &result.results[0];
    let annotations = document
        .annotations
        .as_deref()
        .expect("annotation output must be present");

    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].content.as_deref(), Some(EXPECTED_TEXT));
    assert!(
        document.processing_warnings.iter().all(|warning| {
            warning.source != FALLBACK_WARNING_SOURCE || warning.message != FALLBACK_WARNING_MESSAGE
        })
    );
}

#[tokio::test]
async fn should_emit_identical_annotation_text_once_on_each_page() {
    let config = extraction_config();
    let result = extract(
        ExtractInput::from_bytes(two_page_pdf_with_identical_annotations(), PDF_MIME, None),
        &config,
    )
    .await
    .expect("identical annotations on separate pages must succeed");
    let elements = result.results[0].elements.as_deref().expect("elements must be present");
    let matching: Vec<_> = elements
        .iter()
        .filter(|element| element.text == EXPECTED_TEXT)
        .collect();

    assert_eq!(matching.len(), 2);
    assert_eq!(matching[0].metadata.page_number, Some(1));
    assert_eq!(matching[1].metadata.page_number, Some(2));
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
struct AnnotationScanOcrBackend {
    text: &'static str,
    document_level: bool,
    document_called: Option<Arc<AtomicBool>>,
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl Plugin for AnnotationScanOcrBackend {
    fn name(&self) -> &str {
        "tesseract"
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[async_trait]
impl OcrBackend for AnnotationScanOcrBackend {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<xberg::ExtractedDocument> {
        let mut document = xberg::ExtractedDocument::default();
        document.content = self.text.to_string();
        document.mime_type = Cow::Borrowed("text/plain");
        Ok(document)
    }

    fn supports_language(&self, _language: &str) -> bool {
        true
    }

    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supports_document_processing(&self) -> bool {
        self.document_level
    }

    async fn process_document(&self, _path: &Path, _config: &OcrConfig) -> xberg::Result<xberg::ExtractedDocument> {
        if let Some(document_called) = &self.document_called {
            document_called.store(true, Ordering::SeqCst);
        }
        let mut document = xberg::ExtractedDocument::default();
        document.content = self.text.to_string();
        document.mime_type = Cow::Borrowed("text/plain");
        Ok(document)
    }
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[tokio::test]
#[serial_test::serial]
async fn should_route_scanned_page_to_ocr_despite_long_free_text_annotation() {
    let _ = unregister_ocr_backend("tesseract");
    register_ocr_backend(Arc::new(AnnotationScanOcrBackend {
        text: OCR_TEXT,
        document_level: false,
        document_called: None,
    }))
    .expect("test OCR backend must register");
    let mut config = extraction_config();
    config.ocr = Some(OcrConfig {
        enabled: true,
        backend: "tesseract".to_string(),
        ..Default::default()
    });
    let result = extract(
        ExtractInput::from_bytes(
            rotated_pdf(
                None,
                AnnotationOptions {
                    content: LONG_ANNOTATION_TEXT,
                    ..Default::default()
                },
            ),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await;
    unregister_ocr_backend("tesseract").expect("test OCR backend must unregister");
    let document = &result.expect("OCR-enabled annotation scan must succeed").results[0];

    assert!(
        document.content.contains(OCR_TEXT),
        "OCR output proves the OCR route ran"
    );
    assert!(document.content.contains(LONG_ANNOTATION_TEXT));
    assert_eq!(document.content.matches(LONG_ANNOTATION_TEXT).count(), 1);
    assert!(document.annotations.is_none());
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[tokio::test]
#[serial_test::serial]
async fn should_retain_each_page_from_document_capable_backend_on_byte_input() {
    let _ = unregister_ocr_backend("tesseract");
    register_ocr_backend(Arc::new(AnnotationScanOcrBackend {
        text: OCR_TEXT,
        document_level: true,
        document_called: None,
    }))
    .expect("document OCR backend must register");
    let mut config = extraction_config();
    config.force_ocr = true;
    config.ocr = Some(OcrConfig {
        enabled: true,
        backend: "tesseract".to_string(),
        ..Default::default()
    });
    let result = extract(
        ExtractInput::from_bytes(
            two_page_pdf_with_second_page_annotation([100.0, 100.0, 250.0, 140.0], Some(BODY_TEXT)),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await;
    unregister_ocr_backend("tesseract").expect("document OCR backend must unregister");
    let document = &result.expect("per-page OCR for byte input must succeed").results[0];
    let pages = document.pages.as_deref().expect("pages must remain coherent");
    let boundaries = document
        .metadata
        .pages
        .as_ref()
        .and_then(|pages| pages.boundaries.as_deref())
        .expect("boundaries must remain coherent");

    assert_eq!(document.content.matches(OCR_TEXT).count(), 2);
    assert!(!document.content.contains(BODY_TEXT));
    assert_eq!(document.content.matches(SECOND_PAGE_TEXT).count(), 1);
    assert_eq!(pages[0].content, OCR_TEXT);
    assert!(pages[1].content.contains(OCR_TEXT));
    assert!(pages[1].content.contains(SECOND_PAGE_TEXT));
    let joined_page_content = pages
        .iter()
        .map(|page| page.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    assert_eq!((boundaries[0].byte_start, boundaries[0].byte_end), (0, OCR_TEXT.len()));
    assert_eq!(boundaries[1].byte_start, OCR_TEXT.len() + 2);
    assert_eq!(
        &joined_page_content[boundaries[1].byte_start..boundaries[1].byte_end],
        pages[1].content
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[cfg(not(paddle_ocr))]
#[tokio::test]
#[serial_test::serial]
async fn should_replace_stale_native_pages_after_actual_document_ocr() {
    let _ = unregister_ocr_backend("tesseract");
    let document_called = Arc::new(AtomicBool::new(false));
    register_ocr_backend(Arc::new(AnnotationScanOcrBackend {
        text: OCR_TEXT,
        document_level: true,
        document_called: Some(Arc::clone(&document_called)),
    }))
    .expect("document OCR backend must register");
    let directory = tempfile::tempdir().expect("temporary directory must be created");
    let path = directory.path().join("annotation-document.pdf");
    std::fs::write(
        &path,
        two_page_pdf_with_second_page_annotation([100.0, 100.0, 250.0, 140.0], Some(BODY_TEXT)),
    )
    .expect("temporary PDF must be written");
    let mut config = extraction_config();
    config.force_ocr = true;
    config.ocr = Some(OcrConfig {
        enabled: true,
        backend: "tesseract".to_string(),
        ..Default::default()
    });
    config.content_filter = Some(xberg::core::config::ContentFilterConfig {
        include_headers: true,
        include_footers: true,
        ..Default::default()
    });
    let result = extract(ExtractInput::from_uri(path.to_string_lossy()), &config).await;
    unregister_ocr_backend("tesseract").expect("document OCR backend must unregister");
    let document = &result.expect("whole-document OCR must succeed").results[0];
    let pages = document.pages.as_deref().expect("pages must remain coherent");

    assert!(document_called.load(Ordering::SeqCst), "process_document must run");
    assert!(document.content.contains(OCR_TEXT));
    assert!(!document.content.contains(BODY_TEXT));
    assert_eq!(document.content.matches(SECOND_PAGE_TEXT).count(), 1);
    assert_eq!(pages[0].content, OCR_TEXT);
    assert_eq!(pages[1].content, SECOND_PAGE_TEXT);
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[tokio::test]
#[serial_test::serial]
async fn should_not_treat_annotation_as_substring_duplicate_of_ocr_text() {
    let _ = unregister_ocr_backend("tesseract");
    register_ocr_backend(Arc::new(AnnotationScanOcrBackend {
        text: "SUBTOTAL",
        document_level: false,
        document_called: None,
    }))
    .expect("substring OCR backend must register");
    let mut config = extraction_config();
    config.force_ocr = true;
    config.ocr = Some(OcrConfig {
        enabled: true,
        backend: "tesseract".to_string(),
        ..Default::default()
    });
    let result = extract(
        ExtractInput::from_bytes(
            rotated_pdf(
                None,
                AnnotationOptions {
                    content: TOTAL_TEXT,
                    ..Default::default()
                },
            ),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await;
    unregister_ocr_backend("tesseract").expect("substring OCR backend must unregister");
    let document = &result.expect("substring OCR case must succeed").results[0];

    assert!(document.content.contains("SUBTOTAL"));
    assert_eq!(
        document
            .content
            .lines()
            .filter(|line| line.trim() == TOTAL_TEXT)
            .count(),
        1
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[tokio::test]
#[serial_test::serial]
async fn should_not_duplicate_exact_annotation_already_present_in_ocr_text() {
    let _ = unregister_ocr_backend("tesseract");
    register_ocr_backend(Arc::new(AnnotationScanOcrBackend {
        text: TOTAL_TEXT,
        document_level: false,
        document_called: None,
    }))
    .expect("exact-match OCR backend must register");
    let mut config = extraction_config();
    config.force_ocr = true;
    config.ocr = Some(OcrConfig {
        enabled: true,
        backend: "tesseract".to_string(),
        ..Default::default()
    });
    let result = extract(
        ExtractInput::from_bytes(
            rotated_pdf(
                None,
                AnnotationOptions {
                    content: TOTAL_TEXT,
                    ..Default::default()
                },
            ),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await;
    unregister_ocr_backend("tesseract").expect("exact-match OCR backend must unregister");
    let document = &result.expect("exact-match OCR case must succeed").results[0];

    assert_eq!(
        document
            .content
            .lines()
            .filter(|line| line.trim() == TOTAL_TEXT)
            .count(),
        1
    );
    assert_eq!(
        document
            .elements
            .as_deref()
            .into_iter()
            .flatten()
            .filter(|element| element.metadata.page_number == Some(1) && element.text.trim() == TOTAL_TEXT)
            .count(),
        1
    );
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[tokio::test]
#[serial_test::serial]
async fn should_not_duplicate_annotation_line_inside_structured_ocr_element() {
    let _ = unregister_ocr_backend("tesseract");
    register_ocr_backend(Arc::new(AnnotationScanOcrBackend {
        text: "HEADER\nTOTAL",
        document_level: false,
        document_called: None,
    }))
    .expect("embedded-line OCR backend must register");
    let mut config = extraction_config();
    config.force_ocr = true;
    config.ocr = Some(OcrConfig {
        enabled: true,
        backend: "tesseract".to_string(),
        ..Default::default()
    });
    let result = extract(
        ExtractInput::from_bytes(
            rotated_pdf(
                None,
                AnnotationOptions {
                    content: TOTAL_TEXT,
                    ..Default::default()
                },
            ),
            PDF_MIME,
            None,
        ),
        &config,
    )
    .await;
    unregister_ocr_backend("tesseract").expect("embedded-line OCR backend must unregister");
    let document = &result.expect("embedded-line OCR case must succeed").results[0];

    assert_eq!(
        document
            .content
            .lines()
            .filter(|line| line.trim() == HEADER_TEXT)
            .count(),
        1
    );
    assert_eq!(
        document
            .content
            .lines()
            .filter(|line| line.trim() == TOTAL_TEXT)
            .count(),
        1
    );
    assert_eq!(
        document
            .elements
            .as_deref()
            .into_iter()
            .flatten()
            .filter(|element| {
                element.metadata.page_number == Some(1)
                    && element.text.lines().any(|line| line.trim() == HEADER_TEXT)
                    && element.text.lines().any(|line| line.trim() == TOTAL_TEXT)
            })
            .count(),
        1
    );
}

async fn assert_annotation_is_excluded(annotation: AnnotationOptions<'_>) {
    let config = extraction_config();
    let result = extract(
        ExtractInput::from_bytes(rotated_pdf(None, annotation), PDF_MIME, None),
        &config,
    )
    .await
    .expect("PDF extraction with an excluded annotation must succeed");
    let document = result
        .results
        .first()
        .expect("one input must yield one extracted document");

    assert_eq!(document.content, "");
    assert!(document.annotations.is_none());
    assert!(
        document
            .processing_warnings
            .iter()
            .all(|warning| warning.source != FALLBACK_WARNING_SOURCE),
        "fallback warning must not claim excluded annotation text was recovered"
    );
}
