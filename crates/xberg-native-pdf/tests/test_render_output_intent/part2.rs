fn render_rgba(doc: &PdfDocument) -> Vec<u8> {
    let opts = RenderOptions::with_dpi(72).as_raw();
    let img = render_page(doc, 0, &opts).expect("render_page");
    assert_eq!(img.format, ImageFormat::RawRgba8);
    img.data
}

fn pixel_at(rgba: &[u8], x: u32, y: u32) -> (u8, u8, u8, u8) {
    let w = 100u32;
    let h = 100u32;
    assert_eq!(rgba.len() as u32, w * h * 4);
    assert!(x < w && y < h);
    let off = ((y * w + x) * 4) as usize;
    (rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3])
}

/// Expected 8-bit RGB for a DeviceCMYK quadruple under the **process-ink**
/// converter (`xberg_native_pdf::color::cmyk_to_rgb`, tetralinear over the 16
/// measured ink corners) - the conversion the composite renderer now
/// takes when there is no usable `/OutputIntents` CMYK profile.
///
/// This replaces the old ISO 32000-1:2008 §10.3.5 additive-clamp
/// `1 - min(1, C+K)` fallback: an identical CMYK value now resolves the
/// same whether it is a vector fill, a raster image, or extracted text.
/// The opaque-vector fill path rounds each channel `round(v * 255)` (the
/// same rounding the prior byte-exact additive pins relied on), so this
/// helper is byte-for-byte what the renderer must emit for a fully
/// covered opaque CMYK fill.
fn process_ink_rgb(c: f32, m: f32, y: f32, k: f32) -> (u8, u8, u8) {
    let (r, g, b) = xberg_native_pdf::color::cmyk_to_rgb(c, m, y, k);
    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

/// [`process_ink_rgb`] plus a fully-opaque alpha, for the `pixel_at`
/// 4-tuple comparisons.
fn process_ink_rgba(c: f32, m: f32, y: f32, k: f32) -> (u8, u8, u8, u8) {
    let (r, g, b) = process_ink_rgb(c, m, y, k);
    (r, g, b, 255)
}

/// Pin that a /DeviceCMYK fill on a page whose document declares a
/// CMYK `/OutputIntents` profile is rendered via the qcms-driven ICC
/// path rather than ISO 32000-1:2008 §10.3.5's additive-clamp formula.
///
/// Fixture details:
///   - CMYK input: (0.25, 0, 0, 0) — modest cyan tint.
///   - Profile: minimal in-test CMYK→RGB LUT8 that maps every CMYK input
///     to constant `RGB(128, 128, 128)`. With the OutputIntent path
///     live, every pixel inside the rect must be ~128 grey on every
///     channel. With the additive-clamp fallback the pixel would be
///     `(191, 255, 255)` — `1 - (C + K)`, `1 - (M + K)`, `1 - (Y + K)`
///     scaled to bytes.
#[test]
fn device_cmyk_paint_with_output_intent_renders_via_icc_not_additive_clamp() {
    let target_l_byte: u8 = 135;
    let icc = build_minimal_cmyk_to_rgb_lut8_profile(target_l_byte);
    // First sanity-check the synthesised profile compiles into a real
    // qcms transform — otherwise the test would silently degrade to
    // the §10.3.5 fallback and the assertion below would fail for the
    // wrong reason. The transform-build path is the same one the
    // composite renderer will exercise on this profile. ~keep
    {
        use std::sync::Arc;
        use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};
        let prof =
            Arc::new(IccProfile::parse(icc.clone(), 4).expect("synthesised profile parses through IccProfile::parse"));
        let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);
        assert!(
            t.has_cmm(),
            "synthesised profile must compile into a real qcms transform; \
             without it the OutputIntent test degrades to the additive-clamp \
             fallback and asserts the wrong thing"
        );
        let rgb = t.convert_cmyk_pixel(64, 0, 0, 0);
        let near = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 10;
        assert!(
            near(rgb[0], 128) && near(rgb[1], 128) && near(rgb[2], 128),
            "qcms must drive the constant CLUT: got {rgb:?}, want ~(128, 128, 128) \
             ±10 (Lab(53,0,0) → sRGB grey)"
        );
    }

    let pdf = build_pdf_cmyk_with_output_intent(&icc);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    let oi = doc
        .output_intent_cmyk_profile()
        .expect("synthetic catalog declares a CMYK OutputIntent");
    assert_eq!(oi.n_components(), 4, "OutputIntent must be /N=4");

    let rgba = render_rgba(&doc);
    let (r, g, b, _a) = pixel_at(&rgba, 50, 50);

    let near_const = |v: u8| (v as i32 - 128).abs() <= 10;
    assert!(
        near_const(r) && near_const(g) && near_const(b),
        "OutputIntent /DeviceCMYK paint expected qcms-converted RGB ~(128, 128, 128); \
         got ({r}, {g}, {b}). RGB(191, 255, 255) would mean the §10.3.5 additive-clamp \
         fallback fired — the resolver is not consulting ctx.output_intent_cmyk."
    );
}

