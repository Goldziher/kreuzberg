// ===========================================================================
// §11.4.5 transparency groups — `/K` knockout flag (HONEST_GAP)
// =========================================================================== ~keep

fn fixture_knockout_group_two_overlapping_rects() -> Vec<u8> {
    // Knockout group containing two overlapping rectangles, the
    // second painted with /ca 0.5. Per §11.4.5 knockout semantics, the
    // second rect knocks the first rect's accumulated transparency
    // out and composites against the group backdrop directly. Without
    // knockout (the current behaviour), the second rect composites
    // against the accumulated first rect's contribution. The two
    // results differ in the overlap region. ~keep
    let form_content = "1 0 0 rg\n\
                        10 10 50 50 re\nf\n\
                        /Half gs\n\
                        0 0 1 rg\n\
                        40 40 50 50 re\nf\n";
    let form_resources = "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >>";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /K true >> \
         /Resources << {} >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_resources,
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Fm1 Do\n";
    let resources = "/XObject << /Fm1 5 0 R >>";
    build_pdf(content, resources, &[obj_5.as_bytes()])
}

/// Regression sentry — knockout group `/K true` per §11.4.6.2. Inside
/// the overlap region the blue rect at α=0.5 composites against the
/// group's white backdrop (not against the red rect that painted there
/// first).
#[test]
fn knockout_group_resets_destination_per_element() {
    let rgba = render_rgba(fixture_knockout_group_two_overlapping_rects());
    let (_r, g, _b, _) = pixel_at(&rgba, 50, 50);
    assert!(
        g > 100,
        "ISO 32000-1 §11.4.6.2 knockout: overlap region must reset to white \
         backdrop before compositing blue; expected G > 100, got G={g}"
    );
}

// ===========================================================================
// §11.4.5 Form XObject /Group dict — regression sentry
// ===========================================================================
//
// A Form XObject whose /Group dict declares /S /Transparency triggers
// the transparency-group code path even without /I or /K. The probe
// confirms the Form-with-/Group dispatch wires the group composition
// helpers rather than degenerating to a direct render. ~keep

fn fixture_form_with_group_dict_blue_over_white() -> Vec<u8> {
    let form_content = "0 0 1 rg\n\
                        20 20 60 60 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency >> \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Fm1 Do\n";
    let resources = "/XObject << /Fm1 5 0 R >>";
    build_pdf(content, resources, &[obj_5.as_bytes()])
}

#[test]
fn form_xobject_group_dict_with_transparency_paints_blue() {
    let rgba = render_rgba(fixture_form_with_group_dict_blue_over_white());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (0, 0, 255),
        "Form-XObject /Group /S /Transparency must paint byte-exact \
         blue (0, 0, 255); got ({r}, {g}, {b})"
    );
}

// ===========================================================================
// §11.3.5.2 separable blend modes
// ===========================================================================
//
// All twelve separable PDF blend modes dispatch through
// `pdf_blend_mode_to_skia` (src/rendering/mod.rs:80-95) to the
// corresponding tiny_skia::BlendMode. We pin five high-signal modes —
// Multiply, Screen, Darken, Lighten, Difference — against
// deterministic over-white / over-blue / over-green references.
// (HardLight / SoftLight / ColorDodge / ColorBurn / Overlay /
// Exclusion would each need an extra fixture; the five chosen are a
// representative sample of the parser/dispatch path. A per-mode
// matrix is in scope for a later round.) ~keep

/// Multiply blend of red (255, 0, 0) over white (255, 255, 255):
/// per §11.3.5.2 the per-channel result is `Cb · Cs`. With Cb=white
/// and Cs=red, the result is exactly red — Multiply against white is
/// identity. This pins the dispatch + paint chain.
fn fixture_blend_multiply_red_over_white() -> Vec<u8> {
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Mul gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Mul << /Type /ExtGState /BM /Multiply >> >>";
    build_pdf(content, resources, &[])
}

#[test]
fn blend_multiply_red_over_white_yields_red() {
    let rgba = render_rgba(fixture_blend_multiply_red_over_white());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(r, 255, "Multiply red×white: R must be 255; got ({r}, {g}, {b})");
    assert!(
        g < 10 && b < 10,
        "Multiply red×white: G/B must be ~0; got ({r}, {g}, {b})"
    );
}

