//! JPEG 2000 (`/JPXDecode`) image decoding via hayro-jpeg2000.
//!
//! ISO 32000-1 §7.4.9: a `/JPXDecode` stream is a JPEG 2000 codestream — either a
//! raw J2K codestream or a JP2-boxed file. hayro-jpeg2000 handles both. This decodes
//! the codestream to interleaved 8-bit-per-component samples; the caller maps the
//! component count to a colour space and applies `/Decode`, `/SMask`, etc.

use crate::error::{Error, Result};

/// Pass-through filter for `/JPXDecode`.
///
/// Like `DCTDecode`/`JBIG2Decode`, the JPEG 2000 codestream is not decompressed
/// by the generic filter pipeline — it is handed to the image extractor, which
/// decodes it with hayro-jpeg2000 (`decode_jpx`). So this decoder returns its input
/// unchanged, so the pipeline can surface the codestream; the extractor then
/// decodes it.
pub struct JpxDecoder;

impl super::StreamDecoder for JpxDecoder {
    // `max_output_bytes` (GH#1764) is ignored: this is a pass-through, so output
    // never exceeds input length. ~keep
    fn decode(&self, input: &[u8], _max_output_bytes: usize) -> Result<Vec<u8>> {
        Ok(input.to_vec())
    }

    fn name(&self) -> &str {
        "JPXDecode"
    }
}

/// A decoded JPEG 2000 image: interleaved 8-bit samples plus component count.
#[derive(Debug)]
#[non_exhaustive]
pub struct JpxImage {
    /// `width * height * num_components` bytes, component-interleaved (row-major).
    pub samples: Vec<u8>,
    pub num_components: u8,
    /// The codestream's own image width (`Xsiz - XOsiz`), authoritative over a `/JPXDecode`
    /// dictionary's `/Width` per ISO 32000-1 §7.4.9 (GH#1900). ~keep
    pub width: u32,
    /// The codestream's own image height (`Ysiz - YOsiz`), authoritative over `/Height` for the
    /// same reason as `width`. ~keep
    pub height: u32,
    /// The codestream's opacity channel, one 8-bit sample per pixel, when it carries one. It is
    /// not part of `samples`. A PDF uses it only when the image dictionary sets `/SMaskInData`.
    pub opacity: Option<Vec<u8>>,
}

/// Decode a JP2/J2K codestream to interleaved 8-bit-per-component samples.
///
/// hayro-jpeg2000 yields one f32 plane per component (normalized to the component's
/// bit depth); `DecodedImage::data_u8()` interleaves these to 8-bit samples.
/// Components are assumed to share the image dimensions (no chroma subsampling) —
/// the common case for PDF image XObjects; a subsampled component is rejected with a
/// typed error rather than producing misaligned output.
/// Whether the codestream's last component should be dropped as alpha.
///
/// An alpha channel is a component like any other in the codestream, so counting components raw
/// reports a colour space the image does not have: RGBA read as four components and was mapped to
/// DeviceCMYK, and grey-with-alpha read as two and was rejected outright, dropping the image
/// (GH#1850). It is dropped rather than returned as a soft mask because the caller maps the
/// remaining count straight onto a `PixelFormat`; carrying transparency to an `/SMask` is separate.
///
/// `codestream_says_alpha` is `Image::has_alpha()`, and it is NOT the codestream's own answer for a
/// BARE codestream. `hayro_jpeg2000::j2c::parse` synthesises a colour space for a bare stream from
/// the component count alone -- Greyscale below 3, otherwise `Srgb`, ignoring `Csiz` -- and
/// `resolve_alpha_and_color_space` then reconciles the resulting 4-vs-3 mismatch under the default
/// `strict: false` by taking its repair arm `actual == num_channels + 1 && !has_alpha` and declaring
/// the fourth channel alpha. That arm is tested BEFORE the branch that would correctly infer CMYK at
/// four components, so it shadows it. A bare CMYK codestream therefore reports RGB + alpha, and
/// dropping that "alpha" discarded the K plane: CMY painted as RGB on the render path, and a shape
/// mismatch that dropped the image from every plate on the separation path. Both silent.
///
/// `declared_components` is the only signal that separates those two cases, and ISO 32000-1 §7.4.9
/// makes it authoritative over the codestream. A declared count equal to the actual one means every
/// component is a colour component, so there is no alpha to drop. ~keep
fn alpha_is_droppable(codestream_says_alpha: bool, num_components: usize, declared_components: Option<u8>) -> bool {
    codestream_says_alpha
        && num_components > 1
        && declared_components.is_none_or(|declared| usize::from(declared) != num_components)
}

/// `SOC`, the marker that opens every JPEG 2000 codestream (ISO/IEC 15444-1 A.4.1). ~keep
const SOC_MARKER: [u8; 2] = [0xFF, 0x4F];
/// `SOC` immediately followed by `SIZ` (`0xFF51`), which A.5.1 requires of every codestream: the
/// two are checked together because a `SIZ` anywhere else is not the one A.5.1 describes. ~keep
const SOC_THEN_SIZ: [u8; 4] = [0xFF, 0x4F, 0xFF, 0x51];
/// `jp2c`, the Contiguous Codestream box that holds the codestream in a JP2 file (ISO/IEC 15444-1
/// Annex I.4.2). ~keep
const JP2C_BOX_TYPE: [u8; 4] = *b"jp2c";
/// ISO/IEC 15444-1 Table A-9: within the `SIZ` segment the fields ahead of `Csiz` are all
/// fixed-width -- `Lsiz`(2) `Rsiz`(2) `Xsiz`(4) `Ysiz`(4) `XOsiz`(4) `YOsiz`(4) `XTsiz`(4)
/// `YTsiz`(4) `XTOsiz`(4) `YTOsiz`(4) -- so `Csiz` sits 38 bytes past the `SIZ` marker and 40
/// bytes past the start of the codestream, and `Lsiz` is exactly `38 + 3 x Csiz`. ~keep
const CSIZ_OFFSET_IN_CODESTREAM: usize = 40;
const SIZ_BYTES_BEFORE_COMPONENTS: u16 = 38;
const SIZ_BYTES_PER_COMPONENT: u16 = 3;