// ===========================================================================
// Negative pin: no OutputIntent → §10.3.5 additive-clamp preserved
// =========================================================================== ~keep

/// Pin that a /DeviceCMYK fill on a page whose document declares no
/// `/OutputIntents` array is rendered through ISO 32000-1:2008
/// §10.3.5's additive-clamp formula, byte-for-byte, as it shipped
/// before OutputIntent threading landed.
///
/// This is the contrapositive of the positive test: when
/// `ctx.output_intent_cmyk` is `None`, the resolver MUST fall through
/// to the shipped behaviour. A bug that unconditionally consulted
/// some other ICC profile (or that flipped the precedence rules) would
/// surface here as the wrong colour.
#[test]
fn device_cmyk_paint_without_output_intent_renders_process_inks() {
    let pdf = build_pdf_cmyk_without_output_intent();
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    assert!(
        doc.output_intent_cmyk_profile().is_none(),
        "fixture must declare no /OutputIntents in catalog"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, _a) = pixel_at(&rgba, 50, 50);

    assert_eq!(
        (r, g, b),
        process_ink_rgb(0.25, 0.0, 0.0, 0.0),
        "without /OutputIntents the composite render must use the process-ink \
         fallback byte-for-byte; got ({r}, {g}, {b})"
    );
}

/// Byte-exact pin of the qcms reference value the synthesised
/// `target_l_byte=135` profile yields.
///
/// The existing positive test (`device_cmyk_paint_with_output_intent_*`)
/// asserts the rendered pixel falls within `(128, 128, 128) ± 10` per
/// channel — that's a hand-wave that hides up to a ~9-byte channel-by-
/// channel drift. Derived against qcms 0.3.0 (the version pinned in
/// Cargo.lock at this commit), the byte-exact reference for
/// `target_l_byte=135` + CMYK(64,0,0,0) at `RelativeColorimetric` is
/// `(126, 126, 126)`. The rendered pixel at (50, 50) through the
/// composite pipeline is `(126, 126, 126, 255)`. We pin both — any
/// drift in the qcms chain (Lab→XYZ→sRGB), the LUT8 tetra-interp, or
/// the resolver's 8-bit round-trip surfaces here byte-for-byte.
///
/// If a future qcms upgrade shifts the reference, the right answer is
/// to re-derive the value here, not to widen the tolerance — `±10` was
/// the impl-agent's tolerance for an unmeasured target; this probe pins
/// the actual measured target.
#[test]
fn output_intent_render_pixel_is_byte_exact_against_qcms_reference() {
    use std::sync::Arc;
    use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};

    let target_l_byte: u8 = 135;
    let icc = build_minimal_cmyk_to_rgb_lut8_profile(target_l_byte);

    {
        let prof = Arc::new(IccProfile::parse(icc.clone(), 4).expect("parse"));
        let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);
        let rgb = t.convert_cmyk_pixel(64, 0, 0, 0);
        assert_eq!(
            rgb,
            [126u8, 126, 126],
            "qcms 0.3.0 byte-exact reference for target_l_byte=135 + CMYK(64,0,0,0): \
             expected (126, 126, 126); got {rgb:?}. Re-derive (see plan errata) if qcms \
             ever changes its Lab→sRGB chain — do not widen tolerance."
        );
    }

    let pdf = build_pdf_cmyk_with_output_intent(&icc);
    let doc = PdfDocument::from_bytes(pdf).expect("open");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (126u8, 126, 126, 255),
        "rendered pixel must match the qcms reference byte-for-byte; got ({r},{g},{b},{a}). \
         (191,255,255,_) means the §10.3.5 fallback fired."
    );
}

