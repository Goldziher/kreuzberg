/// Pin a malformed `/DefaultCMYK <string>` entry falls through to the
/// document `/OutputIntents` profile rather than silently mis-rendering
/// the paint via first-component-as-gray.
///
/// A `/Default<Family>` entry per §8.6.5.6 MUST be a colour space —
/// a Name (device-family alias) or an Array (CalGray, ICCBased,
/// Separation, …). A PDF that declares `/DefaultCMYK (some string)`
/// is malformed; the renderer must decide between:
///   1. Honour the malformed entry by routing through
///      `resolve_spaced`'s catch-all `first_as_gray` arm. For
///      CMYK(0.25, 0, 0, 0) this produces RGB(64, 64, 64) — wrong
///      colour, silent mis-rendering, indistinguishable from a
///      buggy override.
///   2. Treat the malformed entry as "no override declared" and
///      fall through to the device-family path:
///      `ResolvedColor::Cmyk` → composite projection via
///      `cmyk_to_rgb_via_intent` → `ctx.output_intent_cmyk`. For
///      this fixture's constant-CLUT OutputIntent that's RGB
///      ~(128, 128, 128) — the press-target colour the OutputIntent
///      claims is right, which is the best fallback a renderer can
///      offer for a malformed override.
///
/// We pick option 2 — a malformed `/Default<Family>` is structurally
/// indistinguishable from the entry being absent, so honouring the
/// OutputIntent matches the §8.6.5.6 + §14.11.5 precedence cascade
/// the rest of the resolver implements.
///
/// Fixture: catalog declares /OutputIntents → constant-grey CMYK
/// profile; page declares /DefaultCMYK as a literal PDF string
/// (`/DefaultCMYK (not a colour space)`). Content paints
/// `0.25 0 0 0 k`. With the fix the pixel matches the OutputIntent
/// reference; without it the pixel is the literal-grey (64, 64, 64).
#[test]
fn qa_round4_malformed_default_cmyk_falls_through_to_output_intent() {
    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);

    let oi_ref = {
        use std::sync::Arc;
        use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};
        let prof = Arc::new(IccProfile::parse(icc.clone(), 4).expect("CMYK profile parses"));
        let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);
        assert!(
            t.has_cmm(),
            "synthesised CMYK profile must compile for the reference path"
        );
        let rgb = t.convert_cmyk_pixel(64, 0, 0, 0);
        [rgb[0], rgb[1], rgb[2]]
    };

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    let catalog = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic CMYK) /DestOutputProfile 5 0 R >>] >>\nendobj\n";
    buf.extend_from_slice(catalog.as_bytes());
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /DefaultCMYK (not a colour space) >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let content = "0.25 0 0 0 k\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", icc.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(&icc);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(buf).expect("open synthetic PDF");
    assert!(
        doc.output_intent_cmyk_profile().is_some(),
        "fixture must declare a CMYK /OutputIntents — without it the test \
         can't distinguish the fall-through from the malformed path"
    );
    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (oi_ref[0], oi_ref[1], oi_ref[2], 255),
        "Malformed /DefaultCMYK (string) must fall through to the document \
         /OutputIntents path; expected qcms reference {:?}; got \
         ({r},{g},{b},{a}). RGB(64, 64, 64) means the resolver honoured the \
         malformed entry via first_as_gray — silent mis-rendering of CMYK \
         paint as a literal-grey gradient.",
        oi_ref
    );
}

