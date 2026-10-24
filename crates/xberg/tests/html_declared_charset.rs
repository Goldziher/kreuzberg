//! An HTML page that is not UTF-8 must be read in the encoding it declares. The extractor decoded
//! every such page as UTF-8, so a `<meta charset>` or `<meta http-equiv>` declaration was ignored
//! and each non-ASCII character became U+FFFD. ~keep

#![cfg(feature = "html")]

use xberg::core::config::ExtractionConfig;

mod helpers;
use helpers::extract_bytes_document;

const TEXT: &str = "Café – it’s naïve";

fn page(head: &str) -> String {
    format!("<html><head>{head}</head><body><p>{TEXT}</p></body></html>")
}

async fn extract(bytes: &[u8]) -> (String, Vec<String>) {
    let result = extract_bytes_document(bytes, "text/html", &ExtractionConfig::default())
        .await
        .expect("the page extracts");
    let sources = result
        .processing_warnings
        .iter()
        .map(|warning| warning.source.to_string())
        .collect();
    (result.content, sources)
}

#[tokio::test]
async fn should_read_a_page_in_the_charset_its_meta_declares() {
    for head in [
        r#"<meta charset="windows-1252">"#,
        r#"<meta http-equiv="Content-Type" content="text/html; charset=iso-8859-1">"#,
        r#"<meta content="text/html; charset='windows-1252'" http-equiv="content-type">"#,
    ] {
        let bytes = encoding_rs::WINDOWS_1252.encode(&page(head)).0.into_owned();
        let (content, warnings) = extract(&bytes).await;
        assert_eq!(content.trim(), TEXT, "{head}");
        assert!(warnings.is_empty(), "{head}: no character was lost, got {warnings:?}");
    }

    let japanese = "<html><head><meta charset=\"shift_jis\"></head><body><p>日本語のテキスト</p></body></html>";
    let (content, _) = extract(&encoding_rs::SHIFT_JIS.encode(japanese).0).await;
    assert_eq!(content.trim(), "日本語のテキスト");
}

#[tokio::test]
async fn should_skip_commented_out_and_unsupported_charset_declarations() {
    for head in [
        r#"<!-- <meta charset=shift_jis> --><meta charset="windows-1252">"#,
        // As in the HTML standard's prescan, `<!-->` is a whole comment.
        r#"<!--><meta charset="windows-1252">"#,
        r#"<meta charset="x-no-such-charset"><meta charset="windows-1252">"#,
    ] {
        let bytes = encoding_rs::WINDOWS_1252.encode(&page(head)).0.into_owned();
        let (content, warnings) = extract(&bytes).await;
        assert_eq!(content.trim(), TEXT, "{head}");
        assert!(warnings.is_empty(), "{head}: no character was lost, got {warnings:?}");
    }
}

#[tokio::test]
async fn should_read_a_utf16_page_by_its_byte_order_mark() {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in page("").encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }

    let (content, _) = extract(&bytes).await;

    assert_eq!(content.trim(), TEXT);
}