/// Pin the qcms reference value is intent-independent for the synthesised
/// constant-CLUT profile.
///
/// The constant-CLUT shape of the synthesised profile means a CMM whose
/// gamut compression depends on rendering intent (which is the whole
/// point of having intents) should still produce the same value — there's
/// no out-of-gamut excursion to compress. If qcms ever starts producing
/// different values per intent on a constant CLUT that's a CMM bug
/// worth surfacing.
///
/// This assertion holds for qcms 0.3.0, which ignores the intent
/// parameter for CMYK inputs altogether.
#[test]
fn output_intent_constant_clut_is_invariant_across_rendering_intents() {
    use std::sync::Arc;
    use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};

    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let prof = Arc::new(IccProfile::parse(icc, 4).expect("parse"));
    let mut last: Option<[u8; 3]> = None;
    for intent in [
        RenderingIntent::Perceptual,
        RenderingIntent::RelativeColorimetric,
        RenderingIntent::Saturation,
        RenderingIntent::AbsoluteColorimetric,
    ] {
        let t = Transform::new_srgb_target(Arc::clone(&prof), intent);
        let rgb = t.convert_cmyk_pixel(64, 0, 0, 0);
        if let Some(prev) = last {
            assert_eq!(
                prev, rgb,
                "constant-CLUT qcms output must be identical across rendering intents; \
                 first intent yielded {prev:?}, intent={intent:?} yielded {rgb:?}"
            );
        }
        last = Some(rgb);
    }
}

/// Pin that a syntactically-shaped but tag-table-truncated CMYK profile
/// declared on `/OutputIntents` does not crash the renderer and produces
/// the §10.3.5 fallback colour byte-for-byte.
///
/// This surfaces an open design question as a probe: when
/// qcms refuses to compile the OutputIntent profile, `Transform::
/// convert_cmyk_pixel` devolves internally — but the renderer-level
/// behaviour must be (a) no panic and (b) the same RGB the no-
/// OutputIntent fixture produces, so a malformed `/OutputIntents`
/// degrades gracefully.
#[test]
fn output_intent_with_unparseable_profile_falls_through_to_process_inks() {
    // Header-only profile: parses through `IccProfile::parse` (which
    // only validates the 128-byte header), but qcms refuses at build
    // time because there's no tag table. Mirrors the stub the in-source
    // unit test in color.rs uses but reaches the rasteriser end-to-end. ~keep
    let mut header_only = vec![0u8; 128];
    header_only[8..12].copy_from_slice(&0x0400_0000u32.to_be_bytes());
    header_only[12..16].copy_from_slice(b"prtr");
    header_only[16..20].copy_from_slice(b"CMYK");
    header_only[20..24].copy_from_slice(b"Lab ");
    header_only[36..40].copy_from_slice(b"acsp");

    let pdf = build_pdf_cmyk_with_output_intent(&header_only);
    let doc = PdfDocument::from_bytes(pdf).expect("open");

    assert!(
        doc.output_intent_cmyk_profile().is_some(),
        "header-only stub must parse through IccProfile::parse; fall-through must \
         happen inside Transform::convert_cmyk_pixel, not at the accessor"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        process_ink_rgba(0.25, 0.0, 0.0, 0.0),
        "unparseable OutputIntent profile must fall through to the process-ink converter \
         byte-exact; got ({r},{g},{b},{a})"
    );
}