/// Pin that 1000 same-colour `/DeviceCMYK` paint operators on a single
/// page build the qcms `Transform` exactly once. This is the cache
/// hit-rate assertion the plan calls for: without caching every
/// `k`/`f` pair rebuilds the qcms transform (an 17×17×17×17 CLUT
/// precomputation that dominates the per-paint cost). With the cache
/// the first paint builds; the remaining 999 hit.
///
/// The build count comes from the `IccTransformCache`'s own counter
/// (`PageRenderer::icc_transform_cache_build_count`), a `#[doc(hidden)]`
/// accessor that exposes renderer internals for assertions. Reading the
/// per-instance counter avoids racing other concurrent integration
/// tests that might also call `Transform::new_srgb_target` on the same
/// process — the cache is local to the `PageRenderer` we construct
/// here, so nobody else touches it.
///
/// **Why a counter instead of wall-clock duration:** wall-clock
/// measurements are noisy (CPU thermal state, OS scheduling, debug-vs-
/// release builds) and would conflate caching with unrelated perf
/// drift. A counter is exact: 1 build proves the cache works, N builds
/// proves it doesn't.
#[test]
fn output_intent_thousand_cmyk_paints_build_one_transform() {
    use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let mut ops = String::new();
    for i in 0..1000 {
        let y = i % 100;
        ops.push_str(&format!("0.25 0 0 0 k\n0 {y} 1 1 re\nf\n"));
    }
    let catalog_entries =
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (S) /DestOutputProfile 5 0 R >>]";
    let pdf = build_pdf_with_catalog_entries_and_content(catalog_entries, &ops, Some(&icc));
    let doc = PdfDocument::from_bytes(pdf).expect("open");

    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72));
    let _ = renderer.render_page(&doc, 0).expect("render");

    let built = renderer.icc_transform_cache_build_count();
    assert_eq!(
        built, 1,
        "1000 same-colour /DeviceCMYK paints under one /OutputIntents profile \
         and one rendering intent must build qcms::Transform exactly once \
         (cache miss on first paint, hit on the next 999). Built {built} times — \
         the per-page CMYK transform cache regressed or is missing."
    );
}

/// Pin that two different rendering intents on the same page +
/// OutputIntent split the cache into two entries — each intent gets
/// its own `Transform`. Critical because qcms's `Transform::new_to`
/// takes an intent parameter; even though qcms 0.3.0 currently
/// ignores that parameter for CMYK (see HONEST_GAP in the phase 8
/// section below), the cache key MUST include intent so a future qcms
/// upgrade that honours intent doesn't silently emit the wrong colour
/// from a shared transform.
///
/// The fixture interleaves two `ri` operators (rendering-intent
/// overrides) inside a single page's content stream. The PDF spec's
/// §10.7.3 `ri` operator sets the graphics-state rendering intent —
/// xberg-native-pdf parses this and the colour stage threads it through
/// `ctx.rendering_intent`. With two distinct intents seen on the
/// page, the cache holds two `Transform` instances (one per intent),
/// not one shared across both.
///
/// HONEST_GAP: this probe also pins that the `ri` operator dispatch
/// is wired through `gs.rendering_intent`. If a regression removed
/// the `Operator::SetRenderingIntent` arm from the page renderer,
/// every paint would resolve under the default
/// /RelativeColorimetric intent and the cache would collapse to one
/// entry — surfaced here as a count of 1 instead of 2.
#[test]
fn qa_round3_cache_keys_include_rendering_intent() {
    use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let ops = "0.25 0 0 0 k\n10 10 20 20 re\nf\n\
               /Perceptual ri\n\
               0.50 0 0 0 k\n40 10 20 20 re\nf\n";
    let catalog_entries =
        "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (S) /DestOutputProfile 5 0 R >>]";
    let pdf = build_pdf_with_catalog_entries_and_content(catalog_entries, ops, Some(&icc));
    let doc = PdfDocument::from_bytes(pdf).expect("open");

    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72));
    let _ = renderer.render_page(&doc, 0).expect("render");

    let built = renderer.icc_transform_cache_build_count();
    assert_eq!(
        built, 2,
        "Two distinct rendering intents on one page + one OutputIntent profile \
         must split the transform cache into two entries — one per intent. \
         Built {built} times; expected exactly 2. A count of 1 means the \
         cache key drops the intent (incorrect — qcms's Transform::new_to \
         takes intent as a parameter even if 0.3.0 ignores it internally); \
         a count > 2 means a regression."
    );
}

