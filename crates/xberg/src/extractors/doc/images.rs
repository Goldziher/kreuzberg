use super::*;

pub(super) fn push_doc_image(doc: &mut InternalDocument, picture: &DocPicture, visible: bool) {
    #[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
    if !visible {
        doc.push_element(InternalElement::image_ocr_text_anchor(picture.image.image_index, None));
        return;
    }
    let _ = visible;
    doc.push_element(InternalElement::text(
        ElementKind::Image {
            image_index: picture.image.image_index,
        },
        "",
        0,
    ));
}

pub(super) fn push_paragraph_runs(
    doc: &mut InternalDocument,
    kind: ElementKind,
    text: &str,
    pictures: &[DocPicture],
    visible_images: bool,
) {
    let mut previous = 0;
    for picture in pictures.iter().filter(|picture| !picture.standalone) {
        let offset = picture.text_offset.min(text.len());
        if let Some(run) = text.get(previous..offset).filter(|run| !run.is_empty()) {
            doc.push_element(InternalElement::text(kind, run, 0));
        }
        push_doc_image(doc, picture, visible_images);
        previous = offset;
    }
    if let Some(run) = text.get(previous..).filter(|run| !run.is_empty()) {
        doc.push_element(InternalElement::text(kind, run, 0));
    }
}

pub(super) fn parse_doc(
    content: &[u8],
    limits: Option<&crate::extractors::security::SecurityLimits>,
) -> Result<DocWithImages> {
    match limits {
        Some(limits) => extract_doc_with_images(content, Some(limits)),
        None => Ok(DocWithImages {
            text: extract_doc_text(content)?,
            pictures: Vec::new(),
        }),
    }
}