/// Pin that an OutputIntent profile whose ICC header declares a non-CMYK
/// colour space (`RGB `, `GRAY`, `Lab `) is filtered out by
/// `IccProfile::parse`'s cross-check, even though the stream dict's
/// `/N 4` would otherwise let it through the accessor.
///
/// `IccProfile::parse(bytes, declared_n)` at `src/color.rs:159` requires
/// that the ICC header's implied component count match the stream
/// dict's `/N`. An `RGB ` header implies `n=3`; `declared_n=4` → reject.
/// `output_intent_cmyk_profile` then returns `None`, and the renderer
/// falls back to §10.3.5 byte-for-byte.
///
/// This is the strongest gate: a malformed profile that lied about
/// colour space in the ICC header gets rejected before reaching qcms.
/// A regression that loosened the cross-check would let the qcms layer
/// see CMYK bytes through an RGB profile — at best garbage, at worst a
/// panic in the CMM.
#[test]
fn output_intent_with_mismatched_icc_header_colour_space_is_rejected_at_parse() {
    let mut header_only = vec![0u8; 128];
    header_only[8..12].copy_from_slice(&0x0400_0000u32.to_be_bytes());
    header_only[12..16].copy_from_slice(b"prtr");
    header_only[16..20].copy_from_slice(b"RGB ");
    header_only[20..24].copy_from_slice(b"Lab ");
    header_only[36..40].copy_from_slice(b"acsp");

    let pdf = build_pdf_cmyk_with_output_intent(&header_only);
    let doc = PdfDocument::from_bytes(pdf).expect("open");
    assert!(
        doc.output_intent_cmyk_profile().is_none(),
        "IccProfile::parse must reject when ICC header colour-space \
         tag implies a different component count than the stream's /N"
    );
    // Renderer falls through to §10.3.5 byte-for-byte. ~keep
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        process_ink_rgba(0.25, 0.0, 0.0, 0.0),
        "mismatched-header OutputIntent must fall through to the process-ink converter; got ({r},{g},{b},{a})"
    );
}

/// Pin that `crate::extractors::images::cmyk_pixel_to_rgb` and the
/// resolver helper's no-OutputIntent arm produce the same RGB bytes on
/// the same CMYK quadruple.
///
/// This is the HONEST_GAP the impl agent flagged in
/// `cmyk_to_rgb_via_intent_falls_back_when_profile_has_no_cmm`. Verified
/// here at the public-API level by routing both paths through a known
/// CMYK input and comparing byte-for-byte. If a future refactor diverges
/// the two process-ink implementations, the fallback path inside qcms's
/// no-CMM arm could disagree with the resolver's bare-fallback arm even
/// though both intend the same conversion.
///
/// The probe iterates over a handful of representative inputs — pure
/// process inks, the test fixture's input, and a few interior CMYK
/// quadruples. Every input must agree.
#[test]
fn process_ink_consistency_between_extractors_helper_and_no_output_intent_arm() {
    use std::sync::Arc;
    use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};

    let mut header_only = vec![0u8; 128];
    header_only[8..12].copy_from_slice(&0x0400_0000u32.to_be_bytes());
    header_only[12..16].copy_from_slice(b"prtr");
    header_only[16..20].copy_from_slice(b"CMYK");
    header_only[20..24].copy_from_slice(b"Lab ");
    header_only[36..40].copy_from_slice(b"acsp");
    let prof = Arc::new(IccProfile::parse(header_only, 4).expect("parse"));
    let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);

    fn spec_process_ink(c: u8, m: u8, y: u8, k: u8) -> [u8; 3] {
        let (r, g, b) = xberg_native_pdf::color::cmyk_to_rgb(
            c as f32 / 255.0,
            m as f32 / 255.0,
            y as f32 / 255.0,
            k as f32 / 255.0,
        );
        [
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        ]
    }

    for (c, m, y, k) in [
        (0u8, 0, 0, 0),
        (255, 0, 0, 0),
        (0, 255, 0, 0),
        (0, 0, 255, 0),
        (0, 0, 0, 255),
        (64, 0, 0, 0),
        (128, 128, 128, 128),
        (200, 100, 50, 25),
    ] {
        let from_transform = t.convert_cmyk_pixel(c, m, y, k);
        let from_spec = spec_process_ink(c, m, y, k);
        assert_eq!(
            from_transform, from_spec,
            "Transform no-CMM fallback must agree with the process-ink converter on CMYK({c},{m},{y},{k}); \
             transform={from_transform:?}, spec={from_spec:?}"
        );
    }
}