/// The component count (`Csiz`) the codestream's own `SIZ` marker segment declares.
///
/// This reads the header rather than asking hayro-jpeg2000, because hayro's answer for a bare
/// codestream is synthesised from the count rather than read from it -- see [`alpha_is_droppable`].
/// ISO/IEC 15444-1 A.5.1 fixes both where `Csiz` lives and that `Lsiz == 38 + 3 x Csiz`; that
/// identity is checked as a cross-brace, so a header whose length field disagrees with its own
/// component count is rejected rather than guessed at. A JP2-boxed file (Annex I.4) is handled by
/// walking its top-level boxes to the `jp2c` box that contains the codestream. Every failure --
/// a missing or misplaced marker, a truncated header, an inconsistent `Lsiz`, a count that cannot
/// be a PDF image's component count -- returns `None`, so a caller changes nothing. ~keep
fn codestream_component_count(bytes: &[u8]) -> Option<u8> {
    let codestream = locate_codestream(bytes)?;
    if !codestream.starts_with(&SOC_THEN_SIZ) {
        return None;
    }
    let lsiz = u16::from_be_bytes(codestream.get(4..6)?.try_into().ok()?);
    let csiz_end = CSIZ_OFFSET_IN_CODESTREAM + 2;
    let csiz = u16::from_be_bytes(codestream.get(CSIZ_OFFSET_IN_CODESTREAM..csiz_end)?.try_into().ok()?);
    let expected_lsiz = SIZ_BYTES_BEFORE_COMPONENTS.checked_add(SIZ_BYTES_PER_COMPONENT.checked_mul(csiz)?)?;
    if lsiz != expected_lsiz {
        return None;
    }
    u8::try_from(csiz).ok().filter(|&n| n > 0)
}

/// The codestream within `bytes`: `bytes` itself when it is bare, otherwise the contents of the
/// `jp2c` box. Only top-level boxes are walked -- `jp2c` is always one. ~keep
fn locate_codestream(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.starts_with(&SOC_MARKER) {
        return Some(bytes);
    }
    find_box(bytes, &JP2C_BOX_TYPE)
}

/// The contents of the first box of type `wanted` among the boxes laid end to end in `bytes`.
///
/// ISO/IEC 15444-1 Annex I.4: a JP2 box is `LBox`(4) `TBox`(4) `DBox`, where `LBox == 1` means the
/// real length follows as a 64-bit `XLBox` and `LBox == 0` means the box runs to end of file. A box
/// whose length runs past the end of `bytes` yields what is there. ~keep
fn find_box<'a>(bytes: &'a [u8], wanted: &[u8; 4]) -> Option<&'a [u8]> {
    let mut offset = 0usize;
    // `checked_add` rather than `offset + 8 <= len`: a 64-bit `XLBox` is attacker-controlled and
    // can advance `offset` to anywhere in the usize range. ~keep
    while offset.checked_add(8).is_some_and(|end| end <= bytes.len()) {
        let lbox = u32::from_be_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?);
        let box_type = bytes.get(offset + 4..offset + 8)?;
        let (header_len, box_len) = match lbox {
            1 => {
                let xlbox = u64::from_be_bytes(bytes.get(offset + 8..offset + 16)?.try_into().ok()?);
                (16usize, usize::try_from(xlbox).ok()?)
            }
            0 => (8usize, bytes.len() - offset),
            len => (8usize, usize::try_from(len).ok()?),
        };
        if box_len < header_len {
            return None;
        }
        if box_type == wanted {
            let end = offset.saturating_add(box_len).min(bytes.len());
            return bytes.get(offset + header_len..end);
        }
        offset = offset.checked_add(box_len)?;
    }
    None
}

/// The number of columns in a JP2 file's `pclr` palette box (ISO/IEC 15444-1 I.5.3.4), the
/// component count the palette resolves each index to, or `None` when the file carries no palette.
pub fn codestream_palette_columns(bytes: &[u8]) -> Option<u8> {
    let header = find_box(bytes, b"jp2h")?;
    find_box(header, b"pclr")?.get(2).copied()
}

/// The component count to treat as declared when the image dictionary named no `/ColorSpace` at all
/// -- legal for `/JPXDecode` per ISO 32000-1 Table 89, and the one case where §7.4.9's authoritative
/// entry is absent (GH#1883).
///
/// Only a BARE codestream is answered. JPEG 2000 types a component as opacity in the JP2 channel
/// definition box (`cdef`, ISO/IEC 15444-1 I.5.3.6); a bare codestream carries no boxes, so nothing
/// in it can mark a component as alpha and all `Csiz` of its components are colour components.
/// hayro's `has_alpha` there is invented from the count, as documented on [`alpha_is_droppable`],
/// and taking `Csiz` instead is what keeps a bare CMYK image's K plane. A JP2-boxed stream does
/// carry `colr`/`cdef`, so hayro's alpha answer for it is grounded in the file and must not be
/// overridden -- `None` leaves that answer in force. ~keep
fn undeclared_component_count(bytes: &[u8]) -> Option<u8> {
    if !bytes.starts_with(&SOC_MARKER) {
        return None;
    }
    codestream_component_count(bytes)
}

