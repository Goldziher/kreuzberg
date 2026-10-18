/// Build a minimal "valid header but no usable tags" ICC profile: passes
/// `IccProfile::parse`'s header / `acsp` / `/N` cross-check but qcms's
/// `Profile::new_from_slice` rejects it (no `A2B0`, no matrix/curve
/// tags), so `Transform::has_cmm()` returns false. Used to verify the
/// fallback path in `resolve_iccbased` kicks in cleanly.
fn build_iccbased_header_only_cmyk_profile() -> Vec<u8> {
    let mut profile = vec![0u8; 128];
    let total: u32 = 128 + 4;
    profile[0..4].copy_from_slice(&total.to_be_bytes());
    profile[8..12].copy_from_slice(&0x0240_0000u32.to_be_bytes());
    profile[12..16].copy_from_slice(b"prtr");
    profile[16..20].copy_from_slice(b"CMYK");
    profile[20..24].copy_from_slice(b"Lab ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[64..68].copy_from_slice(&0u32.to_be_bytes());
    profile[68..72].copy_from_slice(&0x0000_F6D6u32.to_be_bytes());
    profile[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    profile[76..80].copy_from_slice(&0x0000_D32Du32.to_be_bytes());
    profile.extend_from_slice(&0u32.to_be_bytes());
    profile
}

/// Sanity-pin: the header-only profile parses through `IccProfile::parse`
/// but produces a transform with no CMM. This is the precondition that
/// makes the `resolve_iccbased` fallback path observable: if either
/// branch flipped (parse failed OR has_cmm became true) the edge probes
/// below would conflate two failure modes.
#[test]
fn qa_round2_header_only_cmyk_profile_parses_without_cmm() {
    use std::sync::Arc;
    use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};
    let bytes = build_iccbased_header_only_cmyk_profile();
    let prof = IccProfile::parse(bytes, 4).expect(
        "header-only profile should pass IccProfile::parse — only IccHeader::parse and /N \
         cross-check run there",
    );
    let prof = Arc::new(prof);
    let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);
    assert!(
        !t.has_cmm(),
        "header-only profile must NOT compile to a usable qcms CMM; otherwise the fallback \
         path in resolve_iccbased can't be probed"
    );
}

/// Embedded /ICCBased N=4 whose profile parses through
/// `IccProfile::parse` but is rejected by qcms (`has_cmm() == false`).
/// `resolve_iccbased` must fall through to the device-family hint, which
/// emits `ResolvedColor::Cmyk` for N=4, which the composite projection
/// then runs through `cmyk_to_rgb_via_intent` against the document
/// /OutputIntents profile. Expected pixel: (126, 126, 126, 255) — the
/// OutputIntent profile A's constant CLUT.
#[test]
fn qa_round2_iccbased_n4_no_cmm_falls_through_to_output_intent() {
    let profile_a = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let profile_b_no_cmm = build_iccbased_header_only_cmyk_profile();
    let pdf = build_pdf_embedded_iccbased_with_different_output_intent(&profile_a, &profile_b_no_cmm);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (126u8, 126, 126, 255),
        "embedded ICCBased N=4 whose profile parses but has no CMM must fall through to \
         the device-family path → ResolvedColor::Cmyk → cmyk_to_rgb_via_intent → \
         document /OutputIntents profile A reference (126,126,126,255); got \
         ({r},{g},{b},{a}). (191,255,255,_) means §10.3.5 additive-clamp fired (fallback \
         path bypassed OutputIntent). (194,194,194,_) means the embedded profile's CMM \
         compiled (precondition pin was wrong)."
    );
}

/// Embedded /ICCBased N=4 with garbage bytes (no valid `acsp` header).
/// `IccProfile::parse` returns None, so the fallback path emits
/// `ResolvedColor::Cmyk` → routed through `cmyk_to_rgb_via_intent` →
/// document /OutputIntents.
#[test]
fn qa_round2_iccbased_n4_unparseable_bytes_fall_through_to_output_intent() {
    let profile_a = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let garbage = vec![0u8; 128];
    let pdf = build_pdf_embedded_iccbased_with_different_output_intent(&profile_a, &garbage);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (126u8, 126, 126, 255),
        "embedded ICCBased N=4 with unparseable bytes must fall through to the \
         device-family path → ResolvedColor::Cmyk → document /OutputIntents \
         (126,126,126,255); got ({r},{g},{b},{a})."
    );
}