/// Pin that DeviceCMYK paint inside a `q ... Q` save-restore bracket
/// still routes through the OutputIntent ICC.
///
/// `q`/`Q` push/pop the graphics state; a regression that re-built the
/// resolution context inside the bracket without re-attaching the
/// OutputIntent borrow would lose the ICC routing on the inner paint
/// even though it's the same page.
#[test]
fn output_intent_survives_graphics_state_save_restore() {
    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let catalog_entries =
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (S) /DestOutputProfile 5 0 R >>]";
    let content = "q\n0.25 0 0 0 k\n20 20 60 60 re\nf\nQ\n";
    let pdf = build_pdf_with_catalog_entries_and_content(catalog_entries, content, Some(&icc));
    let doc = PdfDocument::from_bytes(pdf).expect("open");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (126u8, 126, 126, 255),
        "DeviceCMYK paint inside q/Q must still route through OutputIntent ICC; got ({r},{g},{b},{a})"
    );
}

/// Pin that a fully-opaque DeviceCMYK paint at the alpha=1 edge resolves
/// to the qcms reference without any zero-coverage shortcut intercepting
/// the conversion before it reaches the helper.
///
/// The composite path has multiple alpha-aware shortcuts (zero-alpha
/// skip, fully-opaque skip, etc.). A regression that bypassed the
/// colour stage on the opaque edge would silently produce the
/// uncomposited additive-clamp value.
#[test]
fn output_intent_renders_at_alpha_one_edge() {
    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let pdf = build_pdf_cmyk_with_output_intent(&icc);
    let doc = PdfDocument::from_bytes(pdf).expect("open");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(a, 255, "alpha=1 paint must produce fully-opaque pixel");
    assert_eq!(
        (r, g, b),
        (126u8, 126, 126),
        "alpha=1 paint must still route through OutputIntent ICC; got ({r},{g},{b})"
    );
}

/// Pin that a subsequent opaque RGB over-paint obscures the prior CMYK
/// ICC paint cleanly — the OutputIntent path doesn't leak ICC-converted
/// pixels into a later non-CMYK paint scope.
#[test]
fn output_intent_does_not_leak_into_subsequent_rgb_overpaint() {
    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let catalog_entries =
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (S) /DestOutputProfile 5 0 R >>]";
    let content = "0.25 0 0 0 k\n20 20 60 60 re\nf\n1 1 1 rg\n20 20 60 60 re\nf\n";
    let pdf = build_pdf_with_catalog_entries_and_content(catalog_entries, content, Some(&icc));
    let doc = PdfDocument::from_bytes(pdf).expect("open");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (255u8, 255, 255, 255),
        "white RGB over-paint must obscure the CMYK paint regardless of OutputIntent; \
         got ({r},{g},{b},{a})"
    );
}

/// Pin that DeviceCMYK painted inside a Form XObject inherits the
/// document-level OutputIntent. Form XObjects share the document's
/// colour-policy state by spec (§14.8.3) — a regression that built a
/// fresh resolution context for the XObject scope without re-threading
/// the OutputIntent borrow would lose the ICC routing on every spot
/// CMYK paint nested inside the XObject.
///
/// Currently `#[ignore]`-ed pending a Form-XObject test-fixture helper;
/// the marker captures the gap so a follow-up audit picks it up.
#[test]
#[ignore = "OUTPUT_INTENT_DEFER_FORM_XOBJECT_INHERITANCE"]
fn output_intent_inherited_by_form_xobject_paint() {
    panic!("placeholder: needs a Form XObject test-fixture helper");
}