/// `declared_components` is how many colour components the image dictionary's `/ColorSpace`
/// implies, when it named one, and `None` when it did not. ISO 32000-1 §7.4.9 makes that entry
/// authoritative over anything in the JPEG 2000 data, and for a JP2-boxed stream it is the only way
/// to tell a 4-component CMYK codestream from an RGBA one -- see the alpha decision below. When it
/// is `None`, [`undeclared_component_count`] supplies the codestream's own `Csiz` in its place for
/// the one shape where that is sound. ~keep
pub fn decode_jpx(bytes: &[u8], declared_components: Option<u8>) -> Result<JpxImage> {
    use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image};

    // Nothing declared a count, so fall back to the codestream's own `SIZ` header where that is
    // meaningful. See [`undeclared_component_count`]. ~keep
    let declared_components = declared_components.or_else(|| undeclared_component_count(bytes));

    let image = Image::new(bytes, &DecodeSettings::default())
        .map_err(|e| Error::UnsupportedFilter(format!("JPXDecode: JPEG 2000 decode failed: {e:?}")))?;

    let width = image.width();
    let height = image.height();
    let npix = width as usize * height as usize;

    let mut ctx = DecoderContext::default();
    let decoded = image
        .decode(&mut ctx)
        .map_err(|e| Error::UnsupportedFilter(format!("JPXDecode: JPEG 2000 decode failed: {e:?}")))?;

    let comps = decoded.components();
    if comps.is_empty() {
        return Err(Error::UnsupportedFilter(
            "JPXDecode: JPEG 2000 image has no components".to_string(),
        ));
    }
    let num_components = comps.len();

    let has_alpha = alpha_is_droppable(image.has_alpha(), num_components, declared_components);
    let colour_components = if has_alpha { num_components - 1 } else { num_components };

    // Fast path: every component is full-resolution (the common case) → use the
    // decoder's own interleave. ~keep
    if comps.iter().all(|c| c.samples().len() == npix) {
        let mut samples = decoded.data_u8();
        let mut opacity = None;
        if has_alpha {
            opacity = Some(
                samples
                    .iter()
                    .skip(num_components - 1)
                    .step_by(num_components)
                    .copied()
                    .collect(),
            );
            samples = drop_last_channel(&samples, num_components);
        }
        return Ok(JpxImage {
            samples,
            num_components: colour_components as u8,
            width,
            height,
            opacity,
        });
    }

    decode_subsampled(comps, width, height, colour_components)
}

/// Decode the palette indices of a JPEG 2000 image whose dictionary names an `/Indexed` colour
/// space, one byte per pixel, for the caller to look up in the dictionary's own palette. The
/// result is a one-component [`JpxImage`] at the codestream's own size, which the caller uses in
/// place of the dictionary's `/Width` and `/Height` as [`decode_jpx`]'s callers do (GH#1900).
///
/// ISO 32000-1 §7.4.9 says a `/ColorSpace` entry overrides any colour specification in the
/// JPEG 2000 data, and a `pclr` palette box is one: pdf.js decodes such an image with the
/// codestream palette switched off for the same reason. A lossily coded index plane rings around
/// every edge, so samples land a little outside the palette. Rounding and clamping to
/// `highest_index`, the last entry the dictionary's palette holds, maps that ringing to the nearest
/// real index. ~keep
///
/// The index plane is the first component. hayro-jpeg2000 reports the colour channels it found,
/// and with palette resolution off it reports an image carrying a `pclr` box as one grey channel
/// and clears its alpha flag, even when a second component is the image's opacity. So the channel
/// count, not the component count, says whether the image is an index plane; any component after
/// the first is opacity and is dropped, as [`decode_jpx`] drops it for every other image. ~keep
pub fn decode_jpx_indices(bytes: &[u8], highest_index: u8) -> Result<JpxImage> {
    use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image};

    let settings = DecodeSettings {
        resolve_palette_indices: false,
        ..DecodeSettings::default()
    };
    let image = Image::new(bytes, &settings)
        .map_err(|e| Error::UnsupportedFilter(format!("JPXDecode: JPEG 2000 decode failed: {e:?}")))?;

    let mut ctx = DecoderContext::default();
    let decoded = image
        .decode(&mut ctx)
        .map_err(|e| Error::UnsupportedFilter(format!("JPXDecode: JPEG 2000 decode failed: {e:?}")))?;

    let index_components = image.color_space().num_channels();
    let [indices, ..] = decoded.components() else {
        return Err(Error::UnsupportedFilter(
            "JPXDecode: JPEG 2000 image has no components".to_string(),
        ));
    };
    if index_components != 1 {
        return Err(Error::UnsupportedFilter(format!(
            "JPXDecode: an /Indexed JPEG 2000 image needs one index component, found {index_components}"
        )));
    }
    let (indices, clamped) = clamp_indices(indices.samples(), highest_index);
    let opacity = codestream_opacity_component(bytes)
        .and_then(|component| decoded.components().get(component))
        .and_then(|component| {
            let samples = component.samples();
            (samples.len() == image.width() as usize * image.height() as usize).then(|| {
                samples
                    .iter()
                    .map(|&value| value.round().clamp(0.0, 255.0) as u8)
                    .collect()
            })
        });
    if clamped > 0 {
        tracing::warn!(
            clamped,
            total = indices.len(),
            highest_index,
            "JPXDecode: clamped out-of-range palette indices to the palette"
        );
    }
    Ok(JpxImage {
        samples: indices,
        num_components: 1,
        width: image.width(),
        height: image.height(),
        opacity,
    })
}

/// The codestream component mapped to a JP2 channel-definition entry of type opacity.
///
/// A palette maps output channels back to codestream components through `cmap`, so the opacity
/// channel number in `cdef` is not necessarily its component number. (GH#1885) ~keep
fn codestream_opacity_component(bytes: &[u8]) -> Option<usize> {
    let header = find_box(bytes, b"jp2h")?;
    let definitions = find_box(header, b"cdef")?;
    let count = usize::from(u16::from_be_bytes(definitions.get(0..2)?.try_into().ok()?));
    let entries = definitions.get(2..2 + count.checked_mul(6)?)?;
    let channel = entries.chunks_exact(6).find_map(|entry| {
        let channel = u16::from_be_bytes(entry[0..2].try_into().ok()?);
        let kind = u16::from_be_bytes(entry[2..4].try_into().ok()?);
        matches!(kind, 1 | 2).then_some(usize::from(channel))
    })?;
    let Some(mapping) = find_box(header, b"cmap") else {
        return Some(channel);
    };
    let entry = mapping.get(channel.checked_mul(4)?..channel.checked_add(1)?.checked_mul(4)?)?;
    Some(usize::from(u16::from_be_bytes(entry[0..2].try_into().ok()?)))
}

