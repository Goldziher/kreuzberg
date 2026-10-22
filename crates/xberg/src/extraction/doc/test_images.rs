use super::tests::{
    TestPiece, build_doc_ole, build_fib, build_plc_pcd, build_table_stream, test_rg_fc_lcb_offset, write_u32,
};
use crate::core::config::ExtractionConfig;
use crate::extractors::doc::DocExtractor;
use crate::plugins::InternalDocumentExtractor;
use std::io::{Cursor, Read, Write};

pub(crate) fn picture_doc(text: &str, referenced: bool) -> (Vec<u8>, Vec<u8>) {
    let mut output = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(1, 1, image::Rgb([255, 255, 255]))
        .write_to(&mut output, image::ImageFormat::Png)
        .expect("PNG pixel");
    let png = output.into_inner();
    (picture_doc_with_png(text, referenced, &png), png)
}

fn picture_doc_with_png(text: &str, referenced: bool, png: &[u8]) -> Vec<u8> {
    let mut data = vec![0u8; 68];
    data[4..6].copy_from_slice(&68u16.to_le_bytes());
    data[6..8].copy_from_slice(&100u16.to_le_bytes());
    data.extend_from_slice(&0x000fu16.to_le_bytes());
    data.extend_from_slice(&0xf004u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    let mut blip = vec![0u8; 17];
    blip.extend_from_slice(png);
    let mut bse = vec![0u8; 36];
    bse.extend_from_slice(&0x6e00u16.to_le_bytes());
    bse.extend_from_slice(&0xf01eu16.to_le_bytes());
    bse.extend_from_slice(&(blip.len() as u32).to_le_bytes());
    bse.extend_from_slice(&blip);
    data.extend_from_slice(&0x0062u16.to_le_bytes());
    data.extend_from_slice(&0xf007u16.to_le_bytes());
    data.extend_from_slice(&(bse.len() as u32).to_le_bytes());
    data.extend_from_slice(&bse);
    let len = data.len() as u32;
    write_u32(&mut data, 0, len);
    let text_bytes = text.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>();
    let mut word = build_fib(2048, (text_bytes.len() / 2) as u32, 0, 0, 0);
    word[1024..1024 + text_bytes.len()].copy_from_slice(&text_bytes);
    let pieces = build_plc_pcd(&[TestPiece {
        cp_start: 0,
        cp_end: (text_bytes.len() / 2) as u32,
        fc_raw: 1024,
    }]);
    let mut table = build_table_stream(&mut word, &pieces);
    if referenced {
        let marker = text.find('\x01').expect("picture character");
        let image_fc = 1024 + text[..marker].encode_utf16().count() * 2;
        let bte = table.len();
        table.extend_from_slice(&1024u32.to_le_bytes());
        table.extend_from_slice(&((1024 + text_bytes.len()) as u32).to_le_bytes());
        table.extend_from_slice(&3u32.to_le_bytes());
        write_u32(&mut word, test_rg_fc_lcb_offset() + 12 * 8, bte as u32);
        write_u32(&mut word, test_rg_fc_lcb_offset() + 12 * 8 + 4, 12);
        let fkp = &mut word[1536..2048];
        write_u32(fkp, 0, image_fc as u32);
        write_u32(fkp, 4, image_fc as u32 + 2);
        fkp[8] = 10;
        fkp[20] = 9;
        fkp[21..30].copy_from_slice(&[0x55, 0x08, 1, 3, 0x6a, 0, 0, 0, 0]);
        fkp[511] = 1;
    }
    let bytes = build_doc_ole(&word, &table);
    let mut comp = cfb::CompoundFile::open(Cursor::new(bytes)).expect("open CFB");
    comp.create_stream("/Data")
        .expect("Data stream")
        .write_all(&data)
        .expect("picture data");
    comp.into_inner().into_inner()
}

async fn extract(text: &str) -> (crate::types::internal::InternalDocument, Vec<u8>) {
    let (bytes, png) = picture_doc(text, true);
    let mut config = ExtractionConfig::default();
    config.images = Some(crate::core::config::ImageExtractionConfig {
        extract_images: true,
        ..Default::default()
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &config)
        .await
        .expect("DOC extraction");
    (doc, png)
}

#[tokio::test]
async fn should_extract_live_picture_from_image_only_doc() {
    let (doc, png) = extract("\x01\r").await;
    assert_eq!(doc.images.len(), 1);
    assert_eq!(doc.images[0].data.as_ref(), png.as_slice());
}

#[tokio::test]
async fn should_keep_mixed_doc_picture_between_native_paragraphs() {
    let (doc, png) = extract("BEFORE\r\x01\rAFTER\r").await;
    assert_eq!(doc.images.len(), 1);
    assert_eq!(doc.images[0].data.as_ref(), png.as_slice());
    assert_eq!(doc.elements.len(), 3);
    assert_eq!(doc.elements[0].text, "BEFORE");
    assert!(matches!(
        doc.elements[1].kind,
        crate::types::internal::ElementKind::Image { image_index: 0 }
    ));
    assert_eq!(doc.elements[2].text, "AFTER");
}

#[tokio::test]
async fn should_ignore_unreferenced_picture_data() {
    let (bytes, _) = picture_doc("NATIVE\r", false);
    let mut config = ExtractionConfig::default();
    config.images = Some(crate::core::config::ImageExtractionConfig {
        extract_images: true,
        ..Default::default()
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &config)
        .await
        .expect("DOC extraction");
    assert_eq!(doc.images.len(), 0);
    assert_eq!(doc.elements[0].text, "NATIVE");
}

#[tokio::test]
async fn should_not_read_doc_picture_bytes_for_text_only_extraction() {
    let (bytes, _) = picture_doc("BEFORE\r\x01\rAFTER\r", true);
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &ExtractionConfig::default())
        .await
        .expect("DOC extraction");
    assert_eq!(doc.images.len(), 0);
    assert_eq!(
        doc.elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>(),
        ["BEFORE", "AFTER"]
    );
}

#[tokio::test]
async fn should_reject_doc_picture_when_content_budget_is_exceeded() {
    let (bytes, _) = picture_doc("\x01\r", true);
    let mut config = ExtractionConfig::default();
    config.images = Some(crate::core::config::ImageExtractionConfig {
        extract_images: true,
        ..Default::default()
    });
    config.security_limits = Some(crate::extractors::security::SecurityLimits {
        max_content_size: 16,
        ..Default::default()
    });
    let result = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &config)
        .await;
    assert!(matches!(result, Err(crate::XbergError::Validation { .. })));
}

fn image_config() -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({"images": {"extract_images": true}, "use_cache": false}))
        .expect("image config")
}