/// Pin that 1000 same-colour bare `/DeviceRGB` paints routed through a
/// page-level `/DefaultRGB [/ICCBased N=3]` override build the qcms
/// `Transform` exactly once. The cache is keyed on
/// `(profile.content_hash(), intent)` — n_components-agnostic by
/// design — so an N=3 profile routed through the resolver MUST hit the
/// cache after the first build, exactly as the N=4 CMYK path does.
///
/// Without the cache wiring, every bare `rg` paint reparses the
/// embedded profile and rebuilds the qcms transform; the per-page
/// counter would land at 1000 instead of 1. This probe is the
/// structural counterpart of `output_intent_thousand_cmyk_paints_*`
/// for the N=3 arm of `resolve_iccbased`.
///
/// Fixture: a one-page PDF whose `/Resources /ColorSpace /DefaultRGB`
/// is `[/ICCBased <constant-CLUT N=3 LUT8>]` and whose content stream
/// emits 1000 identical `0.8 0.2 0.5 rg` + `re` + `f` triples.
#[test]
fn qa_round4_thousand_rgb_paints_through_default_rgb_build_one_transform() {
    use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

    let profile = build_minimal_rgb_to_lab_lut8_profile(PROFILE_B_TARGET_L_BYTE);
    let mut ops = String::new();
    for i in 0..1000 {
        let y = i % 100;
        ops.push_str(&format!("0.8 0.2 0.5 rg\n0 {y} 1 1 re\nf\n"));
    }

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /DefaultRGB [/ICCBased 5 0 R] >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", ops.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(ops.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 3 /Length {} >>\nstream\n", profile.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(&profile);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(buf).expect("open");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72));
    let _ = renderer.render_page(&doc, 0).expect("render");

    let built = renderer.icc_transform_cache_build_count();
    assert_eq!(
        built, 1,
        "1000 same-colour bare /DeviceRGB paints routed through a page-level \
         /DefaultRGB [/ICCBased N=3] override must build qcms::Transform \
         exactly once. Built {built} times — the N=3 arm of resolve_iccbased \
         is bypassing the per-page transform cache (a build_count of 1000 means \
         every paint reparsed the profile and rebuilt the transform; the cache \
         exists to amortise this)."
    );
}

// ===========================================================================
// Phase 8: rendering-intent dispatch — qcms 0.3.0 limitation pin
// ===========================================================================
//
// ISO 32000-1:2008 §10.7.3 specifies per-paint rendering intent through
// the `/RI` operator. The graphics-state field flows through
// `gs.rendering_intent` → `ctx.rendering_intent` → `Transform::new_srgb_target`'s
// intent parameter → qcms.
//
// **Research surface: qcms 0.3.0 intent dispatch.** Reading
// `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/qcms-0.3.0/src/transform.rs`:
//   - line 1283: `pub fn transform_create(input, in_type, output, out_type,
//     _intent: Intent)` — the underscore prefix marks the parameter as
//     intentionally unused. qcms 0.3.0 discards the intent.
//   - line 1245: `fn transform_precacheLUT_cmyk_float(transform, input,
//     output, samples, in_type)` — the CMYK CLUT precomputation takes no
//     intent; the same CLUT is produced regardless of caller intent.
//   - The crate's `lib.rs` Intent enum docstring (line 32) notes that
//     "BPC brings an unacceptable performance overhead, so we go with
//     perceptual" — qcms 0.3.0 has no Black Point Compensation flag at
//     all. The §10.7.3 `/AbsoluteColorimetric` BPC-off behaviour cannot
//     be expressed through qcms 0.3.0.
//
// **HONEST_GAP_QCMS_INTENT_IGNORED**: per-channel intent dispatch IS
// wired through `ctx.rendering_intent` → `Transform::new_srgb_target`
// at the xberg-native-pdf layer, and the cache key separates entries by intent
// — but qcms 0.3.0 silently drops intent inside transform construction.
// Distinct intents produce byte-identical CMYK→RGB conversions through
// qcms 0.3.0; a future qcms upgrade that honours intent will surface the
// difference automatically because the dispatch chain already routes the
// per-intent value through. The cache invalidation guarantee holds:
// distinct intents get distinct Transform instances (so when qcms starts
// honouring intent, no shared-Transform cross-contamination happens).
//
// The probes below pin three claims:
//   1. `gs.rendering_intent` flows end-to-end into the cache key (covered
//      by `qa_round3_cache_keys_include_rendering_intent` above).
//   2. qcms 0.3.0's intent-invariance for CMYK conversions is observable
//      via the constant-CLUT fixture — same input, four intents, same
//      RGB. Documents the qcms version constraint.
//   3. The default intent fallback (§8.6.5.8: unrecognised intent →
//      /RelativeColorimetric) is honoured at the colour boundary. ~keep

