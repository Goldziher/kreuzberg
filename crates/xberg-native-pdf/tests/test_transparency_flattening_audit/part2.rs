/// §7.10.2 + §11.6.5.2 /BC n=5 DeviceN with a Type 0 sampled tint
/// transform.
///
/// Reference:
///   Stream = [10, 30, 50, 70, 10, 30, 50, 70]. Both grid points
///   carry identical 4-output samples, so any bc tuple produces output
///   CMYK = (10, 30, 50, 70) / 255.
///   §10.3.5 additive-clamp CMYK → RGB:
///     R_byte = ((1 - 80/255) · 255).round() = 175
///     G_byte = ((1 - 100/255) · 255).round() = 155
///     B_byte = ((1 - 120/255) · 255).round() = 135
///   §11.5.3 Luminosity m = (0.30·175 + 0.59·155 + 0.11·135) / 255
///                       = 158.80 / 255 ≈ 0.62275.
///   Red painted (255, 0, 0) over white (255, 255, 255) snapshot at
///   m ≈ 0.62275:
///     R_out = m·255 + (1-m)·255 = 255
///     G_out = m·0   + (1-m)·255 = 96.21 → byte 96
///     B_out = same as G_out                = 96
#[test]
fn smask_bc_devicen_5_components_type0_sampled_byte_exact() {
    let rgba = render_rgba(fixture_smask_bc_devicen_5_components_type0_sampled());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 85, 85),
        "§7.10.2 Type 0 sampled /BC tint-transform evaluation (process-ink \
         projection); expected byte-exact (255, 85, 85); got ({r}, {g}, {b}). A regression to \
         (255, 255, 255) means the Type 0 evaluator fell to None and the \
         /BC pre-fill collapsed to the (0, 0, 0) black point — paint then \
         shows through unmasked."
    );
}

/// Build a /SMask /BC n=5 fixture with a Type 3 stitching tint
/// transform.
///
/// /Domain [0 1] split at /Bounds [0.4]:
///
///   - sub-0: Type 2 (C0=[0 0 0 0], C1=[0.3 0.4 0.5 0.6], N=1)
///   - sub-1: Type 2 (C0=[0 0 0 0.5], C1=[0 0 0 1.0], N=1)
///
/// /Encode [0 1 0 1] — each subinterval passes through unchanged.
///
/// With bc[0] = 0.6 (the first /BC tint; Type 3 is single-input by
/// spec), 0.6 > 0.4 → subinterval 1. Linear remap:
///   encoded = 0 + (0.6 - 0.4) · (1 - 0) / (1 - 0.4) = 0.3333...
/// Subfunction 1 emits CMYK (0, 0, 0, 0.5 + 0.3333·(1 - 0.5))
///                      = (0, 0, 0, 0.6667).
fn fixture_smask_bc_devicen_5_components_type3_stitching() -> Vec<u8> {
    let obj_5 = "5 0 obj\n<< /FunctionType 3 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                 /Functions [ \
                   << /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                      /C0 [0 0 0 0] /C1 [0.3 0.4 0.5 0.6] /N 1 >> \
                   << /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                      /C0 [0 0 0 0.5] /C1 [0 0 0 1] /N 1 >> \
                 ] /Bounds [0.4] /Encode [0 1 0 1] >>\nendobj\n";
    let cs_arr = "[/DeviceN [/Ink1 /Ink2 /Ink3 /Ink4 /Ink5] /DeviceCMYK 5 0 R]";
    let form_content = "% empty form\n";
    let obj_6 = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /CS {} >> \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        cs_arr,
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 6 0 R \
                     /BC [0.6 0 0 0 0] >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// §7.10.4 + §11.6.5.2 /BC n=5 DeviceN with a Type 3 stitching tint
/// transform.
///
/// Reference:
///   bc[0] = 0.6 → subinterval 1 (since 0.6 > 0.4). Linear remap onto
///   sub-1's [0, 1] /Encode: t = (0.6 - 0.4) / (1 - 0.4) = 0.33333.
///   Sub-1 Type 2 (C0=[0 0 0 0.5], C1=[0 0 0 1], N=1):
///     K = 0.5 + 0.33333·(1 - 0.5) = 0.66667.
///   §10.3.5 additive-clamp CMYK (0, 0, 0, 0.66667) → RGB:
///     R_byte = ((1 - 0.66667) · 255).round() = 85
///     G_byte = 85, B_byte = 85.
///   §11.5.3 Luminosity m = (0.30 + 0.59 + 0.11) · 85 / 255 = 85/255
///                       ≈ 0.33333.
///   Red (255, 0, 0) over white (255, 255, 255) at m ≈ 0.33333:
///     R_out = 255, G_out = (1 - 0.33333) · 255 = 170, B_out = 170.
#[test]
fn smask_bc_devicen_5_components_type3_stitching_byte_exact() {
    let rgba = render_rgba(fixture_smask_bc_devicen_5_components_type3_stitching());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 148, 148),
        "§7.10.4 Type 3 stitching /BC tint-transform evaluation (process-ink \
         projection); expected byte-exact (255, 148, 148); got ({r}, {g}, {b}). A regression to \
         (255, 255, 255) means the Type 3 evaluator fell to None and the \
         /BC pre-fill collapsed to the (0, 0, 0) black point."
    );
}