fn change_stream(bytes: Vec<u8>, path: &str, change: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut comp = cfb::CompoundFile::open(Cursor::new(bytes)).expect("CFB");
    let mut stream = Vec::new();
    comp.open_stream(path)
        .expect("stream")
        .read_to_end(&mut stream)
        .expect("read stream");
    change(&mut stream);
    comp.create_stream(path)
        .expect("rewrite stream")
        .write_all(&stream)
        .expect("write stream");
    comp.into_inner().into_inner()
}

#[tokio::test]
async fn should_keep_unicode_and_spaces_around_same_paragraph_picture() {
    let (doc, _) = extract("α BEFORE \x01 AFTER β  \r").await;
    assert_eq!(doc.images.len(), 1);
    assert_eq!(doc.elements.len(), 3);
    assert_eq!(doc.elements[0].text, "α BEFORE ");
    assert!(matches!(
        doc.elements[1].kind,
        crate::types::internal::ElementKind::Image { image_index: 0 }
    ));
    assert_eq!(doc.elements[2].text, " AFTER β");
    assert_eq!(
        doc.elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<String>(),
        "α BEFORE  AFTER β"
    );
}

#[tokio::test]
async fn should_warn_about_floating_doc_pictures_only_when_requested() {
    let (bytes, _) = picture_doc("BEFORE\r\x08\rAFTER\r", false);
    for requested in [false, true] {
        let config = if requested {
            image_config()
        } else {
            ExtractionConfig::default()
        };
        let doc = DocExtractor::new()
            .extract_content(&bytes, "application/msword", &config)
            .await
            .expect("DOC");
        assert_eq!(doc.images.len(), 0);
        assert_eq!(
            doc.processing_warnings
                .iter()
                .filter(|warning| warning.message.contains("Floating pictures"))
                .count(),
            usize::from(requested)
        );
        assert_eq!(
            doc.elements
                .iter()
                .map(|element| element.text.as_str())
                .collect::<Vec<_>>(),
            ["BEFORE", "AFTER"]
        );
    }
}