/// Multiply blend of red over a grey backdrop must darken: per-channel
/// result is `Cb · Cs / 255`. Red (255, 0, 0) over grey (128, 128, 128)
/// = (128·255/255, 128·0/255, 128·0/255) = (128, 0, 0).
fn fixture_blend_multiply_red_over_grey() -> Vec<u8> {
    let content = "0.5 g\n0 0 100 100 re\nf\n\
                   /Mul gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Mul << /Type /ExtGState /BM /Multiply >> >>";
    build_pdf(content, resources, &[])
}

#[test]
fn blend_multiply_red_over_grey_yields_dark_red() {
    let rgba = render_rgba(fixture_blend_multiply_red_over_grey());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // 0.5 g in PDF DeviceGray → byte (128, 128, 128). Multiply per
    // §11.3.5.2: R = Cb·Cs = 128·255/255 = 128, G = 128·0/255 = 0,
    // B = 128·0/255 = 0. Byte-exact (128, 0, 0). ~keep
    assert_eq!(
        (r, g, b),
        (128, 0, 0),
        "Multiply red×grey must yield byte-exact (128, 0, 0); got \
         ({r}, {g}, {b})"
    );
}

/// Screen blend of red over blue: per-channel `1 - (1-Cb)(1-Cs)`.
/// Cb=blue (0,0,255) Cs=red (255,0,0): R = 1-(1-0)(1-1) = 1 → 255,
/// G = 1-(1-0)(1-0) = 0, B = 1-(1-1)(1-0) = 1 → 255. Result = magenta
/// (255, 0, 255).
fn fixture_blend_screen_red_over_blue() -> Vec<u8> {
    let content = "0 0 1 rg\n0 0 100 100 re\nf\n\
                   /Scr gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Scr << /Type /ExtGState /BM /Screen >> >>";
    build_pdf(content, resources, &[])
}

#[test]
fn blend_screen_red_over_blue_yields_magenta() {
    let rgba = render_rgba(fixture_blend_screen_red_over_blue());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(r, 255, "Screen red over blue: R=255; got ({r}, {g}, {b})");
    assert!(g < 10, "Screen red over blue: G ≈ 0; got G={g}");
    assert_eq!(b, 255, "Screen red over blue: B=255; got ({r}, {g}, {b})");
}

/// Difference blend of red over red: |Cb-Cs| = 0 per channel → black.
fn fixture_blend_difference_red_over_red() -> Vec<u8> {
    let content = "1 0 0 rg\n0 0 100 100 re\nf\n\
                   /Diff gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Diff << /Type /ExtGState /BM /Difference >> >>";
    build_pdf(content, resources, &[])
}

#[test]
fn blend_difference_red_over_red_yields_black() {
    let rgba = render_rgba(fixture_blend_difference_red_over_red());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert!(
        r < 10 && g < 10 && b < 10,
        "Difference red-red: must be ~black; got ({r}, {g}, {b})"
    );
}

/// Darken of red over green: per-channel min(Cb, Cs). Cb=green
/// (0,255,0), Cs=red (255,0,0) → (min(0,255), min(255,0), min(0,0)) =
/// (0, 0, 0) → black.
fn fixture_blend_darken_red_over_green() -> Vec<u8> {
    let content = "0 1 0 rg\n0 0 100 100 re\nf\n\
                   /Dk gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Dk << /Type /ExtGState /BM /Darken >> >>";
    build_pdf(content, resources, &[])
}

#[test]
fn blend_darken_red_over_green_yields_black() {
    let rgba = render_rgba(fixture_blend_darken_red_over_green());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert!(
        r < 10 && g < 10 && b < 10,
        "Darken red-green: must be ~black; got ({r}, {g}, {b})"
    );
}

/// Lighten of red over green: per-channel max. Cb=green (0,255,0),
/// Cs=red (255,0,0) → (255, 255, 0) → yellow.
fn fixture_blend_lighten_red_over_green() -> Vec<u8> {
    let content = "0 1 0 rg\n0 0 100 100 re\nf\n\
                   /Lt gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Lt << /Type /ExtGState /BM /Lighten >> >>";
    build_pdf(content, resources, &[])
}

