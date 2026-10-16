//! Regression tests for Layer 4 font-name-set cache combined-hash verification.
//!
//! The first fix added a *single-font* spot-check (only the first font alphabetically).
//! The follow-up replaced it with a combined hash over ALL fonts in the set.
//!
//! These tests cover the gap: when F1 matches but F2 or F3 differs across pages,
//! the single-font check would pass and return wrong fonts; the combined-hash
//! check correctly misses and reloads.

use xberg_native_pdf::PdfDocument;

fn tounicode_cmap(target_unicode: &str) -> String {
    format!(
        "/CIDInit /ProcSet findresource begin\n\
         12 dict begin\n\
         begincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n\
         /CMapType 2 def\n\
         1 begincodespacerange\n\
         <00> <FF>\n\
         endcodespacerange\n\
         1 beginbfchar\n\
         <41> <{target_unicode}>\n\
         endbfchar\n\
         endcmap\n\
         CMapName currentdict /CMap defineresource pop\n\
         end\n\
         end\n"
    )
}

/// Build a 2-page PDF where both pages have font keys {F1, F2, F3}.
///
/// - Page 1: F1→char1_1, F2→char2_1, F3→char3_1
/// - Page 2: F1→char1_2, F2→char2_2, F3→char3_2
///
/// Content stream writes byte 0x41 with each font at absolute positions via Tm.
/// `tag` ensures unique BaseFont names across tests.
fn build_three_font_two_page_pdf(
    tag: &str,
    char1_1: char,
    char2_1: char,
    char3_1: char,
    char1_2: char,
    char2_2: char,
    char3_2: char,
) -> Vec<u8> {
    let cmap = |c: char| tounicode_cmap(&format!("{:04X}", c as u32));

    let c11 = cmap(char1_1);
    let c21 = cmap(char2_1);
    let c31 = cmap(char3_1);
    let c12 = cmap(char1_2);
    let c22 = cmap(char2_2);
    let c32 = cmap(char3_2);

    let content = "BT /F1 12 Tf 1 0 0 1 72 720 Tm (A) Tj \
                   /F2 12 Tf 1 0 0 1 72 700 Tm (A) Tj \
                   /F3 12 Tf 1 0 0 1 72 680 Tm (A) Tj ET\n";

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
    push!("<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>");

    push!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 10 0 R /F2 12 0 R /F3 14 0 R >> >> \
         /Contents 5 0 R >>"
    );
    push!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
         /Resources << /Font << /F1 20 0 R /F2 22 0 R /F3 24 0 R >> >> \
         /Contents 6 0 R >>"
    );

    push!(format!("<< /Length {} >>\nstream\n{content}endstream", content.len()));
    push!(format!("<< /Length {} >>\nstream\n{content}endstream", content.len()));

    for _ in 7..=9 {
        push!("<< >>");
    }

    push!(format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /{tag}-F1-P1 /Encoding /WinAnsiEncoding /ToUnicode 11 0 R >>"
    ));
    push!(format!("<< /Length {} >>\nstream\n{c11}endstream", c11.len()));

    push!(format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /{tag}-F2-P1 /Encoding /WinAnsiEncoding /ToUnicode 13 0 R >>"
    ));
    push!(format!("<< /Length {} >>\nstream\n{c21}endstream", c21.len()));

    push!(format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /{tag}-F3-P1 /Encoding /WinAnsiEncoding /ToUnicode 15 0 R >>"
    ));
    push!(format!("<< /Length {} >>\nstream\n{c31}endstream", c31.len()));

    for _ in 16..=19 {
        push!("<< >>");
    }

    push!(format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /{tag}-F1-P2 /Encoding /WinAnsiEncoding /ToUnicode 21 0 R >>"
    ));
    push!(format!("<< /Length {} >>\nstream\n{c12}endstream", c12.len()));

    push!(format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /{tag}-F2-P2 /Encoding /WinAnsiEncoding /ToUnicode 23 0 R >>"
    ));
    push!(format!("<< /Length {} >>\nstream\n{c22}endstream", c22.len()));

    push!(format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /{tag}-F3-P2 /Encoding /WinAnsiEncoding /ToUnicode 25 0 R >>"
    ));
    push!(format!("<< /Length {} >>\nstream\n{c32}endstream", c32.len()));

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