#[tokio::test]
async fn should_preserve_native_doc_text_when_picture_records_are_malformed() {
    for mutation in 0..6 {
        let (bytes, _) = picture_doc("BEFORE\r\x01\rAFTER\r", true);
        let bytes = match mutation {
            0 => change_stream(bytes, "/Data", |stream| stream.truncate(10)),
            1 => change_stream(bytes, "/Data", |stream| write_u32(stream, 0, u32::MAX)),
            2 => change_stream(bytes, "/Data", |stream| {
                stream[4..6].copy_from_slice(&2u16.to_le_bytes())
            }),
            3 => change_stream(bytes, "/Data", |stream| write_u32(stream, 124, u32::MAX)),
            4 => change_stream(bytes, "/WordDocument", |stream| stream[1536 + 8] = 255),
            _ => change_stream(bytes, "/WordDocument", |stream| stream[1536 + 23] = 0),
        };
        let doc = DocExtractor::new()
            .extract_content(&bytes, "application/msword", &image_config())
            .await
            .expect("partial DOC");
        assert_eq!(doc.images.len(), 0, "mutation {mutation}");
        assert_eq!(doc.processing_warnings.len(), 1, "mutation {mutation}");
        assert_eq!(
            doc.elements
                .iter()
                .map(|element| element.text.as_str())
                .collect::<Vec<_>>(),
            ["BEFORE", "AFTER"]
        );
    }
}

#[tokio::test]
async fn should_reject_doc_picture_count_depth_and_iteration_limits() {
    for limit in 0..3 {
        let (bytes, _) = picture_doc("\x01\r", true);
        let mut config = image_config();
        let mut limits = crate::extractors::security::SecurityLimits::default();
        match limit {
            0 => limits.max_files_in_archive = 0,
            1 => limits.max_nesting_depth = 0,
            _ => limits.max_iterations = 0,
        }
        config.security_limits = Some(limits);
        let result = DocExtractor::new()
            .extract_content(&bytes, "application/msword", &config)
            .await;
        assert!(
            matches!(result, Err(crate::XbergError::Validation { .. })),
            "limit {limit}"
        );
    }
}

#[tokio::test]
async fn should_extract_two_uid_png_without_returning_uid_bytes() {
    let (bytes, png) = picture_doc("\x01\r", true);
    let bytes = change_stream(bytes, "/Data", |stream| {
        stream.splice(128..128, [0u8; 16]);
        stream[120..122].copy_from_slice(&0x6e10u16.to_le_bytes());
        let length = stream.len() as u32;
        write_u32(stream, 0, length);
        write_u32(stream, 80, length - 84);
        write_u32(stream, 124, length - 128);
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &image_config())
        .await
        .expect("DOC");
    assert_eq!(doc.images.len(), 1);
    assert_eq!(doc.images[0].data.as_ref(), png.as_slice());
}

#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
#[tokio::test]
async fn should_ocr_doc_pictures_through_public_pipeline_and_respect_flags() {
    // Locally rendered synthetic text; only PNG IHDR/IDAT/IEND chunks. ~keep
    let png = include_bytes!("test_images/marker.png");
    let bytes = picture_doc_with_png("BEFORE\r\x01\rAFTER\r", true, png);
    for (extract_images, embedded, disabled, force, expected_ocr) in [
        (false, true, false, false, true),
        (true, true, false, false, true),
        (true, false, false, false, false),
        (true, true, true, false, false),
        (false, true, false, true, true),
    ] {
        let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
            "use_cache": false, "disable_ocr": disabled, "force_ocr": force,
            "ocr_embedded_images": embedded,
            "images": {"extract_images": extract_images},
            "ocr": {"backend": "tesseract", "language": "eng", "tesseract_config": {
                "language": "eng", "use_cache": false, "output_format": "text"
            }}
        }))
        .expect("OCR configuration");
        let output = crate::extract(
            crate::ExtractInput::from_bytes(bytes.clone(), "application/msword", None),
            &config,
        )
        .await
        .expect("public DOC extraction");
        assert_eq!(output.errors.len(), 0, "{:?}", output.errors);
        assert_eq!(output.results.len(), 1);
        let result = &output.results[0];
        assert_eq!(result.content.contains("DOC IMAGE MARKER ALPHA BRAVO"), expected_ocr);
        assert_eq!(result.images.as_ref().map_or(0, Vec::len), usize::from(extract_images));
        let before = result.content.find("BEFORE").expect("native before");
        let after = result.content.find("AFTER").expect("native after");
        assert!(before < after);
        if expected_ocr {
            let marker = result.content.find("DOC IMAGE MARKER ALPHA BRAVO").expect("OCR marker");
            assert!(before < marker && marker < after);
        }
    }
}