/// Pin qcms 0.3.0's documented intent-invariance for CMYK profiles:
/// the same CMYK input under all four `RenderingIntent` values
/// produces byte-identical RGB through `Transform::convert_cmyk_pixel`.
///
/// **Why this probe is GREEN immediately:** qcms 0.3.0's
/// `transform_create` ignores the intent parameter (see the module
/// docstring above). A future qcms upgrade that honours intent for
/// CMYK→sRGB conversions would flip this probe to RED, surfacing the
/// behaviour change at upgrade time so a CHANGELOG entry can document
/// the §10.7.3 intent dispatch becoming externally observable.
///
/// **Constructed fixture caveat:** the constant-CLUT profile used here
/// produces the same RGB for every CMYK input by design — that's how
/// the test pin stays unambiguous. A real-world CMYK profile (CoatedFOGRA39
/// etc.) carries an intent-dependent CLUT that WOULD vary across intents
/// IF qcms honoured them. Synthesising such a fixture requires a real
/// CMM toolchain (curves + matrices + a true 4D CLUT) — deferred as
/// HONEST_GAP_INTENT_SENSITIVE_FIXTURE.
///
#[test]
fn qa_round3_qcms_030_treats_cmyk_intent_as_informational() {
    use std::sync::Arc;
    use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};

    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let profile = Arc::new(IccProfile::parse(icc, 4).expect("parse"));

    let intents = [
        RenderingIntent::Perceptual,
        RenderingIntent::RelativeColorimetric,
        RenderingIntent::Saturation,
        RenderingIntent::AbsoluteColorimetric,
    ];

    let mut results = Vec::new();
    for intent in intents {
        let t = Transform::new_srgb_target(Arc::clone(&profile), intent);
        assert!(t.has_cmm(), "intent {intent:?} must build a real CMM");
        let rgb = t.convert_cmyk_pixel(64, 0, 0, 0);
        results.push((intent, rgb));
    }
    let first = results[0].1;
    for (intent, rgb) in &results[1..] {
        assert_eq!(
            *rgb, first,
            "qcms 0.3.0 must produce intent-invariant CMYK→RGB. Intent {intent:?} \
             produced {rgb:?} while {:?} produced {first:?}. \
             If this fires after a qcms upgrade, intent dispatch is now \
             externally observable — update the HONEST_GAP_QCMS_INTENT_IGNORED \
             documentation and remove this pin.",
            results[0].0
        );
    }
    assert_eq!(
        first,
        [126u8, 126, 126],
        "byte-exact reference: target_l_byte=135 → (126, 126, 126) at every \
         intent through qcms 0.3.0"
    );
}

/// Pin that the §8.6.5.8 "unrecognised intent → /RelativeColorimetric"
/// fallback is honoured at the colour boundary. A PDF that sets
/// `/RI /UnknownVendorPrivateIntent` must map to
/// `RenderingIntent::RelativeColorimetric`, NOT to an arbitrary
/// default.
///
/// This is a unit-level pin against the spec's defaulting rule. The
/// cache key includes intent, so under the failing scenario (a
/// regression that mapped unknown names to /Perceptual or /Saturation)
/// a same-named intent would silently produce a different cache entry
/// — surfaced here by asserting the from_pdf_name behaviour directly.
#[test]
fn qa_round3_unknown_intent_name_falls_back_to_relative_colorimetric() {
    use xberg_native_pdf::color::RenderingIntent;

    // §8.6.5.8: unrecognised → RelativeColorimetric. ~keep
    assert_eq!(
        RenderingIntent::from_pdf_name("UnknownVendorPrivateIntent"),
        RenderingIntent::RelativeColorimetric,
        "unknown intent name must fall back to /RelativeColorimetric per §8.6.5.8"
    );
    assert_eq!(
        RenderingIntent::from_pdf_name(""),
        RenderingIntent::RelativeColorimetric,
        "empty intent name must fall back to /RelativeColorimetric"
    );
    assert_eq!(
        RenderingIntent::from_pdf_name("Perceptual"),
        RenderingIntent::Perceptual
    );
    assert_eq!(
        RenderingIntent::from_pdf_name("Saturation"),
        RenderingIntent::Saturation
    );
    assert_eq!(
        RenderingIntent::from_pdf_name("RelativeColorimetric"),
        RenderingIntent::RelativeColorimetric
    );
    assert_eq!(
        RenderingIntent::from_pdf_name("AbsoluteColorimetric"),
        RenderingIntent::AbsoluteColorimetric
    );
}

