//! Integration tests for the enumerate-then-materialize image API.
//!
//! Verifies that `page_image_handles` returns correct metadata without
//! decompressing streams, and that `decode()` / `raw_compressed_bytes()`
//! materialise the image on demand.

mod common;

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::images::PdfFilter;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const MINIMAL_JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
    0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08,
    0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A,
    0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31, 0x34,
    0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00,
    0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
    0x0A, 0x0B, 0xFF, 0xD9,
];

/// Build a minimal PDF whose page paints the given JPEG placements
/// (`(x, y, width, height)` -- the on-page display rectangle) as Image
/// XObjects `Im0..Im{n-1}`, in content-stream order. Each XObject
/// dictionary declares `/Width 1 /Height 1` (MINIMAL_JPEG's true decoded
/// dimensions, per its own SOF marker) -- the placement `width`/`height`
/// only scale the `cm` transform used to paint it. This mirrors how the
/// (now-removed) writer's JPEG-embedding path re-derived the declared
/// XObject dimensions from the JPEG's own header rather than from any
/// caller-supplied `ImageContent::new(..., width, height)` size --
/// `page_image_handles_returns_one_handle_for_single_jpeg` below asserts
/// exactly that: it places a 100x80 rect but still expects a 1x1 handle.
fn build_pdf_with_jpegs(placements: &[(f32, f32, f32, f32)]) -> Vec<u8> {
    let mut content = String::new();
    for (i, (x, y, w, h)) in placements.iter().enumerate() {
        content.push_str(&format!("q {w} 0 0 {h} {x} {y} cm /Im{i} Do Q\n"));
    }

    let mut xobject_entries = String::new();
    for i in 0..placements.len() {
        xobject_entries.push_str(&format!("/Im{i} {} 0 R ", 5 + i));
    }

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Contents 4 0 R /Resources << /XObject << {xobject_entries}>> >> >>\nendobj\n"
        )
        .as_bytes(),
    );

    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    for i in 0..placements.len() {
        debug_assert_eq!(offsets.len(), 5 + i);
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
                 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
                5 + i,
                MINIMAL_JPEG.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(MINIMAL_JPEG);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    common::finalize_pdf(pdf, &offsets)
}

/// Build a minimal PDF containing a single JPEG image on page 0.
fn build_pdf_with_jpeg(width: u32, height: u32) -> Vec<u8> {
    build_pdf_with_jpegs(&[(0.0, 0.0, width as f32, height as f32)])
}

fn build_pdf_with_referenced_mask() -> Vec<u8> {
    build_pdf_with_referenced_mask_options(false, true)
}

fn build_pdf_with_referenced_mask_decode(inverted: bool) -> Vec<u8> {
    build_pdf_with_referenced_mask_options(inverted, true)
}

fn build_pdf_with_referenced_mask_options(inverted: bool, include_bits_per_component: bool) -> Vec<u8> {
    let content = b"q 100 0 0 100 0 0 cm /Im0 Do Q";
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
          /Resources << /XObject << /Im0 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
          /BitsPerComponent 8 /Mask 6 0 R /Length 1 >>\nstream\n\x80\nendstream\nendobj\n",
    );
    offsets.push(pdf.len());
    let (decode, sample) = if inverted { (" /Decode [1 0]", 0x80) } else { ("", 0x00) };
    let bits_per_component = if include_bits_per_component {
        " /BitsPerComponent 1"
    } else {
        ""
    };
    pdf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ImageMask true \
             {bits_per_component}{decode} /Length 1 >>\nstream\n"
        )
        .as_bytes(),
    );
    pdf.push(sample);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    common::finalize_pdf(pdf, &offsets)
}

#[test]
fn page_image_handles_returns_one_handle_for_single_jpeg() {
    let pdf_bytes = build_pdf_with_jpeg(100, 80);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");

    let handles = doc.page_image_handles(0).expect("page_image_handles");

    assert_eq!(handles.len(), 1, "expected exactly one image handle");
    let h = &handles[0];
    assert_eq!(h.width, 1);
    assert_eq!(h.height, 1);
    assert!(!h.is_inline);
    assert_eq!(h.paint_order, 0);
}