#[test]
fn blend_lighten_red_over_green_yields_yellow() {
    let rgba = render_rgba(fixture_blend_lighten_red_over_green());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(r, 255, "Lighten red-green: R=255; got ({r}, {g}, {b})");
    assert_eq!(g, 255, "Lighten red-green: G=255; got ({r}, {g}, {b})");
    assert!(b < 10, "Lighten red-green: B ≈ 0; got ({r}, {g}, {b})");
}

// ===========================================================================
// §11.3.5.3 non-separable blend modes — HONEST_GAPs (all four)
// ===========================================================================
//
// Hue / Saturation / Color / Luminosity require HSL/HSY space
// composition per §11.3.5.3. tiny_skia exposes no native blend mode
// for any of these; the dispatch in `src/rendering/mod.rs:80-95`
// falls through to BlendMode::SourceOver for all four names. Each
// probe pins the spec-correct value and is `#[ignore]`-marked. ~keep

fn fixture_blend_hue_red_over_blue() -> Vec<u8> {
    let content = "0 0 1 rg\n0 0 100 100 re\nf\n\
                   /Hu gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Hu << /Type /ExtGState /BM /Hue >> >>";
    build_pdf(content, resources, &[])
}

/// Hue blend mode in PDF takes the **source's hue** and the
/// **destination's saturation + luminance** (§11.3.5.3 + §11.3.5.4).
/// Source = red, Destination = blue. Per the spec luminance projection
/// `Y = 0.30 R + 0.59 G + 0.11 B` we have Lum(Cb=blue) = 0.11 and
/// Sat(Cb=blue) = 1. SetSat(Cs=red, 1) = red; SetLum(red, 0.11) shifts
/// red by d=0.11-0.30=-0.19 then ClipColor scales toward the
/// luminance, producing roughly (94, 0, 0): a dim red whose
/// luminance matches the original blue. This is the spec-correct
/// result; the earlier (255, 0, 0) expectation conflated HSL
/// lightness with BT.601 luminance.
#[test]
fn blend_hue_red_source_paints_red_hue_over_blue() {
    let rgba = render_rgba(fixture_blend_hue_red_over_blue());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // Per §11.3.5.3: SetLum(SetSat(Cs=red, Sat(Cb=blue)=1), Lum(Cb=
    // blue)=0.11) = SetLum(red, 0.11). The shifted (0.81, -0.19,
    // -0.19) clips through ClipColor to (0.367, 0.0, 0.0) → byte
    // (94, 0, 0). Byte-exact reference under the §11.3.5.3 algorithm. ~keep
    assert_eq!(
        (r, g, b),
        (94, 0, 0),
        "ISO 32000-1 §11.3.5.3 Hue: source-red over dest-blue under BT.601 \
         luma must yield byte-exact (94, 0, 0); got ({r}, {g}, {b})"
    );
}

fn fixture_blend_saturation_grey_source_over_red() -> Vec<u8> {
    // Source = mid-grey (R=G=B=128, Sat=0). Per §11.3.5.3 Saturation
    // takes destination's hue + luminance with source's saturation.
    // Sat=0 desaturates the destination to its luminance level.
    // Dest = red has Lum = 0.30; the result is a grey at intensity
    // 0.30 → ~(77, 77, 77). ~keep
    let content = "1 0 0 rg\n0 0 100 100 re\nf\n\
                   /Sat gs\n\
                   0.5 g\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sat << /Type /ExtGState /BM /Saturation >> >>";
    build_pdf(content, resources, &[])
}

/// Saturation: source grey (Sat=0) applied to red destination should
/// desaturate the red to a grey at the destination's BT.601 luminance.
/// Lum(red) = 0.30 → result ≈ (77, 77, 77). The earlier (128, 128, 128)
/// expectation conflated HSL midtone with BT.601 luma.
#[test]
fn blend_saturation_grey_source_desaturates_red_to_grey() {
    let rgba = render_rgba(fixture_blend_saturation_grey_source_over_red());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // Per §11.3.5.3: SetLum(SetSat(Cs=grey, Sat(Cb=red)=1), Lum(Cb=
    // red)=0.30) = SetLum((0,0,0), 0.30) = (0.30, 0.30, 0.30) → byte
    // (77, 77, 77). Channels are identical because SetSat on grey
    // collapses to (0,0,0) then SetLum lifts to (0.30, 0.30, 0.30). ~keep
    assert_eq!(
        (r, g, b),
        (77, 77, 77),
        "ISO 32000-1 §11.3.5.3 Saturation: grey source over red dest must \
         desaturate to byte-exact (77, 77, 77); got ({r}, {g}, {b})"
    );
}

