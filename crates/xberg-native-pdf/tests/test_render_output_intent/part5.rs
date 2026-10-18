// ===========================================================================
// Real-corpus regression: vendor branding logo equivalent
// ===========================================================================
//
// The user surfaced an early-project regression where a vendor branding
// logo's green mark rendered too vivid/lime through the §10.3.5
// additive-clamp fallback instead of muted/olive (the press-target colour
// the CMYK profile maps the build to).
//
// We don't have the real branding-logo PDF on disk. Building a synthetic
// equivalent that demonstrates the same shape (a green CMYK build whose
// additive-clamp value diverges from the qcms-converted value in the
// "too vivid → muted" direction) is the next-best regression sentry:
// it pins the OutputIntent pipeline shifts the green-mark colour in the
// predicted direction, byte-for-byte.
//
// Real-corpus probe: when a future engineer adds the real branding-logo
// PDF to `tests/fixtures/`, the assertion shape carries over byte-for-
// byte (the qcms reference for the embedded profile's mapping of the
// green build). The synthetic version uses our constant-CLUT
// `target_l_byte=200` profile — that maps every CMYK input to a
// neutral grey, NOT a specific colour. This is the limitation of the
// synthetic fixture: it proves "OutputIntent shifted the colour"
// directionally, not "OutputIntent shifted it TOWARDS the real
// press-target value." That's what a real press profile would prove. ~keep

/// Pin that a CMYK paint that matches the typical "green logo build"
/// (C=0.30, M=0.05, Y=0.95, K=0.05) renders DIFFERENTLY through the
/// OutputIntent ICC vs through the §10.3.5 additive-clamp fallback,
/// AND that the OutputIntent direction is towards the constant-CLUT
/// reference (proxy for "muted press target") rather than the vivid
/// additive-clamp value.
///
/// Why this matters: the original regression user-surfaced was that a
/// vendor branding logo's green mark printed muted-olive on the press
/// but rendered vivid-lime on screen (additive-clamp). Wiring the
/// OutputIntent profile through the renderer is the fix that closes
/// the press-vs-screen divergence. The synthetic profile here uses
/// the constant-CLUT target — every CMYK input maps to ~(194, 194,
/// 194), a muted neutral. Distinct from the §10.3.5 vivid-lime value
/// for the same input, so the directional check fires.
///
/// Three pins:
///   1. The render WITHOUT OutputIntent produces the §10.3.5 additive-
///      clamp value for the green CMYK build. That's the pre-fix
///      baseline.
///   2. The render WITH OutputIntent produces the qcms reference value
///      for the profile's mapping of that build.
///   3. The OutputIntent value is closer to the constant-CLUT
///      reference (~194) than to the §10.3.5 value, demonstrating
///      directional correctness even on a synthetic fixture.
#[test]
fn qa_round4_branding_green_mark_routes_through_output_intent() {
    // Green-mark CMYK build matching the typical vendor-logo colour
    // recipe — high yellow, moderate cyan, minimal magenta and black.
    // The §10.3.5 additive-clamp value is:
    //   R = 1 - (0.30 + 0.05) = 0.65 → 166
    //   G = 1 - (0.05 + 0.05) = 0.90 → 230
    //   B = 1 - (0.95 + 0.05) = 0.00 → 0
    // i.e. RGB(166, 230, 0) — vivid lime-green, brand-mismatched.
    //
    // The OutputIntent path through profile B (target_l_byte=200) maps
    // every CMYK input to roughly RGB(194, 194, 194) — muted neutral.
    // This is a proxy for "muted olive that the real press profile
    // would produce"; the synthetic fixture's constant CLUT doesn't
    // produce the actual olive value, but it proves the conversion
    // ROUTE shifts the colour towards the press target rather than
    // emitting the raw additive-clamp value. ~keep

    {
        let content = "0.30 0.05 0.95 0.05 k\n20 20 60 60 re\nf\n";
        let pdf = build_pdf_with_catalog_entries_and_content("", content, None);
        let doc = PdfDocument::from_bytes(pdf).expect("open no-OI fixture");
        assert!(
            doc.output_intent_cmyk_profile().is_none(),
            "no-OI fixture must declare no /OutputIntents"
        );
        let rgba = render_rgba(&doc);
        let (r, g, b, a) = pixel_at(&rgba, 50, 50);
        assert_eq!(
            (r, g, b, a),
            process_ink_rgba(0.30, 0.05, 0.95, 0.05),
            "without /OutputIntents the green-mark CMYK build must produce the \
             process-ink value for CMYK(0.30, 0.05, 0.95, 0.05) - the no-OI \
             baseline; got ({r},{g},{b},{a})"
        );
    }

    let profile_b = build_minimal_cmyk_to_rgb_lut8_profile(PROFILE_B_TARGET_L_BYTE);
    let qcms_ref = {
        use std::sync::Arc;
        use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};
        let prof = Arc::new(IccProfile::parse(profile_b.clone(), 4).expect("parse B"));
        let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);
        t.convert_cmyk_pixel(77, 13, 242, 13)
    };

    let content = "0.30 0.05 0.95 0.05 k\n20 20 60 60 re\nf\n";
    let catalog_entries = "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic Press) /DestOutputProfile 5 0 R >>]";
    let pdf = build_pdf_with_catalog_entries_and_content(catalog_entries, content, Some(&profile_b));
    let doc = PdfDocument::from_bytes(pdf).expect("open with-OI fixture");
    assert!(
        doc.output_intent_cmyk_profile().is_some(),
        "with-OI fixture must expose the OutputIntent profile"
    );
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (qcms_ref[0], qcms_ref[1], qcms_ref[2], 255),
        "with /OutputIntents the green-mark CMYK build must render through the qcms \
         reference {qcms_ref:?}; got ({r},{g},{b},{a}). (166,230,0,_) would mean the \
         OutputIntent route was bypassed and the §10.3.5 additive-clamp fired."
    );

    let additive = (166u8, 230, 0);
    let oi = (qcms_ref[0], qcms_ref[1], qcms_ref[2]);
    let saturation = |c: (u8, u8, u8)| -> f32 {
        let max = c.0.max(c.1).max(c.2) as f32;
        let min = c.0.min(c.1).min(c.2) as f32;
        if max == 0.0 { 0.0 } else { (max - min) / max }
    };
    let sat_additive = saturation(additive);
    let sat_oi = saturation(oi);
    assert!(
        sat_oi < sat_additive,
        "OutputIntent value must be LESS saturated than the additive-clamp value \
         to demonstrate the press-target direction-of-shift; saturation additive={:.3} \
         vs OutputIntent={:.3}",
        sat_additive,
        sat_oi
    );
}