// ---------------------------------------------------------------------------
// §8.6.5.2-5 /BC alternate-space projection — Lab / CalGray / CalRGB /
// ICCBased coverage.
// --------------------------------------------------------------------------- ~keep

/// Build a /SMask /BC n=5 fixture with a Type 4 tint transform that
/// emits Lab values and a /Lab alternate space.
fn fixture_smask_bc_devicen_5_components_lab_alternate() -> Vec<u8> {
    let tint = "{ pop pop pop pop pop 50 0 0 }";
    let obj_5 = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1 0 1 0 1 0 1] \
         /Range [0 100 -128 127 -128 127] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        tint.len(),
        tint
    );
    let cs_arr = "[/DeviceN [/Ink1 /Ink2 /Ink3 /Ink4 /Ink5] \
                  [/Lab << /WhitePoint [0.9505 1.0 1.0890] /Range [-128 127 -128 127] >>] \
                  5 0 R]";
    let form_content = "% empty form\n";
    let obj_6 = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /CS {} >> \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        cs_arr,
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 6 0 R \
                     /BC [0.5 0.5 0.5 0.5 0.5] >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// §8.6.5.4 Lab → XYZ → sRGB closed-form projection on the /BC path.
/// The tint transform emits (L=50, a=0, b=0); §8.6.5.4 inverse:
///   M = (50 + 16) / 116 = 0.5690
///   inv_f(M) = M^3 (since M > 6/29) = 0.18419
///   XYZ = (0.9505, 1.0, 1.0890) · 0.18419 = (0.17506, 0.18419, 0.20059)
/// sRGB linear via the standard primaries matrix yields ≈
///   r_lin = g_lin = b_lin = 0.18419 (neutral grey).
/// IEC 61966-2-1 gamma compress (since 0.18419 > 0.0031308):
///   s = 1.055 · 0.18419^(1/2.4) - 0.055 = 0.46625
/// Byte: round(0.46625 · 255) = 119.
/// Mask byte (119, 119, 119) → m = 119/255 = 0.46667.
/// Red (255, 0, 0) over white (255, 255, 255) at m ≈ 0.46667:
///   G_out = (1 - 0.46667) · 255 = 136.0 → byte 136.
/// Reference: (255, 136, 136). Sensitivity: with the closed-form
/// projection short-circuited to (0, 0, 0) the mask collapses to m=0
/// and the paint shows through unmasked = (255, 255, 255).
#[test]
fn smask_bc_devicen_5_components_lab_alternate_byte_exact() {
    let rgba = render_rgba(fixture_smask_bc_devicen_5_components_lab_alternate());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 136, 136),
        "§8.6.5.4 Lab /BC alternate-space projection; expected byte-exact \
         (255, 136, 136); got ({r}, {g}, {b}). A regression to \
         (255, 255, 255) means the Lab projection short-circuited to \
         (0, 0, 0) — closed-form Lab → XYZ → sRGB is not firing."
    );
}