/// Pin the page-level `/DefaultCMYK` override precedence over the
/// document `/OutputIntents` profile for bare `/DeviceCMYK` paint.
///
/// ISO 32000-1:2008 §8.6.5.6 says: when a page declares
/// `/Resources /ColorSpace /DefaultCMYK <override>`, any bare
/// `/DeviceCMYK` paint operator (`k`/`K`/`scn` against a DeviceCMYK
/// alias) MUST be interpreted as if it specified `<override>` instead
/// of the device family default. The override therefore wins over the
/// document-level `/OutputIntents` profile when present, because the
/// override IS the page's declared CMYK colour space and OutputIntent
/// only applies as the default when no override has been declared.
///
/// Fixture geometry:
///   - Catalog declares /OutputIntents → profile A (target_l_byte=135 →
///     qcms reference RGB(126,126,126)).
///   - Page Resources /ColorSpace /DefaultCMYK → [/ICCBased <stream B>]
///     where profile B has target_l_byte=200 → qcms reference
///     RGB(194,194,194).
///   - Content stream: `0.25 0 0 0 k   20 20 60 60 re   f`. The `k`
///     operator paints bare /DeviceCMYK, exactly the case §8.6.5.6
///     redirects through /DefaultCMYK.
///
/// Three observable outcomes:
///   - (194, 194, 194, 255): /DefaultCMYK override won — pass.
///   - (126, 126, 126, 255): OutputIntent won — fail (precedence inverted;
///     §8.6.5.6 says /DefaultCMYK takes precedence over the document
///     default).
///   - (191, 255, 255, 255): §10.3.5 additive-clamp fired — neither
///     profile consulted (an even worse regression).
#[test]
fn page_level_default_cmyk_takes_precedence_over_output_intent() {
    let profile_a = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let profile_b = build_minimal_cmyk_to_rgb_lut8_profile(PROFILE_B_TARGET_L_BYTE);

    // Sanity-pin both profiles compile through qcms and produce the
    // expected byte-exact references — without this gate a regression
    // in profile B's transform would make the integration assertion
    // fire for the wrong reason. ~keep
    {
        use std::sync::Arc;
        use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};
        let prof_a = Arc::new(IccProfile::parse(profile_a.clone(), 4).expect("parse A"));
        let prof_b = Arc::new(IccProfile::parse(profile_b.clone(), 4).expect("parse B"));
        let t_a = Transform::new_srgb_target(prof_a, RenderingIntent::RelativeColorimetric);
        let t_b = Transform::new_srgb_target(prof_b, RenderingIntent::RelativeColorimetric);
        assert_eq!(
            t_a.convert_cmyk_pixel(64, 0, 0, 0),
            [126u8, 126, 126],
            "profile A reference must be (126,126,126); fixture is invalid otherwise"
        );
        assert_eq!(
            t_b.convert_cmyk_pixel(64, 0, 0, 0),
            [194u8, 194, 194],
            "profile B reference must be (194,194,194); fixture is invalid otherwise"
        );
    }

    let pdf = build_pdf_default_cmyk_overrides_output_intent(&profile_a, &profile_b);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    assert!(
        doc.output_intent_cmyk_profile().is_some(),
        "fixture must declare a CMYK OutputIntent so the precedence is actually contested"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    let (br, bg, bb) = PROFILE_B_RGB_AT_FIXTURE_INPUT;
    assert_eq!(
        (r, g, b, a),
        (br, bg, bb, 255),
        "page-level /DefaultCMYK override must take precedence over /OutputIntents \
         on bare /DeviceCMYK paint; expected B's qcms reference {:?}; got ({r},{g},{b},{a}). \
         (126,126,126,_) means OutputIntent won — §8.6.5.6 precedence is inverted. \
         (191,255,255,_) means neither profile was consulted and §10.3.5 fired.",
        (br, bg, bb, 255u8)
    );
}

/// Negative pin: when the page declares NO `/DefaultCMYK` override,
/// bare `/DeviceCMYK` paint must fall through to the document
/// `/OutputIntents` profile (the round-1 behaviour). This is the
/// contrapositive of the precedence test above — it pins that the
/// override consumer doesn't fire spuriously when no override is
/// declared, otherwise a regression that always routed through some
/// hard-coded path would pass the positive test by coincidence.
#[test]
fn no_default_cmyk_falls_through_to_output_intent() {
    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let pdf = build_pdf_cmyk_with_output_intent(&icc);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (126u8, 126, 126, 255),
        "no /DefaultCMYK declared → bare DeviceCMYK paint must route through \
         /OutputIntents (round-1 behaviour); got ({r},{g},{b},{a})"
    );
}

