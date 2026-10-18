//! Shared utilities for Markdown and MDX extractors.
//!
//! Provides the data-URI image decoding used by both the Markdown and MDX extractors while they
//! build their `InternalDocument`, and the footnote-reference queue they share with the Djot
//! extractor.

use crate::types::ExtractedImage;
use crate::types::internal_builder::InternalDocumentBuilder;
use base64::Engine;
use bytes::Bytes;
use std::borrow::Cow;

/// The open block a footnote reference was read in: the block whose text surrounds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FootnoteRefHolder {
    Heading,
    Paragraph,
    /// A list item at this nesting depth, so a sublist item never takes its parent's references.
    ListItem(usize),
    Table,
    DefinitionTerm,
    DefinitionDescription,
}

/// Footnote references waiting for the block that holds them.
///
/// These extractors push a block when it closes, and renderers attach a `FootnoteRef` to the
/// block it follows. A reference pushed the moment it was read therefore landed on the block
/// before. Each reference waits for its own holder: a list item's reference still follows that
/// item when a heading or a table nested in it closes first.
#[derive(Debug, Default)]
pub(crate) struct PendingFootnoteRefs(Vec<(FootnoteRefHolder, String)>);

impl PendingFootnoteRefs {
    pub(crate) fn hold(&mut self, holder: FootnoteRefHolder, name: &str) {
        self.0.push((holder, name.to_string()));
    }

    /// Push the references read in `holder`, in reading order, once `holder` itself is pushed.
    pub(crate) fn flush(&mut self, builder: &mut InternalDocumentBuilder, holder: FootnoteRefHolder) {
        self.0.retain(|(owner, name)| {
            if *owner != holder {
                return true;
            }
            builder.push_footnote_ref(name, name, None);
            false
        });
    }

    /// Push every reference still waiting, so a holder that never closed loses none.
    pub(crate) fn flush_all(&mut self, builder: &mut InternalDocumentBuilder) {
        for (_, name) in self.0.drain(..) {
            builder.push_footnote_ref(&name, &name, None);
        }
    }
}

/// Decode a data URI into an `ExtractedImage`.
///
/// Supports base64-encoded PNG, JPEG, GIF, WebP, and SVG data URIs.
/// Returns `None` for non-base64 encodings or unsupported formats.
///
/// # Arguments
///
/// * `uri` - The full `data:image/...;base64,...` URI string
/// * `index` - The zero-based image index within the document
pub(crate) fn decode_data_uri_image(uri: &str, index: u32) -> Option<ExtractedImage> {
    let after_data = uri.strip_prefix("data:")?;
    let (mime_and_encoding, data) = after_data.split_once(',')?;

    if !mime_and_encoding.contains("base64") {
        return None;
    }

    // `image/svg+xml` is recognized alongside the raster formats below (issue #145):
    // inline SVG data URIs used to be dropped from Markdown entirely, even though the HTML
    // extractor already handles inline SVG via `InlineImageFormat::Svg`.
    let format: &str = if mime_and_encoding.contains("image/png") {
        "png"
    } else if mime_and_encoding.contains("image/jpeg") {
        "jpeg"
    } else if mime_and_encoding.contains("image/gif") {
        "gif"
    } else if mime_and_encoding.contains("image/webp") {
        "webp"
    } else if mime_and_encoding.contains("image/svg+xml") {
        "svg"
    } else {
        return None;
    };

    let cleaned = data.replace(['\n', '\r'], "");
    let decoded = base64::engine::general_purpose::STANDARD.decode(&cleaned).ok()?;

    let (image_kind, kind_confidence) =
        crate::extraction::image_kind::classify(crate::extraction::image_kind::ImageClassifyInput {
            bytes: &decoded,
            format,
            width: None,
            height: None,
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
        });

    Some(ExtractedImage {
        data: Bytes::from(decoded),
        format: Cow::Borrowed(format),
        image_index: index,
        page_number: None,
        width: None,
        height: None,
        colorspace: None,
        bits_per_component: None,
        is_mask: false,
        description: None,
        ocr_result: None,
        bounding_box: None,
        source_path: None,
        image_kind: Some(image_kind),
        kind_confidence: Some(kind_confidence),
        cluster_id: None,
        caption: None,
        qr_codes: None,
        data_base64: None,
    })
}