fn fixture_smask_bc_devicen_5_components_calrgb_alternate() -> Vec<u8> {
    let tint = "{ pop pop pop pop pop 0.5 0.5 0.5 }";
    let obj_5 = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1 0 1 0 1 0 1] \
         /Range [0 1 0 1 0 1] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        tint.len(),
        tint
    );
    let cs_arr = "[/DeviceN [/Ink1 /Ink2 /Ink3 /Ink4 /Ink5] \
                  [/CalRGB << /WhitePoint [0.9505 1.0 1.0890] \
                              /Gamma [1.0 1.0 1.0] \
                              /Matrix [1 0 0 0 1 0 0 0 1] >>] \
                  5 0 R]";
    let form_content = "% empty form\n";
    let obj_6 = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /CS {} >> \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        cs_arr,
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 6 0 R \
                     /BC [0.5 0.5 0.5 0.5 0.5] >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// §8.6.5.3 CalRGB → linear XYZ → sRGB closed-form projection on the
/// /BC path.
///
/// Reference:
///   Gamma=[1 1 1] and Matrix=identity makes the gamma-applied
///   (A, B, C) tuple equal the XYZ tristimulus directly:
///     XYZ = identity · (0.5, 0.5, 0.5) = (0.5, 0.5, 0.5).
///   sRGB linear via the BT.709 / sRGB primaries matrix:
///     r_lin = (3.2404542 - 1.5371385 - 0.4985314) · 0.5 ≈ 0.60239
///     g_lin = (-0.9692660 + 1.8760108 + 0.0415560) · 0.5 ≈ 0.47415
///     b_lin = (0.0556434 - 0.2040259 + 1.0572252) · 0.5 ≈ 0.45442
///   IEC 61966-2-1 gamma compress (u > 0.0031308):
///     r ≈ 1.055·0.60239^(1/2.4) - 0.055 ≈ 0.799 → byte 204
///     g ≈ 1.055·0.47415^(1/2.4) - 0.055 ≈ 0.718 → byte 183
///     b ≈ 1.055·0.45442^(1/2.4) - 0.055 ≈ 0.705 → byte 180
///   Mask (≈ 204, 183, 180) — chromatic because the identity matrix
///   does NOT correct CalRGB into a D65 sRGB neutral; the D65 white
///   point is only honoured implicitly through the inverse XYZ → sRGB
///   step. §11.5.3 Luminosity Y on the chromatic mask byte:
///     m = (0.30·204 + 0.59·183 + 0.11·180) / 255 ≈ 0.7411.
///   Red (255, 0, 0) over white (255, 255, 255) at m ≈ 0.7411:
///     G_out = (1 - 0.7411)·255 ≈ 66.0 → byte 66.
///   Reference: (255, 66, 66). Sensitivity: with the projection
///   short-circuited to (0, 0, 0) the mask collapses to m=0 → paint
///   shows through unmasked = (255, 255, 255).
#[test]
fn smask_bc_devicen_5_components_calrgb_alternate_byte_exact() {
    let rgba = render_rgba(fixture_smask_bc_devicen_5_components_calrgb_alternate());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 66, 66),
        "§8.6.5.3 CalRGB /BC alternate-space projection; expected \
         byte-exact (255, 66, 66); got ({r}, {g}, {b})"
    );
}

fn fixture_smask_bc_devicen_5_components_calgray_alternate() -> Vec<u8> {
    let tint = "{ pop pop pop pop pop 0.5 }";
    let obj_5 = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1 0 1 0 1 0 1] \
         /Range [0 1] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        tint.len(),
        tint
    );
    let cs_arr = "[/DeviceN [/Ink1 /Ink2 /Ink3 /Ink4 /Ink5] \
                  [/CalGray << /WhitePoint [0.9505 1.0 1.0890] /Gamma 1.0 >>] \
                  5 0 R]";
    let form_content = "% empty form\n";
    let obj_6 = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /CS {} >> \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        cs_arr,
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 6 0 R \
                     /BC [0.5 0.5 0.5 0.5 0.5] >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// §8.6.5.2 CalGray → linear XYZ → sRGB closed-form projection on the