#[test]
fn page_image_handles_includes_referenced_mask() {
    let doc = PdfDocument::from_bytes(build_pdf_with_referenced_mask()).expect("open PDF");

    let handles = doc.page_image_handles(0).expect("enumerate page image handles");

    assert_eq!(
        handles.len(),
        2,
        "the image and its referenced /Mask must both be OCR candidates"
    );
    assert_eq!((handles[0].width, handles[0].height), (1, 1));
    assert_eq!((handles[1].width, handles[1].height), (1, 1));
}

/// GH#1825: an `/ImageMask` legitimately omits `/ColorSpace` (ISO 32000-1 §8.9.6.2 forbids
/// it), and `resolve_image_xobject_metadata` defaults such an image to `/DeviceGray` before
/// decoding. The handle builder must report the same placeholder rather than the plain
/// missing-`/ColorSpace` default of `/DeviceRGB`, or a handle and the image it later decodes
/// disagree about colour space for every mask in the wild.
#[test]
fn page_image_handles_mask_with_no_colorspace_reports_device_gray() {
    use xberg_native_pdf::extractors::ColorSpace;

    let doc = PdfDocument::from_bytes(build_pdf_with_referenced_mask()).expect("open PDF");
    let handles = doc.page_image_handles(0).expect("enumerate page image handles");

    assert_eq!(
        handles[1].color_space,
        ColorSpace::DeviceGray,
        "an /ImageMask with no /ColorSpace must report DeviceGray, matching extraction's own \
         default_mask_color_space placeholder, not the plain DeviceRGB default"
    );
}

#[test]
fn referenced_image_mask_defaults_missing_bits_per_component_to_one() {
    let doc = PdfDocument::from_bytes(build_pdf_with_referenced_mask_options(false, false)).expect("open PDF");

    let handles = doc.page_image_handles(0).expect("enumerate page image handles");

    assert_eq!(
        handles[1].bits_per_component, 1,
        "ImageMask defaults to one bit per component"
    );
    handles[1]
        .decode()
        .expect("decode ImageMask with omitted BitsPerComponent");
}

#[test]
fn referenced_image_mask_zero_sample_keeps_base_pixel_opaque() {
    let doc = PdfDocument::from_bytes(build_pdf_with_referenced_mask()).expect("open PDF");

    let rendered = render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("render masked image");
    let center = ((50 * rendered.width + 50) * 4) as usize;
    let pixel = &rendered.data[center..center + 4];

    assert!(
        pixel[0] < 200 && pixel[1] < 200 && pixel[2] < 200,
        "the mask's zero sample is ink and must preserve the gray base pixel, got {pixel:?}"
    );
}

#[test]
fn referenced_image_mask_inverted_decode_one_sample_keeps_base_pixel_opaque() {
    let doc = PdfDocument::from_bytes(build_pdf_with_referenced_mask_decode(true)).expect("open PDF");

    let rendered = render_page(&doc, 0, &RenderOptions::with_dpi(72).as_raw()).expect("render masked image");
    let center = ((50 * rendered.width + 50) * 4) as usize;
    let pixel = &rendered.data[center..center + 4];

    assert!(
        pixel[0] < 200 && pixel[1] < 200 && pixel[2] < 200,
        "the inverted mask's one sample must preserve the gray base pixel, got {pixel:?}"
    );
}

#[test]
fn page_image_handles_jpeg_has_dct_filter() {
    let pdf_bytes = build_pdf_with_jpeg(50, 50);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");

    let handles = doc.page_image_handles(0).expect("page_image_handles");
    assert!(!handles.is_empty());

    let h = &handles[0];
    assert!(
        h.filter_chain.contains(&PdfFilter::DCTDecode),
        "JPEG XObject must report DCTDecode in filter_chain, got {:?}",
        h.filter_chain
    );
}

#[test]
fn page_image_handles_decode_produces_valid_image() {
    let pdf_bytes = build_pdf_with_jpeg(1, 1);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");

    let handles = doc.page_image_handles(0).expect("page_image_handles");
    assert_eq!(handles.len(), 1);

    let handle = handles.into_iter().next().unwrap();
    let image = handle.decode().expect("decode");

    assert_eq!(image.width(), 1);
    assert_eq!(image.height(), 1);
}

