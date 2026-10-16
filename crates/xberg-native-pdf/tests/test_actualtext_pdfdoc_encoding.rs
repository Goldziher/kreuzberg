//! End-to-end tests for /ActualText with PDFDocEncoding non-ASCII bytes.
//!
//! PDF Spec §14.9.4: /ActualText values are PDF text strings (§7.9.2), meaning
//! they use PDFDocEncoding unless they have a UTF-16 BOM.  The text extractor's
//! decode_pdf_text_string must correctly handle PDFDocEncoding bytes (0x80–0xFF)
//! and must not substitute U+FFFD for valid PDFDocEncoding characters.

use xberg_native_pdf::PdfDocument;

/// Build a minimal 1-page PDF where the content stream wraps the visible text
/// glyph sequence in a BDC marked-content operator with /ActualText set to
/// the provided raw bytes (treated as a PDF string literal).
///
/// The content stream produces a glyph using font F1 (Helvetica, /Encoding
/// /WinAnsiEncoding), but the extraction should prefer /ActualText when present.
fn build_pdf_with_actual_text(actual_text_bytes: &[u8]) -> Vec<u8> {
    let hex: String = actual_text_bytes.iter().map(|b| format!("{:02X}", b)).collect();

    let content = format!(
        "/Span << /ActualText <{hex}> >> BDC\n\
         BT /F1 12 Tf 72 720 Td (X) Tj ET\n\
         EMC\n"
    );

    let font_obj = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
                    /Encoding /WinAnsiEncoding >>";

    let mut out: Vec<u8> = Vec::new();
    let mut off: Vec<usize> = vec![0];

    out.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

    macro_rules! push {
        ($body:expr) => {{
            off.push(out.len());
            let id = off.len() - 1;
            out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", id, $body).as_bytes());
        }};
    }

    push!("<< /Type /Catalog /Pages 2 0 R >>");
    push!("<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    push!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>"
    );
    push!(font_obj);
    push!(format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content));

    let xref_offset = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", off.len()).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for &o in &off[1..] {
        out.extend_from_slice(format!("{:010} 00000 n \n", o).as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            off.len(),
            xref_offset
        )
        .as_bytes(),
    );
    out
}

/// PDFDocEncoding byte 0xE9 in /ActualText must decode as 'é' (U+00E9),
/// not as U+FFFD (replacement character from from_utf8_lossy).
#[test]
fn actualtext_pdfdocencoding_latin_char_e9() {
    let pdf = build_pdf_with_actual_text(&[0xE9]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(
        text.contains('é'),
        "ActualText byte 0xE9 must decode as 'é' via PDFDocEncoding; got: {text:?}"
    );
    assert!(
        !text.contains('\u{FFFD}'),
        "U+FFFD replacement char must not appear (means from_utf8_lossy was used); got: {text:?}"
    );
}

/// PDFDocEncoding byte 0x80 in /ActualText must decode as '•' (U+2022 BULLET).
#[test]
fn actualtext_pdfdocencoding_bullet_0x80() {
    let pdf = build_pdf_with_actual_text(&[0x80]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(
        text.contains('•'),
        "ActualText byte 0x80 must decode as bullet '•'; got: {text:?}"
    );
}

/// PDFDocEncoding byte 0x84 in /ActualText must decode as '—' (U+2014 EM DASH).
#[test]
fn actualtext_pdfdocencoding_emdash_0x84() {
    let pdf = build_pdf_with_actual_text(&[0x84]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(
        text.contains('—'),
        "ActualText byte 0x84 must decode as em-dash '—'; got: {text:?}"
    );
}

/// A mixed ASCII + PDFDocEncoding ActualText string like [0x41, 0xE9] → "Aé".
#[test]
fn actualtext_pdfdocencoding_mixed_ascii_and_latin() {
    let pdf = build_pdf_with_actual_text(&[0x41, 0xE9]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(
        text.contains('A') && text.contains('é'),
        "ActualText [0x41, 0xE9] must decode as 'Aé'; got: {text:?}"
    );
}

/// UTF-16BE BOM in /ActualText must still work correctly (regression guard).
#[test]
fn actualtext_utf16be_bom_still_works() {
    let pdf = build_pdf_with_actual_text(&[0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69]);
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let text = doc.extract_text(0).unwrap();
    assert!(
        text.contains('H') && text.contains('i'),
        "ActualText UTF-16BE BOM 'Hi' must decode correctly; got: {text:?}"
    );
}