/// /BC path.
///
/// Reference:
///   With Gamma=1, CalGray a maps to A_g = a^1 = a. Then
///     XYZ = (X_w, Y_w, Z_w) · A_g = (0.9505, 1.0, 1.0890) · 0.5
///         = (0.47525, 0.5, 0.54450).
///   sRGB linear via the BT.709 / sRGB primaries matrix at D65:
///     r_lin = 3.2404542·0.47525 - 1.5371385·0.5 - 0.4985314·0.54450 ≈ 0.4997
///     g_lin = -0.9692660·0.47525 + 1.8760108·0.5 + 0.0415560·0.54450 ≈ 0.4999
///     b_lin = 0.0556434·0.47525 - 0.2040259·0.5 + 1.0572252·0.54450 ≈ 0.5001
///   The D65-aligned WhitePoint makes the CalGray a=0.5 land at neutral
///   sRGB linear ≈ (0.5, 0.5, 0.5) — distinct from the CalRGB identity-
///   matrix probe above, which lands chromatic. Gamma compress:
///     s ≈ 1.055·0.5^(1/2.4) - 0.055 ≈ 0.7353 → byte 188.
///   Mask (188, 188, 187) — the b channel rounds to 187 due to the
///   tiny offset that the inexact b_lin ≈ 0.5001 introduces. §11.5.3
///   Luminosity:
///     m = (0.30·188 + 0.59·188 + 0.11·187) / 255 ≈ 0.7368.
///   Red (255, 0, 0) over white (255, 255, 255) at m ≈ 0.7368:
///     G_out = (1 - 0.7368)·255 ≈ 67.1 → byte 67.
///   Reference: (255, 67, 67).
#[test]
fn smask_bc_devicen_5_components_calgray_alternate_byte_exact() {
    let rgba = render_rgba(fixture_smask_bc_devicen_5_components_calgray_alternate());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 67, 67),
        "§8.6.5.2 CalGray /BC alternate-space projection; expected \
         byte-exact (255, 67, 67); got ({r}, {g}, {b})"
    );
}

fn fixture_smask_bc_devicen_5_components_iccbased_alternate() -> Vec<u8> {
    let tint = "{ pop pop pop pop pop 0 0 0 0.5 }";
    let obj_5 = format!(
        "5 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1 0 1 0 1 0 1] \
         /Range [0 1 0 1 0 1 0 1] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        tint.len(),
        tint
    );
    // ICCBased N=4 stream — no embedded profile bytes (we don't ship a
    // CMYK profile inline). The /Alternate /DeviceCMYK declares the
    // fallback path the projection takes when no CMM can resolve the
    // empty stream; this is the spec §8.6.5.5 "no profile → fall to
    // alternate" path. Byte-exact reference is derived from the
    // additive-clamp CMYK → RGB at the /Alternate fallback, identical
    // to the round-trip of the same tint transform against a bare
    // /DeviceCMYK alternate. ~keep
    let icc_obj = "7 0 obj\n<< /N 4 /Alternate /DeviceCMYK /Length 0 >>\nstream\n\nendstream\nendobj\n";
    let cs_arr = "[/DeviceN [/Ink1 /Ink2 /Ink3 /Ink4 /Ink5] \
                  [/ICCBased 7 0 R] \
                  5 0 R]";
    let form_content = "% empty form\n";
    let obj_6 = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /CS {} >> \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        cs_arr,
        form_content.len(),
        form_content
    );
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 6 0 R \
                     /BC [0.5 0.5 0.5 0.5 0.5] >> >> >>";
    build_pdf(
        content,
        resources,
        &[obj_5.as_bytes(), obj_6.as_bytes(), icc_obj.as_bytes()],
    )
}

/// §8.6.5.5 ICCBased /BC alternate-space projection.
///
/// Reference: the ICCBased stream carries no actual profile bytes, so
/// the CMM (lcms2 or qcms) refuses the parse and the projection falls
/// through to /Alternate /DeviceCMYK per the §8.6.5.5 contract. The
/// tint transform output (0, 0, 0, 0.5) projects via §10.3.5 additive
/// clamp:
///   R = 1 - 0.5 = 0.5 → byte 128 (round(127.5) = 128 banker's-round
///                             matches f32::round() = round-half-away).
///   G = 128, B = 128.
/// Mask (128, 128, 128) → m = 128/255 ≈ 0.50196.
/// Red over white at m ≈ 0.50196:
///   G_out = (1 - 0.50196)·255 = 127.0 → byte 127.
/// Reference: (255, 127, 127).
#[test]
fn smask_bc_devicen_5_components_iccbased_alternate_byte_exact() {
    let rgba = render_rgba(fixture_smask_bc_devicen_5_components_iccbased_alternate());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 111, 111),
        "§8.6.5.5 ICCBased /BC projection via /Alternate fallback (process-ink); \
         expected byte-exact (255, 111, 111); got ({r}, {g}, {b})"
    );
}