/// HONEST_GAP marker: real branding-logo PDF fixture is not on disk in
/// this worktree. The synthetic test above exercises the conversion
/// route; a real-corpus probe with a vendor-issued press profile would
/// pin the qcms reference is within a documented ΔE threshold of the
/// commercial-viewer baseline. Until that fixture lands the directional
/// sanity check (saturation collapses through the OutputIntent path) is
/// the proxy.
const HONEST_GAP_NO_REAL_BRANDING_FIXTURE: &str = "HONEST_GAP_NO_REAL_BRANDING_FIXTURE: no real branding-logo PDF on disk; \
     the synthetic green-mark probe exercises the route but uses a constant-CLUT \
     profile rather than a real press profile. A future engineer with a vendor-\
     issued ICC and the branding-logo PDF should add a CIEDE2000 ΔE assertion \
     against the commercial-viewer baseline.";

/// Pin the HONEST_GAP marker is referenced so the compile-time string
/// constant doesn't drop. Acts as the documentation point for the
/// missing real-corpus fixture; surface it in any future audit of
/// outstanding press-quality probes.
#[test]
fn qa_round4_real_branding_fixture_honest_gap() {
    let _ = HONEST_GAP_NO_REAL_BRANDING_FIXTURE;
}

// ===========================================================================
// Edge-case regression sentries: document-/OutputIntents-shape oddities
// ===========================================================================
//
// These probes pin the renderer's response to malformed or unusual
// `/OutputIntents` array shapes. Each probe builds a PDF whose catalog
// declares an OutputIntent entry that the accessor MUST refuse to surface
// (wrong `/N`, missing `/DestOutputProfile`, garbage stream contents) and
// asserts the renderer falls through to the §10.3.5 additive-clamp value
// for a `/DeviceCMYK` paint of (0.25, 0, 0, 0). They are regression
// sentries — current behaviour, not failing-first probes — so a future
// change that loosens the accessor's filtering (e.g. accepting an N=3
// OutputIntent as CMYK) would flip them RED and surface the regression. ~keep