/// **Per-plate routing under embedded-ICCBased N=4.** §8.6.5.5 says the
/// ICCBased profile is the conversion source — but for *composite* output
/// only. The per-plate output is the press-target ink coverage: the four
/// raw CMYK components are exactly what the C/M/Y/K plates must carry.
/// Stripping the channel decomposition because the composite path
/// happens to want ICC-converted RGB would make plate output unusable
/// for prepress workflows shipping packaging artwork with embedded-ICC-
/// tagged CMYK.
///
/// Fixture paints `0.25 0 0 0 scn` against an embedded ICCBased N=4
/// space. The cyan plate must carry the 0.25 tint (≈ 63 in byte space —
/// the renderer's f32→u8 path produces the same value as the bare
/// `/DeviceCMYK` counter-pin below); magenta/yellow/black plates must
/// stay at zero.
///
/// If the cyan plate is zero, the per-plate path saw `ResolvedColor::Rgba`
/// from the resolver, the `OverprintResolver` produced an empty
/// participating list, and `InkRouter` returned `Skip` for every plate
/// — that's the regression vector. The resolver must emit a variant that
/// carries BOTH the composite-side ICC-converted RGB (for §8.6.5.5) and
/// the per-plate CMYK decomposition (for press output).
#[test]
fn qa_round2_iccbased_n4_with_embedded_profile_routes_cmyk_to_plates() {
    use xberg_native_pdf::rendering::render_separations;
    let profile_a = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let profile_b = build_minimal_cmyk_to_rgb_lut8_profile(200);
    let pdf = build_pdf_embedded_iccbased_with_different_output_intent(&profile_a, &profile_b);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    let plates = render_separations(&doc, 0, 72).expect("render_separations");
    let sample = |p: &xberg_native_pdf::rendering::SeparationPlate| {
        let w = p.width as usize;
        p.data[50 * w + 50]
    };
    let by_name = |name: &str| {
        plates
            .iter()
            .find(|p| p.ink_name == name)
            .map(sample)
            .unwrap_or_else(|| panic!("plate {name} should be present in the result"))
    };
    let cyan = by_name("Cyan");
    assert!(
        (60..=68).contains(&cyan),
        "Cyan plate should carry the ~0.25 tint from `0.25 0 0 0 scn` through the \
         embedded /ICCBased N=4 space — same per-plate path the bare /DeviceCMYK \
         counter-pin exercises (renderer quantises to ~63). Got {cyan}. \
         Zero means the embedded-ICC arm of resolve_iccbased dropped the CMYK \
         channel decomposition: the per-plate `OverprintResolver` saw \
         `ResolvedColor::Rgba` and produced an empty participating list, so \
         `InkRouter` returned `Skip` for every plate. The fix is to emit a \
         variant that carries both the composite-side ICC-converted RGB AND \
         the original CMYK quadruple for the per-plate router."
    );
    assert_eq!(by_name("Magenta"), 0, "Magenta plate should be zero for `0.25 0 0 0`");
    assert_eq!(by_name("Yellow"), 0, "Yellow plate should be zero for `0.25 0 0 0`");
    assert_eq!(by_name("Black"), 0, "Black plate should be zero for `0.25 0 0 0`");
}

/// Counter-pin: bare /DeviceCMYK paint with no embedded ICC override
/// continues to produce per-plate coverage as before. This guards
/// against a regression where the round-2 fix accidentally widened to
/// the bare /DeviceCMYK arm too.
#[test]
fn qa_round2_bare_devicecmyk_paint_still_produces_separation_coverage() {
    use xberg_native_pdf::rendering::render_separations;
    let pdf = build_pdf_cmyk_without_output_intent();
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    let plates = render_separations(&doc, 0, 72).expect("render_separations");
    let sample = |p: &xberg_native_pdf::rendering::SeparationPlate| {
        let w = p.width as usize;
        p.data[50 * w + 50]
    };
    let by_name = |name: &str| plates.iter().find(|p| p.ink_name == name).map(sample).unwrap_or(0);
    let cyan = by_name("Cyan");
    assert!(
        (60..=68).contains(&cyan),
        "Cyan plate should carry the ~0.25 tint from `0.25 0 0 0 k` (renderer quantises \
         to ~63). Got {cyan}. If zero the bare DeviceCMYK arm regressed too."
    );
    assert_eq!(by_name("Magenta"), 0, "Magenta should be zero");
    assert_eq!(by_name("Yellow"), 0, "Yellow should be zero");
    assert_eq!(by_name("Black"), 0, "Black should be zero");
}