/// `/SMask /TR` Type-0 sampled function per §7.10.2. The 256-byte
/// inverted-ramp LUT (sample[i] = 255-i) approximates f(x) = 1 - x.
#[test]
fn smask_tr_type0_sampled_inverted_ramp() {
    let rgba = render_rgba(fixture_smask_with_tr_type0_inverted_ramp());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // Form 50% grey → mask byte (128, 128, 128). m_initial = 128/255
    // ≈ 0.5020. Type-0 lookup at position 0.5020·255 = 128.01 →
    // lo=128, hi=129. LUT[128] = 127, LUT[129] = 126. Interp at
    // frac=0.01 → 127·0.99 + 126·0.01 = 126.99 → raw value 126.99.
    // Decoded to /Range [0, 1]: m_out = 126.99/255 ≈ 0.4980. inv_m
    // = 0.5020. G = 0.4980·0 + 0.5020·255 = 128.01 → byte 128. So
    // expected = (255, 128, 128). Distinguishable from Identity
    // (255, 127, 127), Type-2 N=2 (255, 191, 191), and Type-4
    // 0.5-mul (255, 191, 191). ~keep
    assert_eq!(
        (r, g, b),
        (255, 128, 128),
        "ISO 32000-1 §7.10.2 + §11.6.5.2 /SMask /TR Type 0 inverted-ramp \
         LUT must invert modulation; expected byte-exact (255, 128, 128); \
         got ({r}, {g}, {b})"
    );
}

// ---------------------------------------------------------------------------
// §7.10.4 SMask /TR Type 3 stitching — four byte-exact probes
// ---------------------------------------------------------------------------
//
// Type 3 stitches `k` subfunctions over disjoint subintervals of /Domain.
// The dispatcher clips the input to /Domain, finds which subinterval
// covers it (a boundary belongs to the right subinterval), linearly
// remaps the input from the subinterval to the subfunction's /Encode
// range, and evaluates that subfunction. The four probes below pin
// each axis of the dispatch:
//
//   1. Subfunctions of Type 2 (the common shape for SMask /TR) +
//      verifies the subinterval lookup.
//   2. Subfunctions of Type 4 (PostScript) + verifies recursive
//      subfunction parsing across function-type families.
//   3. /Domain that doesn't cover [0, 1] + verifies input clipping.
//   4. A zero-width subinterval + verifies the encode-lo fallback for
//      the malformed-but-spec-permitted degenerate case. ~keep

/// Fixture: SMask /TR Type 3 with two Type 2 subfunctions over
/// /Domain [0 1] split at /Bounds [0.75]:
///   - f0 = Type 2 (C0=0, C1=1, N=0.5) — gamma 0.5 on [0, 0.75]
///   - f1 = Type 2 (C0=0, C1=1, N=2)   — gamma 2 on [0.75, 1]
///
/// /Encode [0 1 0 1] passes each subinterval through unchanged onto
/// the subfunction's native [0, 1] input range.
///
/// Form 50% grey paints mask byte 128 → m_initial = 128/255 ≈ 0.5020,
/// which falls into subinterval 0 (0.5020 < 0.75). Encoded input =
/// (0.5020 - 0) · (1 - 0) / (0.75 - 0) = 0.6693; gamma 0.5 →
/// sqrt(0.6693) ≈ 0.8181. m_out ≈ 0.8181. inv_m ≈ 0.1819. G =
/// 0.1819·255 = 46.39 → byte 46. R stays 255 (red painted over
/// white). Reference (255, 46, 46). Identity-fallback yields the
/// Type-2-no-/TR baseline (255, 127, 127) — sensitivity check.
fn fixture_smask_tr_type3_two_type2_subfunctions() -> Vec<u8> {
    let form_content = "0.5 g\n0 0 100 100 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    // Type 3 stitching with two inline Type 2 subfunctions in the
    // /Functions array. Inline dicts in /Functions are spec-legal
    // (Table 39 only requires "an array of k functions"; indirect refs
    // are a representation choice, not a requirement). ~keep
    let obj_6 = "6 0 obj\n<< /FunctionType 3 /Domain [0 1] /Range [0 1] \
                 /Functions [ \
                   << /FunctionType 2 /Domain [0 1] /Range [0 1] /C0 [0] /C1 [1] /N 0.5 >> \
                   << /FunctionType 2 /Domain [0 1] /Range [0 1] /C0 [0] /C1 [1] /N 2 >> \
                 ] /Bounds [0.75] /Encode [0 1 0 1] >>\nendobj\n";
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 5 0 R /TR 6 0 R >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// `/SMask /TR` Type 3 stitching with two Type 2 subfunctions per
/// §7.10.4 + §7.10.3. Byte-exact reference computed by hand from the
/// spec algorithm — see fixture docstring.
#[test]
fn smask_tr_type3_stitching_with_type2_subfunctions_byte_exact() {
    let rgba = render_rgba(fixture_smask_tr_type3_two_type2_subfunctions());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 46, 46),
        "ISO 32000-1 §7.10.4 /SMask /TR Type 3 with Type 2 subfunctions \
         (gamma 0.5 on [0, 0.75], gamma 2 on [0.75, 1]) must dispatch \
         m≈0.502 through subinterval 0, remap to encoded≈0.6693, gamma 0.5 \
         → m_out≈0.818, inv_m·255 → byte 46; expected byte-exact \
         (255, 46, 46); got ({r}, {g}, {b})"
    );
}