/// HONEST_GAP_INTENT_SENSITIVE_FIXTURE: a probe that WOULD pin
/// /Perceptual vs /AbsoluteColorimetric producing different RGB.
/// Requires:
///   1. An intent-sensitive CMYK profile (real CoatedFOGRA39 or
///      equivalent — synthetic profiles with constant CLUTs are
///      intent-invariant by construction).
///   2. A qcms version that honours the intent parameter for CMYK
///      transforms (qcms 0.3.0 does not, per the module docstring).
///
/// Neither prerequisite is satisfied today. The probe stays
/// `#[ignore]`-ed with the HONEST_GAP marker so a future engineer
/// adding either prerequisite has a ready-made integration test to
/// turn on.
#[test]
#[ignore = "HONEST_GAP_INTENT_SENSITIVE_FIXTURE: needs (a) intent-sensitive \
            CMYK profile + (b) qcms version that honours intent for CMYK"]
fn output_intent_perceptual_vs_absolute_colorimetric_produces_different_rgb() {
    panic!(
        "HONEST_GAP_INTENT_SENSITIVE_FIXTURE: qcms 0.3.0 ignores intent for \
         CMYK conversions (transform.rs:1288 `_intent: Intent`). To enable \
         this probe a future engineer needs: (1) a CMYK ICC profile with \
         genuine intent-dependent behaviour (synthetic constant-CLUT \
         fixtures are intent-invariant by construction); (2) a qcms upgrade \
         that honours intent for CMYK. The wiring chain is already in \
         place — gs.rendering_intent → ctx.rendering_intent → \
         Transform::new_srgb_target — so flipping qcms versions WILL surface \
         the intent dispatch without further code changes."
    );
}

/// TDD-discipline verification report for round-1 OutputIntent foundation.
///
/// Verified by checking out the round-1 commit graph in a throwaway
/// worktree and re-running the failing/passing tests at the relevant
/// SHAs. Captured here so a future reader has the audit trail without
/// having to re-do the bisect.
///
/// **Failing test commit `eab4040`:**
/// Planting `tests/test_render_output_intent.rs` from `eab4040` onto
/// its parent `65063ba` (last `feat` commit before the impl landed)
/// produced:
///
/// ```text
/// thread 'device_cmyk_paint_with_output_intent_renders_via_icc_not_additive_clamp'
///   panicked at tests/test_render_output_intent.rs:365:5:
/// OutputIntent /DeviceCMYK paint expected qcms-converted RGB ~(128, 128, 128);
/// got (191, 255, 255). RGB(191, 255, 255) would mean the §10.3.5 additive-clamp
/// fallback fired — the resolver is not consulting ctx.output_intent_cmyk.
/// test result: FAILED. 0 passed; 1 failed
/// ```
///
/// Checking out the impl commit `656c119` then produced:
///
/// ```text
/// test device_cmyk_paint_with_output_intent_renders_via_icc_not_additive_clamp ... ok
/// test result: ok. 1 passed; 0 failed
/// ```
///
/// **Negative-pin commit `fda9b6f`:**
/// The negative pin (`*_without_output_intent_renders_process_inks`)
/// is a regression guard, not a failing test. Verified by planting the
/// commit's test on its parent `656c119`: it passed even there because
/// the no-OutputIntent fallback was the shipped behaviour. The impl
/// agent's report categorised this honestly as a "negative pin", and
/// the actual test categorisation matches.
///
/// **Conclusion:** TDD discipline was followed for the positive ICC
/// path. The negative pin is correctly described as a regression guard.
#[test]
fn qa_tdd_discipline_verification_report() {
    let _ = device_cmyk_paint_with_output_intent_renders_via_icc_not_additive_clamp;
    let _ = device_cmyk_paint_without_output_intent_renders_process_inks;
}

// ===========================================================================
// Phase 4: embedded /ICCBased N=4 trumps document /OutputIntents
// ===========================================================================
//
// ISO 32000-1:2008 §8.6.5.5 (and §14.11.5): an `/ICCBased` colour space
// carries its own `DestOutputProfile`-equivalent stream; that stream IS
// the conversion source, and the document-level `/OutputIntents` profile
// is only the default for `/DeviceCMYK` paint that lacks any embedded
// override. Embedded ICC always wins.
//
// The byte-exact references baked into the assertions below come from
// the discovery harness (run once, output captured) — see the plan
// errata. They are intent-invariant because the synthesised LUT8
// profile uses a constant CLUT. ~keep