/// ICCBased **N=3** (RGB) with a document CMYK /OutputIntents declared:
/// the OutputIntent applies only to CMYK conversion paths per §8.6.5.5;
/// an RGB ICCBased space neither consults nor cares about the document
/// OutputIntent. Pixel at the painted rect = direct sRGB-like
/// pass-through of the 3 components (fallback path; the device-family
/// hint at /N=3 emits `three_as_rgb`).
#[test]
fn qa_round2_iccbased_n3_with_cmyk_output_intent_ignores_output_intent() {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK) /DestOutputProfile 5 0 R >>] >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    buf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /CS1 [/ICCBased 6 0 R] >> >> /Contents 4 0 R >>\nendobj\n");
    let stream_off = buf.len();
    let content = "/CS1 cs\n0.5 0.25 0.75 scn\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_a = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let icc_a_off = buf.len();
    let icc_a_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", icc_a.len());
    buf.extend_from_slice(icc_a_hdr.as_bytes());
    buf.extend_from_slice(&icc_a);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    // ICCBased N=3 stream — we don't need a valid qcms-compilable profile
    // for the N=3 case because the device-family hint at N=3 in
    // resolve_iccbased emits three_as_rgb directly (the embedded-ICC
    // branch is gated on N=4). Just declare /N 3 with empty stream
    // bytes; parse will fail (no acsp) and the fallback path fires. ~keep
    let icc_b_off = buf.len();
    let bogus = vec![0u8; 128];
    let icc_b_hdr = format!("6 0 obj\n<< /N 3 /Length {} >>\nstream\n", bogus.len());
    buf.extend_from_slice(icc_b_hdr.as_bytes());
    buf.extend_from_slice(&bogus);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    let obj_count = 7;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_a_off, icc_b_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    let doc = PdfDocument::from_bytes(buf).expect("open synthetic PDF");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    // The renderer's f32→u8 round produces 128 / 64 / 191 for the
    // (0.5, 0.25, 0.75) triple. ~keep
    assert_eq!(
        (r, g, b, a),
        (128u8, 64, 191, 255),
        "ICCBased N=3 with document CMYK /OutputIntents declared must pass the three \
         components through unchanged — the OutputIntent applies only to CMYK \
         conversion paths. Got ({r},{g},{b},{a})."
    );
}

/// ICCBased **N=1** (gray) with a document CMYK /OutputIntents declared:
/// same as N=3 — no spec interaction. Fallback path at N=1 emits
/// `first_as_gray`, so a single-component paint of 0.5 produces
/// RGB(128,128,128).
#[test]
fn qa_round2_iccbased_n1_with_cmyk_output_intent_ignores_output_intent() {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK) /DestOutputProfile 5 0 R >>] >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    buf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /CS1 [/ICCBased 6 0 R] >> >> /Contents 4 0 R >>\nendobj\n");
    let stream_off = buf.len();
    let content = "/CS1 cs\n0.5 scn\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_a = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let icc_a_off = buf.len();
    let icc_a_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", icc_a.len());
    buf.extend_from_slice(icc_a_hdr.as_bytes());
    buf.extend_from_slice(&icc_a);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_b_off = buf.len();
    let bogus = vec![0u8; 128];
    let icc_b_hdr = format!("6 0 obj\n<< /N 1 /Length {} >>\nstream\n", bogus.len());
    buf.extend_from_slice(icc_b_hdr.as_bytes());
    buf.extend_from_slice(&bogus);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    let obj_count = 7;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_a_off, icc_b_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    let doc = PdfDocument::from_bytes(buf).expect("open synthetic PDF");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (128u8, 128, 128, 255),
        "ICCBased N=1 with document CMYK /OutputIntents declared must produce a neutral \
         grey from the single component (0.5 → 128); OutputIntent is not consulted. \
         Got ({r},{g},{b},{a})."
    );
}