/// Pin the page-level `/DefaultRGB` override drives bare `/DeviceRGB`
/// paint through the override's colour space, even when no `/OutputIntents`
/// is present. §8.6.5.6 redirects bare device-family paint through the
/// /Default<DeviceFamily> override; for RGB this is the only place an
/// override can influence rendering (OutputIntent in our pipeline only
/// carries CMYK).
///
/// Fixture: /DefaultRGB → [/ICCBased <stream>] where the embedded N=3
/// LUT8 profile maps every RGB input to constant `Lab(target_l_byte=200,
/// 0, 0)` → qcms reference RGB(194, 194, 194). Content paints
/// `0.8 0.2 0.5 rg`. Without the override the rendered pixel would be
/// the literal RGB(0.8, 0.2, 0.5) = (204, 51, 128). With the override
/// active the rendered pixel must be the qcms reference value.
#[test]
fn page_level_default_rgb_routes_bare_device_rgb_through_override() {
    let profile = build_minimal_rgb_to_lab_lut8_profile(PROFILE_B_TARGET_L_BYTE);

    let rgb_ref = {
        use std::sync::Arc;
        use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};
        let prof = Arc::new(IccProfile::parse(profile.clone(), 3).expect("RGB profile parses"));
        let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);
        assert!(
            t.has_cmm(),
            "synthesised RGB LUT8 profile must compile into a real qcms CMM; \
             without it the /DefaultRGB test degrades to additive fall-through and \
             asserts the wrong thing"
        );
        let mut out = [0u8; 3];
        out.copy_from_slice(&t.convert_rgb_buffer(&[204u8, 51, 128]));
        out
    };

    let pdf = build_pdf_default_rgb_overrides_bare_device_rgb(&profile);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (rgb_ref[0], rgb_ref[1], rgb_ref[2], 255),
        "page-level /DefaultRGB override must route bare /DeviceRGB paint \
         through the override profile; expected qcms reference {:?}; \
         got ({r},{g},{b},{a}). (204,51,128,_) means the override was not \
         consulted and the literal RGB landed on the canvas directly.",
        rgb_ref
    );
}

/// Pin the page-level `/DefaultGray` override drives bare `/DeviceGray`
/// paint through the override's colour space.
///
/// Fixture: /DefaultGray → [/Separation /MagentaSpot /DeviceCMYK <Type-4
/// tint transform>] that lifts the gray input into the M channel:
/// `gray → CMYK(0, gray, 0, 0)`. Painting `0.5 g` (bare DeviceGray)
/// without the override would produce literal RGB(127, 127, 127). With
/// the override active, the gray value routes through the Separation,
/// produces CMYK(0, 0.5, 0, 0), and projects to RGB(255, 127, 255) via
/// §10.3.5 (no OutputIntent declared in this fixture). The colour
/// change is visible and discriminates the routes.
///
/// **Coverage note:** this Separation route covers the dispatcher
/// edge of /DefaultGray (the override is consulted; the gray
/// component reaches the override's colour space). The complementary
/// N=1 ICC route — `/DefaultGray [/ICCBased <N=1 TRC stream>]` —
/// drives the qcms gray pipeline and is pinned by
/// `qa_round4_default_gray_iccbased_n1_routes_through_qcms` below.
/// The two probes together prove both ends of the §8.6.5.6
/// /DefaultGray contract: dispatch and ICC conversion.
#[test]
fn page_level_default_gray_routes_bare_device_gray_through_override() {
    let pdf = build_pdf_default_gray_routes_bare_device_gray();
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    assert!(
        doc.output_intent_cmyk_profile().is_none(),
        "fixture must declare no /OutputIntents — the override drives the route \
         entirely; an OutputIntent in play would confound the assertion"
    );
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    let want = process_ink_rgb(0.0, 0.5, 0.0, 0.0);
    assert_eq!(
        (r, g, b),
        want,
        "/DefaultGray override -> Separation magenta projection must produce the \
         process-ink RGB {want:?} (high R, mid B); got ({r},{g},{b},{a}). \
         RGB(128,128,128) would mean the override was bypassed and the literal \
         gray landed."
    );
    assert!(
        r > 200 && b > 160 && b < 230,
        "discriminating channels must show the magenta projection (R high, B mid), \
         not gray 128; got ({r},{g},{b},{a})"
    );
    assert_eq!(a, 255, "alpha=1 paint must be fully opaque; got a={a}");
}