/// `/N=3` (RGB) OutputIntent with a `/DeviceCMYK` paint operator. The
/// reader at `document.rs:3645-3648` filters on `Some(4)`; an `/N 3`
/// entry is skipped and `output_intent_cmyk_profile()` returns `None`,
/// so the renderer reaches the §10.3.5 additive-clamp fallback.
///
/// Regression sentry: a regression that broadened the `/N` filter to
/// accept N=3 would route CMYK bytes through an RGB profile and either
/// panic at qcms's channel-count assert or emit garbage RGB.
#[test]
fn output_intent_n3_rgb_profile_rejected_at_reader_falls_through_to_process_inks() {
    // Build a PDF whose /OutputIntents entry declares /N 3 on its
    // /DestOutputProfile stream. The body bytes don't have to parse as a
    // real RGB ICC profile because the accessor filters on /N before it
    // ever invokes IccProfile::parse on the stream payload. ~keep
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic RGB) /DestOutputProfile 5 0 R >>] >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R >>\nendobj\n",
    );
    let stream_off = buf.len();
    let content = "0.25 0 0 0 k\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_off = buf.len();
    let bogus = vec![0u8; 128];
    let icc_hdr = format!("5 0 obj\n<< /N 3 /Length {} >>\nstream\n", bogus.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(&bogus);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    let obj_count = 6;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
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

    assert!(
        doc.output_intent_cmyk_profile().is_none(),
        "an /OutputIntents entry with /N 3 (RGB) must be filtered out by \
         output_intent_cmyk_profile(); only /N 4 (CMYK) entries are eligible"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        process_ink_rgba(0.25, 0.0, 0.0, 0.0),
        "an N=3 OutputIntent must fall through to the process-ink converter \
         byte-for-byte for /DeviceCMYK paint; got ({r},{g},{b},{a})"
    );
}

/// `/OutputIntents` entry with `/OutputCondition` text only and **no**
/// `/DestOutputProfile` stream. The accessor's `entry_dict.get("DestOutputProfile")`
/// check at `document.rs:3630-3633` returns `None`, the entry is skipped,
/// and the whole array exhausts → `output_intent_cmyk_profile()` is `None`.
/// CMYK paint falls through to §10.3.5.
///
/// Regression sentry: a regression that materialised a default profile
/// when none was declared would surface here.
#[test]
fn output_intent_with_outputcondition_string_only_no_destoutputprofile_falls_through() {
    // No DestOutputProfile, no profile stream object at all. /OutputCondition
    // is just a human-readable string (PDF/X advisory metadata per
    // §14.11.5 — "the intended printing condition", e.g. "FOGRA39"); it
    // does not carry the ICC bytes itself. ~keep
    let catalog_entries = "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (FOGRA39 (ISO 12647-2:2004)) /OutputConditionIdentifier (FOGRA39) >>]";
    let content_ops = "0.25 0 0 0 k\n20 20 60 60 re\nf\n";
    let pdf = build_pdf_with_catalog_entries_and_content(catalog_entries, content_ops, None);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");

    assert!(
        doc.output_intent_cmyk_profile().is_none(),
        "an /OutputIntents entry without a /DestOutputProfile stream must \
         surface as None; the /OutputCondition string is advisory metadata, \
         not a fallback colour source"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        process_ink_rgba(0.25, 0.0, 0.0, 0.0),
        "a /OutputCondition-only entry must fall through to the process-ink \
         converter byte-for-byte; got ({r},{g},{b},{a})"
    );
}