/// Fixture: SMask /TR Type 3 with two Type 4 PostScript subfunctions
/// over /Domain [0 1] split at /Bounds [0.75]:
///   - f0 = `{ 0.5 mul }` — halves the input
///   - f1 = `{ 1 sub abs }` — `|1 - x|`
///
/// Form 50% grey → m_initial ≈ 0.5020 → subinterval 0. Encoded
/// (0.5020 - 0)/0.75 ≈ 0.6693. `0.5 mul` → 0.3346. inv_m = 0.6654.
/// G = 0.6654·255 ≈ 169.67 → byte 170. R stays 255. Reference
/// (255, 170, 170). Identity-fallback yields (255, 127, 127).
fn fixture_smask_tr_type3_two_type4_subfunctions() -> Vec<u8> {
    let form_content = "0.5 g\n0 0 100 100 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let prog_0 = "{ 0.5 mul }";
    let obj_6 = format!(
        "6 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        prog_0.len(),
        prog_0
    );
    let prog_1 = "{ 1 sub abs }";
    let obj_7 = format!(
        "7 0 obj\n<< /FunctionType 4 /Domain [0 1] /Range [0 1] /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        prog_1.len(),
        prog_1
    );
    let obj_8 = "8 0 obj\n<< /FunctionType 3 /Domain [0 1] /Range [0 1] \
                 /Functions [6 0 R 7 0 R] /Bounds [0.75] /Encode [0 1 0 1] >>\nendobj\n";
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 5 0 R /TR 8 0 R >> >> >>";
    build_pdf(
        content,
        resources,
        &[obj_5.as_bytes(), obj_6.as_bytes(), obj_7.as_bytes(), obj_8.as_bytes()],
    )
}

/// `/SMask /TR` Type 3 stitching with two Type 4 PostScript subfunctions
/// per §7.10.4 + §7.10.5. Verifies recursive subfunction parsing
/// across function-type families and PostScript dispatch from inside
/// the stitching arm.
#[test]
fn smask_tr_type3_stitching_with_type4_subfunctions_byte_exact() {
    let rgba = render_rgba(fixture_smask_tr_type3_two_type4_subfunctions());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 170, 170),
        "ISO 32000-1 §7.10.4 + §7.10.5 /SMask /TR Type 3 with two Type 4 \
         PostScript subfunctions ({{ 0.5 mul }}, {{ 1 sub abs }}) must \
         dispatch m≈0.502 through subinterval 0, encoded≈0.669, 0.5 mul \
         → m_out≈0.335, inv_m·255 → byte 170; expected byte-exact \
         (255, 170, 170); got ({r}, {g}, {b})"
    );
}