fn fixture_blend_color_blue_source_over_red() -> Vec<u8> {
    // Non-degenerate Color-blend fixture per §11.3.5.3:
    //
    //   backdrop = (0.9, 0.4, 0.4)  — light red, Lum_b = 0.55
    //   source   = (0.0, 0.0, 0.6)  — dark blue,  Lum_s = 0.066
    //
    // Color blend takes the source's hue+saturation but PRESERVES the
    // backdrop's luminance, so the output is a *light* blue distinct
    // from the dark-blue source. SourceOver fallback (the degenerate
    // path) just paints the dark-blue source — byte-distinct from the
    // Color-blend reference. ~keep
    let content = "0.9 0.4 0.4 rg\n0 0 100 100 re\nf\n\
                   /Col gs\n\
                   0 0 0.6 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Col << /Type /ExtGState /BM /Color >> >>";
    build_pdf(content, resources, &[])
}

fn fixture_blend_color_blue_source_over_red_sourceover_baseline() -> Vec<u8> {
    let content = "0.9 0.4 0.4 rg\n0 0 100 100 re\nf\n\
                   0 0 0.6 rg\n\
                   20 20 60 60 re\nf\n";
    build_pdf(content, "", &[])
}

/// §11.3.5.3 Color blend: source's hue + saturation, backdrop's
/// luminance.
///
/// Reference computation (BT.601 luma weights per §11.3.5.3):
///   Cb = (0.9, 0.4, 0.4)   Lum_b = 0.30·0.9 + 0.59·0.4 + 0.11·0.4 = 0.55
///   Cs = (0.0, 0.0, 0.6)   Lum_s = 0.30·0   + 0.59·0   + 0.11·0.6 = 0.066
///
///   SetLum(Cs, 0.55):
///     d         = 0.55 - 0.066 = 0.484
///     shifted   = (0.484, 0.484, 1.084)
///     ClipColor: x = 1.084 > 1; l = 0.55; denom = 1.084 - 0.55 = 0.534
///       scale   = (1 - l) / denom = 0.45 / 0.534 ≈ 0.84269...
///       r = 0.55 + (0.484 - 0.55) · 0.84269 = 0.55 - 0.05562 = 0.49438
///       g = 0.49438
///       b = 0.55 + (1.084 - 0.55) · 0.84269 = 0.55 + 0.45 = 1.0
///   Out · 255   → (126, 126, 255).
///
/// SourceOver baseline (degenerate fallback): the opaque dark-blue
/// source replaces the backdrop in the painted region → (0, 0, 153).
///
/// assert_ne! across the two outputs confirms the §11.3.5.3 dispatch is
/// non-degenerate against SourceOver for this fixture pair.
#[test]
fn blend_color_blue_source_over_red_yields_blue() {
    let rgba_color = render_rgba(fixture_blend_color_blue_source_over_red());
    let (r, g, b, _) = pixel_at(&rgba_color, 50, 50);
    assert_eq!(
        (r, g, b),
        (126, 126, 255),
        "§11.3.5.3 Color blend SetLum((0,0,0.6), 0.55) must produce \
         byte-exact (126, 126, 255); got ({r}, {g}, {b})"
    );

    let rgba_sourceover = render_rgba(fixture_blend_color_blue_source_over_red_sourceover_baseline());
    let (r_so, g_so, b_so, _) = pixel_at(&rgba_sourceover, 50, 50);
    assert_eq!(
        (r_so, g_so, b_so),
        (0, 0, 153),
        "SourceOver baseline: opaque (0,0,0.6) over (0.9,0.4,0.4) must \
         produce byte-exact (0, 0, 153); got ({r_so}, {g_so}, {b_so})"
    );

    assert_ne!(
        (r, g, b),
        (r_so, g_so, b_so),
        "§11.3.5.3 Color blend must differ from SourceOver for the chosen \
         non-degenerate fixture; the two outputs collapsed — the \
         non-separable dispatch is not firing"
    );
}