/// Round each index sample and clamp it to `0..=highest_index`, and count the samples the clamp
/// moved. hayro-jpeg2000 hands back each component's samples at the codestream's own scale, not
/// normalised to 8 bits, so this reads a 16-bit index plane correctly too. ~keep
fn clamp_indices(samples: &[f32], highest_index: u8) -> (Vec<u8>, usize) {
    let highest = f32::from(highest_index);
    let mut clamped = 0;
    let indices = samples
        .iter()
        .map(|&v| {
            let rounded = v.round();
            if !(0.0..=highest).contains(&rounded) {
                clamped += 1;
            }
            rounded.clamp(0.0, highest) as u8
        })
        .collect();
    (indices, clamped)
}

/// The chroma-subsampled path (WS1.7), separated so [`decode_jpx`] stays readable.
///
/// hayro-jpeg2000 0.4 does not expose per-component dimensions, so only the unambiguous 2x2 (4:2:0)
/// case is recovered: a component with ceil(w/2)*ceil(h/2) samples is nearest-upsampled to full
/// resolution. Any other ratio, or a non-8-bit depth where the f32 -> u8 scaling would differ, stays
/// unsupported rather than guessed at. Components are interleaved by hand because `data_u8` assumes
/// equal plane sizes. The alpha plane, when present, is the last one and is simply not
/// interleaved -- `colour_components` already excludes it. ~keep
fn decode_subsampled(
    comps: &[hayro_jpeg2000::ComponentData],
    width: u32,
    height: u32,
    colour_components: usize,
) -> Result<JpxImage> {
    let npix = width as usize * height as usize;
    let (w, h) = (width as usize, height as usize);
    let (sw, sh) = (width.div_ceil(2) as usize, height.div_ceil(2) as usize);
    let mut planes: Vec<Vec<u8>> = Vec::with_capacity(comps.len());
    for (ci, comp) in comps.iter().enumerate() {
        if comp.bit_depth() != 8 {
            return Err(Error::UnsupportedFilter(format!(
                "JPXDecode: subsampled component {ci} with {}-bit depth not supported",
                comp.bit_depth()
            )));
        }
        let s = comp.samples();
        let plane = if s.len() == npix {
            s.iter().map(|&v| v.round().clamp(0.0, 255.0) as u8).collect()
        } else if s.len() == sw * sh {
            upsample_nearest_u8(s, sw, sh, w, h)
        } else {
            return Err(Error::UnsupportedFilter(format!(
                "JPXDecode: subsampled component {ci} ({} samples) — only 2×2 (4:2:0) \
                 subsampling of a {width}×{height} image is supported",
                s.len()
            )));
        };
        planes.push(plane);
    }

    let mut samples = vec![0u8; npix * colour_components];
    for (ci, plane) in planes.iter().take(colour_components).enumerate() {
        for (i, &px) in plane.iter().enumerate() {
            samples[i * colour_components + ci] = px;
        }
    }
    Ok(JpxImage {
        samples,
        num_components: colour_components as u8,
        width,
        height,
        opacity: planes.get(colour_components).cloned(),
    })
}

/// Drop the last channel of a component-interleaved buffer, narrowing it from `stride`
/// channels per pixel to `stride - 1`.
fn drop_last_channel(samples: &[u8], stride: usize) -> Vec<u8> {
    debug_assert!(stride > 1, "narrowing a single-channel buffer would leave nothing");
    let kept = stride - 1;
    let mut out = Vec::with_capacity(samples.len() / stride * kept);
    for pixel in samples.chunks_exact(stride) {
        out.extend_from_slice(&pixel[..kept]);
    }
    out
}