#[tokio::test]
async fn should_not_read_a_picture_from_an_fkp_outside_its_bte_range() {
    let (bytes, _) = picture_doc("BEFORE\r\x01\rAFTER\r", true);
    let bytes = change_stream(bytes, "/1Table", |stream| {
        let bte = stream.len() - 12;
        write_u32(stream, bte, 0);
        write_u32(stream, bte + 4, 100);
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &image_config())
        .await
        .expect("DOC");
    assert_eq!(doc.images.len(), 0);
    assert_eq!(doc.processing_warnings.len(), 1);
    assert_eq!(
        doc.elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>(),
        ["BEFORE", "AFTER"]
    );
}

#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
#[tokio::test]
async fn should_ocr_image_only_doc_without_placeholders_or_retained_bytes() {
    let png = include_bytes!("test_images/marker.png");
    for extract_images in [false, true] {
        let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
            "use_cache": false,
            "images": {"extract_images": extract_images, "inject_placeholders": false},
            "ocr": {"backend": "tesseract", "language": "eng", "tesseract_config": {
                "language": "eng", "use_cache": false, "output_format": "text"
            }}
        }))
        .expect("image-only OCR config");
        let bytes = picture_doc_with_png("\x01\r", true, png);
        let output = crate::extract(
            crate::ExtractInput::from_bytes(bytes, "application/msword", None),
            &config,
        )
        .await
        .expect("image-only extraction");
        assert_eq!(output.errors.len(), 0, "{:?}", output.errors);
        assert_eq!(output.results.len(), 1);
        assert!(output.results[0].content.contains("DOC IMAGE MARKER ALPHA BRAVO"));
        assert_eq!(
            output.results[0].images.as_ref().map_or(0, Vec::len),
            usize::from(extract_images)
        );
    }
}

