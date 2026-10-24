mod helpers;

use base64::Engine;
use helpers::extract_bytes_document;
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat};

async fn extract_markdown(source: &str) -> ExtractedDocument {
    extract_bytes_document(source.as_bytes(), "text/markdown", &ExtractionConfig::default())
        .await
        .expect("Markdown extraction should succeed")
}

#[tokio::test]
async fn should_preserve_brackets_in_link_labels_without_destinations() {
    for source in [
        "their workflow[[1]](https://example.com/#fn1)[[2]](https://example.com/#fn2) next",
        r"their workflow[\[1\]](https://example.com/#fn1)[\[2\]](https://example.com/#fn2) next",
    ] {
        let result = extract_markdown(source).await;
        assert_eq!(result.content, "their workflow[1][2] next", "source: {source}");
        let uris = result.uris.expect("Link destinations should remain available");
        assert_eq!(uris.len(), 2);
        assert_eq!(uris[0].url, "https://example.com/#fn1");
        assert_eq!(uris[1].url, "https://example.com/#fn2");
    }
}

#[tokio::test]
async fn should_preserve_paragraph_boundaries_inside_list_items() {
    for source in ["- alpha\n\n  omega", "1. alpha\n\n   omega"] {
        assert_eq!(
            extract_markdown(source).await.content,
            "alpha\n\nomega",
            "source: {source}"
        );
    }
    assert_eq!(extract_markdown("- alpha\n- omega").await.content, "alpha\nomega");
}

#[tokio::test]
async fn should_keep_image_alt_text_inline_without_its_destination() {
    let result = extract_markdown("alpha ![Logo](https://example.com/i.png) omega").await;
    assert_eq!(result.content, "alpha Logo omega");
    let uris = result.uris.expect("Image destination should remain available");
    assert_eq!(uris.len(), 1);
    assert_eq!(uris[0].url, "https://example.com/i.png");
    assert_eq!(uris[0].label.as_deref(), Some("Logo"));
    assert!(result.images.is_none_or(|images| images.is_empty()));
}

#[tokio::test]
async fn should_omit_empty_linked_image_without_joining_or_doubling_words() {
    let source = "alpha [![](https://example.com/i.png)](https://example.com/x) omega";
    let result = extract_markdown(source).await;
    assert_eq!(result.content, "alpha omega");
    let uris = result
        .uris
        .expect("Image and link destinations should remain available");
    assert_eq!(uris.len(), 2);
    assert_eq!(uris[0].url, "https://example.com/i.png");
    assert_eq!(uris[1].url, "https://example.com/x");
}

#[tokio::test]
async fn should_preserve_literal_brackets_entities_code_and_source_spacing() {
    for (source, expected) in [
        ("alpha  omega", "alpha  omega"),
        (
            r"alpha \[literal\] &amp; `[code]` omega",
            "alpha [literal] & [code] omega",
        ),
        ("alpha  \nomega", "alpha omega"),
        ("    alpha  omega", "alpha  omega"),
    ] {
        assert_eq!(extract_markdown(source).await.content, expected, "source: {source}");
    }
}

#[tokio::test]
async fn should_keep_explicit_markdown_link_syntax() {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_bytes_document(b"alpha [label](https://example.com/x) omega", "text/markdown", &config)
        .await
        .expect("Markdown output should succeed");
    assert_eq!(result.content, "alpha [label](https://example.com/x) omega\n");
}

#[tokio::test]
async fn should_keep_genuine_wikilinks_alongside_bracketed_standard_links() {
    let result = extract_markdown("[[guide]] [[1]](https://example.com/#fn1) [[target|label]]").await;
    assert_eq!(result.content, "guide [1] label");
    let uris = result.uris.expect("All link destinations should remain available");
    assert_eq!(
        uris.iter().map(|uri| uri.url.as_str()).collect::<Vec<_>>(),
        ["guide", "https://example.com/#fn1", "target"]
    );
}