#[tokio::test]
async fn should_keep_reading_a_valid_utf8_page_as_utf8_whatever_it_declares() {
    let (content, warnings) = extract(page(r#"<meta charset="windows-1252">"#).as_bytes()).await;

    assert_eq!(content.trim(), TEXT);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[tokio::test]
async fn should_find_charset_after_misleading_tags_and_attributes() {
    for head in [
        r#"<meta data-charset=shift_jis charset=windows-1252>"#,
        r#"<meta-data charset=shift_jis><meta charset=windows-1252>"#,
        r#"<meta:note charset=shift_jis><meta charset=windows-1252>"#,
        r#"<meta content="charset=shift_jis"><meta charset=windows-1252>"#,
        r#"<meta http-equiv=refresh content="charset=shift_jis"><meta charset=windows-1252>"#,
        r#"<meta data-extra=">" charset=windows-1252>"#,
        r#"<div data-x='<meta charset=shift_jis>'></div><meta charset=windows-1252>"#,
    ] {
        let bytes = encoding_rs::WINDOWS_1252.encode(&page(head)).0.into_owned();
        let (content, warnings) = extract(&bytes).await;
        assert_eq!(content.trim(), TEXT, "{head}");
        assert_eq!(warnings, Vec::<String>::new(), "{head}");
    }
}

#[tokio::test]
async fn should_prefer_direct_charset_in_either_attribute_order() {
    for head in [
        r#"<meta content="charset=shift_jis" http-equiv=content-type charset=windows-1252>"#,
        r#"<meta charset=windows-1252 http-equiv=content-type content="charset=shift_jis">"#,
        r#"<meta content="charset=shift_jis" charset=invalid http-equiv=content-type><meta charset=windows-1252>"#,
        r#"<meta charset=invalid content="charset=shift_jis" http-equiv=content-type><meta charset=windows-1252>"#,
    ] {
        let bytes = encoding_rs::WINDOWS_1252.encode(&page(head)).0.into_owned();
        let (content, warnings) = extract(&bytes).await;
        assert_eq!(content.trim(), TEXT, "{head}");
        assert_eq!(warnings, Vec::<String>::new(), "{head}");
    }
}

#[tokio::test]
async fn should_keep_first_duplicate_encoding_attributes() {
    for head in [
        r#"<meta charset=invalid charset=shift_jis><meta charset=windows-1252>"#,
        r#"<meta http-equiv=refresh HTTP-EQUIV=content-type content="charset=shift_jis"><meta charset=windows-1252>"#,
        concat!(
            r#"<meta http-equiv=content-type content="charset=invalid" CONTENT="charset=shift_jis">"#,
            "<meta charset=windows-1252>"
        ),
    ] {
        let bytes = encoding_rs::WINDOWS_1252.encode(&page(head)).0.into_owned();
        let (content, warnings) = extract(&bytes).await;
        assert_eq!(content.trim(), TEXT, "{head}");
        assert_eq!(warnings, Vec::<String>::new(), "{head}");
    }
}

#[tokio::test]
async fn should_keep_lossy_text_and_warning_without_eligible_charset() {
    let text_bytes = encoding_rs::WINDOWS_1252.encode(TEXT).0;
    let expected = String::from_utf8_lossy(&text_bytes);
    for head in [
        "<meta content='charset=windows-1252'>",
        "<meta http-equiv=refresh content='charset=windows-1252'>",
        "<meta data-charset=windows-1252>",
        "<meta charset='windows-1252 garbage'>",
        "<meta-data charset=windows-1252>",
        "<meta:note charset=windows-1252>",
        "<div data-x='<meta charset=windows-1252>'>",
        "<meta charset=invalid charset=windows-1252>",
    ] {
        let bytes = encoding_rs::WINDOWS_1252.encode(&page(head)).0.into_owned();
        assert!(std::str::from_utf8(&bytes).is_err(), "{head}");
        let (content, warnings) = extract(&bytes).await;
        assert_eq!(content.trim(), expected, "{head}");
        assert_eq!(warnings, ["html"], "{head}");
    }
}

#[tokio::test]
async fn should_finish_meta_attributes_before_the_prescan_limit() {
    const PRESCAN_BYTES: usize = 1024;
    let text_bytes = encoding_rs::WINDOWS_1252.encode(TEXT).0;
    let lossy = String::from_utf8_lossy(&text_bytes);
    for (prefix, suffix, eligible) in [
        ("<meta charset=\"windows-1252\">", "", true),
        ("<meta charset=\"windows-1252", "\">", false),
        ("<meta charset=windows-1252", ">", false),
        ("<meta charset=\"windows-1252\"", ">", false),
    ] {
        let mut html = " ".repeat(PRESCAN_BYTES - prefix.len());
        html.push_str(prefix);
        assert_eq!(html.len(), PRESCAN_BYTES);
        html.push_str(suffix);
        html.push_str(&format!("<p>{TEXT}</p>"));
        let bytes = encoding_rs::WINDOWS_1252.encode(&html).0.into_owned();
        let (content, warnings) = extract(&bytes).await;
        assert_eq!(content.trim(), if eligible { TEXT } else { &lossy }, "{prefix}{suffix}");
        let expected_warnings: &[&str] = if eligible { &[] } else { &["html"] };
        assert_eq!(warnings, expected_warnings, "{prefix}{suffix}");
    }
}