#[test]
fn page_image_handles_raw_compressed_bytes_non_empty() {
    let pdf_bytes = build_pdf_with_jpeg(1, 1);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");

    let handles = doc.page_image_handles(0).expect("page_image_handles");
    let handle = handles.into_iter().next().expect("handle");

    let raw = handle.raw_compressed_bytes().expect("raw bytes");
    assert!(!raw.is_empty(), "raw compressed bytes must be non-empty");
}

#[test]
fn page_image_handles_filter_then_decode_skips_small_images() {
    let pdf_bytes = build_pdf_with_jpeg(200, 200);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");

    let handles = doc.page_image_handles(0).expect("page_image_handles");

    let decoded: Vec<_> = handles
        .into_iter()
        .filter(|h| h.width >= 100 && h.height >= 100)
        .map(|h| h.decode())
        .collect::<Result<_, _>>()
        .expect("decode");

    assert_eq!(decoded.len(), 0);
}

#[test]
fn page_image_handles_empty_page_returns_empty_vec() {
    let pdf_bytes = common::build_minimal_pdf_raw(b"", b"/Type /Page /Parent 2 0 R /MediaBox [0 0 595 842]");

    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");
    let handles = doc.page_image_handles(0).expect("page_image_handles");

    assert!(handles.is_empty(), "empty page must yield zero handles");
}

#[test]
fn pdf_filter_from_name_roundtrip() {
    assert_eq!(PdfFilter::from_name("DCTDecode"), PdfFilter::DCTDecode);
    assert_eq!(PdfFilter::from_name("DCT"), PdfFilter::DCTDecode);
    assert_eq!(PdfFilter::from_name("FlateDecode"), PdfFilter::FlateDecode);
    assert_eq!(PdfFilter::from_name("Fl"), PdfFilter::FlateDecode);
    assert_eq!(PdfFilter::from_name("JPXDecode"), PdfFilter::JPXDecode);
    assert_eq!(PdfFilter::from_name("LZWDecode"), PdfFilter::LZWDecode);
    assert_eq!(PdfFilter::from_name("CCITTFaxDecode"), PdfFilter::CCITTFaxDecode);
    assert_eq!(
        PdfFilter::from_name("UnknownFilter"),
        PdfFilter::Other("UnknownFilter".to_string())
    );
}

/// Build a minimal PDF whose page Resources/XObject dictionary contains a
/// Form XObject (not an Image XObject).  The PDF has no actual image content
/// inside the Form; `page_image_handles` correctly returns an empty vec because
/// there are no Image XObjects reachable from the page content (recursion into
/// the Form is performed, but the Form itself contains nothing paintable).
fn build_pdf_with_form_xobject_only() -> Vec<u8> {
    // We construct the PDF bytes manually so we can place a /Subtype /Form
    // XObject in the resources without using the writer (which only writes
    // Image XObjects).
    //
    // Object layout:
    //   1 Catalog → 2
    //   2 Pages   → [3]
    //   3 Page    → Resources: { XObject: { Im0: 5 0 R } }, Contents: 4 0 R
    //   4 Content stream  (just a `Do` operator that paints the form)
    //   5 Form XObject stream  (/Type /XObject /Subtype /Form) ~keep
    let content = b"q /Im0 Do Q";
    let form_stream_data = b"";
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>\nendobj\n",
    );

    let off4 = pdf.len();
    let content_str = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    pdf.extend_from_slice(content_str.as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off5 = pdf.len();
    let form_str = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\nstream\n",
        form_stream_data.len()
    );
    pdf.extend_from_slice(form_str.as_bytes());
    pdf.extend_from_slice(form_stream_data);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    pdf
}

#[test]
fn form_xobject_does_not_produce_image_handle() {
    let pdf_bytes = build_pdf_with_form_xobject_only();
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");
    let handles = doc.page_image_handles(0).expect("page_image_handles");
    assert_eq!(
        handles.len(),
        0,
        "Form with no images inside must yield 0 handles; got {}",
        handles.len()
    );
}