/// Core regression: 3-font page where F1 is the same across pages but F2/F3 differ.
/// The single-font spot-check would PASS on F1 and return the wrong F2/F3.
/// The combined-hash check detects the full set mismatch and reloads.
#[test]
fn layer4_combined_hash_catches_f2_mismatch_when_f1_matches() {
    let pdf = build_three_font_two_page_pdf("L4Combined-F2Mismatch", 'X', 'Y', 'Z', 'X', 'P', 'Q');

    let baseline_p1 = {
        let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
        doc.extract_text(1).unwrap()
    };
    assert!(
        baseline_p1.contains('P'),
        "isolated p1 must contain 'P' (F2); got: {baseline_p1:?}"
    );
    assert!(
        baseline_p1.contains('Q'),
        "isolated p1 must contain 'Q' (F3); got: {baseline_p1:?}"
    );

    let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
    let _p0 = doc.extract_text(0).unwrap();
    let p1 = doc.extract_text(1).unwrap();

    assert!(
        p1.contains('P'),
        "sequential p1 must contain 'P' (F2 from page 2 CMap), not 'Y' (page 1 CMap); \
         got: {p1:?} — single-font spot-check would miss this, combined-hash catches it"
    );
    assert!(
        p1.contains('Q'),
        "sequential p1 must contain 'Q' (F3 from page 2 CMap); got: {p1:?}"
    );
    assert!(!p1.contains('Y'), "must not leak 'Y' from page 1's F2; got: {p1:?}");
    assert!(!p1.contains('Z'), "must not leak 'Z' from page 1's F3; got: {p1:?}");
}

/// Symmetric case: F3 is the same but F1 and F2 differ.
#[test]
fn layer4_combined_hash_catches_f1_f2_mismatch_when_f3_matches() {
    let pdf = build_three_font_two_page_pdf("L4Combined-F1F2Mismatch", 'A', 'B', 'C', 'D', 'E', 'C');

    let baseline_p1 = {
        let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
        doc.extract_text(1).unwrap()
    };
    assert!(baseline_p1.contains('D'), "isolated p1 must contain 'D' (F1)");
    assert!(baseline_p1.contains('E'), "isolated p1 must contain 'E' (F2)");

    let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
    let _p0 = doc.extract_text(0).unwrap();
    let p1 = doc.extract_text(1).unwrap();

    assert!(p1.contains('D'), "sequential p1 must contain 'D' (F1); got: {p1:?}");
    assert!(p1.contains('E'), "sequential p1 must contain 'E' (F2); got: {p1:?}");
    assert!(!p1.contains('A'), "must not leak 'A' from page 1's F1; got: {p1:?}");
    assert!(!p1.contains('B'), "must not leak 'B' from page 1's F2; got: {p1:?}");
}

/// When ALL fonts are truly identical across pages the cache correctly reuses them.
#[test]
fn layer4_combined_hash_allows_cache_when_all_fonts_match() {
    let pdf = build_three_font_two_page_pdf("L4Combined-AllMatch", 'R', 'S', 'T', 'R', 'S', 'T');

    let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
    let p0 = doc.extract_text(0).unwrap();
    let p1 = doc.extract_text(1).unwrap();

    assert!(
        p0.contains('R') && p0.contains('S') && p0.contains('T'),
        "p0 must contain R,S,T; got: {p0:?}"
    );
    assert!(
        p1.contains('R') && p1.contains('S') && p1.contains('T'),
        "p1 must contain R,S,T; got: {p1:?}"
    );
}

/// Non-whitespace character count must be identical in isolation and sequential
/// mode — the core correctness guarantee.
#[test]
fn layer4_sequential_nonws_count_equals_isolation_with_three_fonts() {
    let pdf = build_three_font_two_page_pdf("L4Combined-CountParity", 'G', 'H', 'I', 'J', 'K', 'L');

    let nonws = |s: &str| s.chars().filter(|c| !c.is_whitespace()).count();

    let iso_p0 = {
        let d = PdfDocument::from_bytes(pdf.clone()).unwrap();
        d.extract_text(0).unwrap()
    };
    let iso_p1 = {
        let d = PdfDocument::from_bytes(pdf.clone()).unwrap();
        d.extract_text(1).unwrap()
    };

    let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
    let seq_p0 = doc.extract_text(0).unwrap();
    let seq_p1 = doc.extract_text(1).unwrap();

    assert_eq!(
        nonws(&iso_p0),
        nonws(&seq_p0),
        "p0: isolation nonws={} must match sequential nonws={}",
        nonws(&iso_p0),
        nonws(&seq_p0)
    );
    assert_eq!(
        nonws(&iso_p1),
        nonws(&seq_p1),
        "p1: isolation nonws={} must match sequential nonws={}",
        nonws(&iso_p1),
        nonws(&seq_p1)
    );
}