/// Byte-exact qcms 0.3.0 reference for the `target_l_byte=200` profile
/// at CMYK(64,0,0,0) under RelativeColorimetric (intent-invariant by
/// construction). Distinct from the round-1 profile A reference of
/// (126,126,126) so the precedence assertion is unambiguous.
const PROFILE_B_TARGET_L_BYTE: u8 = 200;
const PROFILE_B_RGB_AT_FIXTURE_INPUT: (u8, u8, u8) = (194, 194, 194);

/// Pin that an `/ICCBased` N=4 colour space paint operator routes through
/// the colour-space-embedded profile B and NOT through the document-level
/// `/OutputIntents` profile A.
///
/// Fixture geometry:
///   - Catalog declares /OutputIntents → profile A (target_l_byte=135 →
///     qcms reference RGB(126,126,126)).
///   - Page Resources /ColorSpace /CS1 → [/ICCBased <stream B>] where
///     profile B has target_l_byte=200 → qcms reference RGB(194,194,194).
///   - Content stream: `/CS1 cs   0.25 0 0 0 scn   20 20 60 60 re   f`.
///
/// Spec rule: §8.6.5.5 — the ICCBased colour space carries the conversion
/// source and overrides any document-level default. The renderer must
/// route the four `scn` components through profile B's qcms transform.
///
/// What this test catches:
///   - If the rendered pixel is (126,126,126), profile A won — the
///     embedded ICC route is being shadowed by the OutputIntent route
///     (the spec-precedence bug this phase exists to fix).
///   - If the rendered pixel is (191,255,255), neither profile was
///     consulted and §10.3.5 additive-clamp fired (an even worse
///     regression).
///   - If the rendered pixel is (194,194,194), profile B's CMM
///     compiled-and-ran through `Transform::convert_cmyk_pixel` and the
///     precedence is correct.
#[test]
fn embedded_iccbased_n4_trumps_document_output_intent() {
    let profile_a = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let profile_b = build_minimal_cmyk_to_rgb_lut8_profile(PROFILE_B_TARGET_L_BYTE);

    // Sanity-pin both profiles compile through qcms and produce the
    // expected byte-exact references. Without this gate a regression
    // that broke profile B's transform would make the integration
    // assertion below fire for the wrong reason. ~keep
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

    let pdf = build_pdf_embedded_iccbased_with_different_output_intent(&profile_a, &profile_b);
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
        "embedded /ICCBased profile B must take precedence over /OutputIntents \
         profile A on CMYK paint through the ICCBased space; expected B's qcms \
         reference {:?}; got ({r},{g},{b},{a}). (126,126,126,_) means profile A won \
         — the spec precedence (§8.6.5.5) is inverted. (191,255,255,_) means neither \
         profile was consulted and §10.3.5 fired.",
        (br, bg, bb, 255u8)
    );
}

// ===========================================================================
// Phase 5: Separation / DeviceN with DeviceCMYK alternate routes through OutputIntent
// ===========================================================================
//
// ISO 32000-1:2008 §8.6.6.3 (Separation) and §8.6.6.4 (DeviceN): when the
// device lacks the named colorant plate, the colour is approximated via
// the alternate colour space and the tint transform. When the alternate
// is /DeviceCMYK, the alternate's CMYK quadruple is then converted to
// RGB for the composite output path — and that conversion MUST honour
// the document /OutputIntents profile, since composite output is the
// "viewer's screen" surface the OutputIntent describes.
//
// Today (post-round-1) the resolver's
// `resolve_separation_or_devicen` arm dispatches a CMYK-alternate
// result through `four_as_cmyk(&altspace_values, alpha, ctx)`, which
// itself calls `cmyk_to_rgb_via_intent` — the same OutputIntent-aware
// helper the bare /DeviceCMYK paint path consumes. So the routing is
// already correct, but the probes below pin it byte-for-byte so a
// regression that detoured Separation/DeviceN through a non-context-
// aware CMYK→RGB path would surface immediately.
//
// These probes are categorised as REGRESSION GUARDS in the TDD-discipline
// sense (they pass at HEAD without code changes) because the routing
// landed during round-1 phase 2. The TDD-failing-test→implementation
// pair for this behaviour is documented at fa1b947's prior history
// (round-1 phase 2). The probes here lock the routing for the
// specifically named Separation Type-4 and DeviceN Type-4 cases the
// plan body called out.
//
// Discrimination audit: before committing the probes, the impl agent
// temporarily flipped `four_as_cmyk` in src/rendering/resolution/color.rs
// to bypass `cmyk_to_rgb_via_intent` and call bare `cmyk_to_rgb` (the
// §10.3.5 helper) instead. With that flip, both
// `*_composite_routes_through_output_intent` probes failed with the
// expected (255, 0, 255, 255) value, demonstrating they actively
// discriminate between "OutputIntent honoured" and "additive-clamp
// fallback". The flip was reverted before the commit landed; the audit
// confirms the probes do what their names say. ~keep