/// /ICCBased N=4 paint **inside a Form XObject**: precedence survives
/// the Form scope. The embedded ICCBased CS1 is declared on the page,
/// the Form XObject's content paints `/CS1 cs 0.25 0 0 0 scn ... f`,
/// and the page invokes the Form with `q /Fm1 Do Q`. Expected pixel:
/// profile B's reference (194, 194, 194, 255).
#[test]
fn qa_round2_iccbased_n4_precedence_survives_form_xobject_scope() {
    let profile_a = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let profile_b = build_minimal_cmyk_to_rgb_lut8_profile(PROFILE_B_TARGET_L_BYTE);

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK A) /DestOutputProfile 5 0 R >>] >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    buf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /CS1 [/ICCBased 6 0 R] >> /XObject << /Fm1 7 0 R >> >> /Contents 4 0 R >>\nendobj\n");
    let stream_off = buf.len();
    let content = "q\n/Fm1 Do\nQ\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_a_off = buf.len();
    let icc_a_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", profile_a.len());
    buf.extend_from_slice(icc_a_hdr.as_bytes());
    buf.extend_from_slice(&profile_a);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_b_off = buf.len();
    let icc_b_hdr = format!("6 0 obj\n<< /N 4 /Length {} >>\nstream\n", profile_b.len());
    buf.extend_from_slice(icc_b_hdr.as_bytes());
    buf.extend_from_slice(&profile_b);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let form_content = "/CS1 cs\n0.25 0 0 0 scn\n20 20 60 60 re\nf\n";
    let form_off = buf.len();
    let form_hdr = format!(
        "7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << /ColorSpace << /CS1 [/ICCBased 6 0 R] >> >> /Length {} >>\nstream\n",
        form_content.len()
    );
    buf.extend_from_slice(form_hdr.as_bytes());
    buf.extend_from_slice(form_content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    let obj_count = 8;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_a_off, icc_b_off, form_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            obj_count, xref_off
        )
        .as_bytes(),
    );
    let doc = PdfDocument::from_bytes(buf).expect("open synthetic PDF");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    let (br, bg, bb) = PROFILE_B_RGB_AT_FIXTURE_INPUT;
    assert_eq!(
        (r, g, b, a),
        (br, bg, bb, 255),
        "embedded /ICCBased N=4 precedence must survive Form XObject scope — Form paint \
         routed through the page-declared CS1's embedded profile B, not the document \
         /OutputIntents A. Expected ({br},{bg},{bb},255); got ({r},{g},{b},{a}). \
         (126,126,126,_) means Form scope dropped the embedded-ICC routing and the \
         document OutputIntent fired. (191,255,255,_) means neither profile was \
         consulted."
    );
}

// ===========================================================================
// Phase 6: ICC v4 support verification through qcms 0.3.0
// ===========================================================================
//
// qcms 0.3.0 reports ICC v4 support via `iccv4-enabled` (a default feature
// in our build). The `check_profile_version` function at
// `qcms-0.3.0/src/iccread.rs:274` reads only the reserved bytes 10..12 of
// the header and rejects them if non-zero; the major/minor comparison is
// commented out with the comment "Checking the version doesn't buy us
// anything". So a profile whose header advertises v4 (`0x04 0x00 0x00
// 0x00`) parses through qcms identically to a v2 (`0x02 0x40 0x00 0x00`)
// header — the reserved bytes are zero in both cases.
//
// The TRUE ICC v4 difference at the wire level is in the A2B0 tag body:
// v2 uses `mft1` (LUT8) or `mft2` (LUT16); v4 introduces the `mAB ` tag
// form with separate input curves, a matrix, a CLUT, and output curves
// (ICC.1:2010 §10.10). qcms parses both: for the A2B0 transform
// direction the dispatch is at `iccread.rs:1675-1681` (RGB) and
// `:1716-1722` (CMYK) — `mft1`/`mft2` → `read_tag_lutType`; `mAB ` →
// `read_tag_lutmABType`. So a CMYK profile carrying a v4 header AND an
// mAB body would also work; we don't synthesise that combination here
// because constructing a valid mAB tag requires four full input curves
// + a 4D CLUT + three output curves + their offsets, none of which the
// constant-CLUT fixture trick benefits from.
//
// The probe below pins three claims:
//   1. A CMYK profile whose header declares ICC v4 parses through
//      `IccProfile::parse`.
//   2. qcms 0.3.0 builds a real CMM (`Transform::has_cmm() == true`)
//      from that v4-header profile.
//   3. The byte-exact RGB output for `convert_cmyk_pixel(64, 0, 0, 0)`
//      matches the v2 reference (126, 126, 126) — i.e. the version-byte
//      flip is non-destructive when the underlying LUT8 body is
//      identical. ~keep