// ── Test B: N images → paint_order is [0, 1, 2] ────────────────────────────── ~keep

fn build_pdf_with_n_jpegs(n: usize) -> Vec<u8> {
    let placements: Vec<(f32, f32, f32, f32)> = (0..n).map(|i| ((i as f32) * 50.0, 0.0, 40.0, 40.0)).collect();
    build_pdf_with_jpegs(&placements)
}

#[test]
fn three_images_have_paint_order_zero_one_two() {
    let pdf_bytes = build_pdf_with_n_jpegs(3);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");
    let handles = doc.page_image_handles(0).expect("page_image_handles");

    assert_eq!(handles.len(), 3, "expected 3 handles");
    let paint_orders: Vec<usize> = handles.iter().map(|h| h.paint_order).collect();
    assert_eq!(
        paint_orders,
        vec![0, 1, 2],
        "paint_order values must be [0, 1, 2] in content-stream order, got {:?}",
        paint_orders
    );
}

#[test]
fn raw_compressed_bytes_starts_with_jpeg_soi_marker() {
    let pdf_bytes = build_pdf_with_jpeg(1, 1);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");
    let handles = doc.page_image_handles(0).expect("page_image_handles");
    let handle = handles.into_iter().next().expect("handle");

    let raw = handle.raw_compressed_bytes().expect("raw bytes");
    assert!(
        raw.len() >= 2,
        "raw bytes must be at least 2 bytes for SOI check, got {}",
        raw.len()
    );
    assert_eq!(
        &raw[..2],
        &[0xFF, 0xD8],
        "JPEG raw bytes must start with SOI marker FF D8, got {:02X} {:02X}",
        raw[0],
        raw[1]
    );
}

#[test]
fn out_of_bounds_page_index_returns_err() {
    let pdf_bytes = build_pdf_with_jpeg(1, 1);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");
    // The PDF has only 1 page (index 0); index 999 must be an error. ~keep
    let result = doc.page_image_handles(999);
    assert!(
        result.is_err(),
        "page_image_handles(999) on a 1-page PDF must return Err, got Ok"
    );
}

// The xberg-native-pdf writer does not currently expose an API for inserting inline
// images (BI/ID/EI sequences) into content streams. Constructing a correct
// inline-image PDF by hand requires matching the exact byte offsets expected
// by the content-stream parser, which is fragile without writer support.
//
// This test is left as an `#[ignore]` placeholder until the writer gains
// first-class inline-image support, at which point it should be filled in
// following the pattern of the decode test above. ~keep
#[test]
#[ignore = "inline-image writer support not yet available; fill in once PdfWriter exposes BI/ID/EI"]
fn inline_image_decode_produces_valid_image() {}

/// Build a minimal PDF where the page paints a Form XObject, and the Form
/// itself paints a real Image XObject (the 1×1 MINIMAL_JPEG).
///
/// This exercises the new `page_image_handles` Form recursion path:
///   Page content Do → Form → (Form Resources) Image Do
/// The returned handle must have correct paint_order, dimensions, filter chain,
/// and decode() must succeed.
fn build_pdf_with_image_inside_form() -> Vec<u8> {
    // Object layout (all generation 0):
    //   1 Catalog
    //   2 Pages (Kids=[3])
    //   3 Page (Resources: XObject → Fm0 6 0 R, Contents 4 0 R)
    //   4 Page content stream:  "q /Fm0 Do Q"
    //   5 Image XObject (MINIMAL_JPEG, 1×1)
    //   6 Form XObject (Resources: XObject → Im0 5 0 R, content: "q /Im0 Do Q") ~keep
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Contents 4 0 R /Resources << /XObject << /Fm0 6 0 R >> >> >>\nendobj\n",
    );

    let page_content = b"q /Fm0 Do Q";
    let off4 = pdf.len();
    let content_str = format!("4 0 obj\n<< /Length {} >>\nstream\n", page_content.len());
    pdf.extend_from_slice(content_str.as_bytes());
    pdf.extend_from_slice(page_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let img_data = MINIMAL_JPEG;
    let off5 = pdf.len();
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
        img_data.len()
    );
    pdf.extend_from_slice(img_hdr.as_bytes());
    pdf.extend_from_slice(img_data);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let form_content = b"q /Im0 Do Q";
    let form_res = b"<< /XObject << /Im0 5 0 R >> >>";
    let off6 = pdf.len();
    let form_hdr = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources {} /Length {} >>\nstream\n",
        String::from_utf8_lossy(form_res),
        form_content.len()
    );
    pdf.extend_from_slice(form_hdr.as_bytes());
    pdf.extend_from_slice(form_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off6).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    pdf
}