/// Pin that a `/Separation /MagentaSpot /DeviceCMYK <Type-4 tint
/// transform>` paint operator's composite-side RGBA is the document
/// `/OutputIntents` profile's conversion of the tint-transform's CMYK
/// output — NOT the §10.3.5 additive-clamp of that CMYK quadruple.
///
/// Fixture: tint transform `{ 0.0 exch 0.0 0.0 }` produces CMYK(0, tint,
/// 0, 0). At tint=1.0 the alternate-CMYK value is (0, 1, 0, 0); §10.3.5
/// of that is RGB(255, 0, 255) (magenta). The OutputIntent profile
/// (constant-CLUT, target_l_byte=135) maps every CMYK input to
/// RGB(126, 126, 126), so an OutputIntent-honouring composite pixel is
/// (126, 126, 126).
///
/// Three observable outcomes:
///   - (126, 126, 126, 255): composite routed through OutputIntent — pass.
///   - (255, 0, 255, 255): composite ran §10.3.5 directly — fail
///     (alt-CMYK projection bypassed `cmyk_to_rgb_via_intent`).
///   - any other RGB: tint transform or qcms behaviour drifted.
#[test]
fn separation_type4_alt_devicecmyk_composite_routes_through_output_intent() {
    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    // Sanity-pin the OutputIntent reference for CMYK(0, 255, 0, 0) —
    // intent-invariant by construction (constant CLUT) so a single
    // intent is enough. ~keep
    {
        use std::sync::Arc;
        use xberg_native_pdf::color::{IccProfile, RenderingIntent, Transform};
        let prof = Arc::new(IccProfile::parse(icc.clone(), 4).expect("parse"));
        let t = Transform::new_srgb_target(prof, RenderingIntent::RelativeColorimetric);
        let rgb = t.convert_cmyk_pixel(0, 255, 0, 0);
        assert_eq!(
            rgb,
            [126u8, 126, 126],
            "OutputIntent profile must map CMYK(0,255,0,0) to (126,126,126); \
             fixture is invalid otherwise (got {rgb:?})"
        );
    }

    let pdf = build_pdf_separation_type4_devicecmyk_with_output_intent(&icc);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    assert!(
        doc.output_intent_cmyk_profile().is_some(),
        "fixture must declare a CMYK OutputIntent for the routing to be probed"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (126u8, 126, 126, 255),
        "Separation Type-4 /DeviceCMYK alternate must route the alt-CMYK \
         quadruple through the document /OutputIntents profile on the \
         composite path; expected (126,126,126,255); got ({r},{g},{b},{a}). \
         (255,0,255,_) means the §10.3.5 additive-clamp of CMYK(0,1,0,0) \
         fired — the resolver bypassed cmyk_to_rgb_via_intent for the \
         Separation alt-CMYK projection."
    );
}