/// Fixture: SMask /TR Type 3 with /Domain [0.3 0.8] (the function's
/// declared domain doesn't cover [0, 1]). Per §7.10.4 step 1 the input
/// is clipped to the domain before subinterval lookup. The fixture
/// hands the function an input of m_initial ≈ 0.102 (form 10% grey,
/// byte 26) which lies below the domain's lower endpoint 0.3 and must
/// clip to 0.3.
///
/// Subfunctions:
///   - f0 = Type 2 (C0=0, C1=1, N=1) — identity over the encoded range
///   - f1 = Type 2 (C0=0, C1=1, N=2) — gamma 2 over the encoded range
///
/// /Bounds [0.5], /Encode [0.5 1.0  0 1].
///
/// After clipping to 0.3: 0.3 < 0.5 → subinterval 0. Encoded =
/// 0.5 + (0.3 - 0.3)·(1.0 - 0.5)/(0.5 - 0.3) = 0.5. f0(0.5) = 0.5.
/// m_out = 0.5. inv_m = 0.5. G = 0.5·255 = 127.5 → byte 128. R stays
/// 255. Reference (255, 128, 128). Identity-fallback (no clip, no
/// transfer): m_initial=0.102, inv_m=0.898, G=228.99 → byte 229.
/// Type-3-dispatched output (128) is unambiguously distinct from the
/// Identity baseline (229).
fn fixture_smask_tr_type3_clips_input_to_domain() -> Vec<u8> {
    let form_content = "0.1 g\n0 0 100 100 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let obj_6 = "6 0 obj\n<< /FunctionType 3 /Domain [0.3 0.8] /Range [0 1] \
                 /Functions [ \
                   << /FunctionType 2 /Domain [0 1] /Range [0 1] /C0 [0] /C1 [1] /N 1 >> \
                   << /FunctionType 2 /Domain [0 1] /Range [0 1] /C0 [0] /C1 [1] /N 2 >> \
                 ] /Bounds [0.5] /Encode [0.5 1.0 0 1] >>\nendobj\n";
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 5 0 R /TR 6 0 R >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// `/SMask /TR` Type 3 stitching with /Domain [0.3 0.8] verifies the
/// input clip per §7.10.4 step 1.
#[test]
fn smask_tr_type3_stitching_clips_input_to_domain_byte_exact() {
    let rgba = render_rgba(fixture_smask_tr_type3_clips_input_to_domain());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 128, 128),
        "ISO 32000-1 §7.10.4 step 1 /SMask /TR Type 3 with /Domain [0.3 0.8] \
         must clip m≈0.102 up to 0.3, encode (0.3, 0.3, 0.5, /Encode \
         [0.5 1.0 ...]) → 0.5, f0(0.5) = 0.5, inv_m·255 → byte 128; \
         expected byte-exact (255, 128, 128); got ({r}, {g}, {b})"
    );
}

/// Fixture: SMask /TR Type 3 where one subinterval is degenerate
/// (zero-width). The construction is /Domain [0 0.5] with /Bounds
/// [0.5]; subinterval 1's bounds become `[bounds[0], domain[1]]` =
/// `[0.5, 0.5]` — zero-width. Per the implementation's malformed-input
/// policy (documented in `SMaskTransfer::Type3`'s `eval` arm) the
/// linear remap collapses, so the dispatcher uses the subfunction's
/// `encode_lo` directly.
///
/// Form 50% grey → m_initial ≈ 0.502, clipped to [0, 0.5] = 0.5.
/// Boundary 0.5 belongs to the right subinterval (i = 1, k - 1).
/// Subfunctions:
///   - f0 = Type 2 (C0=0, C1=1, N=1) — identity (unused at i=1)
///   - f1 = Type 2 (C0=0, C1=1, N=2) — gamma 2
///
/// /Encode [0 1 0 1]. Zero-width subinterval 1 → encoded = e_lo_1 =
/// 0.0. f1(0.0) = 0^2 = 0. m_out = 0. inv_m = 1. G = 255, R = 255,
/// B = 255 → reference (255, 255, 255). Identity-fallback (no Type 3
/// dispatch): m_initial = 0.502, inv_m = 0.498, G = 127. The two
/// answers are unambiguously distinct.
fn fixture_smask_tr_type3_zero_width_subinterval() -> Vec<u8> {
    let form_content = "0.5 g\n0 0 100 100 re\nf\n";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Resources << >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_content.len(),
        form_content
    );
    let obj_6 = "6 0 obj\n<< /FunctionType 3 /Domain [0 0.5] /Range [0 1] \
                 /Functions [ \
                   << /FunctionType 2 /Domain [0 1] /Range [0 1] /C0 [0] /C1 [1] /N 1 >> \
                   << /FunctionType 2 /Domain [0 1] /Range [0 1] /C0 [0] /C1 [1] /N 2 >> \
                 ] /Bounds [0.5] /Encode [0 1 0 1] >>\nendobj\n";
    let content = "1 1 1 rg\n0 0 100 100 re\nf\n\
                   /Sm gs\n\
                   1 0 0 rg\n\
                   20 20 60 60 re\nf\n";
    let resources = "/ExtGState << /Sm << /Type /ExtGState \
                     /SMask << /Type /Mask /S /Luminosity /G 5 0 R /TR 6 0 R >> >> >>";
    build_pdf(content, resources, &[obj_5.as_bytes(), obj_6.as_bytes()])
}

