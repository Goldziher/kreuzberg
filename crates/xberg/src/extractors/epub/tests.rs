use super::*;

fn epub_with_inline_image() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut archive = zip::ZipWriter::new(Cursor::new(&mut bytes));
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        archive.start_file("mimetype", stored).unwrap();
        std::io::Write::write_all(&mut archive, b"application/epub+zip").unwrap();
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
            std::io::Write::write_all(&mut archive, content).unwrap();
        }
        archive
            .start_file("OEBPS/label.png", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut archive, b"\x89PNG").unwrap();
        archive.finish().unwrap();
    }
    bytes
}

#[test]
fn test_epub_extractor_plugin_interface() {
    let extractor = EpubExtractor::new();
    assert_eq!(extractor.name(), "epub-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert_eq!(extractor.priority(), 60);
    assert!(!extractor.supported_mime_types().is_empty());
}

#[test]
fn test_epub_extractor_default() {
    let extractor = EpubExtractor;
    assert_eq!(extractor.name(), "epub-extractor");
}

#[tokio::test]
async fn test_epub_extractor_initialize_shutdown() {
    let extractor = EpubExtractor::new();
    assert!(extractor.initialize().is_ok());
    assert!(extractor.shutdown().is_ok());
}

#[tokio::test]
async fn should_not_cache_markdown_before_spine_image_ocr_runs() {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ocr: Some(crate::core::config::OcrConfig::default()),
        ..Default::default()
    };

    let doc = EpubExtractor::new()
        .extract_content(&epub_with_inline_image(), "application/epub+zip", &config)
        .await
        .expect("EPUB extraction must succeed");

    assert!(doc.pre_rendered_content.is_none());
    assert_eq!(doc.images.len(), 1);
}

#[test]
fn test_epub_extractor_supported_mime_types() {
    let extractor = EpubExtractor::new();
    let supported = extractor.supported_mime_types();
    assert!(supported.contains(&"application/epub+zip"));
    assert!(supported.contains(&"application/x-epub+zip"));
    assert!(supported.contains(&"application/vnd.epub+zip"));
}

#[test]
fn should_detect_cover_asset_already_rendered_by_spine() {
    let documents = vec![content::EpubSpineDocument {
        file_path: "OEBPS/text/cover.xhtml".to_string(),
        xhtml: r#"<html><body><img src="../images/cover.jpg" alt="Cover"/></body></html>"#.to_string(),
    }];

    assert!(EpubExtractor::spine_references_asset(
        &documents,
        "OEBPS/images/cover.jpg"
    ));
    assert!(!EpubExtractor::spine_references_asset(
        &documents,
        "OEBPS/images/other.jpg"
    ));
}

#[test]
fn test_epub_full_dublin_core_metadata() {
    let opf = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Book</dc:title>
    <dc:creator>Test Author</dc:creator>
    <dc:language>en</dc:language>
    <dc:coverage>Worldwide</dc:coverage>
    <dc:format>application/epub+zip</dc:format>
    <dc:relation>http://example.com/related</dc:relation>
    <dc:source>Original Manuscript</dc:source>
    <dc:type>Text</dc:type>
    <dc:publisher>Test Publisher</dc:publisher>
    <dc:description>A test book</dc:description>
    <dc:rights>CC BY 4.0</dc:rights>
    <meta name="cover" content="cover-img"/>
  </metadata>
  <manifest>
    <item id="cover-img" href="images/cover.jpg" media-type="image/jpeg"/>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;

    let mut budget = crate::extractors::security::SecurityBudget::with_defaults();
    let (package, _warnings) = metadata::parse_opf(opf, "", &mut budget).expect("Metadata parse failed");
    let epub_meta = &package.metadata;
    assert_eq!(epub_meta.title, Some("Test Book".to_string()));
    assert_eq!(epub_meta.coverage, Some("Worldwide".to_string()));
    assert_eq!(epub_meta.format, Some("application/epub+zip".to_string()));
    assert_eq!(epub_meta.relation, Some("http://example.com/related".to_string()));
    assert_eq!(epub_meta.source, Some("Original Manuscript".to_string()));
    assert_eq!(epub_meta.dc_type, Some("Text".to_string()));
    assert_eq!(epub_meta.cover_image_href, Some("images/cover.jpg".to_string()));

    let format_meta = FormatMetadata::Epub(EpubMetadata {
        coverage: epub_meta.coverage.clone(),
        dc_format: epub_meta.format.clone(),
        relation: epub_meta.relation.clone(),
        source: epub_meta.source.clone(),
        dc_type: epub_meta.dc_type.clone(),
        cover_image: epub_meta.cover_image_href.clone(),
    });
    match &format_meta {
        FormatMetadata::Epub(em) => {
            assert_eq!(em.coverage.as_deref(), Some("Worldwide"));
            assert_eq!(em.dc_format.as_deref(), Some("application/epub+zip"));
            assert_eq!(em.relation.as_deref(), Some("http://example.com/related"));
            assert_eq!(em.source.as_deref(), Some("Original Manuscript"));
            assert_eq!(em.dc_type.as_deref(), Some("Text"));
            assert_eq!(em.cover_image.as_deref(), Some("images/cover.jpg"));
        }
        _ => panic!("Expected FormatMetadata::Epub variant"),
    }

    let additional = metadata::build_additional_metadata(epub_meta);
    assert!(additional.contains_key("publisher"));
    assert!(additional.contains_key("description"));
    assert!(additional.contains_key("rights"));
    assert!(!additional.contains_key("coverage"));
    assert!(!additional.contains_key("format"));
    assert!(!additional.contains_key("relation"));
    assert!(!additional.contains_key("source"));
    assert!(!additional.contains_key("type"));
    assert!(!additional.contains_key("cover_image"));
}