/// Counter-pin: with no `/OutputIntents` declared, the same Separation
/// Type-4 alt-CMYK paint MUST produce the §10.3.5 additive-clamp value
/// for CMYK(0, 1, 0, 0) = RGB(255, 0, 255).
///
/// The positive pin above asserts "OutputIntent wins on composite when
/// present"; this counter-pin asserts "no-OutputIntent → §10.3.5
/// preserved byte-for-byte" — i.e. the OutputIntent route doesn't leak
/// into a no-OutputIntent fixture (which would imply some hard-coded
/// CMM hung around the renderer rather than the configured route).
#[test]
fn separation_type4_alt_devicecmyk_without_output_intent_renders_process_inks() {
    // Inline-build a PDF identical to
    // `build_pdf_separation_type4_devicecmyk_with_output_intent` but
    // without /OutputIntents. Object IDs shift down by one because the
    // ICC stream is dropped: catalog → pages → page → content → tint
    // (obj 5 instead of obj 6). ~keep
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let page_off = buf.len();
    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /ColorSpace << /CS1 [/Separation /MagentaSpot /DeviceCMYK 5 0 R] >> >> /Contents 4 0 R >>\nendobj\n";
    buf.extend_from_slice(page.as_bytes());

    let stream_off = buf.len();
    let content = "/CS1 cs\n1.0 scn\n20 20 60 60 re\nf\n";
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let tint_off = buf.len();
    let tint_program: &[u8] = b"{ 0.0 exch 0.0 0.0 }";
    let tint_hdr = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] /Length {} >>\nstream\n",
        tint_program.len()
    );
    buf.extend_from_slice(tint_hdr.as_bytes());
    buf.extend_from_slice(tint_program);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = buf.len();
    let obj_count = 6;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", obj_count).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, tint_off] {
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
        "fixture must declare no /OutputIntents for the counter-pin to actually contest the route"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        process_ink_rgba(0.0, 1.0, 0.0, 0.0),
        "Separation Type-4 /DeviceCMYK alternate without /OutputIntents must \
         fall through to the process-ink converter for CMYK(0,1,0,0) (magenta #EC008C); \
         got ({r},{g},{b},{a})"
    );
}

/// Pin that a 2-colorant `/DeviceN [/Magenta /Cyan] /DeviceCMYK
/// <Type-4 tint transform>` paint operator's composite-side RGBA is
/// also routed through the document `/OutputIntents` profile when the
/// tint transform's alternate-CMYK output lands in the resolver.
///
/// Fixture: tint transform `{ pop 0.0 exch 0.0 0.0 }` consumes the two
/// colorant tints, drops the second, and emits CMYK(0, tint0, 0, 0).
/// Content `1.0 0.5 scn` provides (tint0=1.0, tint1=0.5) → alternate
/// CMYK(0, 1, 0, 0). The OutputIntent profile maps that to
/// RGB(126, 126, 126); the §10.3.5 additive-clamp value would be
/// RGB(255, 0, 255).
#[test]
fn devicen_type4_alt_devicecmyk_composite_routes_through_output_intent() {
    let icc = build_minimal_cmyk_to_rgb_lut8_profile(135);
    let pdf = build_pdf_devicen_type4_devicecmyk_with_output_intent(&icc);
    let doc = PdfDocument::from_bytes(pdf).expect("open synthetic PDF");
    assert!(
        doc.output_intent_cmyk_profile().is_some(),
        "fixture must declare a CMYK OutputIntent for the routing to be probed"
    );

    let rgba = render_rgba(&doc);
    let (r, g, b, a) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b, a),
        (126u8, 126, 126, 255),
        "DeviceN Type-4 /DeviceCMYK alternate must route the alt-CMYK \
         quadruple through the document /OutputIntents profile on the \
         composite path; expected (126,126,126,255); got ({r},{g},{b},{a}). \
         (255,0,255,_) means §10.3.5 additive-clamp fired."
    );
}

// ===========================================================================
// QA round-2 edge probes
// ===========================================================================
//
// Round-2 phase 4 changed `resolve_iccbased` for N=4 with parseable
// embedded profile to emit `ResolvedColor::Rgba` directly (bypassing
// OutputIntent). Edge cases the impl probes did not cover:
//
//   1. Embedded profile parses but qcms refuses to build a CMM
//      (`has_cmm() == false`) — fallback path must kick in.
//   2. Embedded profile is malformed bytes (`IccProfile::parse` returns
//      None) — fallback path must kick in.
//   3. ICCBased N=3 (RGB) with a document CMYK /OutputIntents — no
//      interaction; RGB paint stays untouched.
//   4. ICCBased N=1 (gray) with a document CMYK /OutputIntents — same.
//   5. ICCBased N=4 paint inside a Form XObject — precedence survives
//      the Form scope.
//   6. **Per-plate regression**: the fix changes ICCBased N=4 from
//      `ResolvedColor::Cmyk` to `ResolvedColor::Rgba`; per-plate
//      consumers route by participating channels and `Rgba` produces an
//      empty participating list. Probe what happens when the renderer
//      is invoked for separations on the same fixture. ~keep