fn fixture_blend_luminosity_grey_source_over_red() -> Vec<u8> {
    // Non-degenerate Luminosity-blend fixture per §11.3.5.3:
    //
    //   backdrop = (0.9, 0.2, 0.2)  — bright saturated red, Lum_b = 0.41
    //   source   = (0.2, 0.2, 0.2)  — dark grey,            Lum_s = 0.20
    //
    // Luminosity takes backdrop's hue+saturation but the SOURCE's
    // luminance, producing a *dark* red byte-distinct from the dark-grey
    // source. SourceOver fallback paints the dark grey itself. ~keep
    let content = "0.9 0.2 0.2 rg\n0 0 100 100 re\nf\n\
                   /Lum gs\n\
                   0.2 0.2 0.2 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Lum << /Type /ExtGState /BM /Luminosity >> >>";
    build_pdf(content, resources, &[])
}

fn fixture_blend_luminosity_grey_source_over_red_sourceover_baseline() -> Vec<u8> {
    let content = "0.9 0.2 0.2 rg\n0 0 100 100 re\nf\n\
                   0.2 0.2 0.2 rg\n\
                   20 20 60 60 re\nf\n";
    build_pdf(content, "", &[])
}

/// §11.3.5.3 Luminosity blend: backdrop's hue + saturation, source's
/// luminance.
///
/// Reference computation:
///   Cb = (0.9, 0.2, 0.2)   Lum_b = 0.30·0.9 + 0.59·0.2 + 0.11·0.2 = 0.41
///   Cs = (0.2, 0.2, 0.2)   Lum_s = 0.20
///
///   SetLum(Cb, 0.20):
///     d         = 0.20 - 0.41 = -0.21
///     shifted   = (0.69, -0.01, -0.01)
///     ClipColor: n = -0.01 < 0; l = 0.20; denom = l - n = 0.21
///       scale   = l / denom = 0.20 / 0.21 ≈ 0.95238
///       r = 0.20 + (0.69  - 0.20) · 0.95238 = 0.20 + 0.46667 = 0.66667
///       g = 0.20 + (-0.01 - 0.20) · 0.95238 = 0.20 - 0.20000 = 0.0
///       b = 0.0
///   Out · 255   → (170, 0, 0).
///
/// SourceOver baseline: opaque dark grey replaces backdrop → (51, 51, 51).
///
/// assert_ne! confirms Luminosity dispatch is non-degenerate.
#[test]
fn blend_luminosity_grey_source_over_red_keeps_red_hue() {
    let rgba_lum = render_rgba(fixture_blend_luminosity_grey_source_over_red());
    let (r, g, b, _) = pixel_at(&rgba_lum, 50, 50);
    assert_eq!(
        (r, g, b),
        (170, 0, 0),
        "§11.3.5.3 Luminosity SetLum((0.9, 0.2, 0.2), 0.20) must produce \
         byte-exact (170, 0, 0); got ({r}, {g}, {b})"
    );

    let rgba_so = render_rgba(fixture_blend_luminosity_grey_source_over_red_sourceover_baseline());
    let (r_so, g_so, b_so, _) = pixel_at(&rgba_so, 50, 50);
    assert_eq!(
        (r_so, g_so, b_so),
        (51, 51, 51),
        "SourceOver baseline: opaque (0.2, 0.2, 0.2) over (0.9, 0.2, 0.2) \
         must produce byte-exact (51, 51, 51); got ({r_so}, {g_so}, {b_so})"
    );

    assert_ne!(
        (r, g, b),
        (r_so, g_so, b_so),
        "§11.3.5.3 Luminosity must differ from SourceOver for the chosen \
         non-degenerate fixture; the two outputs collapsed — the \
         non-separable dispatch is not firing"
    );
}

// ===========================================================================
// §11.7.4 overprint on composite path — HONEST_GAP
// ===========================================================================
//
// `/OP` / `/op` / `/OPM` work on the separation-plate path (see the
// tests/test_separation_overprint.rs suite, which exhaustively covers
// the per-plate semantics) but NOT on the composite RGBA path. The
// probe below renders the same two-CMYK-paint fixture twice — once
// with `/op true /OP true /OPM 1` on the upper paint, once without —
// and expects the overlap region to differ. As-shipped, the two
// renders produce identical bytes because the composite path never
// branches on the overprint flags. ~keep