/// `/SMask /TR` Type 3 stitching with a zero-width subinterval per
/// the malformed-but-spec-permitted edge case in §7.10.4. The
/// implementation's defensible policy is to use the subfunction's
/// `encode_lo` directly when `(hi_i - lo_i) == 0`.
#[test]
fn smask_tr_type3_zero_width_subinterval_uses_encode_lo_byte_exact() {
    let rgba = render_rgba(fixture_smask_tr_type3_zero_width_subinterval());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    assert_eq!(
        (r, g, b),
        (255, 255, 255),
        "ISO 32000-1 §7.10.4 /SMask /TR Type 3 with zero-width subinterval \
         (Bounds [0.5] on Domain [0 0.5]) must use encode_lo when the \
         subinterval collapses; encoded = 0 → f1(0) = 0 → m_out = 0 → \
         destination = backdrop (255, 255, 255); got ({r}, {g}, {b})"
    );
}

// ===========================================================================
// §11.4.5 transparency groups — `/I` isolated flag
// ===========================================================================
//
// Isolated transparency groups: `/Group /S /Transparency /I true` —
// the group's initial backdrop is fully transparent; group content
// composites against itself, then the composited group is over-blended
// onto the parent. xberg-native-pdf implements this correctly per
// page_renderer.rs:2837-2862. The probe pins the boundary case where
// /I affects observable output: a red rect at α=0.5 inside an isolated
// group, with the group's own background empty, composited over a
// blue parent. Non-isolated would composite the red onto the blue
// inside the group; isolated lets the group's transparent backdrop
// reach the parent. ~keep

fn fixture_isolated_group_alpha_red_over_blue() -> Vec<u8> {
    let form_content = "/Half gs\n\
                        1 0 0 rg\n\
                        20 20 60 60 re\nf\n";
    let form_resources = "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >>";
    let obj_5 = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /Type /Group /S /Transparency /I true >> \
         /Resources << {} >> /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_resources,
        form_content.len(),
        form_content
    );
    let content = "0 0 1 rg\n0 0 100 100 re\nf\n\
                   /Fm1 Do\n";
    let resources = "/XObject << /Fm1 5 0 R >>";
    build_pdf(content, resources, &[obj_5.as_bytes()])
}

/// Pin: isolated transparency group composites internally then
/// over-blends onto the parent. The centre pixel reflects red-over-
/// blue at the group's effective alpha.
#[test]
fn isolated_transparency_group_composites_red_over_blue() {
    let rgba = render_rgba(fixture_isolated_group_alpha_red_over_blue());
    let (r, g, b, _) = pixel_at(&rgba, 50, 50);
    // The isolated group composites red at α=0.5 onto its transparent
    // backdrop, then the group (α effectively 0.5) is over-blended
    // onto the blue parent:
    //   group post-composition rgba = (128, 0, 0, 127)
    //   over blue (0, 0, 255, 255):
    //     r = 128 + (1 - 127/255)·0 = 128
    //     g = 0
    //     b = 0 + (1 - 127/255)·255 ≈ 127
    // Byte-exact reference under tiny_skia's premul math:
    // (128, 0, 127). The half-channel arithmetic is deterministic so
    // the exact reference is enforced. ~keep
    assert_eq!(
        (r, g, b),
        (128, 0, 127),
        "isolated group: expected byte-exact (128, 0, 127) from \
         red-α-half over blue parent; got ({r}, {g}, {b})"
    );
}