/// Positive test: images inside Form XObjects are now enumerated by
/// page_image_handles (the core requirement from the maintainer review).
#[test]
fn form_xobject_recursion_returns_inner_image_handles() {
    let pdf_bytes = build_pdf_with_image_inside_form();
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");

    let handles = doc.page_image_handles(0).expect("page_image_handles");

    assert_eq!(
        handles.len(),
        1,
        "expected exactly one image handle from inside the Form; got {}",
        handles.len()
    );

    let h0 = handles
        .into_iter()
        .next()
        .expect("at least one handle from Form recursion");
    assert_eq!(h0.paint_order, 0, "the single image must have paint_order 0");
    assert_eq!(h0.width, 1);
    assert_eq!(h0.height, 1);
    assert!(
        h0.filter_chain.contains(&PdfFilter::DCTDecode),
        "expected DCTDecode filter for the JPEG inside the Form, got {:?}",
        h0.filter_chain
    );

    let decoded = h0.decode().expect("decode image from inside Form");
    assert_eq!(decoded.width(), 1);
    assert_eq!(decoded.height(), 1);
}

/// Basic cycle-safety smoke test for Form recursion.
/// A Form that references itself must not cause infinite recursion or panic.
#[test]
fn form_xobject_self_reference_is_safe() {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Contents 4 0 R /Resources << /XObject << /Fm0 5 0 R >> >> >>\nendobj\n",
    );

    let page_content = b"q /Fm0 Do Q";
    let off4 = pdf.len();
    let c4 = format!("4 0 obj\n<< /Length {} >>\nstream\n", page_content.len());
    pdf.extend_from_slice(c4.as_bytes());
    pdf.extend_from_slice(page_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let form_res = b"<< /XObject << /Fm0 5 0 R >> >>";
    let form_stream = b"";
    let off5 = pdf.len();
    let c5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources {} /Length {} >>\nstream\n",
        String::from_utf8_lossy(form_res),
        form_stream.len()
    );
    pdf.extend_from_slice(c5.as_bytes());
    pdf.extend_from_slice(form_stream);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in [off1, off2, off3, off4, off5] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).expect("open self-referential Form PDF");
    let handles = doc.page_image_handles(0).expect("page_image_handles on cyclic Form");
    assert_eq!(
        handles.len(),
        0,
        "self-referential Form with no images must yield 0 handles"
    );
}

#[test]
fn handle_methods_take_ref_and_clone() {
    // 8c: decode() / raw_compressed_bytes() borrow &self, so a single handle
    // supports a two-phase inspect → raw → decode flow, and PdfImageHandle is
    // Clone for ergonomic filtering/retaining. ~keep
    let pdf_bytes = build_pdf_with_jpeg(64, 64);
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("open PDF");
    let handles = doc.page_image_handles(0).expect("handles");
    let h = &handles[0];

    let raw = h.raw_compressed_bytes().expect("raw bytes");
    assert!(!raw.is_empty(), "raw JPEG bytes must be present");
    assert!(h.decode().is_ok(), "decode on &handle must succeed");

    let h2 = h.clone();
    assert_eq!(h2.width, h.width);
    assert!(h.raw_compressed_bytes().is_ok(), "original handle still usable");
}