/// Pin that qcms 0.3.0 accepts an ICC-v4-versioned CMYK profile and
/// drives the same byte-exact CMYK→RGB conversion as the v2-headered
/// equivalent through a `Transform`.
///
/// This is the unit-level v4 verification — proves the qcms CMM
/// dispatch handles the v4 version byte without rejecting the profile
/// or falling back to the §10.3.5 additive-clamp wrapper inside
/// `Transform::convert_cmyk_pixel`.
///
/// Reference value: `target_l_byte=135` projects through the
/// Lab→XYZ→sRGB chain to RGB(126, 126, 126). Independently verified
/// via the round-1 byte-exact harness; intent-invariant by
/// construction (constant CLUT).
#[test]
fn qa_round3_iccbased_v4_profile_compiles_through_qcms_to_same_reference() {
    use std::sync::Arc;
    use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};

    let v2 = build_minimal_cmyk_to_rgb_lut8_profile_with_version(135, IccProfileVersion::V2);
    let v4 = build_minimal_cmyk_to_rgb_lut8_profile_with_version(135, IccProfileVersion::V4);

    // Confirm the version bytes are what we intended at the wire level.
    // Without this gate a regression in the builder could silently emit
    // the same header twice and the test would pass for the wrong reason. ~keep
    assert_eq!(v2[8..12], [0x02, 0x40, 0x00, 0x00], "v2 header bytes incorrect");
    assert_eq!(v4[8..12], [0x04, 0x00, 0x00, 0x00], "v4 header bytes incorrect");
    assert_eq!(
        v2.len(),
        v4.len(),
        "v2 and v4 profiles must differ only in version bytes"
    );
    for i in 0..v2.len() {
        if (8..12).contains(&i) {
            continue;
        }
        assert_eq!(v2[i], v4[i], "v2/v4 builders diverge at byte {i}");
    }

    let prof_v2 = Arc::new(IccProfile::parse(v2, 4).expect("v2 profile must parse"));
    let prof_v4 = Arc::new(
        IccProfile::parse(v4, 4).expect("v4 profile must parse through IccProfile::parse — qcms 0.3.0 accepts v4"),
    );

    let t_v2 = Transform::new_srgb_target(prof_v2, RenderingIntent::RelativeColorimetric);
    let t_v4 = Transform::new_srgb_target(prof_v4, RenderingIntent::RelativeColorimetric);

    assert!(
        t_v2.has_cmm(),
        "v2 profile must compile into a real qcms transform; \
         without it the v4-vs-v2 byte-exact comparison degenerates"
    );
    assert!(
        t_v4.has_cmm(),
        "v4 profile must compile into a real qcms transform. qcms 0.3.0's \
         check_profile_version (iccread.rs:274) is documented to accept v4 \
         headers; if this assertion fires the qcms version no longer matches \
         the plan's research-confirmed behaviour"
    );

    let rgb_v2 = t_v2.convert_cmyk_pixel(64, 0, 0, 0);
    let rgb_v4 = t_v4.convert_cmyk_pixel(64, 0, 0, 0);
    assert_eq!(
        rgb_v2,
        [126u8, 126, 126],
        "v2 byte-exact reference must be (126,126,126) — round-1 pin"
    );
    assert_eq!(
        rgb_v4,
        [126u8, 126, 126],
        "v4 byte-exact reference must equal v2's (126,126,126); qcms 0.3.0 \
         treats the version byte as informational and drives the same LUT8 \
         body. Got {rgb_v4:?}"
    );
}