#[tokio::test]
async fn should_warn_and_preserve_native_text_for_unsupported_dib_picture() {
    let (bytes, _) = picture_doc("BEFORE\r\x01\rAFTER\r", true);
    let bytes = change_stream(bytes, "/Data", |stream| {
        stream[122..124].copy_from_slice(&0xf01fu16.to_le_bytes())
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &image_config())
        .await
        .expect("partial DOC");
    assert_eq!(doc.images.len(), 0);
    assert_eq!(doc.processing_warnings.len(), 1);
    assert!(doc.processing_warnings[0].message.contains("unsupported image format"));
    assert_eq!(
        doc.elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>(),
        ["BEFORE", "AFTER"]
    );
}

#[tokio::test]
async fn should_warn_about_subdocument_pictures_without_losing_native_text() {
    let bytes = super::tests::build_synthetic_doc(&super::tests::SyntheticDoc {
        body: "BEFORE\r",
        footnotes: &["NOTE\x01\r"],
        header_stories: &[],
        comments: &[],
        plcf_hdd: super::tests::PlcKind::Valid,
        note_plcs: super::tests::PlcKind::Valid,
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &image_config())
        .await
        .expect("partial DOC");
    assert_eq!(doc.images.len(), 0);
    assert!(
        doc.processing_warnings
            .iter()
            .any(|warning| warning.message.contains("pictures in DOC subdocuments"))
    );
    assert!(doc.elements.iter().any(|element| element.text == "BEFORE"));
    assert!(doc.elements.iter().any(|element| element.text.contains("NOTE")));
}

fn with_paragraph_property(bytes: Vec<u8>, text: &str, property: &[u8]) -> Vec<u8> {
    let mut table_offset = 0;
    let end = 1024 + text.encode_utf16().count() as u32 * 2;
    let bytes = change_stream(bytes, "/1Table", |table| {
        table_offset = table.len();
        table.extend_from_slice(&1024u32.to_le_bytes());
        table.extend_from_slice(&end.to_le_bytes());
        table.extend_from_slice(&4u32.to_le_bytes());
    });
    change_stream(bytes, "/WordDocument", |word| {
        word.resize(2560, 0);
        write_u32(word, test_rg_fc_lcb_offset() + 13 * 8, table_offset as u32);
        write_u32(word, test_rg_fc_lcb_offset() + 13 * 8 + 4, 12);
        let page = &mut word[2048..2560];
        write_u32(page, 0, 1024);
        write_u32(page, 4, end);
        page[8] = 128;
        page[256] = (3 + property.len()).div_ceil(2) as u8;
        page[259..259 + property.len()].copy_from_slice(property);
        page[511] = 1;
    })
}

#[tokio::test]
async fn should_warn_for_table_and_list_pictures_including_an_empty_list_item() {
    let table = [0x16, 0x24, 1];
    let list = [0x0b, 0x46, 1, 0];
    for (text, property) in [
        ("BEFORE\x01\r", table.as_slice()),
        ("BEFORE\x01\r", list.as_slice()),
        ("\x01\r", list.as_slice()),
    ] {
        let (bytes, _) = picture_doc(text, true);
        let bytes = with_paragraph_property(bytes, text, property);
        let doc = DocExtractor::new()
            .extract_content(&bytes, "application/msword", &image_config())
            .await
            .expect("partial DOC");
        assert_eq!(doc.images.len(), 0);
        assert_eq!(doc.processing_warnings.len(), 1);
        assert!(
            doc.processing_warnings[0]
                .message
                .contains("table cells or automatic lists")
        );
        if text.starts_with("BEFORE") {
            assert!(
                doc.elements.iter().any(|element| element.text.contains("BEFORE"))
                    || doc
                        .tables
                        .iter()
                        .any(|table| table.cells.iter().flatten().any(|cell| cell.contains("BEFORE")))
            );
        }
    }
}

#[tokio::test]
async fn should_not_extract_binary_data_references_as_pictures() {
    let (bytes, _) = picture_doc("BEFORE\r\x01\rAFTER\r", true);
    let bytes = change_stream(bytes, "/WordDocument", |word| {
        word[1536 + 20] = 12;
        word[1536 + 30..1536 + 33].copy_from_slice(&[0x06, 0x08, 1]);
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &image_config())
        .await
        .expect("partial DOC");
    assert_eq!(doc.images.len(), 0);
    assert_eq!(doc.processing_warnings.len(), 1);
}

#[tokio::test]
async fn should_extract_typed_jpeg_picture_bytes() {
    let mut output = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(1, 1, image::Rgb([255, 255, 255]))
        .write_to(&mut output, image::ImageFormat::Jpeg)
        .expect("JPEG pixel");
    let jpeg = output.into_inner();
    let bytes = picture_doc_with_png("\x01\r", true, &jpeg);
    let bytes = change_stream(bytes, "/Data", |stream| {
        stream[120..122].copy_from_slice(&0x46a0u16.to_le_bytes());
        stream[122..124].copy_from_slice(&0xf01du16.to_le_bytes());
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &image_config())
        .await
        .expect("JPEG DOC");
    assert_eq!(doc.images.len(), 1);
    assert_eq!(doc.images[0].format, "jpeg");
    assert_eq!(doc.images[0].data.as_ref(), jpeg.as_slice());
    assert_eq!(
        image::load_from_memory(&doc.images[0].data)
            .expect("decode JPEG")
            .width(),
        1
    );
}

#[tokio::test]
async fn should_select_authoritative_bte_page_over_stale_overlapping_fkp() {
    let text = "BEFORE\r\x01\rAFTER\r";
    let (bytes, png) = picture_doc(text, true);
    let bytes = change_stream(bytes, "/1Table", |table| {
        let bte = table.len() - 12;
        table.truncate(bte);
        for value in [0u32, 100, 1024 + text.encode_utf16().count() as u32 * 2, 5, 3] {
            table.extend_from_slice(&value.to_le_bytes());
        }
    });
    let bytes = change_stream(bytes, "/WordDocument", |word| {
        write_u32(word, test_rg_fc_lcb_offset() + 12 * 8 + 4, 20);
        let old_page = word[1536..2048].to_vec();
        word.resize(3072, 0);
        word[2560..3072].copy_from_slice(&old_page);
        write_u32(word, 2560 + 26, 1);
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &image_config())
        .await
        .expect("DOC");
    assert_eq!(doc.images.len(), 1);
    assert_eq!(doc.images[0].data.as_ref(), png.as_slice());
    assert_eq!(doc.processing_warnings.len(), 0);
}

#[tokio::test]
async fn should_warn_when_requested_doc_pictures_use_contiguous_text_fallback() {
    let text = "BEFORE\r\x01\rAFTER\r";
    let (bytes, _) = picture_doc(text, true);
    let bytes = change_stream(bytes, "/WordDocument", |word| {
        write_u32(word, super::tests::test_fc_clx_offset(), 0);
        write_u32(word, super::tests::test_fc_clx_offset() + 4, 0);
        write_u32(word, 0x18, 1024);
        write_u32(word, 0x1c, 1024 + text.encode_utf16().count() as u32 * 2);
    });
    let doc = DocExtractor::new()
        .extract_content(&bytes, "application/msword", &image_config())
        .await
        .expect("partial DOC");
    assert_eq!(doc.images.len(), 0);
    assert_eq!(doc.processing_warnings.len(), 1);
    assert!(doc.processing_warnings[0].message.contains("contiguous-text fallback"));
    assert_eq!(
        doc.elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>(),
        ["BEFORE", "AFTER"]
    );
}
