#![cfg(all(feature = "ocr", feature = "tokio-runtime"))]
#![cfg(any(feature = "html", feature = "hwpx", feature = "office", feature = "notebook"))]

mod helpers;

use std::sync::Arc;

#[cfg(any(feature = "hwpx", feature = "office"))]
use std::io::{Cursor, Write};

use async_trait::async_trait;
use helpers::extract_bytes_document;
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::ExtractedDocument;

const OCR_TEXT: &str = "FIXED EMBEDDED IMAGE OCR TEXT";
#[cfg(any(feature = "hwpx", feature = "office"))]
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\nIDATx\x9cc\x00\x01\x00\x00\x05\x00\x01\r\n-\xb4\x00\x00\x00\x00IEND\xaeB`\x82";
#[cfg(any(feature = "html", feature = "office", feature = "notebook"))]
const PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGMAAQAABQABDQottAAAAABJRU5ErkJggg==";

struct FixedTextOcrBackend {
    name: &'static str,
}

#[async_trait]
impl OcrBackend for FixedTextOcrBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supports_language(&self, _: &str) -> bool {
        true
    }

    async fn process_image(&self, _: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let mut document = ExtractedDocument::default();
        document.content = OCR_TEXT.to_string();
        Ok(document)
    }
}

impl Plugin for FixedTextOcrBackend {
    fn name(&self) -> &str {
        self.name
    }

    fn version(&self) -> String {
        "0.0.0".to_string()
    }

    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

struct BackendGuard(&'static str);

impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend(self.0);
    }
}

fn register_backend(name: &'static str) -> BackendGuard {
    register_ocr_backend(Arc::new(FixedTextOcrBackend { name })).expect("fixed OCR backend must register");
    BackendGuard(name)
}

fn config(backend: &str, output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        output_format,
        ocr: Some(OcrConfig {
            backend: backend.to_string(),
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    }
}

fn assert_ocr_text_once(content: &str) {
    assert_eq!(content.matches(OCR_TEXT).count(), 1, "content was {content:?}");
}

#[cfg(feature = "html")]
#[tokio::test]
async fn html_markup_outputs_should_render_inline_image_ocr_text_once() {
    const BACKEND: &str = "html-inline-image-fixed-ocr";
    let _guard = register_backend(BACKEND);
    let html = format!(
        r#"<html><body><p>Stock list</p><img alt="label" src="data:image/png;base64,{PNG_BASE64}"></body></html>"#
    );

    for output_format in [OutputFormat::Markdown, OutputFormat::Djot] {
        let result = extract_bytes_document(html.as_bytes(), "text/html", &config(BACKEND, output_format))
            .await
            .expect("HTML extraction with embedded-image OCR must succeed");

        assert_ocr_text_once(&result.content);
    }
}

#[cfg(feature = "office")]
#[tokio::test]
async fn epub_markup_outputs_should_render_spine_image_ocr_text_once() {
    const BACKEND: &str = "epub-spine-image-fixed-ocr";
    let _guard = register_backend(BACKEND);

    let epub = epub_with_inline_image();
    for output_format in [OutputFormat::Markdown, OutputFormat::Djot] {
        let result = extract_bytes_document(&epub, "application/epub+zip", &config(BACKEND, output_format))
            .await
            .expect("EPUB extraction with embedded-image OCR must succeed");

        assert_ocr_text_once(&result.content);
    }
}

#[cfg(feature = "office")]
#[tokio::test]
async fn fictionbook_markdown_should_render_binary_image_ocr_text_once() {
    const BACKEND: &str = "fictionbook-binary-image-fixed-ocr";
    let _guard = register_backend(BACKEND);
    let fictionbook = format!(
        r##"<FictionBook><body><section><p>Stock list</p><image l:href="#label.png"/></section></body><binary id="label.png" content-type="image/png">{PNG_BASE64}</binary></FictionBook>"##
    );

    let result = extract_bytes_document(
        fictionbook.as_bytes(),
        "application/x-fictionbook+xml",
        &config(BACKEND, OutputFormat::Markdown),
    )
    .await
    .expect("FictionBook extraction with embedded-image OCR must succeed");

    assert_ocr_text_once(&result.content);
}

#[cfg(feature = "office")]
#[tokio::test]
async fn rtf_outputs_should_render_primary_property_group_image_ocr_text_once() {
    const BACKEND: &str = "rtf-property-image-fixed-ocr";
    let _guard = register_backend(BACKEND);
    let image_hex = hex::encode(PNG);
    let rtf = format!(
        concat!(
            r"{{\rtf1 Stock list\par ",
            r"{{\*\shppict{{\pict{{\*\picprop{{\sp{{\sn wzDescription}}{{\sv label}}}}}}",
            r"\pngblip {0}}}}}",
            "\\\r\n  ",
            r"{{\nonshppict{{\pict\pngblip {0}}}}}",
            r"\par }}"
        ),
        image_hex
    );

    for output_format in [
        OutputFormat::Plain,
        OutputFormat::Markdown,
        OutputFormat::Djot,
        OutputFormat::Html,
    ] {
        let result = extract_bytes_document(rtf.as_bytes(), "application/rtf", &config(BACKEND, output_format))
            .await
            .expect("RTF extraction with embedded-image OCR must succeed");

        assert_ocr_text_once(&result.content);
    }
}

#[cfg(feature = "hwpx")]
#[tokio::test]
async fn hwpx_outputs_should_render_extensionless_image_reference_ocr_text_once() {
    const BACKEND: &str = "hwpx-extensionless-image-fixed-ocr";
    let _guard = register_backend(BACKEND);
    let hwpx = hwpx_with_extensionless_image_reference();

    for output_format in [
        OutputFormat::Plain,
        OutputFormat::Markdown,
        OutputFormat::Djot,
        OutputFormat::Html,
    ] {
        let result = extract_bytes_document(&hwpx, "application/haansofthwpx", &config(BACKEND, output_format))
            .await
            .expect("HWPX extraction with embedded-image OCR must succeed");

        assert_ocr_text_once(&result.content);
    }
}

#[cfg(feature = "notebook")]
#[tokio::test]
async fn jupyter_outputs_should_render_attachment_and_output_ocr_text_once_each() {
    const BACKEND: &str = "jupyter-images-fixed-ocr";
    let _guard = register_backend(BACKEND);
    let notebook = serde_json::json!({
        "cells": [
            {
                "cell_type": "markdown",
                "metadata": {},
                "source": [
                    "![inline](data:image/png;base64,",
                    PNG_BASE64,
                    ")\n\n![label](attachment:label.png)"
                ],
                "attachments": {"label.png": {"image/png": PNG_BASE64}}
            },
            {
                "cell_type": "code",
                "execution_count": 1,
                "metadata": {},
                "source": ["show()"],
                "outputs": [{
                    "output_type": "display_data",
                    "metadata": {},
                    "data": {"image/png": PNG_BASE64}
                }]
            }
        ],
        "metadata": {},
        "nbformat": 4,
        "nbformat_minor": 5
    });

    let notebook = serde_json::to_vec(&notebook).expect("notebook must serialize");
    for output_format in [
        OutputFormat::Plain,
        OutputFormat::Markdown,
        OutputFormat::Djot,
        OutputFormat::Html,
    ] {
        let result = extract_bytes_document(&notebook, "application/x-ipynb+json", &config(BACKEND, output_format))
            .await
            .expect("Jupyter extraction with embedded-image OCR must succeed");

        assert_eq!(
            result.content.matches(OCR_TEXT).count(),
            3,
            "content was {:?}",
            result.content
        );
    }
}
#[cfg(feature = "office")]
fn epub_with_inline_image() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut archive = zip::ZipWriter::new(Cursor::new(&mut bytes));
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        archive.start_file("mimetype", stored).unwrap();
        archive.write_all(b"application/epub+zip").unwrap();
        for (path, content) in [
            (
                "META-INF/container.xml",
                br#"<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#.as_slice(),
            ),
            (
                "OEBPS/content.opf",
                br#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier>id</dc:identifier><dc:title>Stock</dc:title><dc:language>en</dc:language></metadata><manifest><item id="c1" href="chapter.xhtml" media-type="application/xhtml+xml"/><item id="p1" href="label.png" media-type="image/png"/></manifest><spine><itemref idref="c1"/></spine></package>"#.as_slice(),
            ),
            (
                "OEBPS/chapter.xhtml",
                br#"<html xmlns="http://www.w3.org/1999/xhtml"><body><p>Stock list</p><img src="label.png"/></body></html>"#.as_slice(),
            ),
        ] {
            archive.start_file(path, zip::write::SimpleFileOptions::default()).unwrap();
            archive.write_all(content).unwrap();
        }
        archive
            .start_file("OEBPS/label.png", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(PNG).unwrap();
        archive.finish().unwrap();
    }
    bytes
}

#[cfg(feature = "hwpx")]
fn hwpx_with_extensionless_image_reference() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut archive = zip::ZipWriter::new(Cursor::new(&mut bytes));
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        archive.start_file("mimetype", stored).unwrap();
        archive.write_all(b"application/hwp+zip").unwrap();

        for (path, content) in [
            (
                "Contents/content.hpf",
                br#"<?xml version="1.0" encoding="UTF-8"?>
<opf:package xmlns:opf="http://www.idpf.org/2007/opf">
  <opf:manifest>
    <opf:item id="section0" href="Contents/section0.xml" media-type="application/xml"/>
    <opf:item id="BIN0001" href="BinData/BIN0001.png" media-type="image/png"/>
  </opf:manifest>
  <opf:spine><opf:itemref idref="section0"/></opf:spine>
</opf:package>"#
                    .as_slice(),
            ),
            (
                "Contents/section0.xml",
                br#"<?xml version="1.0" encoding="UTF-8"?>
<hp:sec xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph"
        xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core">
  <hp:p><hp:run><hp:t>Stock list</hp:t><hp:pic><hc:img binaryItemIDRef="BIN0001"/></hp:pic></hp:run></hp:p>
</hp:sec>"#
                    .as_slice(),
            ),
        ] {
            archive
                .start_file(path, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(content).unwrap();
        }
        archive
            .start_file("BinData/BIN0001.png", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(PNG).unwrap();
        archive.finish().unwrap();
    }
    bytes
}