/// Pin that an ICC v4 profile threaded through a synthetic PDF's
/// `/OutputIntents` array renders the DeviceCMYK paint via the qcms
/// CMM end-to-end, not through the §10.3.5 additive-clamp fallback.
///
/// This is the integration-level v4 probe: confirms the version-byte
/// flip survives the full chain
/// `IccProfile::parse → ResolutionContext::with_output_intent →
/// cmyk_to_rgb_via_intent → Transform::new_srgb_target →
/// Transform::convert_cmyk_pixel`.
///
/// Reference: same `target_l_byte=135` → near-neutral grey ~(128, 128,
/// 128) for any CMYK input including (0.25, 0, 0, 0). The additive-
/// clamp pin for that input is (191, 255, 255); the integration
/// assertion must land in the qcms-converted neighbourhood, not the
/// fallback.
#[test]
fn qa_round3_iccbased_v4_output_intent_drives_render_through_qcms() {
    let icc_v4 = build_minimal_cmyk_to_rgb_lut8_profile_with_version(135, IccProfileVersion::V4);

    // Sanity-pin the v4 profile parses + compiles + produces the round-1
    // byte-exact reference. Without this the integration assertion could
    // fail for the wrong reason (e.g. profile reject → §10.3.5 fallback
    // fires → 191,255,255 instead of ~128). ~keep
    {
        use std::sync::Arc;
        use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};
        let prof = Arc::new(IccProfile::parse(icc_v4.clone(), 4).expect("v4 profile parses through IccProfile::parse"));
        let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);
        assert!(t.has_cmm(), "v4 profile must compile into a real qcms CMM");
        assert_eq!(
            t.convert_cmyk_pixel(64, 0, 0, 0),
            [126u8, 126, 126],
            "v4 profile must produce the round-1 byte-exact reference (126,126,126)"
        );
    }

    let pdf = build_pdf_cmyk_with_output_intent(&icc_v4);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic v4 PDF");
    assert!(
        doc.output_intent_cmyk_profile().is_some(),
        "v4 fixture must expose its OutputIntent via the document accessor; \
         a None here means /N=4 filter or stream decode failed for the v4 \
         profile bytes"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, _a) = pixel_at(&rgba, 50, 50);
    let near = |v: u8| (v as i32 - 128).abs() <= 10;
    assert!(
        near(r) && near(g) && near(b),
        "v4 OutputIntent must drive the render through qcms — expected ~(128,128,128); \
         got ({r}, {g}, {b}). (191,255,255) would mean the §10.3.5 fallback fired."
    );
}

// ===========================================================================
// Black-Point Compensation (BPC) — HONEST_GAP probe
// ===========================================================================
//
// ISO 32000-1:2008 §8.6.5.8 names four rendering intents but does NOT
// mandate Black-Point Compensation as a separate switch — BPC is a CMM-
// implementation knob that piggybacks on the rendering intent (typically
// on /RelativeColorimetric, sometimes /Perceptual) to remap source-profile
// black to destination-profile black, preserving shadow detail when the
// destination has a less-black-than-source black point.
//
// qcms 0.3.0 does NOT implement BPC. Verified at
// `~/.cargo/registry/src/.../qcms-0.3.0/src/lib.rs:29-36`:
//
//   //! ### Black-Point Compensation (BPC)
//   //!
//   //! BPC is currently not supported. Adding it would require
//   //! either pre-multiplying the entire CLUT during transform
//   //! construction (memory and build-time cost) or running an
//   //! extra division per pixel (CPU cost). BPC brings an
//   //! unacceptable performance overhead, so we go with
//   //! perceptual.
//
// Additionally `transform.rs:1283-1289` shows the CMYK transform builder
// declares `_intent: Intent` — the rendering intent parameter is
// underscore-prefixed and unused inside the CMYK path. So for CMYK
// sources, the byte-exact qcms output is invariant across:
//   - All four PDF rendering intents (intent ignored by qcms's CLUT
//     precomputation at `transform_precacheLUT_cmyk_float:1245-1281`).
//   - BPC on vs off (BPC not implemented at all).
//
// What this means for xberg-native-pdf:
//   - The pipeline's intent threading (`gs.rendering_intent` →
//     `ctx.rendering_intent` → `Transform::new_srgb_target`'s intent
//     parameter) is correct end-to-end. qcms is the limiting factor.
//   - The cache key `(profile.content_hash(), intent)` still includes
//     intent so a future qcms upgrade (or a switch to a CMM that
//     honours intent, e.g. lcms2) doesn't silently collapse cache
//     entries across intents.
//
// The probe below pins the current behaviour byte-for-byte. When qcms
// grows BPC (either via fork or upgrade) OR xberg-native-pdf switches to a
// different CMM, this test will go RED at the BPC-aware delta and the
// implementer can re-derive the expected references for the intent
// matrix. ~keep