/// Two `/OutputIntents` entries in catalog-declaration order — first
/// `/N 3` (RGB), second `/N 4` (CMYK). The accessor iterates the array
/// (`document.rs:3621`) and returns the first entry whose `/N` matches 4
/// AND whose stream parses through `IccProfile::parse`. The N=3 entry is
/// skipped at the /N filter; the N=4 entry's CLUT (target_l_byte = 135)
/// is consumed; the rendered pixel matches the qcms reference for that
/// profile (RGB(126, 126, 126)).
///
/// Regression sentry: a regression that returned the FIRST array entry
/// regardless of /N would surface a None and additive-clamp here.
#[test]
fn output_intent_array_picks_first_cmyk_entry_skipping_rgb() {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic RGB) /DestOutputProfile 5 0 R >> << /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK) /DestOutputProfile 6 0 R >>] >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R >>\nendobj\n",
    );
    let stream_off = buf.len();
    let content = "0.25 0 0 0 k\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let rgb_off = buf.len();
    let bogus = vec![0u8; 128];
    let rgb_hdr = format!("5 0 obj\n<< /N 3 /Length {} >>\nstream\n", bogus.len());
    buf.extend_from_slice(rgb_hdr.as_bytes());
    buf.extend_from_slice(&bogus);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let cmyk_profile = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let cmyk_off = buf.len();
    let cmyk_hdr = format!("6 0 obj\n<< /N 4 /Length {} >>\nstream\n", cmyk_profile.len());
    buf.extend_from_slice(cmyk_hdr.as_bytes());
    buf.extend_from_slice(&cmyk_profile);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    let obj_count = 7;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, rgb_off, cmyk_off] {
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

    let profile = doc
        .output_intent_cmyk_profile()
        .expect("the CMYK entry must be picked from a mixed-N array");
    assert_eq!(
        profile.n_components(),
        4,
        "the surfaced profile must be /N=4 — the second array entry"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (126u8, 126, 126, 255),
        "with a mixed [RGB, CMYK] /OutputIntents array the second (CMYK) \
         entry must drive the render byte-for-byte against the qcms \
         reference; got ({r},{g},{b},{a})"
    );
}

/// `/DestOutputProfile` stream whose bytes are pure garbage — not even a
/// 128-byte ICC header, no `acsp` signature, random bytes. The accessor's
/// `IccProfile::parse(bytes, 4)` call at `document.rs:3653` returns `None`,
/// the entry is skipped, and the renderer falls through to §10.3.5.
///
/// This is distinct from the header-only probe at
/// `output_intent_with_unparseable_profile_falls_through_to_process_inks`:
/// that probe pins fall-through happens INSIDE `Transform::convert_cmyk_pixel`
/// (the header parses, qcms refuses to build the CMM). This probe pins the
/// earlier rejection where `IccProfile::parse` returns None up front — no
/// header, nothing to keep.
///
/// Regression sentry: a regression that propagated the un-parsed bytes to
/// qcms (or that silently emitted a degenerate IccProfile from a parse
/// failure) would surface here as a panic or as a non-additive-clamp pixel.
#[test]
fn output_intent_malformed_iccbased_stream_falls_through() {
    let garbage: Vec<u8> = (0u8..=63u8).collect();
    let pdf = build_pdf_cmyk_with_output_intent(&garbage);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");

    assert!(
        doc.output_intent_cmyk_profile().is_none(),
        "IccProfile::parse must reject a 64-byte garbage stream (less than \
         the 128-byte ICC header minimum); accessor must surface None"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        process_ink_rgba(0.25, 0.0, 0.0, 0.0),
        "a malformed /DestOutputProfile stream (sub-header-length garbage) \
         must fall through to the process-ink converter byte-for-byte without \
         panicking; got ({r},{g},{b},{a})"
    );
}

/// Build a single-page PDF that declares one ExtGState with `/ca 0.5`,
/// references it once via `gs`, then emits N opaque CMYK fills covering
/// the whole page. Every fill routes through the
/// `apply_cmyk_compose_after_paint_with_coverage` helper (transparency
/// active → sidecar allocated → coverage path active). With the hoist,
/// the cache must record exactly one build for the single (profile,
/// intent) tuple in play.
fn build_pdf_cmyk_with_transparency_and_repeated_paints(icc_profile_bytes: &[u8], paints: usize) -> Vec<u8> {
    let mut ops = String::new();
    ops.push_str("/T gs\n");
    for _ in 0..paints {
        ops.push_str("0.25 0 0 0 k\n0 0 100 100 re\nf\n");
    }
    let catalog_entries =
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (S) /DestOutputProfile 5 0 R >>]";

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    let catalog = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {} >>\nendobj\n",
        catalog_entries
    );
    buf.extend_from_slice(catalog.as_bytes());

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /T << /Type /ExtGState /ca 0.5 >> >> >> \
         /Contents 4 0 R >>\nendobj\n",
    );

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", icc_profile_bytes.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(icc_profile_bytes);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

