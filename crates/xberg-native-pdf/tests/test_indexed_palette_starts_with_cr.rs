//! An Indexed color space whose palette lookup stream legitimately
//! begins with 0x0D (CR) must not have that byte stripped as a
//! spurious EOL. A 1-entry CMYK palette `0d 0c 0c 04` (light warm
//! grey) must decode to near-white RGB, not be shrunk to 3 bytes and
//! fall into the expander's out-of-range branch — that would render
//! every pixel as solid black.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::ImageData;

/// Minimal 10×10 Indexed PDF with a single DeviceCMYK palette entry
/// `0x0D 0x0C 0x0C 0x04`. The lookup stream's first byte is 0x0D —
/// a naive post-parse CR/LF trimmer would silently drop it.
fn indexed_cmyk_palette_leading_cr_pdf() -> Vec<u8> {
    let w: u32 = 10;
    let h: u32 = 10;
    let image_bytes: Vec<u8> = vec![0u8; (w * h) as usize];
    let palette_bytes: &[u8] = &[0x0D, 0x0C, 0x0C, 0x04];

    let mut out: Vec<u8> = Vec::new();
    let mut offsets: Vec<usize> = vec![0];
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let push_simple = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
        offsets.push(out.len());
        let id = offsets.len() - 1;
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };

    let push_stream = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, dict: &str, stream_bytes: &[u8]| {
        offsets.push(out.len());
        let id = offsets.len() - 1;
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(dict.as_bytes());
        out.extend_from_slice(b"\nstream\n");
        out.extend_from_slice(stream_bytes);
        out.extend_from_slice(b"\nendstream\nendobj\n");
    };

    push_simple(&mut out, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    push_simple(&mut out, &mut offsets, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    push_simple(
        &mut out,
        &mut offsets,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources << /XObject << /Im0 4 0 R >> >> /Contents 6 0 R >>",
    );

    let image_dict = format!(
        "<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
           /ColorSpace 5 0 R /BitsPerComponent 8 /Length {} >>",
        image_bytes.len()
    );
    push_stream(&mut out, &mut offsets, &image_dict, &image_bytes);

    push_simple(&mut out, &mut offsets, b"[/Indexed /DeviceCMYK 0 7 0 R]");

    let cs = b"q 10 0 0 10 0 0 cm /Im0 Do Q";
    let cs_dict = format!("<< /Length {} >>", cs.len());
    push_stream(&mut out, &mut offsets, &cs_dict, cs);

    let pal_dict = format!("<< /Length {} >>", palette_bytes.len());
    push_stream(&mut out, &mut offsets, &pal_dict, palette_bytes);

    let xref_offset = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for &off in &offsets[1..] {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    out
}

#[test]
fn indexed_cmyk_palette_leading_cr_preserved() {
    let pdf = indexed_cmyk_palette_leading_cr_pdf();
    let tmp = tempfile::NamedTempFile::new().expect("temp");
    std::fs::write(tmp.path(), &pdf).unwrap();

    let doc = PdfDocument::open(tmp.path()).expect("open");
    let images = doc.extract_images(0).expect("extract images");
    let img = images
        .iter()
        .find(|img| img.width() == 10 && img.height() == 10)
        .expect("10x10 image present");

    let pixels = match img.data() {
        ImageData::Raw { pixels, .. } => pixels.clone(),
        ImageData::Jpeg(_) => panic!("expected raw-pixel image, got JPEG"),
    };

    // Each pixel maps to palette index 0 = CMYK(13,12,12,4) → near-white RGB.
    // If `trim_leading_stream_whitespace`-style logic stripped the 0x0D
    // byte from the palette stream, the palette would be only 3 bytes,
    // the expander would take the out-of-range branch, and every pixel
    // would be (0,0,0) black. ~keep
    assert_eq!(pixels.len(), 10 * 10 * 3, "expected RGB pixels");
    let r = pixels[0];
    let g = pixels[1];
    let b = pixels[2];
    assert!(
        r > 220 && g > 220 && b > 220,
        "pixel ({r},{g},{b}) should be near-white from CMYK(13,12,12,4); a black \
         result indicates the palette's leading 0x0D byte was stripped"
    );
    for chunk in pixels.as_chunks::<3>().0 {
        assert_eq!(chunk[0], r);
        assert_eq!(chunk[1], g);
        assert_eq!(chunk[2], b);
    }
}