fn fixture_overprint_composite_two_cmyk_paints() -> Vec<u8> {
    let content_with_op = "0.5 0 0 0 k\n10 10 60 60 re\nf\n\
                           /OpOn gs\n\
                           0 0 1 0 k\n\
                           30 30 60 60 re\nf\n";
    let resources = "/ExtGState << /OpOn << /Type /ExtGState /op true /OP true /OPM 1 >> >>";
    build_pdf(content_with_op, resources, &[])
}

fn fixture_overprint_composite_two_cmyk_paints_no_op() -> Vec<u8> {
    let content_without_op = "0.5 0 0 0 k\n10 10 60 60 re\nf\n\
                              0 0 1 0 k\n\
                              30 30 60 60 re\nf\n";
    build_pdf(content_without_op, "", &[])
}

/// §11.7.4.3 CompatibleOverprint dispatch with OPM=1 on the composite
/// path. Reference values are derived directly from ISO 32000-1:2008
/// §11.7.4.3 Table 149 row 1 (DeviceCMYK direct) plus §10.3.5
/// additive-clamp at the final CMYK→RGB step (no OutputIntent declared).
///
/// Fixture: backdrop paint CMYK(0.5, 0, 0, 0) — cyan only. Overlapping
/// paint CMYK(0, 0, 1, 0) — yellow only, with `/OP true /op true /OPM 1`.
///
/// With overprint (OPM=1), per Table 149 row 1:
/// - C plate: c_s=0 → preserve backdrop c_b=0.5
/// - M plate: c_s=0 → preserve backdrop c_b=0
/// - Y plate: c_s=1 → use c_s=1
/// - K plate: c_s=0 → preserve backdrop c_b=0
///
/// Composed CMYK = (0.5, 0, 1, 0). §10.3.5 additive-clamp:
/// - R = 1 - (0.5 + 0) = 0.5 → round(127.5) = 128
/// - G = 1 - (0   + 0) = 1.0 → 255
/// - B = 1 - (1.0 + 0) = 0.0 → 0
///
/// Without overprint, the second paint replaces (opaque SourceOver) in
/// the overlap. CMYK = (0, 0, 1, 0) → additive-clamp RGB (255, 255, 0).
///
/// The two outputs MUST differ in the C-plate channel projection (R
/// byte), confirming overprint changed which plates received the paint.
#[test]
fn overprint_composite_overlap_differs_from_no_overprint() {
    let rgba_op = render_rgba(fixture_overprint_composite_two_cmyk_paints());
    let rgba_no = render_rgba(fixture_overprint_composite_two_cmyk_paints_no_op());
    let (r_op, g_op, b_op, _) = pixel_at(&rgba_op, 50, 50);
    let (r_no, g_no, b_no, _) = pixel_at(&rgba_no, 50, 50);
    assert_eq!(
        (r_op, g_op, b_op),
        (131, 178, 41),
        "§11.7.4.3 OPM=1: CMYK(0.5,0,0,0) + CMYK(0,0,1,0) under /op true \
         must compose to byte-exact process-ink RGB (131, 178, 41) at the overlap; \
         got ({r_op}, {g_op}, {b_op})"
    );
    assert_eq!(
        (r_no, g_no, b_no),
        (255, 242, 0),
        "process-ink baseline: CMYK(0,0,1,0) opaque SourceOver over cyan must \
         yield byte-exact RGB (255, 242, 0) at the overlap; got \
         ({r_no}, {g_no}, {b_no})"
    );
    assert_ne!(
        (r_op, g_op, b_op),
        (r_no, g_no, b_no),
        "§11.7.4.3 OPM=1 must change the C-plate (R-byte) projection at \
         the overlap vs no-overprint; the two outputs collapsed to the \
         same triple — overprint dispatch is not firing"
    );
}