/// Pin that `apply_cmyk_compose_after_paint_with_coverage` builds the
/// ICC transform exactly once per (profile, intent), even across many
/// paints touching the full page under transparency. Before the hoist
/// fix the cache was queried per pixel — for a 100×100 page rendered
/// at 72 DPI that meant one hash of the entire ICC blob for every
/// pixel covered. The counter is exact: 1 = hoisted, > paint count
/// = per-pixel regression.
#[test]
fn cmyk_transparency_compose_hoists_icc_transform_cache_lookup() {
    use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let paints: usize = 8;
    let pdf = build_pdf_cmyk_with_transparency_and_repeated_paints(&icc, paints);
    let doc = PdfDocument::from_bytes(pdf).expect("open");

    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72));
    let _ = renderer.render_page(&doc, 0).expect("render");

    let built = renderer.icc_transform_cache_build_count();
    assert_eq!(
        built, 1,
        "Many full-page transparency CMYK paints under one OutputIntent \
         profile and one rendering intent must build the qcms Transform \
         exactly once. Built {built} times — the per-paint transform \
         cache regressed or is missing."
    );

    // The hoist guards against per-pixel `get_or_build` calls. The
    // cache returns the same `Arc<Transform>` on every hit so
    // `build_count` cannot distinguish "hoisted" from "per-pixel"; the
    // `lookup_count` counter increments on every CALL regardless of
    // hit/miss, so it cleanly does. On a 100×100 page with 8 paints
    // covering the full canvas, the per-pixel regression would record
    // ≈ 8 × 10_000 = 80_000 lookups; the hoisted path records at most
    // a small constant per paint (one for the with_coverage helper). ~keep
    let looked_up = renderer.icc_transform_cache_lookup_count();
    assert!(
        looked_up <= paints * 4,
        "Per-pixel `get_or_build` regression detected in \
         `apply_cmyk_compose_after_paint_with_coverage`: {paints} \
         full-page CMYK paints recorded {looked_up} ICC-transform \
         lookups. The hoist caps lookups at a small constant per paint \
         (one call per `get_or_build` site reached); a per-pixel \
         lookup scales with painted-pixel count (100×100 = 10_000 per \
         paint here)."
    );
}

/// Pin the same hoist for the overprint helper. The fixture toggles
/// `/OP true` on the ExtGState so process-colour overprint composition
/// routes through `apply_overprint_after_paint_with_coverage`. The
/// build count must remain at 1 across many overprint paints.
fn build_pdf_cmyk_with_overprint_and_repeated_paints(icc_profile_bytes: &[u8], paints: usize) -> Vec<u8> {
    let mut ops = String::new();
    ops.push_str("/T gs\n");
    for _ in 0..paints {
        ops.push_str("0.25 0 0 0 k\n0 0 100 100 re\nf\n");
    }
    let catalog_entries =
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (S) /DestOutputProfile 5 0 R >>]";

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    let catalog = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {} >>\nendobj\n",
        catalog_entries
    );
    buf.extend_from_slice(catalog.as_bytes());

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /T << /Type /ExtGState /OP true /op true >> >> >> \
         /Contents 4 0 R >>\nendobj\n",
    );

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", icc_profile_bytes.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(icc_profile_bytes);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

#[test]
fn cmyk_overprint_with_coverage_hoists_icc_transform_cache_lookup() {
    use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let paints: usize = 8;
    let pdf = build_pdf_cmyk_with_overprint_and_repeated_paints(&icc, paints);
    let doc = PdfDocument::from_bytes(pdf).expect("open");

    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72));
    let _ = renderer.render_page(&doc, 0).expect("render");

    let built = renderer.icc_transform_cache_build_count();
    assert_eq!(
        built, 1,
        "Many full-page CMYK overprint paints under one OutputIntent \
         profile and one rendering intent must build the qcms Transform \
         exactly once."
    );

    let looked_up = renderer.icc_transform_cache_lookup_count();
    assert!(
        looked_up <= paints * 4,
        "Per-pixel `get_or_build` regression detected in \
         `apply_overprint_after_paint_with_coverage`: {paints} \
         full-page CMYK overprint paints recorded {looked_up} ICC- \
         transform lookups. Hoist caps lookups at a small constant \
         per paint."
    );
}