#[tokio::test]
async fn should_keep_decoded_image_bytes_and_description_with_inline_alt_text() {
    let source = "alpha ![Logo](data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==) omega";
    let result = extract_markdown(source).await;
    assert_eq!(result.content, "alpha Logo omega");
    let images = result.images.expect("The data URI should still decode");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].format, "png");
    assert_eq!(images[0].description.as_deref(), Some("Logo"));
    let expected = base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==")
        .expect("The known PNG fixture should decode");
    assert_eq!(images[0].data.as_ref(), expected);
}

#[tokio::test]
async fn should_keep_explicit_markdown_image_representation() {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };
    let result = extract_bytes_document(
        b"alpha ![Logo](https://example.com/i.png) omega",
        "text/markdown",
        &config,
    )
    .await
    .expect("Markdown output should succeed");
    assert_eq!(
        result.content,
        "[Image: Logo (https://example.com/i.png)]\n\nalpha omega\n"
    );
}

#[tokio::test]
async fn should_keep_nested_list_paragraph_boundaries() {
    let source = "- alpha\n\n  omega\n  - child\n\n    tail";
    assert_eq!(extract_markdown(source).await.content, "alpha\n\nomega\nchild\n\ntail");
}

#[tokio::test]
async fn should_preserve_spaces_around_visible_image_alt_text() {
    let source = "alpha  ![Logo](https://example.com/i.png)  omega";
    assert_eq!(extract_markdown(source).await.content, "alpha  Logo  omega");
}

#[tokio::test]
async fn should_keep_image_projection_internal_to_the_document() {
    let config = ExtractionConfig {
        include_document_structure: true,
        ..Default::default()
    };
    let result = extract_bytes_document(
        b"alpha ![Logo](https://example.com/i.png) omega",
        "text/markdown",
        &config,
    )
    .await
    .expect("Structured Markdown extraction should succeed");
    assert_eq!(result.content, "alpha Logo omega");
    let structure = result.document.expect("The requested structure should be present");
    assert_eq!(structure.nodes.len(), 2);
    assert!(structure.nodes.iter().any(|node| matches!(&node.content,
        xberg::types::document_structure::NodeContent::Paragraph { text } if text == "alpha  omega"
    )));
    for node in structure.nodes {
        assert!(
            node.attributes
                .is_none_or(|attributes| attributes.keys().all(|key| !key.starts_with("xberg:internal:")))
        );
    }
}

#[tokio::test]
async fn should_attach_footnote_to_an_image_only_paragraph() {
    let result = extract_markdown("![Logo](https://example.com/i.png)[^a]\n\n[^a]: note").await;
    assert_eq!(result.content, "Logo[^1]\n\n\n[^1]: note");
}

#[tokio::test]
async fn should_preserve_image_representation_outside_list_text_buffers() {
    for (source, expected) in [
        (
            "- alpha\n\n  # ![Logo](https://example.com/i.png)\n\n  omega",
            "alpha\n  [Image: https://example.com/i.png]\n\n  Logo\nomega",
        ),
        (
            "- alpha\n\n  | name |\n  | --- |\n  | ![Logo](https://example.com/i.png) |\n\n  omega",
            "alpha\n  [Image: Logo (https://example.com/i.png)]\n\nname\n\n\nomega",
        ),
        (
            "- alpha\n\n[^a]: ![Logo](https://example.com/i.png)\n\n  omega",
            "alpha\n[Image: Logo (https://example.com/i.png)]\n\nomega",
        ),
    ] {
        let result = extract_markdown(source).await;
        assert_eq!(result.content, expected, "source: {source}");
        assert_eq!(result.uris.expect("The source has an image URI").len(), 1);
    }
}

#[tokio::test]
async fn should_keep_adjacent_image_labels_and_empty_image_bridges_in_source_order() {
    for (source, expected) in [
        ("alpha ![](a)![Logo](b) omega", "alpha Logo omega"),
        ("alpha![A](a)![B](b)omega", "alphaABomega"),
        ("alpha ![](a)![](b) omega", "alpha omega"),
    ] {
        assert_eq!(extract_markdown(source).await.content, expected, "source: {source}");
    }
}