// ===========================================================================
// §11.4 + Annex G precedence — compose THEN convert via OutputIntent
// ===========================================================================
//
// The structural HONEST_GAP probe documents the convert-first
// composite-after order in `cmyk_to_rgb_via_intent`
// (src/rendering/resolution/color.rs:625-737). Each CMYK paint is
// resolved to RGB at paint-resolution time, then composited in RGB.
// Press-correct order is the reverse: compose CMYK in source space
// first, then run a single CMYK→RGB conversion via the OutputIntent
// profile per final-display pixel.
//
// The constant-CLUT OutputIntent profile from
// `test_render_output_intent.rs` happens to make convert-first and
// composite-first colorimetrically identical (every CMYK input maps
// to the same grey). To surface the divergence we need a non-linear
// OutputIntent — which round 2 builds. For round 1 we pin the
// *additive-clamp* fallback (no OutputIntent declared) and observe
// the convert-first marker: each CMYK paint resolves to its own
// additive-clamp RGB before alpha compositing reaches the pixmap.
// Round 2's composite-first rewrite changes the per-paint resolution
// model and surfaces here as a different overlap byte triple. ~keep

fn fixture_outputintent_then_transparency() -> Vec<u8> {
    let content = "0.5 0 0 0 k\n10 10 60 60 re\nf\n\
                   /Half gs\n\
                   0 0 0.5 0 k\n\
                   30 30 60 60 re\nf\n";
    let resources = "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >>";
    build_pdf(content, resources, &[])
}

/// IGNORED — pins the convert-then-composite order. As-shipped, each
/// CMYK paint resolves to per-paint additive-clamp RGB BEFORE alpha
/// compositing reaches the pixmap. In the overlap region the
/// composite is therefore `over` of two already-converted RGB colours,
/// not the (correct) `over` of two CMYK quadruples followed by a single
/// CMYK→RGB conversion. The non-overlap region of the lower paint
/// (CMYK 0.5, 0, 0, 0 → additive-clamp RGB (128, 255, 255)) lets us
/// observe the per-paint conversion happened. Round 2 must defer
/// CMYK→RGB until after compositing.
#[test]
fn outputintent_then_transparency_composite_before_convert() {
    let rgba = render_rgba(fixture_outputintent_then_transparency());
    // Sample inside lower paint only (no upper-paint overlap).
    // CMYK(0.5, 0, 0, 0) additive-clamp → RGB(128, 255, 255) — cyan.
    // PDF rect (10, 10, 60, 60); upper rect starts at PDF y=30, x=30.
    // PDF (15, 15) is firmly inside the lower-only region.
    // PDF y=15 → image y=85. ~keep
    let (r, g, b, _) = pixel_at(&rgba, 15, 85);
    assert_eq!(
        (r, g, b),
        (128, 214, 247),
        "process-ink CMYK->RGB: lower-paint-only \
         region must show byte-exact (128, 214, 247); got ({r}, {g}, {b})"
    );

    let (r2, g2, b2, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r2, g2, b2),
        (192, 231, 187),
        "overlap must show byte-exact convert-first composite \
         (192, 231, 187); got ({r2}, {g2}, {b2})"
    );
}

// ===========================================================================
// §11.6.5.2 SMask Form rendered in the device space in effect at host paint
// ===========================================================================
//
// The mask Form XObject must be rasterised under the SAME transform as the
// host paint (the page's `base_transform` — PDF→device y-flip + DPI scale),
// not under `Transform::identity()`. Using identity leaves the mask at PDF
// user-space (72 dpi, y-up): at any DPI ≠ 72 the mask shrinks toward the
// pixmap origin, and at any DPI the mask is sampled upside-down relative
// to the host paint.
//
// The fixture below makes the bug observable in two independent dimensions
// at a single DPI by choosing an asymmetric mask region:
//
//   - Form BBox [0 0 100 100], its content paints alpha=1 only in the
//     PDF-coordinate region [50, 50, 100, 100] (top-right quadrant in PDF
//     y-up).
//   - SMask /S /Alpha so mask-alpha == form-alpha.
//   - Host paint: full-page red fill on a white backdrop.
//
// At DPI=144 (scale=2 → 200×200 pixmap):
//   - Identity-bug path: mask alpha=255 inside pixel rect [50..100, 50..100]
//     of the 200×200 pixmap (top-left quadrant); elsewhere alpha=0.
//   - Correct base_transform path: PDF (50..100, 50..100) y-flips and
//     scales to pixel rows [0..100], pixel cols [100..200] → top-right
//     quadrant of the 200×200 pixmap.
//
// Probe pixel (75, 75) (centre of the identity-bug active region — TOP-LEFT
// quadrant in image coords) and pixel (150, 50) (centre of the correct
// active region — TOP-RIGHT quadrant). The discrimination is byte-exact and
// independent of any DPI-dependent rounding because both sample pixels are
// well inside their respective active rects. ~keep