/// Build a one-page PDF with a 1x1 JPEG (object 5, `/Im0`) and Form XObjects
/// `/Fm0..` (objects 6..). Each form is `(matrix, content, xobject_resources)`.
fn build_pdf_with_forms(page_content: &str, forms: &[(&str, &str, &str)]) -> Vec<u8> {
    let mut page_xobjects = String::from("/Im0 5 0 R ");
    for i in 0..forms.len() {
        page_xobjects.push_str(&format!("/Fm{i} {} 0 R ", 6 + i));
    }

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
             /Resources << /XObject << {page_xobjects}>> >> >>\nendobj\n"
        )
        .as_bytes(),
    );
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", page_content.len()).as_bytes());
    pdf.extend_from_slice(page_content.as_bytes());
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            MINIMAL_JPEG.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(MINIMAL_JPEG);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    for (i, (matrix, content, resources)) in forms.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 1000 1000] /Matrix [{matrix}] \
                 /Resources << /XObject << {resources} >> >> /Length {} >>\nstream\n",
                6 + i,
                content.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(content.as_bytes());
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    common::finalize_pdf(pdf, &offsets)
}

/// `(x0, y0, x1, y1)` of every image handle on page 0, in paint order.
fn handle_boxes(pdf: Vec<u8>) -> Vec<(f32, f32, f32, f32)> {
    let doc = PdfDocument::from_bytes(pdf).expect("open PDF");
    let mut handles = doc.page_image_handles(0).expect("page_image_handles");
    handles.sort_by_key(|h| h.paint_order);
    handles
        .iter()
        .map(|h| (h.bbox.x, h.bbox.y, h.bbox.x + h.bbox.width, h.bbox.y + h.bbox.height))
        .collect()
}

fn assert_box(actual: (f32, f32, f32, f32), expected: (f32, f32, f32, f32), what: &str) {
    let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
    assert!(
        close(actual.0, expected.0)
            && close(actual.1, expected.1)
            && close(actual.2, expected.2)
            && close(actual.3, expected.3),
        "{what}: expected {expected:?}, got {actual:?}"
    );
}

/// GH#2103: the form `/Matrix` applies before the CTM in effect at `Do`
/// (ISO 32000-1 §8.10.1), so a form drawn at `250 500` must not have its
/// translation scaled by the form's own `/Matrix`.
#[test]
fn form_image_bbox_applies_form_matrix_before_ctm() {
    let pdf = build_pdf_with_forms(
        "q 2 0 0 2 250 500 cm /Fm0 Do Q\nq 200 0 0 150 50 100 cm /Im0 Do Q",
        &[("0.25 0 0 0.25 0 0", "q 400 0 0 300 0 0 cm /Im0 Do Q", "/Im0 5 0 R")],
    );
    let boxes = handle_boxes(pdf);
    assert_eq!(boxes.len(), 2, "form image plus direct image");
    assert_box(boxes[0], (250.0, 500.0, 450.0, 650.0), "form image");
    assert_box(boxes[1], (50.0, 100.0, 250.0, 250.0), "direct image");
}

/// A form `/Matrix` with both scale and translation, so that composing it with
/// the CTM in the wrong order cannot coincide with the right answer.
#[test]
fn form_image_bbox_with_scaled_and_translated_form_matrix() {
    let pdf = build_pdf_with_forms(
        "q 2 0 0 3 100 200 cm /Fm0 Do Q",
        &[("0.5 0 0 0.5 10 20", "q 40 0 0 30 0 0 cm /Im0 Do Q", "/Im0 5 0 R")],
    );
    let boxes = handle_boxes(pdf);
    assert_eq!(boxes.len(), 1);
    assert_box(boxes[0], (120.0, 260.0, 160.0, 305.0), "form image");
}

#[test]
fn nested_form_image_bbox_composes_each_form_matrix_before_its_ctm() {
    let pdf = build_pdf_with_forms(
        "q 2 0 0 3 100 200 cm /Fm0 Do Q",
        &[
            ("0.5 0 0 0.5 10 20", "q 2 0 0 2 5 5 cm /Fm1 Do Q", "/Fm1 7 0 R"),
            ("0.5 0 0 0.5 3 7", "q 40 0 0 30 0 0 cm /Im0 Do Q", "/Im0 5 0 R"),
        ],
    );
    let boxes = handle_boxes(pdf);
    assert_eq!(boxes.len(), 1);
    assert_box(boxes[0], (131.0, 288.5, 171.0, 333.5), "nested form image");
}
