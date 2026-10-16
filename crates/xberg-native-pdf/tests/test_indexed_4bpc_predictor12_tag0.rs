//! Decoders must respect the per-row PNG predictor tag byte instead of
//! assuming the numeric /Predictor value applies uniformly. A PDF may
//! declare `/Predictor 12` (Up) on the FlateDecode parameters while
//! writing tag 0 (None) on every row of the 4-bit-per-component
//! Indexed index stream; decoding as if every row were Up-predicted
//! would produce a vertical-cascade noise pattern instead of the
//! intended image.

use flate2::Compression;
use flate2::write::ZlibEncoder;
use std::io::Write;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::ImageData;

fn make_4bpc_indexed_pdf_with_predictor12_tag0() -> Vec<u8> {
    let w: u32 = 10;
    let h: u32 = 10;
    // 4 bits per component → 5 bytes per row of indices, 10 rows. ~keep
    let bytes_per_row = (w as usize * 4).div_ceil(8);
    let mut raw = Vec::with_capacity(h as usize * (bytes_per_row + 1));
    for _ in 0..h {
        raw.push(0u8);
        // All nibbles set to 0xF → every pixel picks palette index 0xF. ~keep
        raw.extend(std::iter::repeat_n(0xFFu8, bytes_per_row));
    }
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&raw).unwrap();
    let compressed = enc.finish().unwrap();

    // 16-entry palette, DeviceRGB. Entries 0..14 are (0,0,0); entry 15 is
    // pure red (255, 0, 0). After correct decoding every pixel resolves
    // to entry 0xF → red. An Up-cascade on rows 1..H would shift the
    // byte values away from 0xFF, breaking the alignment between index
    // and palette entry. ~keep
    let mut palette = vec![0u8; 15 * 3];
    palette.extend_from_slice(&[255, 0, 0]);

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
           /ColorSpace 5 0 R /BitsPerComponent 4 /Filter /FlateDecode \
           /DecodeParms << /Predictor 12 /Colors 1 /BitsPerComponent 4 /Columns {w} >> \
           /Length {} >>",
        compressed.len()
    );
    push_stream(&mut out, &mut offsets, &image_dict, &compressed);

    let palette_hex: String = palette.iter().map(|b| format!("{b:02X}")).collect();
    let cs_array = format!("[/Indexed /DeviceRGB 15 <{palette_hex}>]");
    push_simple(&mut out, &mut offsets, cs_array.as_bytes());

    let cs = b"q 10 0 0 10 0 0 cm /Im0 Do Q";
    let cs_dict = format!("<< /Length {} >>", cs.len());
    push_stream(&mut out, &mut offsets, &cs_dict, cs);

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
fn indexed_4bpc_predictor12_tag0_decodes_raw_rows() {
    let pdf = make_4bpc_indexed_pdf_with_predictor12_tag0();
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
        ImageData::Jpeg(_) => panic!("expected raw pixels"),
    };

    assert_eq!(pixels.len(), 10 * 10 * 3);
    // Every nibble is 0xF, every row tag is 0 (None). With the per-row
    // tag honoured, decoding yields raw nibble stream → every pixel
    // maps to palette entry 15 = pure red. An Up-cascade on rows 1..H
    // would drift pixels away from (255,0,0). ~keep
    for chunk in pixels.as_chunks::<3>().0 {
        assert_eq!(chunk.as_slice(), [255, 0, 0], "every pixel should be pure red");
    }
}