/// Nearest-neighbour upsample of an `sw×sh` f32 sample plane to `fw×fh` u8.
fn upsample_nearest_u8(sub: &[f32], sw: usize, sh: usize, fw: usize, fh: usize) -> Vec<u8> {
    let mut out = vec![0u8; fw * fh];
    for y in 0..fh {
        let sy = (y * sh / fh).min(sh.saturating_sub(1));
        for x in 0..fw {
            let sx = (x * sw / fw).min(sw.saturating_sub(1));
            out[y * fw + x] = sub[sy * sw + sx].round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{alpha_is_droppable, decode_jpx, decode_jpx_indices, upsample_nearest_u8};

    /// Grayscale JP2 codestream from the minimal repro (816x1056 DeviceGray).
    const SAMPLE_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/sample_gray.jp2");

    /// GH#1850 fixtures, 16x16 lossless, generated with Pillow's OpenJPEG encoder. The left
    /// half is opaque `(200, 100, 50)` and the right half is `(10, 220, 90)` at alpha 128, so a
    /// test can tell a dropped alpha channel from a shifted one. ~keep
    const RGBA_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1850_rgba.jp2");
    /// The same image as a bare codestream (no JP2 container), which reaches a different
    /// header path in the decoder.
    const RGBA_J2K: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1850_rgba.j2k");
    /// Greyscale plus alpha: two components, which the unfixed decoder rejected outright.
    const GREY_ALPHA_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1850_grey_alpha.jp2");
    /// A BARE 4-component CMYK codestream, 16x16, no JP2 container and so no `colr` box to
    /// declare CMYK. This is the shape hayro-jpeg2000 misreads as RGB + alpha. ~keep
    const CMYK_QUADRANTS_J2K: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1855_cmyk_quadrants.j2k");

    /// GH#1885 fixtures, 120x40, dark text (index 0) on paper (index 255), generated with Pillow's
    /// OpenJPEG encoder. `PALETTE_CMYK_JP2` is one 8-bit index component coded lossily (9/7
    /// irreversible, rate 4) in a JP2 container carrying a `colr` box naming CMYK, a 256-entry
    /// four-column `pclr` box (entry `i` is CMYK `(0, 0, 0, 255 - i)`) and a `cmap` box routing the
    /// component through all four columns. The lossy coding leaves index samples outside
    /// `0..=255` near the glyph edges. `INDICES_GREY_JP2` is the same picture coded losslessly as
    /// a plain greyscale JP2 with no palette box. ~keep
    const PALETTE_CMYK_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1885_palette_cmyk.jp2");
    const INDICES_GREY_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1885_indices_grey.jp2");
    /// The same picture as one index component plus an opaque alpha component. The `pclr` box routes
    /// the first through the same four-column CMYK palette, and the `cmap` and `cdef` boxes route the
    /// second direct as opacity. With palette resolution off, hayro-jpeg2000 reports this as one
    /// grey channel with no alpha. ~keep
    const PALETTE_ALPHA_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1885_palette_alpha.jp2");
    /// A lossily coded index plane for a 16-entry palette, ink at index 0 and paper at 15, in a plain
    /// greyscale JP2. The lossy coding rings samples up to 16, one past the last palette entry. ~keep
    const HIVAL15_LOSSY_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1885_hival15_lossy.jp2");

    #[test]
    fn a_lossy_palette_codestream_decodes_to_clamped_indices() {
        let indices = decode_jpx_indices(PALETTE_CMYK_JP2, u8::MAX)
            .expect("GH#1885: the index plane must decode")
            .samples;
        assert_eq!(indices.len(), 120 * 40, "one index byte per pixel");
        assert_eq!(indices[0], 255, "the top-left pixel is paper");
        assert!(indices.contains(&0), "the ink index must survive the clamp");
    }

    /// The unresolved plane really rings outside `0..=255`; each such sample must land on the
    /// nearest end of the palette, and every in-range sample must round, not truncate.
    #[test]
    fn lossy_indices_round_and_clamp_samples_outside_the_palette() {
        use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image};
        let settings = DecodeSettings {
            resolve_palette_indices: false,
            ..DecodeSettings::default()
        };
        let image = Image::new(PALETTE_CMYK_JP2, &settings).expect("the index plane must parse");
        let mut ctx = DecoderContext::default();
        let decoded = image.decode(&mut ctx).expect("the index plane must decode");
        let plane = decoded.components()[0].samples();
        let below = plane.iter().filter(|&&v| v.round() < 0.0).count();
        let above = plane.iter().filter(|&&v| v.round() > 255.0).count();
        assert!(
            below > 0 && above > 0,
            "control failed: the lossy plane must ring past both ends, got {below} below and {above} above"
        );
        let rounds_up = plane
            .iter()
            .filter(|&&v| (0.0..255.0).contains(&v) && v.fract() > 0.5)
            .count();
        assert!(
            rounds_up > 0,
            "control failed: the plane must hold samples that round up"
        );

        let indices = decode_jpx_indices(PALETTE_CMYK_JP2, u8::MAX).expect("indices").samples;
        for (&v, &index) in plane.iter().zip(&indices) {
            let expected = if v.round() < 0.0 {
                0
            } else if v.round() > 255.0 {
                255
            } else {
                v.round() as u8
            };
            assert_eq!(index, expected, "sample {v} must map to index {expected}");
        }
        assert_eq!(
            super::clamp_indices(plane, u8::MAX).1,
            below + above,
            "every repaired sample is counted"
        );
    }

    /// hayro-jpeg2000 returns a 16-bit plane's samples unscaled, so a 16-bit index plane reads
    /// its indices directly rather than as a fraction of 65535. The fixture is a lossless 16x16
    /// greyscale JP2, index 3 on the left half and index 200 on the right. ~keep
    #[test]
    fn a_16_bit_index_plane_reads_its_indices_unscaled() {
        const INDICES_16BIT_JP2: &[u8] = include_bytes!("../../tests/fixtures/jpx/gh1885_indices_16bit.jp2");
        let settings = hayro_jpeg2000::DecodeSettings {
            resolve_palette_indices: false,
            ..hayro_jpeg2000::DecodeSettings::default()
        };
        let image = hayro_jpeg2000::Image::new(INDICES_16BIT_JP2, &settings).expect("the fixture must parse");
        let mut ctx = hayro_jpeg2000::DecoderContext::default();
        let decoded = image.decode(&mut ctx).expect("the fixture must decode");
        assert_eq!(
            decoded.components()[0].bit_depth(),
            16,
            "control failed: the fixture must be a 16-bit plane"
        );
        let indices = decode_jpx_indices(INDICES_16BIT_JP2, u8::MAX)
            .expect("a 16-bit index plane must decode")
            .samples;
        assert_eq!(indices.len(), 16 * 16);
        assert_eq!((indices[0], indices[15]), (3, 200), "the indices must not be rescaled");
    }

    /// Decoding a plane the clamp had to repair logs one warning; a clean plane logs none.
    #[test]
    fn a_repaired_index_plane_logs_one_warning() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tracing_subscriber::layer::SubscriberExt as _;

        struct WarnCount(Arc<AtomicUsize>);
        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for WarnCount {
            fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
                if *event.metadata().level() == tracing::Level::WARN {
                    self.0.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
        let warnings = |bytes: &[u8]| {
            // ~keep tracing-core treats a lone live dispatcher as the only one, so a test on another
            // thread that reaches the warning first caches it as "never" from its own empty
            // subscriber. A second live dispatcher makes that registration ask this one as well.
            let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
            let count = Arc::new(AtomicUsize::new(0));
            let subscriber = tracing_subscriber::registry().with(WarnCount(Arc::clone(&count)));
            tracing::subscriber::with_default(subscriber, || decode_jpx_indices(bytes, u8::MAX).expect("indices"));
            count.load(Ordering::SeqCst)
        };
        assert_eq!(warnings(PALETTE_CMYK_JP2), 1, "the lossy plane needs the clamp");
        assert_eq!(
            warnings(INDICES_GREY_JP2),
            0,
            "control: a lossless plane needs no repair"
        );
    }

    /// Every sample the clamp moves is counted, so the decoder can warn with the count.
    #[test]
    fn clamp_indices_counts_the_samples_it_repairs() {
        let (indices, clamped) = super::clamp_indices(&[-0.6, -0.4, 4.5, 15.4, 15.6, 300.0], 15);
        assert_eq!(indices, vec![0, 0, 5, 15, 15, 15]);
        assert_eq!(clamped, 3, "-0.6, 15.6 and 300.0 fall outside 0..=15");
    }

    /// Without a palette box the index plane is the codestream's only component, unchanged.
    #[test]
    fn a_plain_codestream_decodes_to_its_own_samples_as_indices() {
        let decoded = decode_jpx_indices(INDICES_GREY_JP2, u8::MAX).expect("a plain index codestream must decode");
        assert_eq!(decoded.opacity, None, "an unmarked component must not become opacity");
        let indices = decoded.samples;
        let grey = decode_jpx(INDICES_GREY_JP2, None).expect("the same codestream as greyscale");
        assert_eq!(indices, grey.samples);
        assert!(
            indices.contains(&0) && indices.contains(&255),
            "the fixture must hold ink and paper"
        );
    }

    /// An alpha channel is dropped as it is for any other image; three colour components cannot
    /// be palette indices, so that image is refused rather than read through its first plane.
    #[test]
    fn index_decoding_drops_alpha_and_refuses_colour_codestreams() {
        let grey = decode_jpx_indices(GREY_ALPHA_JP2, u8::MAX)
            .expect("grey plus alpha has one index component")
            .samples;
        assert_eq!(grey.len(), 16 * 16);
        assert_eq!(grey[0], 180, "the left half's index must survive");

        let err = decode_jpx_indices(RGBA_JP2, u8::MAX).expect_err("an RGBA codestream has three components");
        assert!(format!("{err:?}").contains("found 3"), "unexpected failure: {err:?}");
    }

    /// A palette box plus an opacity channel: the index plane is the first component and the opacity
    /// plane after it is dropped, not counted as a second index component.
    #[test]
    fn a_palette_codestream_with_an_opacity_channel_decodes_its_index_plane() {
        let decoded = decode_jpx_indices(PALETTE_ALPHA_JP2, u8::MAX).expect("the index plane must decode");
        let indices = decoded.samples;
        let plain = decode_jpx_indices(INDICES_GREY_JP2, u8::MAX)
            .expect("the same picture without alpha")
            .samples;
        assert_eq!(
            indices, plain,
            "the indices must be the plane's, not the opacity plane's"
        );
        assert_eq!(
            decoded.opacity.as_deref(),
            Some(vec![255; 120 * 40].as_slice()),
            "the marked opacity plane must be retained beside the indices"
        );
    }

    /// Ringing past a short palette clamps to its last entry, not to 255: the expander paints an
    /// index past the palette black.
    #[test]
    fn lossy_indices_clamp_to_the_highest_palette_index() {
        let unclamped = decode_jpx_indices(HIVAL15_LOSSY_JP2, u8::MAX)
            .expect("the index plane must decode")
            .samples;
        assert!(
            unclamped.iter().any(|&i| i > 15),
            "control failed: the fixture must ring past index 15"
        );
        let indices = decode_jpx_indices(HIVAL15_LOSSY_JP2, 15)
            .expect("the index plane must decode")
            .samples;
        assert_eq!(
            indices.iter().max(),
            Some(&15),
            "no index may pass the palette's last entry"
        );
        assert!(indices.contains(&0), "the ink index must survive the clamp");
    }

    /// The alpha decision in isolation, so the truth table is readable without decoding anything.
    /// The third row is the GH#1850 case: a declared count equal to the actual one means every
    /// component is colour, so hayro's phantom alpha must be ignored. ~keep
    #[test]
    fn alpha_is_droppable_truth_table() {
        // (codestream_says_alpha, num_components, declared, expected)
        for (says, n, declared, expected) in [
            (false, 4, None, false),
            (true, 1, None, false),
            (true, 4, Some(4), false),
            (true, 4, Some(3), true),
            (true, 4, None, true),
            (true, 2, Some(1), true),
        ] {
            assert_eq!(
                alpha_is_droppable(says, n, declared),
                expected,
                "says={says} n={n} declared={declared:?}"
            );
        }
    }

    /// GH#1883: a bare CMYK codestream must keep its K plane whether or not the image dictionary
    /// declares a colour space. ISO 32000-1 Table 89 lets a `/JPXDecode` image omit `/ColorSpace`,
    /// and the count then has to come from the codestream's own `SIZ` header.
    ///
    /// The first assertion is the NEGATIVE CONTROL, and it pins the upstream misread *directly*
    /// rather than through its consequence: hayro-jpeg2000 reports an alpha channel on a bare
    /// codestream, which has no `cdef` box able to declare one. It has to be asserted on `Image`
    /// because the workaround now yields four components either way, so the decoder's output no
    /// longer distinguishes a fixed upstream from a broken one. When this assertion fails, upstream
    /// has stopped inventing the channel and `undeclared_component_count` can go. ~keep
    #[test]
    fn a_bare_cmyk_codestream_keeps_four_components_whether_or_not_the_dictionary_declares_them() {
        let raw = hayro_jpeg2000::Image::new(CMYK_QUADRANTS_J2K, &hayro_jpeg2000::DecodeSettings::default())
            .expect("bare CMYK codestream must parse");
        assert!(
            raw.has_alpha(),
            "negative control: upstream still invents an alpha channel for a bare codestream, which \
             carries no cdef box to declare one -- if this fails, undeclared_component_count can go"
        );

        let undeclared = decode_jpx(CMYK_QUADRANTS_J2K, None).expect("bare CMYK codestream must decode");
        assert_eq!(
            undeclared.num_components, 4,
            "with no declared colour space the Csiz of 4 must stand and the K plane must survive"
        );
        assert_eq!(
            undeclared.samples.len(),
            16 * 16 * 4,
            "all four planes must be present, 16x16 image"
        );

        let declared =
            decode_jpx(CMYK_QUADRANTS_J2K, Some(4)).expect("bare CMYK codestream must decode with a declared count");
        assert_eq!(
            declared.num_components, 4,
            "a declared 4 must suppress the phantom alpha and keep all four planes"
        );
        assert_eq!(
            declared.samples.len(),
            16 * 16 * 4,
            "all four planes must be present, 16x16 image"
        );
    }

    /// A declared count that does NOT match the component count must leave a real alpha channel
    /// alone: an RGBA codestream under `/DeviceRGB` declares 3 against 4 actual, so the mismatch
    /// arm keeps the existing drop. ~keep
    #[test]
    fn a_declared_count_that_disagrees_does_not_suppress_a_real_alpha_channel() {
        for (label, bytes) in [("jp2", RGBA_JP2), ("j2k", RGBA_J2K)] {
            let img = decode_jpx(bytes, Some(3)).unwrap_or_else(|e| panic!("{label} must decode: {e:?}"));
            assert_eq!(
                img.num_components, 3,
                "{label}: a declared 3 against 4 actual components must still drop the alpha"
            );
        }
    }

    /// GH#1850: an alpha channel is a component like any other in the codestream, so counting
    /// components raw described a colour space the image does not have. Four components were
    /// mapped to DeviceCMYK by the caller, and two were rejected outright, dropping the image.
    ///
    /// Only the JP2-boxed fixture belongs here. Its `cdef` box declares the fourth channel opaque
    /// (ISO/IEC 15444-1 I.5.3.6), so hayro's alpha answer is read from the file and is the one to
    /// honour. The bare variant has no such box -- see
    /// [`a_bare_four_component_codestream_with_nothing_declared_is_read_as_cmyk`]. ~keep
    #[test]
    fn rgba_codestream_reports_three_colour_components() {
        let img = decode_jpx(RGBA_JP2, None).unwrap_or_else(|e| panic!("jp2 must decode: {e:?}"));
        assert_eq!(
            img.num_components, 3,
            "the alpha channel must not be counted as a colour component"
        );
        assert_eq!(img.samples.len(), 16 * 16 * 3, "samples must be RGB-interleaved");
        // Pixel (0,0) is the opaque half; alpha must be gone, not shifted into a channel.
        assert_eq!(&img.samples[..3], &[200, 100, 50], "first pixel must stay RGB");
    }

    /// The cost of the GH#1883 rule, asserted rather than left implicit: a bare four-component
    /// codestream that is really RGBA, in a PDF whose dictionary declares no `/ColorSpace`, is now
    /// read as CMYK. Nothing can separate it from a bare CMYK codestream -- both reach hayro as
    /// four components with no `cdef` box, and hayro invents the same `Srgb` + alpha answer for
    /// each. A PDF carries transparency in `/SMask`, not in the codestream, so resolving the
    /// ambiguity towards four colour components loses a prepress image's K plane in neither
    /// direction while costing only the alpha of a stream that should not have relied on it.
    /// The same fixture under a declared `/DeviceRGB` still drops its alpha -- see
    /// [`a_declared_count_that_disagrees_does_not_suppress_a_real_alpha_channel`]. ~keep
    #[test]
    fn a_bare_four_component_codestream_with_nothing_declared_is_read_as_cmyk() {
        let img = decode_jpx(RGBA_J2K, None).expect("bare four-component codestream must decode");
        assert_eq!(
            img.num_components, 4,
            "a bare codestream cannot type a channel as alpha, so all four Csiz components are colour"
        );
        assert_eq!(img.samples.len(), 16 * 16 * 4, "all four planes must be interleaved");
    }

    /// The two-component case the decoder rejected, so the image never reached the page at all.
    #[test]
    fn grey_plus_alpha_codestream_decodes_as_single_channel_grey() {
        let img = decode_jpx(GREY_ALPHA_JP2, None).expect("grey+alpha must decode rather than be dropped");
        assert_eq!(img.num_components, 1, "alpha must not be counted as a colour component");
        assert_eq!(img.samples.len(), 16 * 16, "samples must be one channel per pixel");
        assert_eq!(img.samples[0], 180, "the left half's grey value must survive");
    }

    /// An 8x8 bare codestream: a lossless grey plane at full resolution, `16 * x + 2 * y`, and an
    /// opacity plane coded 2x2 subsampled, 4x4 samples of `17 * i`. ~keep
    const SUBSAMPLED_GREY_ALPHA_J2K: &[u8] =
        include_bytes!("../../tests/fixtures/jpx/gh1902_subsampled_grey_alpha.j2k");

    /// GH#1902: an opacity plane coded subsampled still reaches `/SMaskInData` at full resolution.
    /// hayro-jpeg2000 0.4 upsamples every component to the image size itself, so the image takes
    /// the full-resolution path and not [`decode_subsampled`]. If the first assertion ever fails,
    /// hayro started returning planes at their coded size and the subsampled path is live. ~keep
    #[test]
    fn a_subsampled_opacity_plane_is_returned_at_full_resolution() {
        use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image};
        let image = Image::new(SUBSAMPLED_GREY_ALPHA_J2K, &DecodeSettings::default()).expect("fixture parses");
        let mut ctx = DecoderContext::default();
        let decoded = image.decode(&mut ctx).expect("fixture decodes");
        let lens: Vec<usize> = decoded.components().iter().map(|c| c.samples().len()).collect();
        assert_eq!(
            lens,
            vec![64, 64],
            "hayro-jpeg2000 upsamples the subsampled plane itself"
        );

        let img = decode_jpx(SUBSAMPLED_GREY_ALPHA_J2K, Some(1)).expect("subsampled grey+alpha must decode");
        assert_eq!(img.num_components, 1, "the opacity plane is not a colour component");
        let grey: Vec<u8> = (0..8u8).flat_map(|y| (0..8u8).map(move |x| 16 * x + 2 * y)).collect();
        assert_eq!(img.samples, grey, "the grey plane must be read as coded");
        let opacity: Vec<u8> = (0..8usize)
            .flat_map(|y| (0..8usize).map(move |x| 17 * (y / 2 * 4 + x / 2) as u8))
            .collect();
        assert_eq!(
            img.opacity,
            Some(opacity),
            "the opacity plane must be upsampled, not dropped"
        );
    }

    /// The `SIZ` reader against every JPEG 2000 fixture in the tree, bare and JP2-boxed, with the
    /// expected `Csiz` taken from each fixture's construction. This is the assertion that the
    /// Annex A.5.1 offsets are right; the decode tests above only see the consequence. ~keep
    #[test]
    fn codestream_component_count_reads_csiz_from_bare_and_jp2_boxed_streams() {
        for (label, bytes, expected) in [
            ("bare cmyk j2k", CMYK_QUADRANTS_J2K, 4u8),
            ("bare rgba j2k", RGBA_J2K, 4),
            ("boxed rgba jp2", RGBA_JP2, 4),
            ("boxed grey+alpha jp2", GREY_ALPHA_JP2, 2),
            ("boxed grey jp2", SAMPLE_JP2, 1),
            ("bare subsampled grey+alpha j2k", SUBSAMPLED_GREY_ALPHA_J2K, 2),
        ] {
            assert_eq!(
                super::codestream_component_count(bytes),
                Some(expected),
                "{label}: Csiz must be read from the SIZ marker segment"
            );
        }
    }

    /// Only a bare stream is answered, because only a bare stream has no `cdef` box that could
    /// have typed a channel as alpha. A JP2-boxed stream must defer to hayro. ~keep
    #[test]
    fn undeclared_component_count_answers_only_for_a_bare_codestream() {
        assert_eq!(super::undeclared_component_count(CMYK_QUADRANTS_J2K), Some(4));
        assert_eq!(super::undeclared_component_count(RGBA_J2K), Some(4));
        assert_eq!(
            super::undeclared_component_count(RGBA_JP2),
            None,
            "a JP2-boxed stream carries colr/cdef, so its alpha answer must not be overridden"
        );
    }

    /// Fail closed: every way the header can fail to be where ISO/IEC 15444-1 A.5.1 says must
    /// return `None` so the caller changes nothing, rather than reading a neighbouring field as a
    /// component count. ~keep
    #[test]
    fn codestream_component_count_fails_closed_on_a_malformed_header() {
        assert_eq!(super::codestream_component_count(&[]), None, "empty input");
        assert_eq!(
            super::codestream_component_count(&CMYK_QUADRANTS_J2K[..41]),
            None,
            "truncated one byte inside Csiz"
        );
        assert_eq!(
            super::codestream_component_count(b"not a jpeg 2000 stream at all, forty-two bytes long"),
            None,
            "neither a SOC marker nor a JP2 box structure"
        );

        let mut no_siz = CMYK_QUADRANTS_J2K.to_vec();
        no_siz[3] = 0x52;
        assert_eq!(
            super::codestream_component_count(&no_siz),
            None,
            "SOC not followed immediately by SIZ"
        );

        let mut bad_lsiz = CMYK_QUADRANTS_J2K.to_vec();
        bad_lsiz[5] = bad_lsiz[5].wrapping_add(1);
        assert_eq!(
            super::codestream_component_count(&bad_lsiz),
            None,
            "Lsiz must equal 38 + 3 x Csiz or the header is not trusted"
        );
    }

    /// The narrowing helper on its own, so a failure above points at the decoder rather than
    /// at the interleave arithmetic.
    #[test]
    fn drop_last_channel_narrows_each_pixel() {
        let rgba = [1u8, 2, 3, 4, 11, 12, 13, 14];
        assert_eq!(super::drop_last_channel(&rgba, 4), vec![1, 2, 3, 11, 12, 13]);
        let la = [7u8, 255, 9, 128];
        assert_eq!(super::drop_last_channel(&la, 2), vec![7, 9]);
    }

    /// WS1.7: nearest-neighbour upsample of a 2×2 subsampled plane to 4×4 —
    /// each source sample fills its 2×2 output block.
    #[test]
    fn upsample_nearest_2x2_to_4x4() {
        let sub = [10.0f32, 20.0, 30.0, 40.0];
        let out = upsample_nearest_u8(&sub, 2, 2, 4, 4);
        assert_eq!(
            out,
            vec![10, 10, 20, 20, 10, 10, 20, 20, 30, 30, 40, 40, 30, 30, 40, 40,]
        );
    }

    /// Odd full dimensions (⌈w/2⌉ source): upsample 2×2 → 3×3 clamps at edges.
    #[test]
    fn upsample_nearest_2x2_to_3x3() {
        let sub = [1.0f32, 2.0, 3.0, 4.0];
        let out = upsample_nearest_u8(&sub, 2, 2, 3, 3);
        assert_eq!(out.len(), 9);
        assert_eq!(out[0], 1);
        assert_eq!(out[8], 4);
    }

    #[test]
    fn decode_jpx_grayscale() {
        let img = decode_jpx(SAMPLE_JP2, None).expect("decode JP2 codestream");

        assert_eq!(img.num_components, 1);
        assert_eq!(img.samples.len(), 816 * 1056);

        // A scanned page is not one flat value. ~keep
        let first = img.samples[0];
        assert!(
            img.samples.iter().any(|&b| b != first),
            "decoded image is uniformly flat — decode likely failed"
        );
    }

    /// The `pclr` box's column count is read from the JP2 header; a file with no palette box and a
    /// bare codestream have none.
    #[test]
    fn codestream_palette_columns_reads_the_pclr_box() {
        assert_eq!(super::codestream_palette_columns(PALETTE_CMYK_JP2), Some(4));
        assert_eq!(super::codestream_palette_columns(PALETTE_ALPHA_JP2), Some(4));
        assert_eq!(super::codestream_palette_columns(INDICES_GREY_JP2), None);
        assert_eq!(super::codestream_palette_columns(RGBA_J2K), None);
    }

    fn jp2_box(kind: &[u8; 4], contents: &[u8]) -> Vec<u8> {
        let mut out = u32::try_from(8 + contents.len())
            .expect("small box")
            .to_be_bytes()
            .to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(contents);
        out
    }

    /// A box with `LBox` 0 runs to the end of the file; one whose `XLBox` runs past it yields what
    /// is there. (GH#1903)
    #[test]
    fn find_box_reads_open_ended_and_extended_lengths() {
        let mut open_ended = jp2_box(b"jP  ", &[0x0D, 0x0A, 0x87, 0x0A]);
        open_ended.extend_from_slice(&[0, 0, 0, 0]);
        open_ended.extend_from_slice(b"pclr");
        open_ended.extend_from_slice(&[9, 8, 7]);
        assert_eq!(super::find_box(&open_ended, b"pclr"), Some(&[9u8, 8, 7][..]));

        let mut extended = vec![0, 0, 0, 1];
        extended.extend_from_slice(b"pclr");
        extended.extend_from_slice(&1000u64.to_be_bytes());
        extended.extend_from_slice(&[5, 6]);
        assert_eq!(super::find_box(&extended, b"pclr"), Some(&[5u8, 6][..]));
        assert_eq!(super::find_box(&extended, b"cmap"), None);
    }
}