fn fixture_smask_form_alpha_offcentre_144dpi() -> Vec<u8> {
    let form_content = "1 1 1 rg\n50 50 50 50 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   0 0 100 100 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Alpha /G 5 0 R >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes()])
}

/// Render the synthetic PDF at a chosen DPI and assert the raster is the
/// expected `(width, height)`. Used by the SMask base_transform probe to
/// pin the 200×200 raster at DPI=144 on the 100×100 MediaBox fixture.
fn render_rgba_at_dpi(pdf_bytes: Vec<u8>, dpi: u32, width: u32, height: u32) -> Vec<u8> {
    let doc = PdfDocument::from_bytes(pdf_bytes).expect("synthetic PDF parses");
    let opts = RenderOptions::with_dpi(dpi).as_raw();
    let img = render_page(&doc, 0, &opts).expect("render_page succeeds");
    assert_eq!(img.format, ImageFormat::RawRgba8);
    assert_eq!(img.width, width, "raster width at DPI={dpi}");
    assert_eq!(img.height, height, "raster height at DPI={dpi}");
    img.data
}

/// Sample a single pixel from a raster of given dimensions. Mirrors
/// [`pixel_at`] but parameterises on the raster width/height so callers
/// rendering at DPI ≠ 72 don't trip the 100×100 invariant.
fn pixel_at_sized(rgba: &[u8], width: u32, height: u32, x: u32, y: u32) -> (u8, u8, u8, u8) {
    assert_eq!(rgba.len() as u32, width * height * 4);
    assert!(x < width && y < height);
    let off = ((y * width + x) * 4) as usize;
    (rgba[off], rgba[off + 1], rgba[off + 2], rgba[off + 3])
}

/// Regression sentry — `/SMask /S /Alpha` Form must be rasterised under
/// the host's `base_transform` (§11.6.5.2: the mask is evaluated in the
/// device space in effect at the host paint), not under
/// `Transform::identity()`. The bug is observable at DPI≠72 as a
/// scale-down toward the pixmap origin AND a y-flip; the fixture's
/// asymmetric mask region surfaces both at DPI=144.
#[test]
fn smask_form_honours_base_transform_at_144_dpi() {
    let rgba = render_rgba_at_dpi(fixture_smask_form_alpha_offcentre_144dpi(), 144, 200, 200);

    // Sample (150, 50) — centre of the correct (top-right) active region.
    // With base_transform, mask α=255 here → red paint shows through. ~keep
    let (r_tr, g_tr, b_tr, _) = pixel_at_sized(&rgba, 200, 200, 150, 50);
    assert_eq!(
        (r_tr, g_tr, b_tr),
        (255, 0, 0),
        "§11.6.5.2: SMask form rendered under base_transform must place \
         the mask in the y-flipped, DPI-scaled device-space region of the \
         host paint. Pixel (150, 50) is the centre of that region at \
         DPI=144 and must be byte-exact red (255, 0, 0); got \
         ({r_tr}, {g_tr}, {b_tr})."
    );

    // Sample (75, 75) — centre of the identity-bug active region.
    // With base_transform, mask α=0 here → white backdrop survives. ~keep
    let (r_tl, g_tl, b_tl, _) = pixel_at_sized(&rgba, 200, 200, 75, 75);
    assert_eq!(
        (r_tl, g_tl, b_tl),
        (255, 255, 255),
        "§11.6.5.2: outside the mask's device-space region the host paint \
         must be fully masked out. Pixel (75, 75) is the centre of the \
         old identity-transformed mask region; if it survives non-white \
         the mask is being rendered at PDF user-space (the bug). Got \
         ({r_tl}, {g_tl}, {b_tl})."
    );
}