/// HONEST_GAP marker: qcms 0.3.0 has no BPC implementation AND silently
/// drops the rendering-intent parameter for CMYK sources. When that
/// changes, every line in this probe is the point of update.
const HONEST_GAP_QCMS_030_NO_BPC: &str = "HONEST_GAP_QCMS_030_NO_BPC: qcms 0.3.0 ignores rendering intent for CMYK \
     (transform.rs:1288 `_intent: Intent`) and has no Black-Point Compensation \
     implementation (lib.rs:29-36 design comment). The BPC-aware shadow-detail \
     preservation that /RelativeColorimetric is documented to provide on \
     near-black CMYK inputs cannot be probed against a CMM that drops both \
     intent and BPC; the assertions below pin the current intent-invariant, \
     BPC-absent behaviour and will go RED when either changes — at which point \
     the probe should be split into per-intent expected references derived \
     against the new CMM.";

/// Pin that a near-black CMYK input (CMYK(0, 0, 0, 0.95) — 95 % K, deep
/// shadow) produces the SAME qcms-converted RGB under both
/// `/RelativeColorimetric` and `/AbsoluteColorimetric` rendering
/// intents. With BPC active on Relative the shadow detail would be
/// elevated (preserved relative to the destination black point); with
/// BPC absent both intents collapse to the same CLUT output.
///
/// Also pin that all four PDF rendering intents produce identical bytes
/// for the same input, mirroring the round-3
/// `qa_round3_qcms_030_treats_cmyk_intent_as_informational` probe but
/// at the deep-shadow region where BPC matters most.
///
/// This probe is `#[ignore]`-marked: the assertions reflect the
/// CURRENT byte-exact qcms 0.3.0 behaviour (which conflates BPC-on with
/// BPC-off and ignores intent altogether), so they always pass at HEAD.
/// The point of the ignore marker is that running this probe under a
/// future CMM that DOES implement BPC will surface the gap by going RED
/// at the per-intent assertion — at which point the implementer
/// re-derives the expected references and removes the ignore.
#[test]
#[ignore = "HONEST_GAP_QCMS_030_NO_BPC"]
fn qa_round4_bpc_paper_white_preservation_under_relative_colorimetric() {
    use std::sync::Arc;
    use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};

    let _ = HONEST_GAP_QCMS_030_NO_BPC;

    // The constant-CLUT fixture used everywhere else in this file
    // collapses every CMYK input to one Lab tuple — useless for a
    // shadow-preservation probe because no input bits make it to the
    // output. We need a NON-constant CLUT here so the deep-shadow
    // input lands in a CLUT cell that could in principle differ across
    // intents. Build a v2 LUT8 with a non-constant CLUT: the 16 grid
    // corners ramp linearly with the input, so CMYK(0,0,0,0.95)
    // resolves to a deep-shadow grey distinct from CMYK(0,0,0,0).
    //
    // qcms-0.3.0 CMYK input handling at `transform.rs:1244-1289`
    // ignores the rendering intent regardless of CLUT shape, so the
    // four-intent assertion below holds for any LUT. ~keep
    let icc = build_minimal_cmyk_to_rgb_lut8_profile_with_shadow_ramp(0..=240);
    let prof = Arc::new(IccProfile::parse(icc, 4).expect("ramp profile parses"));

    // CMYK(0, 0, 0, 242) ≈ 95 % K — deep shadow. Pin the same byte-exact
    // RGB across every intent. With BPC implemented and intent-honouring,
    // these would diverge:
    //   - Perceptual: gamut-compress to preserve overall tone relationships;
    //     deep blacks may map to slightly elevated dest blacks.
    //   - RelativeColorimetric WITH BPC: source black → dest black with
    //     shadow detail preserved (the typical print-house default).
    //   - Saturation: preserve hue purity; not relevant here.
    //   - AbsoluteColorimetric: no white-point adaptation, no BPC; render
    //     source black at the dest's measured black value (paper-relative).
    //
    // With qcms 0.3.0 all four collapse to the same CLUT output because
    // the CLUT is pre-computed without intent dependency and BPC isn't
    // implemented. ~keep
    let mut last: Option<[u8; 3]> = None;
    for intent in [
        RenderingIntent::Perceptual,
        RenderingIntent::RelativeColorimetric,
        RenderingIntent::Saturation,
        RenderingIntent::AbsoluteColorimetric,
    ] {
        let t = Transform::new_srgb_target(Arc::clone(&prof), intent);
        let rgb = t.convert_cmyk_pixel(0, 0, 0, 242);
        if let Some(prev) = last {
            assert_eq!(
                prev, rgb,
                "qcms 0.3.0 must produce intent-invariant bytes for a near-black \
                 CMYK input (BPC absent + CMYK intent dropped): previous intent \
                 yielded {prev:?}, intent={intent:?} yielded {rgb:?}. A divergence \
                 here means qcms grew BPC or started honouring intent — re-derive \
                 the expected references per intent and split this probe."
            );
        }
        last = Some(rgb);
    }
}