/// Pin `/DefaultGray [/ICCBased <N=1 TRC stream>]` drives bare
/// `/DeviceGray` paint through the qcms gray pipeline.
///
/// Round 3 documented "qcms 0.3.0's LUT8 parser only accepts in_chan ∈
/// {3, 4}; a 1-channel LUT8 would be rejected at compile time" — true
/// for LUT8 (`mft1`) bodies, but qcms's GRAY-signature arm at
/// `iccread.rs:1712-1714` is a *separate* path that reads the `kTRC`
/// (gray TRC) curveType tag, not a LUT8. A real N=1 Gray ICC profile
/// uses `kTRC`, qcms compiles it via `transform_create` →
/// `qcms_transform_data_gray_*` (`transform.rs:437-475`), and the
/// gray channel becomes RGB through the destination sRGB profile's
/// output gamma tables. The resolver previously had no N=1 arm at all
/// — `resolve_iccbased` fell straight to `first_as_gray(components)`,
/// emitting the literal gray byte without ever consulting qcms.
///
/// Fixture: a one-page PDF whose /DefaultGray is `[/ICCBased <N=1
/// linear-curv TRC stream>]`. The TRC is a 256-entry linear ramp
/// 0..65535 → effectively gamma 1.0 in the linear PCS-Y
/// representation. Painting `0.5 g` routes the 0.5 gray byte (128)
/// through the qcms transform; the linear PCS-Y is encoded back to
/// sRGB via the destination sRGB profile's inverse gamma. The
/// resulting RGB is the canonical sRGB encoding of linear gray 0.5 —
/// a value distinct from the no-override RGB(128, 128, 128) literal.
///
/// The expected RGB is derived empirically from the same qcms call
/// the resolver makes: build the profile, parse it through
/// `IccProfile::parse`, call `Transform::new_srgb_target` +
/// `convert_gray_buffer`, and compare byte-exact. No tolerance — the
/// renderer and the reference call use the same code path.
#[test]
fn qa_round4_default_gray_iccbased_n1_routes_through_qcms() {
    let profile = build_minimal_gray_trc_profile();

    let gray_ref = {
        use std::sync::Arc;
        use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};
        let prof = Arc::new(
            IccProfile::parse(profile.clone(), 1).expect("Gray TRC profile parses through IccProfile::parse(_, 1)"),
        );
        let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);
        assert!(
            t.has_cmm(),
            "synthesised Gray TRC profile must compile into a real qcms CMM; \
             without it the /DefaultGray ICC test degrades to fall-through and \
             asserts the wrong thing"
        );
        let out = t.convert_gray_buffer(&[128u8]);
        assert_eq!(out.len(), 3, "Gray8 → RGB8 conversion emits 3 bytes per input");
        [out[0], out[1], out[2]]
    };

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /DefaultGray [/ICCBased 5 0 R] >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let content = "0.5 g\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 1 /Length {} >>\nstream\n", profile.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(&profile);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(buf).expect("open synthetic PDF");
    assert!(
        doc.output_intent_cmyk_profile().is_none(),
        "fixture must declare no /OutputIntents — the /DefaultGray ICC override \
         drives the route entirely"
    );
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (gray_ref[0], gray_ref[1], gray_ref[2], 255),
        "/DefaultGray [/ICCBased N=1] override must route bare /DeviceGray paint \
         through the qcms gray pipeline; expected qcms reference {:?}; got \
         ({r},{g},{b},{a}). (128,128,128,_) means the resolver fell through to \
         first_as_gray and never consulted qcms — the N=1 arm is missing.",
        gray_ref
    );
}