// ===========================================================================
// H3b — silent K=0 RGB→CMYK fallback under declared /OutputIntents.
//
// `resolve_rgb_paint_to_cmyk` falls back to the §10.3.5 inverse
// (C, M, Y) = (1−R, 1−G, 1−B), K = 0 whenever it can't get a CMYK
// profile out of the document. That's the correct behaviour when no
// /OutputIntents declaration was made at all (the producer didn't ask
// for press conversion). When the catalog DOES declare /OutputIntents
// but the profile bytes don't parse, the fallback silently degrades
// press output — the K plane goes empty. This probe pins a one-shot
// `tracing::warn!` on that surface so the silent degradation is
// observable until the upstream fix for the swallowed
// profile-parse diagnostic lands.
// =========================================================================== ~keep

/// Capture warn-level tracing events into an in-test buffer so the probe
/// can grep for the H3b diagnostic. Mirrors `CapturingLayer` in
/// `src/rendering/sidecar.rs::tests` — duplicated here because the
/// integration test crate can't reach in-crate test types.
///
/// Installed per-call via `tracing::subscriber::with_default`, which is
/// thread-scoped, so no global install and no buffer-offset bookkeeping.
#[derive(Clone, Default)]
struct WarnCapture {
    buf: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl<S> tracing_subscriber::Layer<S> for WarnCapture
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if *event.metadata().level() > tracing::Level::WARN {
            return;
        }
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        self.buf.lock().unwrap().push(visitor.0);
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }
}

/// Build a single-page PDF that declares `/OutputIntents` with a
/// malformed /DestOutputProfile stream (64 bytes of garbage — below
/// the 128-byte ICC header minimum) and an ExtGState with `/ca 0.5`
/// to force the sidecar / RGB-to-CMYK mirror path. The page paints a
/// non-trivial RGB rect so `resolve_rgb_paint_to_cmyk` runs.
fn build_pdf_rgb_with_unparseable_output_intent() -> Vec<u8> {
    let garbage: Vec<u8> = (0u8..=63u8).collect();
    let ops = "/T gs\n0.4 0.6 0.8 rg\n0 0 100 100 re\nf\n";
    let catalog_entries =
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (S) /DestOutputProfile 5 0 R >>]";

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    let catalog = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {} >>\nendobj\n",
        catalog_entries
    );
    buf.extend_from_slice(catalog.as_bytes());

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /T << /Type /ExtGState /ca 0.5 >> >> >> \
         /Contents 4 0 R >>\nendobj\n",
    );

    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", garbage.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(&garbage);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    buf
}

#[test]
fn h3b_rgb_paint_under_unparseable_output_intent_logs_k_zero_warning() {
    use xberg_native_pdf::rendering::render_separations;

    use tracing_subscriber::layer::SubscriberExt;

    let capture = WarnCapture::default();
    let subscriber = tracing_subscriber::registry().with(capture.clone());

    let pdf = build_pdf_rgb_with_unparseable_output_intent();
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");

    assert!(
        doc.has_output_intents_declaration(),
        "fixture must declare /OutputIntents"
    );
    assert!(
        doc.output_intent_cmyk_profile().is_none(),
        "fixture's /DestOutputProfile is 64 bytes of garbage; \
         IccProfile::parse must reject it and the accessor must \
         surface None"
    );

    tracing::subscriber::with_default(subscriber, || {
        let _plates = render_separations(&doc, 0, 72).expect("render separations");
    });

    let records: Vec<String> = capture.buf.lock().unwrap().clone();
    let saw_warning = records
        .iter()
        .any(|m| m.contains("K=0") && m.contains("/OutputIntents") && m.contains("712"));
    assert!(
        saw_warning,
        "H3b: an RGB paint under a malformed /OutputIntents declaration \
         must emit a `tracing::warn!` naming the K=0 fallback and the \
         upstream issue it hangs on. Captured records since start: {:?}",
        records
    );
}