/// Build a 4×grid LUT8 CMYK→Lab profile with a non-constant CLUT that
/// ramps linearly with the K channel (the dominant axis of typical
/// deep-shadow CMYK builds). Used by the BPC HONEST_GAP probe to feed
/// qcms an input where the CLUT actually depends on the input bits,
/// so a CMM with BPC implementation would in principle produce a
/// shadow-detail-elevated output distinct from the no-BPC reference.
///
/// `l_range` controls the lightness span across the K axis: at K=0 the
/// LUT outputs L*=l_range.start(), at K=255 it outputs L*=l_range.end().
/// The two corners pin the ramp; intermediate grid points are linearly
/// interpolated by qcms's tetrahedral CLUT lookup at runtime.
fn build_minimal_cmyk_to_rgb_lut8_profile_with_shadow_ramp(l_range: std::ops::RangeInclusive<u8>) -> Vec<u8> {
    let in_chan: u8 = 4;
    let out_chan: u8 = 3;
    let grid: u8 = 2;
    let mut lut = Vec::with_capacity(1888);
    lut.extend_from_slice(&0x6d66_7431u32.to_be_bytes());
    lut.extend_from_slice(&0u32.to_be_bytes());
    lut.push(in_chan);
    lut.push(out_chan);
    lut.push(grid);
    lut.push(0);
    let identity: [i32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x0001_0000];
    for v in identity {
        lut.extend_from_slice(&(v as u32).to_be_bytes());
    }
    for _ in 0..in_chan {
        for i in 0..256u16 {
            lut.push(i as u8);
        }
    }
    // 16 grid points for 2^4. CLUT iteration order per ICC.1 §10.8:
    // input channels iterate from MSB outermost — for 4 channels with
    // grid=2 the ordering is C × M × Y × K with K innermost. We want
    // the K-axis to ramp: at K=0 → l_range.start(); at K=255 →
    // l_range.end(). Both other axes are pinned at the same L for
    // simplicity (the LUT8 path then drives shadow ramp linearly on K). ~keep
    let l_low = *l_range.start();
    let l_high = *l_range.end();
    for c_i in 0..2 {
        for m_i in 0..2 {
            for y_i in 0..2 {
                for k_i in 0..2 {
                    let _ = (c_i, m_i, y_i);
                    let l = if k_i == 0 { l_high } else { l_low };
                    lut.push(l);
                    lut.push(128);
                    lut.push(128);
                }
            }
        }
    }
    for _ in 0..out_chan {
        for i in 0..256u16 {
            lut.push(i as u8);
        }
    }
    debug_assert_eq!(lut.len(), 1888, "shadow-ramp LUT8 body size mismatch");

    let mut profile = vec![0u8; 128];
    let total_size: u32 = 128 + 4 + 12 + lut.len() as u32;
    profile[0..4].copy_from_slice(&total_size.to_be_bytes());
    profile[8..12].copy_from_slice(&IccProfileVersion::V2.header_bytes());
    profile[12..16].copy_from_slice(b"prtr");
    profile[16..20].copy_from_slice(b"CMYK");
    profile[20..24].copy_from_slice(b"Lab ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[64..68].copy_from_slice(&0u32.to_be_bytes());
    profile[68..72].copy_from_slice(&0x0000_F6D6u32.to_be_bytes());
    profile[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    profile[76..80].copy_from_slice(&0x0000_D32Du32.to_be_bytes());
    profile.extend_from_slice(&1u32.to_be_bytes());
    profile.extend_from_slice(&0x4132_4230u32.to_be_bytes());
    profile.extend_from_slice(&144u32.to_be_bytes());
    profile.extend_from_slice(&(lut.len() as u32).to_be_bytes());
    profile.extend_from_slice(&lut);
    profile
}
