//! Page renderer using tiny-skia.
//!
//! This module implements the core PDF rendering logic, converting
//! PDF operators into tiny-skia drawing commands.
#![allow(
    clippy::manual_div_ceil,
    clippy::field_reassign_with_default,
    clippy::collapsible_if,
    clippy::needless_borrow,
    clippy::get_first,
    clippy::if_same_then_else,
    clippy::needless_return_with_question_mark,
    clippy::ptr_arg
)]

// TODO(xberg-io/xberg#1567): 4 cyclomatic-complexity and 108 size/complexity findings
// in this file, currently excluded via the quality-debt baseline in alef.toml. Splitting
// these needs compiler-in-the-loop verification, not a mechanical pass. Delete this
// note and the file's baseline entry together once it goes green. Help wanted.

use crate::content::graphics_state::{GraphicsState, GraphicsStateStack, Matrix};
use crate::content::operators::Operator;
use crate::content::parser::parse_content_stream;
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::{Object, ObjectRef};
use crate::rendering::ext_gstate::{ParsedExtGState, parse_ext_g_state_inner};
use crate::rendering::path_rasterizer::PathRasterizer;
use crate::rendering::resolution::{
    DeviceColor, IccTransformCache, LogicalColor, PaintIntent, PaintKind, PaintSide, ResolutionContext,
    ResolutionPipeline, ResolvedColor,
};
use crate::rendering::sidecar::{self as sidecar_mod, CmykSidecar, page_declares_transparency_or_overprint};
use crate::rendering::text_rasterizer::TextRasterizer;
use crate::rendering::{device_bounds_rasterizable, guarded_fill_path, guarded_mask_fill_path, guarded_stroke_path};

use crate::fonts::FontInfo;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tiny_skia::{Color, PathBuilder, Pixmap, PixmapPaint, Transform};

/// Which path-paint side(s) [`PageRenderer::pipeline_resolve_paint_gs`]
/// should resolve for the current operator.
///
/// Text operators (`Tj` / `TJ` / `'` / `"`) use the sibling
/// [`PageRenderer::pipeline_resolve_text_colors`] instead — it returns
/// `Option<ResolvedColors>` rather than `Option<GraphicsState>` so the
/// text rasteriser's internal `current_gs` clone (the one that advances
/// `text_matrix` per glyph or per `TJ` element) is the only
/// `GraphicsState` allocation on the text path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PipelinePaintKind {
    /// `f`, `F`, `f*` — path-fill only.
    PathFill,
    /// `S` — path-stroke only.
    PathStroke,
    /// `B`, `b`, `B*`, `b*` — fill then stroke (one spliced clone covers
    /// both passes; the fill pass reads `fill_*` fields, the stroke pass
    /// reads `stroke_*` fields).
    PathFillStroke,
    /// `Do` with `/Subtype /Image` and `/ImageMask true` — stencil mask
    /// painted with the current fill colour. Behaviourally identical to
    /// [`PipelinePaintKind::PathFill`] inside the helper (one fill-side
    /// resolve, splice into `fill_color_rgb` / `fill_alpha`), but kept as
    /// a distinct variant so the call site reads as "image-mask intent"
    /// rather than "secretly a path fill" — and so a future wave that
    /// needs image-mask-specific routing (e.g. per-pixel overprint
    /// against an image mask painted with a spot colour) can branch on
    /// this without changing the path-fill arms.
    ImageMask,
}

/// Resolved RGBA colours destined for the text rasteriser, side by side.
///
/// The operator arm picks the colours from
/// [`PageRenderer::pipeline_resolve_text_colors`] and hands them to
/// `render_text` / `render_tj_array`. The rasteriser already clones the
/// `GraphicsState` to advance `text_matrix` per glyph or per `TJ`
/// element, so it splices the overrides into that clone — no
/// operator-arm-side allocation happens on the text path.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ResolvedColors {
    /// Fill RGBA, populated when `gs.render_mode` selects the fill side
    /// (Tr ∈ {0, 2, 4, 6}) and the pipeline produced an RGBA result.
    pub(crate) fill: Option<(f32, f32, f32, f32)>,
    /// Stroke RGBA, populated when `gs.render_mode` selects the stroke
    /// side (Tr ∈ {1, 2, 5, 6}) and the pipeline produced an RGBA
    /// result.
    pub(crate) stroke: Option<(f32, f32, f32, f32)>,
}

impl ResolvedColors {
    /// `true` when neither side carries an override.
    pub(crate) fn is_empty(&self) -> bool {
        self.fill.is_none() && self.stroke.is_none()
    }
}

/// Image output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// Portable Network Graphics
    Png,
    /// Joint Photographic Experts Group
    Jpeg,
    /// Raw premultiplied RGBA8888 pixels, row-major, top-left origin.
    /// `data.len() == width * height * 4`. No encoding overhead; callers
    /// that need straight (un-premultiplied) alpha must convert themselves.
    RawRgba8,
}

/// Options for page rendering.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Resolution in dots per inch (default: 150)
    pub dpi: u32,
    /// Output image format (default: PNG)
    pub format: ImageFormat,
    /// Background color (RGBA, default: white)
    pub background: Option<[f32; 4]>,
    /// Whether to render annotations (default: true)
    pub render_annotations: bool,
    /// JPEG quality (1-100, default: 85)
    pub jpeg_quality: u8,
    /// Optional Content Group (layer) names to exclude from rendering.
    ///
    /// When a BDC operator with tag "OC" references an OCG whose /Name matches
    /// one of these entries, all graphical content within that marked content
    /// scope is suppressed (not painted). Empty means render everything.
    pub excluded_layers: HashSet<String>,
    /// Explicit float scale factor set by `render_page_fit`.
    /// When `Some`, bypasses integer-DPI quantization so fit dimensions are
    /// exact. Not part of the public API; set via `render_page_fit` only.
    pub(crate) scale_override: Option<f32>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            dpi: 150,
            format: ImageFormat::Png,
            background: Some([1.0, 1.0, 1.0, 1.0]),
            render_annotations: true,
            jpeg_quality: 85,
            excluded_layers: HashSet::new(),
            scale_override: None,
        }
    }
}

impl RenderOptions {
    /// Set a transparent background (no background fill).
    pub fn with_transparent_background(mut self) -> Self {
        self.background = None;
        self
    }
}

impl RenderOptions {
    /// Create options with specified DPI.
    pub fn with_dpi(dpi: u32) -> Self {
        Self {
            dpi,
            ..Default::default()
        }
    }

    /// Set format to JPEG with quality (clamped to 1-100).
    pub fn as_jpeg(mut self, quality: u8) -> Self {
        self.format = ImageFormat::Jpeg;
        self.jpeg_quality = quality.clamp(1, 100);
        self
    }

    /// Set format to raw premultiplied RGBA8888 (no encoding overhead).
    pub fn as_raw(mut self) -> Self {
        self.format = ImageFormat::RawRgba8;
        self
    }
}

/// A rendered page image.
pub struct RenderedImage {
    /// Raw image data
    pub data: Vec<u8>,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Format of the image data
    pub format: ImageFormat,
}

impl RenderedImage {
    /// Save the image to a file.
    pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        std::fs::write(path, &self.data).map_err(|e| Error::InvalidPdf(format!("Failed to write image: {}", e)))
    }

    /// Get the image data as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

/// Page renderer that converts PDF pages to raster images.
pub struct PageRenderer {
    options: RenderOptions,
    path_rasterizer: PathRasterizer,
    text_rasterizer: TextRasterizer,
    /// Font cache (name -> FontInfo) for current context
    fonts: HashMap<String, Arc<FontInfo>>,
    /// Color space cache (name -> Object) for current context
    color_spaces: HashMap<String, Object>,
    /// Snapshot of `options.excluded_layers` wrapped in an `Arc` so that every
    /// recursive `execute_operators` call holds a cheap reference instead of
    /// deep-cloning the set per nested Form XObject. Recomputed on the first
    /// access per `render_page` invocation. Stays `None` (no allocation) when
    /// the set is empty — the common case.
    excluded_layers_snapshot: Option<Arc<HashSet<String>>>,
    /// Per-page compiled qcms transform cache. The resolution
    /// pipeline borrows this through `ResolutionContext` so every
    /// CMYK paint operator within a page reuses the same compiled
    /// `Transform` for a given `(profile, intent)` pair. Cleared per
    /// page in `render_page_with_options`; lives across paint
    /// operators within the page.
    pub(crate) icc_transform_cache: IccTransformCache,
    /// Depth counter for the SMask materialisation path. Incremented
    /// on entry to [`Self::apply_smask_after_paint`] and decremented
    /// on exit. When the counter reaches [`MAX_SMASK_DEPTH`] further
    /// SMask materialisation is skipped (the paint is left
    /// unmodulated) so adversarial cyclic `/G` references do not
    /// drive unbounded recursion. ISO 32000-1:2008 does not mandate a
    /// numeric cap; 32 levels is well above any realistic nesting and
    /// keeps the stack usage bounded.
    smask_depth: u32,
    /// Per-page CMYK + spot-ink compositing sidecar. When present,
    /// every opaque CMYK paint mirrors its plate values into the
    /// CMYK lanes so the compose-first and overprint-correction
    /// paths read the backdrop CMYK quadruple directly instead of
    /// inverting the post-ICC RGB (lossy under non-linear OutputIntent
    /// profiles). The CMYK lane layout matches the RGBA pixmap:
    /// 4 bytes per pixel (C, M, Y, K), row-major, width × height —
    /// preserved byte-for-byte from the round-4 shape.
    ///
    /// The sidecar additionally carries one tint plane per discovered
    /// spot ink, sized at page setup from the page's resource tree
    /// (ISO 32000-1:2008 §8.6.6.4 / §8.6.6.5 declarations on
    /// `/Resources/ColorSpace` and nested Form XObjects). The spot
    /// lanes sit ALONGSIDE the CMYK blend space per §11.7.3 — they
    /// are NOT a blend space themselves, since §11.3.4 and §11.6.6
    /// (Table 147) forbid `Separation` and `DeviceN` as blend spaces.
    ///
    /// Lazy allocation: stays `None` for pages without an OutputIntent
    /// CMYK profile and pages whose resources declare no transparency
    /// or overprint trigger. The detection-OFF path is byte-identical
    /// to the pre-sidecar behaviour because the consuming helpers
    /// fall back to additive-clamp inversion when the sidecar is
    /// `None`.
    cmyk_sidecar: Option<CmykSidecar>,
    /// When `true`, allocate the CMYK + spot sidecar on every
    /// transparency-detected page regardless of whether the document
    /// declares a CMYK `OutputIntent`. The separation-renderer's
    /// composite-then-decompose entry point flips this so the spot
    /// lanes and the process plane survive the render even for press
    /// jobs whose `OutputIntent` is missing or non-CMYK. The detection
    /// gate ([`page_declares_transparency_or_overprint`]) is still
    /// honoured; detection-OFF pages never allocate a sidecar.
    pub(crate) force_cmyk_sidecar: bool,
    /// Latch on the H3b silent-K=0 warning: when the document declares
    /// `/OutputIntents` but no usable CMYK profile parses out, the
    /// RGB→CMYK fallback emits K=0 (losing the K plane). The first
    /// fallback hit logs once; subsequent paints stay silent so the
    /// log doesn't spam on a degenerate document. Reset on each
    /// `render_page_with_options` entry.
    k_zero_warning_emitted: bool,
    /// Recursion depth for Type 3 glyph rendering. A Type 3 glyph's
    /// CharProcs stream is executed re-entrantly through
    /// [`Self::execute_operators`]; a glyph that (directly or via a Form
    /// XObject) shows text in the same Type 3 font would otherwise recurse
    /// without bound. Incremented on entry to [`Self::render_type3_glyph`]
    /// and decremented on exit; glyphs at or beyond [`MAX_TYPE3_DEPTH`] are
    /// skipped (their advance width is still applied by the caller).
    type3_depth: u32,
    /// Active Type 3 `d1` fill-colour lock. When `Some`, a `d1` glyph
    /// description is being executed: the glyph is a stencil painted with
    /// this fill colour and every colour-setting operator inside it is
    /// ignored (ISO 32000-1:2008 §9.6.5.2). `None` for `d0` glyphs and all
    /// ordinary content, which paint with their own colour operators.
    type3_fill_lock: Option<(f32, f32, f32)>,
}

/// Maximum SMask materialisation recursion depth. A cyclic
/// `/SMask /G` chain (form XObject whose own ExtGState declares the
/// same `/SMask`) would otherwise drive unbounded recursion. The cap
/// is chosen above any realistic nesting depth so legitimate PDFs are
/// unaffected; adversarial inputs fall through to the no-soft-mask
/// branch once the cap engages.
pub(crate) const MAX_SMASK_DEPTH: u32 = 32;

/// Maximum Type 3 glyph rendering recursion depth. A Type 3 CharProcs
/// stream is executed re-entrantly, so a glyph that shows text in the same
/// Type 3 font (directly or through a nested Form XObject) would recurse
/// without bound. The cap sits well above any realistic nesting; glyphs at
/// or beyond it are skipped while their advance width is still applied.
pub(crate) const MAX_TYPE3_DEPTH: u32 = 8;

/// Outcome of resolving a generic (non-device-family) colour space for the
/// plain fill-colour operator (`Operator::SetFillColor`, i.e. `sc`).
/// `HandledNoChange` distinguishes "the space was recognised but leaves
/// `fill_color_rgb` untouched" (`/Separation` / `/DeviceN`, whose colour the
/// paint-time pipeline resolves) from `Unhandled`, for which the caller
/// still applies the `components[0]`-as-gray fallback. ~keep
enum FillColorSpaceResolution {
    Rgb(f32, f32, f32),
    HandledNoChange,
    Unhandled,
}

/// Outcome of resolving a generic (non-device-family) colour space for the
/// pattern-capable `scn`/`SCN` operators (`SetFillColorN` / `SetStrokeColorN`).
/// `RgbCmyk` additionally carries the CMYK quadruple that
/// `/DeviceN /Process`-attributed paints must populate on the graphics state
/// for the §11.7.4.3 overprint blend (see the call sites). ~keep
enum ScnColorSpaceResolution {
    Rgb(f32, f32, f32),
    RgbCmyk((f32, f32, f32), (f32, f32, f32, f32)),
    HandledNoChange,
    Unhandled,
}

/// Read-only paint context threaded through the post-paint colour-lane
/// modulator helpers (`PageRenderer::apply_paint_modulators` /
/// `apply_sidecar_paint_modulators`). Bundles the parameters shared by every
/// modulator call site into one struct instead of passing them positionally
/// at each of the ten call sites. ~keep
struct PaintModulatorContext<'a> {
    gs: &'a GraphicsState,
    doc: &'a PdfDocument,
    page_num: usize,
    resources: &'a Object,
    base_transform: Transform,
    fill_side: bool,
}

/// Pre-paint snapshots consumed by `PageRenderer::apply_paint_modulators`.
/// This is the cadence shared by the combo path ops (`B`/`b`/`B*`/`b*`),
/// text-showing (`Tj`/`'`/`"`/`TJ`), `Do`, and `sh`. ~keep
struct PaintModulatorSnapshots {
    cmyk_compose: Option<Vec<u8>>,
    overprint: Option<Vec<u8>>,
    spot: Option<Vec<u8>>,
    smask: Option<Vec<u8>>,
    smask_spot: Option<Vec<u8>>,
}

/// Pre-paint snapshots consumed by
/// `PageRenderer::apply_sidecar_paint_modulators`. This is the cadence
/// shared by the plain stroke (`S`) and fill (`f`/`F`) path ops. Unlike
/// `PaintModulatorSnapshots`, it additionally mirrors the CMYK and RGB
/// sidecar planes and always runs the (possibly-empty-snapshot) spot
/// mirror. ~keep
struct SidecarPaintModulatorSnapshots {
    cmyk_compose: Option<Vec<u8>>,
    overprint: Option<Vec<u8>>,
    cmyk_sidecar: Option<Vec<u8>>,
    rgb_sidecar: Option<Vec<u8>>,
    smask: Option<Vec<u8>>,
    smask_spot: Option<Vec<u8>>,
}

/// Colour resolved from an `/ICCBased` colour-space array's stream `/N`
/// component count (ISO 32000-1 §8.6.5.5: N=1 DeviceGray-like, N=3
/// DeviceRGB-like, N=4 DeviceCMYK-like). Returns `None` when the stream
/// can't be resolved, isn't a dictionary, `/N` doesn't match one of the
/// three cases, or `components` is too short for that case — matching the
/// original inline `match n { .. _ => {} }` no-op. Shared by all four
/// `sc`/`SC`/`scn`/`SCN` colour-space resolvers below. ~keep
fn resolve_iccbased_stream_color(
    doc: &PdfDocument,
    iccbased_stream_ref: &Object,
    components: &[f32],
) -> Option<(f32, f32, f32)> {
    let dict_obj = doc.resolve_object(iccbased_stream_ref).ok()?;
    let dict = dict_obj.as_dict()?;
    let n = dict.get("N").and_then(|o| o.as_integer()).unwrap_or(3);
    match n {
        1 if !components.is_empty() => Some((components[0], components[0], components[0])),
        3 if components.len() >= 3 => Some((components[0], components[1], components[2])),
        4 if components.len() >= 4 => Some(cmyk_to_rgb(components[0], components[1], components[2], components[3])),
        _ => None,
    }
}

/// Generic (non-device-family) colour-space resolution for the plain `sc`
/// fill-colour operator (`Operator::SetFillColor`). ~keep
fn resolve_generic_fill_color(
    resolved_space: Option<&Object>,
    doc: &PdfDocument,
    components: &[f32],
) -> FillColorSpaceResolution {
    let Some(rs) = resolved_space else {
        return FillColorSpaceResolution::Unhandled;
    };
    let Some(arr) = rs.as_array() else {
        return FillColorSpaceResolution::Unhandled;
    };
    let Some(type_name) = arr.first().and_then(|o| o.as_name()) else {
        return FillColorSpaceResolution::Unhandled;
    };
    match type_name {
        "ICCBased" if arr.len() > 1 => match resolve_iccbased_stream_color(doc, &arr[1], components) {
            Some((r, g, b)) => FillColorSpaceResolution::Rgb(r, g, b),
            None => FillColorSpaceResolution::Unhandled,
        },
        "Separation" | "DeviceN" => FillColorSpaceResolution::HandledNoChange,
        "Indexed" if !components.is_empty() => {
            let g = components[0] / 255.0;
            FillColorSpaceResolution::Rgb(g, g, g)
        }
        _ => FillColorSpaceResolution::Unhandled,
    }
}

/// Generic (non-device-family) colour-space resolution for the plain `SC`
/// stroke-colour operator (`Operator::SetStrokeColor`). Unlike the fill-side
/// resolver above, the original inline match here only ever recognised
/// `/ICCBased` — `/Separation`, `/DeviceN` and `/Indexed` fall through to
/// the caller's gray fallback, matching the asymmetry already present in
/// the pre-extraction code. ~keep
fn resolve_generic_stroke_color(
    resolved_space: Option<&Object>,
    doc: &PdfDocument,
    components: &[f32],
) -> Option<(f32, f32, f32)> {
    let rs = resolved_space?;
    let arr = rs.as_array()?;
    let type_name = arr.first().and_then(|o| o.as_name())?;
    if type_name != "ICCBased" || arr.len() <= 1 {
        return None;
    }
    resolve_iccbased_stream_color(doc, &arr[1], components)
}

impl PageRenderer {
    /// Create a new page renderer with the specified options.
    pub fn new(options: RenderOptions) -> Self {
        Self {
            options,
            path_rasterizer: PathRasterizer::new(),
            text_rasterizer: TextRasterizer::new(),
            fonts: HashMap::new(),
            color_spaces: HashMap::new(),
            excluded_layers_snapshot: None,
            icc_transform_cache: IccTransformCache::new(),
            smask_depth: 0,
            cmyk_sidecar: None,
            force_cmyk_sidecar: false,
            k_zero_warning_emitted: false,
            type3_depth: 0,
            type3_fill_lock: None,
        }
    }

    /// Take ownership of the per-page CMYK + spot-ink sidecar produced
    /// by the most recent [`Self::render_page_with_options`] call.
    /// Leaves the renderer's slot empty so a subsequent render starts
    /// fresh.
    ///
    /// Used by the separation entry point
    /// ([`super::separation_renderer::render_separations`]) to harvest
    /// the populated process + spot lanes after a composite render and
    /// decompose them into per-plate output (ISO 32000-1 §10.5 plates,
    /// §11.7.3 spot lanes, §11.7.4.2 BM split).
    pub(crate) fn take_cmyk_sidecar(&mut self) -> Option<CmykSidecar> {
        self.cmyk_sidecar.take()
    }

    /// Number of qcms transform constructions the per-page cache has
    /// observed since the last `render_page_with_options` call. Exposes
    /// renderer internals for assertions and is not part of the
    /// supported surface. Lets the integration suite assert "1000
    /// same-colour CMYK paints built 1 transform" without racing
    /// concurrent tests that might also trigger `Transform::new_srgb_target`
    /// via the global counter.
    #[doc(hidden)]
    pub fn icc_transform_cache_build_count(&self) -> usize {
        self.icc_transform_cache.build_count()
    }

    /// Total `IccTransformCache::get_or_build` calls (hits + misses)
    /// observed since the last `render_page_with_options` call. Exposes
    /// renderer internals for assertions and is not part of the
    /// supported surface. Distinguishes a properly-hoisted per-paint
    /// lookup from a per-pixel regression: the cache returns a cached
    /// `Arc<Transform>` on every hit so `build_count` stays at 1
    /// either way, but the `content_hash` SipHash over the whole
    /// profile blob runs on every call, hit or miss. A correctly
    /// hoisted hot loop therefore yields lookup_count ≈ paint count;
    /// a per-pixel regression yields lookup_count proportional to
    /// painted pixels.
    #[doc(hidden)]
    pub fn icc_transform_cache_lookup_count(&self) -> usize {
        self.icc_transform_cache.lookup_count()
    }

    /// Number of CMYK→CMYK retarget cache misses observed since the
    /// last `render_page_with_options` call. Exposes renderer internals
    /// for assertions and is not part of the supported surface. Pins
    /// the M2 retarget cache: a page with many DeviceN /Process
    /// /ICCBased N=4 paints under one OutputIntent must build the
    /// retarget transform exactly once per unique `(src_profile,
    /// dst_profile, intent)` tuple, not once per paint.
    #[doc(hidden)]
    pub fn icc_transform_cache_cmyk_retarget_build_count(&self) -> usize {
        self.icc_transform_cache.cmyk_retarget_build_count()
    }

    /// Pixmap dimensions of the per-page compositing sidecar, or
    /// `None` when the sidecar was not allocated for the most recent
    /// `render_page_with_options` call (detection-OFF).
    ///
    /// Exposes renderer internals for assertions and is not part of
    /// the supported surface — gates round-1 spot-ink discovery probes
    /// and round-4 CMYK plane shape probes.
    #[doc(hidden)]
    pub fn cmyk_sidecar_dims(&self) -> Option<(u32, u32)> {
        self.cmyk_sidecar.as_ref().map(CmykSidecar::dims)
    }

    /// Read-only view over the sidecar's packed `(C, M, Y, K)` plane.
    /// `None` when the sidecar is not allocated. Exposes renderer
    /// internals for assertions and is not part of the supported
    /// surface.
    #[doc(hidden)]
    pub fn cmyk_sidecar_cmyk_bytes(&self) -> Option<&[u8]> {
        self.cmyk_sidecar.as_ref().map(CmykSidecar::cmyk)
    }

    /// Ordered list of spot ink names the discovery pre-pass surfaced
    /// for the most recent render (sorted ASCII, deduped, `/All` and
    /// `/None` filtered out per ISO 32000-1 §8.6.6.4). `None` when
    /// the sidecar is not allocated. Exposes renderer internals for
    /// assertions and is not part of the supported surface.
    #[doc(hidden)]
    pub fn cmyk_sidecar_spot_names(&self) -> Option<&[String]> {
        self.cmyk_sidecar.as_ref().map(CmykSidecar::spot_names)
    }

    /// Read-only view over the tint plane for spot ink `index`,
    /// or `None` when the sidecar is not allocated or `index` is
    /// beyond the discovered spot set. Exposes renderer internals for
    /// assertions and is not part of the supported surface.
    #[doc(hidden)]
    pub fn cmyk_sidecar_spot_plane(&self, index: usize) -> Option<&[u8]> {
        self.cmyk_sidecar.as_ref().and_then(|s| s.spot_plane(index))
    }

    /// Render a page to a raster image.
    pub fn render_page(&mut self, doc: &PdfDocument, page_num: usize) -> Result<RenderedImage> {
        self.render_page_with_options(page_num, doc)
    }

    /// Render a page with specific options.
    pub fn render_page_with_options(&mut self, page_num: usize, doc: &PdfDocument) -> Result<RenderedImage> {
        self.fonts.clear();
        self.color_spaces.clear();
        // The qcms transform cache is per-page: dropping every entry
        // keeps memory bounded when the renderer is reused across many
        // pages with distinct /OutputIntents profiles, while still
        // amortising transform construction across paints within a
        // single page. ~keep
        self.icc_transform_cache.clear();
        // Reset the H3b silent-K=0 warning latch so a new page's first
        // RGB-to-CMYK fallback under a declared-but-unparseable
        // /OutputIntents profile logs once on the new page (instead
        // of staying suppressed across all subsequent renders on this
        // long-lived PageRenderer). ~keep
        self.k_zero_warning_emitted = false;
        // Same page scope for the glyph-drop warning latch: a new page warns
        // again per broken font, so bulk ingestion never goes silent. ~keep
        self.text_rasterizer.reset_page_warnings();

        // Refresh the excluded-layers snapshot once per page. The effective
        // set combines (a) the PDF's default-off OCGs per /OCProperties/D
        // (BaseState, /ON, /OFF) — ISO 32000-1 §8.11.4 — with (b) the caller's
        // explicit excluded_layers. This makes the renderer respect the PDF's
        // default visibility configuration, matching a viewer's initial state. ~keep
        let default_off = crate::optional_content::compute_default_off_ocgs(doc);
        let effective: HashSet<String> = default_off
            .into_iter()
            .chain(self.options.excluded_layers.iter().cloned())
            .collect();
        self.excluded_layers_snapshot = if effective.is_empty() {
            None
        } else {
            Some(Arc::new(effective))
        };

        let page_info = doc.get_page_info(page_num)?;
        let media_box = page_info.media_box;

        // Calculate output dimensions, accounting for page rotation
        // `%` is a remainder and preserves sign, so a legal negative /Rotate (e.g. -90,
        // equivalent to 270 per ISO 32000-1 s7.7.3.3 Table 30) matched neither 90 nor
        // 270 below and the page rendered unrotated. rem_euclid normalizes to 0..359,
        // matching get_page_rotation's own `((raw % 360) + 360) % 360` convention. ~keep
        let rotation = page_info.rotation.rem_euclid(360);
        let (page_w, page_h) = if rotation == 90 || rotation == 270 {
            (media_box.height, media_box.width)
        } else {
            (media_box.width, media_box.height)
        };
        let scale = self.options.scale_override.unwrap_or(self.options.dpi as f32 / 72.0);
        let (width, height) = if self.options.scale_override.is_some() {
            // Float scale path: round to avoid off-by-one from exact fractional pixels.
            // Clamp to 1 so extreme aspect ratios never produce a 0-sized pixmap. ~keep
            (
                ((page_w * scale).round() as u32).max(1),
                ((page_h * scale).round() as u32).max(1),
            )
        } else {
            ((page_w * scale).ceil() as u32, (page_h * scale).ceil() as u32)
        };

        let mut pixmap =
            Pixmap::new(width, height).ok_or_else(|| Error::InvalidPdf("Failed to create pixmap".to_string()))?;

        if let Some(bg) = self.options.background {
            let [r, g, b, a] = bg;
            pixmap.fill(Color::from_rgba(r, g, b, a).unwrap_or(Color::WHITE));
        }

        // Create base transform: PDF coordinates to pixel coordinates
        // PDF origin is bottom-left; we flip Y and apply page rotation.
        // Per PDF spec §8.3.2.3, /Rotate specifies clockwise rotation.
        // The approach: first map PDF coords to an unrotated pixel space,
        // then rotate the entire result. ~keep
        let transform = match rotation {
            90 => {
                // 90° CW rotation: portrait PDF → landscape display
                // PDF y-up (x,y) → screen y-down: screen_x = y*s, screen_y = x*s ~keep
                Transform::from_translate(-media_box.x, -media_box.y)
                    .post_concat(Transform::from_row(0.0, scale, scale, 0.0, 0.0, 0.0))
            }
            180 => Transform::from_translate(-media_box.x, -media_box.y)
                .post_scale(-scale, scale)
                .post_translate(media_box.width * scale, 0.0),
            270 => {
                // 270° CW: PDF (x,y) → screen_x = (H - y)*s, screen_y = (W - x)*s.
                //
                // The `y` row used to be `screen_y = x*s`, which put the page's
                // TOP-LEFT corner at the top-left of the raster; under a 270° turn
                // it belongs at the BOTTOM-left. That is not merely a wrong angle -
                // it is a MIRROR: the old matrix has a POSITIVE determinant, while
                // 0°/90°/180° all have a negative one (they carry the PDF y-up →
                // raster y-down flip). Text came out reversed. ~keep
                Transform::from_translate(-media_box.x, -media_box.y).post_concat(Transform::from_row(
                    0.0,
                    -scale,
                    -scale,
                    0.0,
                    media_box.height * scale,
                    media_box.width * scale,
                ))
            }
            _ => Transform::from_translate(-media_box.x, -media_box.y)
                .post_scale(scale, -scale)
                .post_translate(0.0, page_h * scale),
        };

        let resources = doc.get_page_resources(page_num)?;

        self.load_resources(doc, &resources)?;

        // Decide whether to allocate the CMYK + spot-ink sidecar. The
        // CMYK plane costs `4·width·height` bytes per page and mirrors
        // every opaque CMYK paint so the compose-first and overprint
        // correction helpers can read the backdrop CMYK quadruple
        // directly instead of inverting the post-ICC RGB. Each spot
        // ink adds one extra plane of `width·height` bytes.
        //
        // Allocation is gated on (a) the OutputIntent declares a
        // CMYK profile — without one, the process-side helpers would
        // not fire at all — and (b) the page resources declare
        // ExtGState entries that could drive transparency or
        // overprint, or the page's Form XObjects declare /Group dicts
        // or /SMask entries (which trigger transparency-group
        // compositing). When either condition is false the sidecar
        // stays `None` and the per-paint mirror is a no-op; the
        // detection-OFF path is byte-identical to the pre-sidecar
        // behaviour.
        //
        // The spot ink set is discovered with the same walker the
        // separation renderer's per-plate path uses (§8.6.6.4 /
        // §8.6.6.5: `/Separation` and non-process `/DeviceN`
        // colorants, with `/All` and `/None` filtered out). Sizing
        // the sidecar's spot lanes up front means subsequent paint
        // operators can blind-index by ink without re-walking the
        // resource tree. ~keep
        self.cmyk_sidecar = None;
        // ISO 32000-1 §11.7.3 + §11.7.4.2 + §10.5: the sidecar carries
        // the composite-then-separate workflow's process + spot lanes.
        // The default page-renderer path gates on the OutputIntent CMYK
        // profile because the compose-first / overprint-correction
        // helpers only fire when there is a non-trivial CMYK→RGB
        // transform to compose under. The separation entry point flips
        // `force_cmyk_sidecar` so the sidecar lives on every
        // detection-ON page regardless of OutputIntent — the per-plate
        // output is meaningful even without a press ICC profile (it is
        // the raw subtractive tint at every pixel). ~keep
        let needs_cmyk_sidecar = (self.force_cmyk_sidecar || doc.output_intent_cmyk_profile().is_some())
            && page_declares_transparency_or_overprint(doc, &resources);
        if needs_cmyk_sidecar {
            let spot_names = sidecar_mod::discover_page_spot_inks(doc, page_num);
            self.cmyk_sidecar = Some(CmykSidecar::new(width, height, spot_names));
        }

        let content_data = doc.get_page_content_data(page_num)?;

        let operators = match parse_content_stream(&content_data) {
            Ok(ops) => ops,
            Err(e) => {
                return Err(e);
            }
        };

        self.execute_operators(&mut pixmap, transform, &operators, doc, page_num, &resources)?;

        if self.options.render_annotations {
            self.render_annotations(&mut pixmap, transform, doc, page_num)?;
        }

        let data = match self.options.format {
            ImageFormat::Png => encode_png(&pixmap)?,
            ImageFormat::Jpeg => self.encode_jpeg(&pixmap)?,
            ImageFormat::RawRgba8 => pixmap.data().to_vec(),
        };

        Ok(RenderedImage {
            data,
            width,
            height,
            format: self.options.format,
        })
    }

    /// Load resources (fonts, color spaces) into local cache.
    fn load_resources(&mut self, doc: &PdfDocument, resources: &Object) -> Result<()> {
        if let Object::Dictionary(res_dict) = resources {
            tracing::debug!("Loading resources, keys: {:?}", res_dict.keys());
            if let Some(font_obj) = res_dict.get("Font") {
                tracing::debug!("Found Font resource");
                let font_dict_obj = doc.resolve_object(font_obj)?;
                if let Some(font_dict) = font_dict_obj.as_dict() {
                    for (name, f_obj) in font_dict {
                        match doc.get_or_load_font_for_rendering(f_obj) {
                            Ok(info) => {
                                tracing::debug!(
                                    "Resolved font '{}': subtype={}, encoding={:?}, has_to_unicode={}, has_embedded={}",
                                    info.base_font,
                                    info.subtype,
                                    info.encoding,
                                    info.to_unicode.is_some(),
                                    info.embedded_font_data.is_some()
                                );
                                self.fonts.insert(name.clone(), info);
                            }
                            Err(error) => {
                                tracing::warn!(
                                    target: crate::LOG_TARGET_ROOT,
                                    operation = "load_render_font",
                                    error_code = error.telemetry_code(),
                                    error_offset = ?error.telemetry_offset(),
                                    "rendering text with fallback font data"
                                );
                            }
                        }
                    }
                }
            }

            if let Some(cs_obj) = res_dict.get("ColorSpace") {
                tracing::debug!("Found ColorSpace resource");
                let cs_dict_obj = doc.resolve_object(cs_obj)?;
                if let Some(cs_dict) = cs_dict_obj.as_dict() {
                    for (name, o) in cs_dict {
                        if let Ok(resolved_cs) = doc.resolve_object(o) {
                            tracing::debug!("Resolved color space '{}': {:?}", name, resolved_cs);
                            self.color_spaces.insert(name.clone(), resolved_cs);
                        }
                    }
                }
            }

            if let Some(xobj_obj) = res_dict.get("XObject") {
                let xobj_dict_obj = doc.resolve_object(xobj_obj)?;
                if let Some(xobj_dict) = xobj_dict_obj.as_dict() {
                    tracing::debug!("XObject dict keys: {:?}", xobj_dict.keys());
                }
            }
        }

        // Share TrueType CMaps between matching fonts (essential for CID fonts with missing ToUnicode)
        // ~keep
        self.share_truetype_cmaps();
        Ok(())
    }

    /// Share TrueType cmap tables between fonts with matching base font names.
    ///
    /// Donor selection lives in `fonts::unicode_decode::best_truetype_cmaps`,
    /// shared with the extraction path: best coverage wins, deterministic
    /// tie-break, strict subset-prefix stripping. Only the write loop is
    /// local, because this table stores fonts behind a `OnceLock` cmap slot.
    fn share_truetype_cmaps(&mut self) {
        let best_cmaps = crate::fonts::unicode_decode::best_truetype_cmaps(self.fonts.values().map(|f| &**f));
        if best_cmaps.is_empty() {
            return;
        }

        for font in self.fonts.values() {
            if font.subtype == "Type0" && font.truetype_cmap().is_none() {
                let base_name = crate::fonts::unicode_decode::strip_subset_prefix(&font.base_font);
                if let Some((shared_cmap, _)) = best_cmaps.get(base_name) {
                    font.truetype_cmap.set(Some(shared_cmap.clone())).ok();
                }
            }
        }
    }

    /// Generic (non-device-family) colour-space resolution shared by the
    /// pattern-capable `scn`/`SCN` operators (`SetFillColorN` /
    /// `SetStrokeColorN`). The two call sites are identical apart from which
    /// `GraphicsState` fields they write, so the resolution logic lives here
    /// once. ~keep
    fn resolve_scn_color(
        &self,
        resolved_space: Option<&Object>,
        doc: &PdfDocument,
        components: &[f32],
        rendering_intent: &str,
    ) -> ScnColorSpaceResolution {
        let Some(rs) = resolved_space else {
            return ScnColorSpaceResolution::Unhandled;
        };
        let Some(arr) = rs.as_array() else {
            return ScnColorSpaceResolution::Unhandled;
        };
        let Some(type_name) = arr.first().and_then(|o| o.as_name()) else {
            return ScnColorSpaceResolution::Unhandled;
        };
        match type_name {
            "ICCBased" if arr.len() > 1 => match resolve_iccbased_stream_color(doc, &arr[1], components) {
                Some((r, g, b)) => ScnColorSpaceResolution::Rgb(r, g, b),
                None => ScnColorSpaceResolution::Unhandled,
            },
            "Separation" | "DeviceN" => {
                // §11.7.4.3 CompatibleOverprint reads `gs.fill_color_cmyk` /
                // `gs.stroke_color_cmyk` (when populated) to recover the
                // source CMYK for the `B(c_b, c_s)` blend function. A
                // `/DeviceN` paint that declares `/Process` attribution
                // (§8.6.6.5) carries process colorants directly in its
                // source tints, so that identity must be recovered here. ~keep
                if type_name == "DeviceN" {
                    let intent = crate::color::RenderingIntent::from_pdf_name(rendering_intent);
                    if let Some(cmyk) = crate::rendering::sidecar::extract_process_paint_cmyk(
                        rs,
                        components,
                        doc,
                        intent,
                        Some(&self.icc_transform_cache),
                    ) {
                        let rgb = cmyk_to_rgb(cmyk.0, cmyk.1, cmyk.2, cmyk.3);
                        return ScnColorSpaceResolution::RgbCmyk(rgb, cmyk);
                    }
                }
                ScnColorSpaceResolution::HandledNoChange
            }
            "Indexed" => ScnColorSpaceResolution::HandledNoChange,
            _ => ScnColorSpaceResolution::Unhandled,
        }
    }

    /// Applies the `cmyk_compose → overprint → spot-mirror → SMask`
    /// post-paint modulator cadence shared by the combo path ops, text
    /// showing, `Do`, and `sh`. Extracted verbatim from the identical
    /// four-`if let` sequence that was duplicated at every one of those call
    /// sites — each modulator still only runs when its snapshot is `Some`,
    /// in the same order as before. ~keep
    fn apply_paint_modulators(
        &mut self,
        pixmap: &mut Pixmap,
        snapshots: PaintModulatorSnapshots,
        coverage: Option<&[u8]>,
        ctx: PaintModulatorContext<'_>,
    ) -> Result<()> {
        if let Some(snap) = snapshots.cmyk_compose {
            self.apply_cmyk_compose_after_paint(pixmap, &snap, ctx.gs, ctx.doc, ctx.fill_side);
        }
        if let Some(snap) = snapshots.overprint {
            self.apply_overprint_after_paint(pixmap, &snap, ctx.gs, ctx.doc, ctx.fill_side);
        }
        if let Some(snap) = snapshots.spot {
            self.mirror_spot_paint_into_sidecar_with_coverage(pixmap, &snap, coverage, ctx.gs, ctx.fill_side);
        }
        if let Some(snap) = snapshots.smask {
            self.apply_smask_after_paint(
                pixmap,
                &snap,
                snapshots.smask_spot.as_deref(),
                ctx.gs,
                ctx.doc,
                ctx.page_num,
                ctx.resources,
                ctx.base_transform,
            )?;
        }
        Ok(())
    }

    /// Applies the sidecar-mirroring post-paint modulator cadence shared by
    /// the plain stroke (`S`) and fill (`f`/`F`) path ops. Extracted
    /// verbatim from the identical five-block sequence that was duplicated
    /// at both call sites. ~keep
    fn apply_sidecar_paint_modulators(
        &mut self,
        pixmap: &mut Pixmap,
        snapshots: SidecarPaintModulatorSnapshots,
        coverage: Option<&[u8]>,
        ctx: PaintModulatorContext<'_>,
    ) -> Result<()> {
        if let Some(snap) = snapshots.cmyk_compose {
            self.apply_cmyk_compose_after_paint_with_coverage(pixmap, &snap, coverage, ctx.gs, ctx.doc, ctx.fill_side);
        }
        if let Some(snap) = snapshots.overprint {
            self.apply_overprint_after_paint_with_coverage(pixmap, &snap, coverage, ctx.gs, ctx.doc, ctx.fill_side);
        }
        if let Some(snap) = snapshots.cmyk_sidecar {
            self.mirror_cmyk_paint_into_sidecar_with_coverage(pixmap, &snap, coverage, ctx.gs, ctx.doc, ctx.fill_side);
        }
        if let Some(snap) = snapshots.rgb_sidecar {
            self.mirror_rgb_paint_into_sidecar_with_coverage(pixmap, &snap, coverage, ctx.gs, ctx.doc, ctx.fill_side);
        }
        self.mirror_spot_paint_into_sidecar_with_coverage(pixmap, &[], coverage, ctx.gs, ctx.fill_side);
        if let Some(snap) = snapshots.smask {
            self.apply_smask_after_paint(
                pixmap,
                &snap,
                snapshots.smask_spot.as_deref(),
                ctx.gs,
                ctx.doc,
                ctx.page_num,
                ctx.resources,
                ctx.base_transform,
            )?;
        }
        Ok(())
    }

    /// Execute PDF operators to render content.
    ///
    /// OCG layer exclusion is sourced from `self.options.excluded_layers`;
    /// BDC/EMC operators referencing matching layers cause graphical operators
    /// inside that scope to be silently dropped.
    fn execute_operators(
        &mut self,
        pixmap: &mut Pixmap,
        base_transform: Transform,
        operators: &[Operator],
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
    ) -> Result<()> {
        // Per-render snapshot lives on `self.excluded_layers_snapshot` (filled
        // by `render_page_with_options`). Recursive calls into this function
        // reuse the same `Arc` without any allocation. We snapshot it as a
        // local `Arc::clone` (cheap pointer copy) so the operator loop below
        // can hold a `&HashSet` reference while still calling `&mut self`
        // methods through the inner match arms. ~keep
        let snapshot: Option<Arc<HashSet<String>>> = self.excluded_layers_snapshot.clone();
        static EMPTY: std::sync::OnceLock<HashSet<String>> = std::sync::OnceLock::new();
        let empty_ref: &HashSet<String> = EMPTY.get_or_init(HashSet::new);
        let excluded_layers: &HashSet<String> = snapshot.as_deref().unwrap_or(empty_ref);
        let mut gs_stack = GraphicsStateStack::new();

        {
            let gs = gs_stack.current_mut();
            gs.fill_color_space = "DeviceGray".to_string();
            gs.stroke_color_space = "DeviceGray".to_string();
            gs.fill_color_rgb = (0.0, 0.0, 0.0);
            gs.stroke_color_rgb = (0.0, 0.0, 0.0);
        }

        // Type 3 `d1` glyph description: seed the fill colour with the locked
        // current colour so the stencil paints in it. Set the same fields the
        // `rg` operator would (RGB, colour space, and components) so the
        // colour-resolution pipeline reproduces it exactly. Colour operators
        // inside the glyph are ignored below (ISO 32000-1:2008 §9.6.5.2). ~keep
        if let Some((r, g, b)) = self.type3_fill_lock {
            let gs = gs_stack.current_mut();
            gs.fill_color_rgb = (r, g, b);
            gs.fill_color_space = "DeviceRGB".to_string();
            gs.fill_color_components.clear();
            gs.fill_color_components.extend_from_slice(&[r, g, b]);
        }

        let mut in_text_object = false;
        let mut current_path = PathBuilder::new();
        let mut pending_clip: Option<(tiny_skia::Path, tiny_skia::FillRule)> = None;
        let mut clip_stack: Vec<Option<tiny_skia::Mask>> = vec![None];
        // ~keep

        // WS1.5b — text-clip accumulator (ISO 32000-1 §9.3.6 / Table 106,
        // `Tr` modes 4–7). Text render modes ≥4 add the union of their glyph
        // outlines to a "text clip path" that is intersected into the current
        // clip at `ET`. We accumulate that union as opaque glyph coverage in a
        // page-sized scratch pixmap's alpha channel (unioned across shows via
        // SourceOver), then convert it to a `Mask` at `ET`. This stays
        // allocation-free for the normal-text hot path: the scratch is created
        // lazily only when a mode-≥4 show actually fires, and modes 0–3 never
        // touch it. Complexity is inherently capped — coverage folds into a
        // fixed-size buffer, so no unbounded path growth is possible. ~keep
        let mut text_clip_accum: Option<Pixmap> = None;

        // OCG layer exclusion tracking.
        // `excluded_layer_depth` counts how many nested BDC/OC scopes we are
        // inside that match an excluded layer. >0 means content is suppressed.
        // `marked_content_depth` tracks total BDC/BMC nesting so EMC correctly
        // decrements only when it pops an excluded-layer entry. ~keep
        let mut excluded_layer_depth: u32 = 0;
        let mut marked_content_is_excluded: Vec<bool> = Vec::new();

        // Per-`execute_operators` resolved ExtGState resource dictionary. PDF
        // content streams often invoke `gs<N>` thousands of times per page
        // (vector scatter / contour plots emit one `gs` per marker — a
        // dense plot page can have ~10 000 such calls per Form XObject with
        // ~10 000 unique names because each marker carries its own alpha).
        // Without this hoist, every `gs` op called `doc.resolve_object(...)`
        // which deep-clones the *entire* per-form ExtGState dict (10 000+
        // entries) — that single clone dominated render time. Resolving the
        // resource dict once at the top of the operator loop and keeping a
        // borrow into it collapses the per-`gs` work to a small `get` +
        // resolve of just the inner state dict. ~keep
        let ext_g_state_resolved: Option<Object> = match resources {
            Object::Dictionary(rd) => rd.get("ExtGState").and_then(|o| doc.resolve_object(o).ok()),
            _ => None,
        };
        let ext_g_states: Option<&std::collections::HashMap<String, Object>> =
            ext_g_state_resolved.as_ref().and_then(|o| o.as_dict());
        // Cache parsed state per `dict_name` so the inner-dict resolve happens
        // at most once per unique name in scope. ~keep
        let mut ext_g_state_cache: std::collections::HashMap<String, ParsedExtGState> =
            std::collections::HashMap::new();
        for op in operators {
            // While a Type 3 `d1` glyph stencil is being painted, colour-
            // setting operators are ignored so the glyph keeps the current
            // fill colour (ISO 32000-1:2008 §9.6.5.2). ~keep
            if self.type3_fill_lock.is_some() && op.is_color_setting() {
                continue;
            }
            match op {
                Operator::SaveState => {
                    gs_stack.save();
                    // Clone current clip for the new graphics state level
                    // This allows the current level to modify its clip without affecting parents
                    // ~keep
                    let current_clip = clip_stack.last().cloned().flatten();
                    clip_stack.push(current_clip);
                    tracing::trace!(
                        "q (SaveState), depth={}, clip_stack depth={}",
                        gs_stack.depth(),
                        clip_stack.len()
                    );
                }
                Operator::RestoreState => {
                    gs_stack.restore();
                    if clip_stack.len() > 1 {
                        clip_stack.pop();
                    }
                    tracing::trace!(
                        "Q (RestoreState), depth={}, clip_stack depth={}",
                        gs_stack.depth(),
                        clip_stack.len()
                    );
                }
                Operator::Cm { a, b, c, d, e, f } => {
                    let matrix = Matrix {
                        a: *a,
                        b: *b,
                        c: *c,
                        d: *d,
                        e: *e,
                        f: *f,
                    };
                    let current = gs_stack.current_mut();
                    // PDF spec ISO 32000-1:2008 §8.3.4: cm concatenates as M_cm × CTM ~keep
                    current.ctm = matrix.multiply(&current.ctm);
                    tracing::trace!(
                        "cm: [{}, {}, {}, {}, {}, {}], CTM now: {:?}",
                        a,
                        b,
                        c,
                        d,
                        e,
                        f,
                        current.ctm
                    );
                }

                Operator::SetFillRgb { r, g, b } => {
                    let gs = gs_stack.current_mut();
                    gs.fill_color_rgb = (*r, *g, *b);
                    gs.fill_color_space = "DeviceRGB".to_string();
                    gs.fill_color_components.clear();
                    gs.fill_color_components.extend_from_slice(&[*r, *g, *b]);
                    // Device-family fill paint: per §11.7.3 the source
                    // covers only the process channels, so any spot ink
                    // identity recorded by a prior /Separation or
                    // /DeviceN paint is no longer the active source.
                    // The sidecar's per-paint spot mirror reads this
                    // empty list as "no spot lane writes for this paint". ~keep
                    gs.fill_spot_inks.clear();
                    // ISO 32000-1 §8.6.3: the fill colour and colour
                    // space are coupled — switching to /DeviceRGB
                    // invalidates any prior /DeviceCMYK identity. Failing
                    // to clear `fill_color_cmyk` here means the §11.7.4.3
                    // overprint path would still see the prior paint's
                    // CMYK quadruple as the "current source colour",
                    // producing wrong B(c_b, c_s) = c_s values for the
                    // new RGB paint's region. ~keep
                    gs.fill_color_cmyk = None;
                    tracing::trace!("SetFillRgb: [{}, {}, {}]", r, g, b);
                }
                Operator::SetStrokeRgb { r, g, b } => {
                    let gs = gs_stack.current_mut();
                    gs.stroke_color_rgb = (*r, *g, *b);
                    gs.stroke_color_space = "DeviceRGB".to_string();
                    gs.stroke_color_components.clear();
                    gs.stroke_color_components.extend_from_slice(&[*r, *g, *b]);
                    gs.stroke_spot_inks.clear();
                    gs.stroke_color_cmyk = None;
                    tracing::trace!("SetStrokeRgb: [{}, {}, {}]", r, g, b);
                }
                Operator::SetFillGray { gray } => {
                    let g = *gray;
                    let gs = gs_stack.current_mut();
                    gs.fill_color_rgb = (g, g, g);
                    gs.fill_color_space = "DeviceGray".to_string();
                    gs.fill_color_components.clear();
                    gs.fill_color_components.push(g);
                    gs.fill_spot_inks.clear();
                    gs.fill_color_cmyk = None;
                    tracing::trace!("SetFillGray: {}", g);
                }
                Operator::SetStrokeGray { gray } => {
                    let g = *gray;
                    let gs = gs_stack.current_mut();
                    gs.stroke_color_rgb = (g, g, g);
                    gs.stroke_color_space = "DeviceGray".to_string();
                    gs.stroke_color_components.clear();
                    gs.stroke_color_components.push(g);
                    gs.stroke_spot_inks.clear();
                    gs.stroke_color_cmyk = None;
                    tracing::trace!("SetStrokeGray: {}", g);
                }
                Operator::SetFillCmyk { c, m, y, k } => {
                    let (r, g, b) = cmyk_to_rgb(*c, *m, *y, *k);
                    let gs = gs_stack.current_mut();
                    gs.fill_color_rgb = (r, g, b);
                    gs.fill_color_cmyk = Some((*c, *m, *y, *k));
                    gs.fill_color_space = "DeviceCMYK".to_string();
                    gs.fill_color_components.clear();
                    gs.fill_color_components.extend_from_slice(&[*c, *m, *y, *k]);
                    gs.fill_spot_inks.clear();
                    tracing::trace!("SetFillCmyk: [{}, {}, {}, {}] -> {:?}", c, m, y, k, (r, g, b));
                }
                Operator::SetStrokeCmyk { c, m, y, k } => {
                    let (r, g, b) = cmyk_to_rgb(*c, *m, *y, *k);
                    let gs = gs_stack.current_mut();
                    gs.stroke_color_rgb = (r, g, b);
                    gs.stroke_color_cmyk = Some((*c, *m, *y, *k));
                    gs.stroke_color_space = "DeviceCMYK".to_string();
                    gs.stroke_color_components.clear();
                    gs.stroke_color_components.extend_from_slice(&[*c, *m, *y, *k]);
                    gs.stroke_spot_inks.clear();
                    tracing::trace!("SetStrokeCmyk: [{}, {}, {}, {}] -> {:?}", c, m, y, k, (r, g, b));
                }

                Operator::SetFillColorSpace { name } => {
                    // ISO 32000-1 §8.6.8: the `cs` operator shall also
                    // set the current colour to its initial value, which
                    // depends on the colour space. For Separation /
                    // DeviceN the initial tint is 1.0 per colorant
                    // (§8.6.6.4 / §8.6.6.5); for DeviceCMYK the initial
                    // colour is (0, 0, 0, 1); device-family RGB / Gray
                    // start at all-zeros. Failing to reset the colour
                    // here means a paint after `cs /CS_B` without an
                    // intervening `scn` would carry the prior space's
                    // identity and tint, including its spot ink list —
                    // round 2 QA pinned that the spot mirror would then
                    // write the prior /CS_A's spot lane. ~keep
                    let resolved = self.color_spaces.get(name).cloned();
                    // §10.7.3: the §8.6.8 initial-colour evaluation runs an
                    // ICC retarget for DeviceN /Process /ICCBased; thread
                    // the live gs intent through so a prior `/Perceptual ri`
                    // / ExtGState /RI propagates into the retarget tag pick. ~keep
                    let intent_for_initial =
                        crate::color::RenderingIntent::from_pdf_name(&gs_stack.current().rendering_intent);
                    let initial = sidecar_mod::initial_colour_for_space(
                        name,
                        resolved.as_ref(),
                        doc,
                        intent_for_initial,
                        Some(&self.icc_transform_cache),
                    );
                    let gs = gs_stack.current_mut();
                    gs.fill_color_space = name.clone();
                    gs.fill_color_rgb = initial.rgb;
                    gs.fill_color_cmyk = initial.cmyk;
                    gs.fill_color_components.clear();
                    gs.fill_color_components.extend_from_slice(&initial.components);
                    gs.fill_spot_inks = initial.spot_inks;
                    // Selecting a colour space clears any previously selected
                    // fill pattern; a fresh scn must re-name it (§8.7.3). ~keep
                    gs.fill_pattern_name = None;
                    tracing::trace!("SetFillColorSpace: {}", name);
                }
                Operator::SetStrokeColorSpace { name } => {
                    let resolved = self.color_spaces.get(name).cloned();
                    let intent_for_initial =
                        crate::color::RenderingIntent::from_pdf_name(&gs_stack.current().rendering_intent);
                    let initial = sidecar_mod::initial_colour_for_space(
                        name,
                        resolved.as_ref(),
                        doc,
                        intent_for_initial,
                        Some(&self.icc_transform_cache),
                    );
                    let gs = gs_stack.current_mut();
                    gs.stroke_color_space = name.clone();
                    gs.stroke_color_rgb = initial.rgb;
                    gs.stroke_color_cmyk = initial.cmyk;
                    gs.stroke_color_components.clear();
                    gs.stroke_color_components.extend_from_slice(&initial.components);
                    gs.stroke_spot_inks = initial.spot_inks;
                }
                Operator::SetFillColor { components } => {
                    let gs = gs_stack.current_mut();
                    let space_name = gs.fill_color_space.clone();
                    let resolved_space = self.color_spaces.get(&space_name);
                    gs.fill_color_components.clear();
                    gs.fill_color_components.extend_from_slice(components);
                    // ISO 32000-1 §8.6.3 + §11.7.4.3: `sc` mutates the
                    // current fill colour for the active colour space.
                    // Clear any stale CMYK identity left over from a
                    // prior DeviceCMYK paint; the DeviceCMYK arm below
                    // refills it. Without this clear, a SetFillColor on
                    // a non-CMYK space leaves the prior CMYK quadruple
                    // visible to the §11.7.4.3 overprint path and
                    // corrupts the per-channel B(c_b, c_s) result. ~keep
                    gs.fill_color_cmyk = None;

                    match space_name.as_str() {
                        "DeviceGray" | "G" if !components.is_empty() => {
                            let g = components[0];
                            gs.fill_color_rgb = (g, g, g);
                        }
                        "DeviceRGB" | "RGB" if components.len() >= 3 => {
                            gs.fill_color_rgb = (components[0], components[1], components[2]);
                        }
                        "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                            gs.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                            gs.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                        }
                        // Inline Separation/DeviceN evaluation used to live
                        // here as a partial reimplementation of the colour-
                        // resolver's tint-transform path. Wave 5 promoted
                        // the pipeline to the single source of truth — the
                        // pipeline runs the full Type 2 / 3 / 4 evaluator at
                        // paint time and splices the resulting RGBA via
                        // pipeline_resolve_paint_gs. The dispatcher just
                        // records the components on gs.fill_color_components
                        // above; the pipeline reads those when the paint op
                        // fires. See `resolve_generic_fill_color`. ~keep
                        _ => match resolve_generic_fill_color(resolved_space, doc, components) {
                            FillColorSpaceResolution::Rgb(r, g, b) => gs.fill_color_rgb = (r, g, b),
                            FillColorSpaceResolution::HandledNoChange => {}
                            FillColorSpaceResolution::Unhandled => {
                                if !components.is_empty() {
                                    let g = components[0];
                                    gs.fill_color_rgb = (g, g, g);
                                }
                            }
                        },
                    }
                    // Per ISO 32000-1 §8.6.6.4 / §8.6.6.5: when the fill
                    // colour space is /Separation or /DeviceN, record the
                    // colorant names + tints for the sidecar's per-paint
                    // spot lane mirror. Other spaces clear the slot so a
                    // subsequent paint does not inherit stale spot data
                    // from a prior /Separation set. ~keep
                    gs.fill_spot_inks = resolved_space
                        .map(|rs| crate::rendering::sidecar::extract_paint_spot_inks(rs, components, doc))
                        .unwrap_or_default();
                    tracing::trace!(
                        "SetFillColor: {} {:?} -> {:?}",
                        space_name,
                        components,
                        gs.fill_color_rgb
                    );
                }
                Operator::SetStrokeColor { components } => {
                    let gs = gs_stack.current_mut();
                    let space_name = gs.stroke_color_space.clone();
                    let resolved_space = self.color_spaces.get(&space_name);
                    gs.stroke_color_components.clear();
                    gs.stroke_color_components.extend_from_slice(components);
                    gs.stroke_color_cmyk = None;

                    match space_name.as_str() {
                        "DeviceGray" | "G" if !components.is_empty() => {
                            let g = components[0];
                            gs.stroke_color_rgb = (g, g, g);
                        }
                        "DeviceRGB" | "RGB" if components.len() >= 3 => {
                            gs.stroke_color_rgb = (components[0], components[1], components[2]);
                        }
                        "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                            gs.stroke_color_rgb =
                                cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                            gs.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                        }
                        _ => match resolve_generic_stroke_color(resolved_space, doc, components) {
                            Some((r, g, b)) => gs.stroke_color_rgb = (r, g, b),
                            None => {
                                if !components.is_empty() {
                                    let g = components[0];
                                    gs.stroke_color_rgb = (g, g, g);
                                }
                            }
                        },
                    }
                    gs.stroke_spot_inks = resolved_space
                        .map(|rs| crate::rendering::sidecar::extract_paint_spot_inks(rs, components, doc))
                        .unwrap_or_default();
                    tracing::trace!(
                        "SetStrokeColor: {} {:?} -> {:?}",
                        space_name,
                        components,
                        gs.stroke_color_rgb
                    );
                }
                Operator::SetFillColorN { components, name } => {
                    let gs = gs_stack.current_mut();
                    let space_name = gs.fill_color_space.clone();
                    let resolved_space = self.color_spaces.get(&space_name);
                    gs.fill_color_components.clear();
                    gs.fill_color_components.extend_from_slice(components);
                    gs.fill_color_cmyk = None;
                    // §8.7.3: retain the pattern name for the Fill path when the
                    // active fill space is /Pattern; clear it otherwise so a
                    // later device-colour scn cannot paint a stale pattern. ~keep
                    gs.fill_pattern_name = if space_name == "Pattern" {
                        name.as_ref().map(|n| n.as_str().to_string())
                    } else {
                        None
                    };

                    match space_name.as_str() {
                        "DeviceGray" | "G" if !components.is_empty() => {
                            let g = components[0];
                            gs.fill_color_rgb = (g, g, g);
                        }
                        "DeviceRGB" | "RGB" if components.len() >= 3 => {
                            gs.fill_color_rgb = (components[0], components[1], components[2]);
                        }
                        "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                            gs.fill_color_rgb = cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                            gs.fill_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                        }
                        // Pipeline owns the colour for /Separation /
                        // /DeviceN and /Indexed at paint time — the
                        // dispatcher just records the components above. The
                        // §11.7.4.3 CompatibleOverprint CMYK-identity
                        // recovery for /Process-attributed /DeviceN is
                        // handled inside `resolve_scn_color`. ~keep
                        _ => match self.resolve_scn_color(resolved_space, doc, components, &gs.rendering_intent) {
                            ScnColorSpaceResolution::Rgb(r, g, b) => gs.fill_color_rgb = (r, g, b),
                            ScnColorSpaceResolution::RgbCmyk(rgb, cmyk) => {
                                gs.fill_color_cmyk = Some(cmyk);
                                gs.fill_color_rgb = rgb;
                            }
                            ScnColorSpaceResolution::HandledNoChange => {}
                            ScnColorSpaceResolution::Unhandled => {
                                if !components.is_empty() {
                                    let g = components[0];
                                    gs.fill_color_rgb = (g, g, g);
                                }
                            }
                        },
                    }
                    gs.fill_spot_inks = resolved_space
                        .map(|rs| crate::rendering::sidecar::extract_paint_spot_inks(rs, components, doc))
                        .unwrap_or_default();
                    tracing::trace!(
                        "SetFillColorN: {} {:?} -> {:?}",
                        space_name,
                        components,
                        gs.fill_color_rgb
                    );
                }
                Operator::SetStrokeColorN { components, .. } => {
                    let gs = gs_stack.current_mut();
                    let space_name = gs.stroke_color_space.clone();
                    let resolved_space = self.color_spaces.get(&space_name);
                    gs.stroke_color_components.clear();
                    gs.stroke_color_components.extend_from_slice(components);
                    gs.stroke_color_cmyk = None;
                    match space_name.as_str() {
                        "DeviceGray" | "G" if !components.is_empty() => {
                            let g = components[0];
                            gs.stroke_color_rgb = (g, g, g);
                        }
                        "DeviceRGB" | "RGB" if components.len() >= 3 => {
                            gs.stroke_color_rgb = (components[0], components[1], components[2]);
                        }
                        "DeviceCMYK" | "CMYK" if components.len() >= 4 => {
                            gs.stroke_color_rgb =
                                cmyk_to_rgb(components[0], components[1], components[2], components[3]);
                            gs.stroke_color_cmyk = Some((components[0], components[1], components[2], components[3]));
                        }
                        // Pipeline owns the colour at paint time — see the
                        // matching comment in the SetFillColorN arm above.
                        // The §11.7.4.3 CompatibleOverprint source-CMYK
                        // reconstruction for /Process-attributed /DeviceN
                        // runs the same way as the fill side via
                        // `resolve_scn_color`. ~keep
                        _ => match self.resolve_scn_color(resolved_space, doc, components, &gs.rendering_intent) {
                            ScnColorSpaceResolution::Rgb(r, g, b) => gs.stroke_color_rgb = (r, g, b),
                            ScnColorSpaceResolution::RgbCmyk(rgb, cmyk) => {
                                gs.stroke_color_cmyk = Some(cmyk);
                                gs.stroke_color_rgb = rgb;
                            }
                            ScnColorSpaceResolution::HandledNoChange => {}
                            ScnColorSpaceResolution::Unhandled => {
                                if !components.is_empty() {
                                    let g = components[0];
                                    gs.stroke_color_rgb = (g, g, g);
                                }
                            }
                        },
                    }
                    gs.stroke_spot_inks = resolved_space
                        .map(|rs| crate::rendering::sidecar::extract_paint_spot_inks(rs, components, doc))
                        .unwrap_or_default();
                    tracing::trace!(
                        "SetStrokeColorN: {} {:?} -> {:?}",
                        space_name,
                        components,
                        gs.stroke_color_rgb
                    );
                }

                Operator::SetLineWidth { width } => {
                    gs_stack.current_mut().line_width = *width;
                }
                Operator::SetLineCap { cap_style } => {
                    gs_stack.current_mut().line_cap = *cap_style;
                }
                Operator::SetLineJoin { join_style } => {
                    gs_stack.current_mut().line_join = *join_style;
                }
                Operator::SetMiterLimit { limit } => {
                    gs_stack.current_mut().miter_limit = *limit;
                }
                Operator::SetDash { array, phase } => {
                    gs_stack.current_mut().dash_pattern = (array.clone(), *phase);
                }
                Operator::SetRenderingIntent { intent } => {
                    // ISO 32000-1:2008 §10.7.3 `/RI` operator. Updates
                    // the graphics-state rendering-intent string; the
                    // colour stage reads `gs.rendering_intent` and
                    // dispatches qcms with the matching intent
                    // (`crate::color::RenderingIntent::from_pdf_name`
                    // maps unknown names back to /RelativeColorimetric
                    // per the spec's "unrecognised → relative" rule).
                    // Without this dispatch the parser would update
                    // the operator stream but the gs.rendering_intent
                    // field would stay at its default forever; the
                    // CMYK transform cache would collapse every
                    // intent's paint into a single shared entry. ~keep
                    gs_stack.current_mut().rendering_intent = intent.clone();
                }

                Operator::MoveTo { x, y } => {
                    current_path.move_to(*x, *y);
                }
                Operator::LineTo { x, y } => {
                    current_path.line_to(*x, *y);
                }
                Operator::CurveTo { x1, y1, x2, y2, x3, y3 } => {
                    current_path.cubic_to(*x1, *y1, *x2, *y2, *x3, *y3);
                }
                Operator::CurveToV { x2, y2, x3, y3 } => {
                    if let Some(last) = current_path.last_point() {
                        current_path.cubic_to(last.x, last.y, *x2, *y2, *x3, *y3);
                    }
                }
                Operator::CurveToY { x1, y1, x3, y3 } => {
                    current_path.cubic_to(*x1, *y1, *x3, *y3, *x3, *y3);
                }
                Operator::Rectangle { x, y, width, height } => {
                    // Normalize negative width/height per PDF spec:
                    // re with negative dimensions means the rect extends in the opposite direction
                    // ~keep
                    let (nx, nw) = if *width < 0.0 {
                        (x + width, -width)
                    } else {
                        (*x, *width)
                    };
                    let (ny, nh) = if *height < 0.0 {
                        (y + height, -height)
                    } else {
                        (*y, *height)
                    };
                    if let Some(rect) = tiny_skia::Rect::from_xywh(nx, ny, nw, nh) {
                        current_path.push_rect(rect);
                    }
                }
                Operator::ClosePath => {
                    current_path.close();
                }

                Operator::Stroke | Operator::CloseStroke => {
                    if excluded_layer_depth == 0 {
                        apply_pending_clip(&mut pending_clip, &mut clip_stack, pixmap, base_transform, &gs_stack);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        // ISO 32000-1 §8.5.3.2 Table 60: `s` is exactly `h S`. ~keep
                        if matches!(op, Operator::CloseStroke) {
                            current_path.close();
                        }
                        if let Some(path) = current_path.finish() {
                            let gs_clone = gs_stack.current().clone();
                            // Stroke side mirrors the path-fill routing —
                            // route through the pipeline so Type 4 Separation
                            // strokes resolve correctly. Line width / cap /
                            // join / dash come from the cloned `gs`
                            // unchanged, so the stroke geometry is unaffected
                            // by the colour splice. ~keep
                            let spliced = self.pipeline_resolve_paint_gs(doc, &gs_clone, PipelinePaintKind::PathStroke);
                            let render_gs: &GraphicsState = spliced.as_ref().unwrap_or(&gs_clone);
                            let transform = combine_transforms(base_transform, &gs_clone.ctm);
                            let smask_snap = self.smask_snapshot(pixmap, &gs_clone);
                            let smask_spot_snap = self.smask_spot_snapshot(&gs_clone);
                            let overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, false);
                            let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, false);
                            let cmyk_sidecar_snap = self.cmyk_sidecar_snapshot(pixmap, &gs_clone, false);
                            let rgb_sidecar_snap = self.cmyk_sidecar_snapshot_for_rgb_paint(pixmap, &gs_clone, false);
                            let cmyk_coverage = self.rasterise_stroke_coverage(&path, transform, &gs_clone, clip);
                            self.path_rasterizer
                                .stroke_path_clipped(pixmap, &path, transform, render_gs, clip);
                            self.apply_sidecar_paint_modulators(
                                pixmap,
                                SidecarPaintModulatorSnapshots {
                                    cmyk_compose: cmyk_compose_snap,
                                    overprint: overprint_snap,
                                    cmyk_sidecar: cmyk_sidecar_snap,
                                    rgb_sidecar: rgb_sidecar_snap,
                                    smask: smask_snap,
                                    smask_spot: smask_spot_snap,
                                },
                                cmyk_coverage.as_deref(),
                                PaintModulatorContext {
                                    gs: &gs_clone,
                                    doc,
                                    page_num,
                                    resources,
                                    base_transform,
                                    fill_side: false,
                                },
                            )?;
                        }
                    } else {
                        let _ = current_path.finish();
                    }
                    current_path = PathBuilder::new();
                }
                Operator::Fill => {
                    if excluded_layer_depth == 0 {
                        apply_pending_clip(&mut pending_clip, &mut clip_stack, pixmap, base_transform, &gs_stack);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        if let Some(path) = current_path.finish() {
                            let gs_clone = gs_stack.current().clone();
                            // Resolve the active fill colour through the
                            // pipeline (PostScript Type 4 tint transforms,
                            // ICCBased N=4, etc.) and splice the resulting
                            // RGBA into a transient GraphicsState copy the
                            // rasteriser consumes. ~keep
                            let spliced = self.pipeline_resolve_paint_gs(doc, &gs_clone, PipelinePaintKind::PathFill);
                            let render_gs: &GraphicsState = spliced.as_ref().unwrap_or(&gs_clone);
                            let transform = combine_transforms(base_transform, &gs_clone.ctm);
                            // §8.7.3: a Pattern-space fill routes to the
                            // tiling-pattern rasteriser first. When it paints
                            // the region the solid-colour paint below is
                            // skipped; unsupported/shading patterns return
                            // false and fall through to the solid fallback. ~keep
                            if gs_clone.fill_color_space == "Pattern"
                                && gs_clone.fill_pattern_name.is_some()
                                && self.fill_with_tiling_pattern(
                                    pixmap,
                                    &path,
                                    base_transform,
                                    transform,
                                    tiny_skia::FillRule::Winding,
                                    clip,
                                    &gs_clone,
                                    doc,
                                    page_num,
                                    resources,
                                )?
                            {
                                // Painted by the tiling pattern. ~keep
                            } else {
                                // §11.4.7 + §11.7.4: snapshot before the
                                // paint so the post-paint modulators can
                                // blend the backdrop (snapshot) with the
                                // painted result. ~keep
                                let smask_snap = self.smask_snapshot(pixmap, &gs_clone);
                                let smask_spot_snap = self.smask_spot_snapshot(&gs_clone);
                                let overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, true);
                                let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, true);
                                let cmyk_sidecar_snap = self.cmyk_sidecar_snapshot(pixmap, &gs_clone, true);
                                let rgb_sidecar_snap =
                                    self.cmyk_sidecar_snapshot_for_rgb_paint(pixmap, &gs_clone, true);
                                let cmyk_coverage =
                                    self.rasterise_fill_coverage(&path, transform, tiny_skia::FillRule::Winding, clip);
                                self.path_rasterizer.fill_path_clipped(
                                    pixmap,
                                    &path,
                                    transform,
                                    render_gs,
                                    tiny_skia::FillRule::Winding,
                                    clip,
                                );
                                self.apply_sidecar_paint_modulators(
                                    pixmap,
                                    SidecarPaintModulatorSnapshots {
                                        cmyk_compose: cmyk_compose_snap,
                                        overprint: overprint_snap,
                                        cmyk_sidecar: cmyk_sidecar_snap,
                                        rgb_sidecar: rgb_sidecar_snap,
                                        smask: smask_snap,
                                        smask_spot: smask_spot_snap,
                                    },
                                    cmyk_coverage.as_deref(),
                                    PaintModulatorContext {
                                        gs: &gs_clone,
                                        doc,
                                        page_num,
                                        resources,
                                        base_transform,
                                        fill_side: true,
                                    },
                                )?;
                            }
                        }
                    } else {
                        let _ = current_path.finish();
                    }
                    current_path = PathBuilder::new();
                }
                Operator::FillStroke | Operator::CloseFillStroke | Operator::CloseFillStrokeEvenOdd => {
                    if excluded_layer_depth == 0 {
                        apply_pending_clip(&mut pending_clip, &mut clip_stack, pixmap, base_transform, &gs_stack);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        // ISO 32000-1 §8.5.3.1 Table 60: `b` and `b*` close
                        // the path before fill+stroke. Neither parser
                        // decomposes them, so the dispatcher must perform the
                        // close itself or the final segment of an open subpath
                        // will not be painted by the stroke. The same holds for
                        // `s` via `Operator::CloseStroke`: only the byte-level
                        // parser decomposes it, the object parser does not. ~keep
                        if matches!(op, Operator::CloseFillStroke | Operator::CloseFillStrokeEvenOdd) {
                            current_path.close();
                        }
                        if let Some(path) = current_path.finish() {
                            let gs_clone = gs_stack.current().clone();
                            let transform = combine_transforms(base_transform, &gs_clone.ctm);
                            let fill_rule = if matches!(op, Operator::CloseFillStrokeEvenOdd) {
                                tiny_skia::FillRule::EvenOdd
                            } else {
                                tiny_skia::FillRule::Winding
                            };
                            // Combos resolve fill and stroke independently
                            // through the pipeline (two `PaintIntent`s per
                            // operator). Each side falls back to the
                            // GraphicsState's existing RGBA if its colour
                            // can't be resolved, so a Type 4 Separation on
                            // the fill side and a plain DeviceRGB on the
                            // stroke side route correctly without
                            // entangling the two.
                            //
                            // Single splice for both sides — the rasteriser
                            // reads fill fields for the fill pass and stroke
                            // fields for the stroke pass, so one clone with
                            // both sides written is equivalent to two
                            // single-side clones. ~keep
                            let spliced =
                                self.pipeline_resolve_paint_gs(doc, &gs_clone, PipelinePaintKind::PathFillStroke);
                            let render_gs: &GraphicsState = spliced.as_ref().unwrap_or(&gs_clone);

                            // §8.7.3: Pattern-space fills route to the tiling
                            // rasteriser first; on success the solid fill side
                            // is skipped (the stroke side still runs below). ~keep
                            let fill_by_pattern = gs_clone.fill_color_space == "Pattern"
                                && gs_clone.fill_pattern_name.is_some()
                                && self.fill_with_tiling_pattern(
                                    pixmap,
                                    &path,
                                    base_transform,
                                    transform,
                                    fill_rule,
                                    clip,
                                    &gs_clone,
                                    doc,
                                    page_num,
                                    resources,
                                )?;

                            // Fill side: snapshot before paint, paint,
                            // then run compose-first / overprint / SMask
                            // correctors against the fill-side gs fields.
                            // The §11.7.4 + §11.4.7 + §11.4 rules apply
                            // to combos exactly as they do to plain `f`
                            // — the only difference here is the stroke
                            // pass also lays paint on top, so each side
                            // gets its own snapshot/apply cycle. ~keep
                            if !fill_by_pattern {
                                let fill_smask_snap = self.smask_snapshot(pixmap, &gs_clone);
                                let fill_smask_spot_snap = self.smask_spot_snapshot(&gs_clone);
                                let fill_overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, true);
                                let fill_cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, true);
                                let fill_spot_snap = self.spot_paint_snapshot(pixmap, &gs_clone, true);
                                // §11.7.3 + §11.3.3 require per-pixel
                                // coverage on every lane. The path-Fill
                                // helper uses `rasterise_fill_coverage`;
                                // the combo arm uses the same call so AA
                                // edges receive fractional coverage and an
                                // alternate-CS RGB collision with backdrop
                                // does not mask the paint from the spot
                                // mirror's diff branch. ~keep
                                let fill_cmyk_coverage =
                                    self.rasterise_fill_coverage(&path, transform, fill_rule, clip);
                                self.path_rasterizer
                                    .fill_path_clipped(pixmap, &path, transform, render_gs, fill_rule, clip);
                                self.apply_paint_modulators(
                                    pixmap,
                                    PaintModulatorSnapshots {
                                        cmyk_compose: fill_cmyk_compose_snap,
                                        overprint: fill_overprint_snap,
                                        spot: fill_spot_snap,
                                        smask: fill_smask_snap,
                                        smask_spot: fill_smask_spot_snap,
                                    },
                                    fill_cmyk_coverage.as_deref(),
                                    PaintModulatorContext {
                                        gs: &gs_clone,
                                        doc,
                                        page_num,
                                        resources,
                                        base_transform,
                                        fill_side: true,
                                    },
                                )?;
                            }

                            let stroke_smask_snap = self.smask_snapshot(pixmap, &gs_clone);
                            let stroke_smask_spot_snap = self.smask_spot_snapshot(&gs_clone);
                            let stroke_overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, false);
                            let stroke_cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, false);
                            let stroke_spot_snap = self.spot_paint_snapshot(pixmap, &gs_clone, false);
                            let stroke_cmyk_coverage =
                                self.rasterise_stroke_coverage(&path, transform, &gs_clone, clip);
                            self.path_rasterizer
                                .stroke_path_clipped(pixmap, &path, transform, render_gs, clip);
                            self.apply_paint_modulators(
                                pixmap,
                                PaintModulatorSnapshots {
                                    cmyk_compose: stroke_cmyk_compose_snap,
                                    overprint: stroke_overprint_snap,
                                    spot: stroke_spot_snap,
                                    smask: stroke_smask_snap,
                                    smask_spot: stroke_smask_spot_snap,
                                },
                                stroke_cmyk_coverage.as_deref(),
                                PaintModulatorContext {
                                    gs: &gs_clone,
                                    doc,
                                    page_num,
                                    resources,
                                    base_transform,
                                    fill_side: false,
                                },
                            )?;
                        }
                    } else {
                        let _ = current_path.finish();
                    }
                    current_path = PathBuilder::new();
                }
                Operator::FillEvenOdd | Operator::FillStrokeEvenOdd => {
                    if excluded_layer_depth == 0 {
                        apply_pending_clip(&mut pending_clip, &mut clip_stack, pixmap, base_transform, &gs_stack);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        if let Some(path) = current_path.finish() {
                            let gs_clone = gs_stack.current().clone();
                            let transform = combine_transforms(base_transform, &gs_clone.ctm);
                            // One unified resolve covers both fill and the
                            // optional stroke pass — for plain `f*` the
                            // helper produces a fill-only splice; for
                            // `B*`/`b*` both sides are spliced into the
                            // same clone. Either way, the rasteriser reads
                            // the side it needs from `render_gs`. ~keep
                            let kind = if matches!(op, Operator::FillStrokeEvenOdd) {
                                PipelinePaintKind::PathFillStroke
                            } else {
                                PipelinePaintKind::PathFill
                            };
                            let spliced = self.pipeline_resolve_paint_gs(doc, &gs_clone, kind);
                            let render_gs: &GraphicsState = spliced.as_ref().unwrap_or(&gs_clone);

                            let fill_by_pattern = gs_clone.fill_color_space == "Pattern"
                                && gs_clone.fill_pattern_name.is_some()
                                && self.fill_with_tiling_pattern(
                                    pixmap,
                                    &path,
                                    base_transform,
                                    transform,
                                    tiny_skia::FillRule::EvenOdd,
                                    clip,
                                    &gs_clone,
                                    doc,
                                    page_num,
                                    resources,
                                )?;

                            if !fill_by_pattern {
                                let fill_smask_snap = self.smask_snapshot(pixmap, &gs_clone);
                                let fill_smask_spot_snap = self.smask_spot_snapshot(&gs_clone);
                                let fill_overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, true);
                                let fill_cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, true);
                                let fill_spot_snap = self.spot_paint_snapshot(pixmap, &gs_clone, true);
                                let fill_cmyk_coverage =
                                    self.rasterise_fill_coverage(&path, transform, tiny_skia::FillRule::EvenOdd, clip);
                                self.path_rasterizer.fill_path_clipped(
                                    pixmap,
                                    &path,
                                    transform,
                                    render_gs,
                                    tiny_skia::FillRule::EvenOdd,
                                    clip,
                                );
                                self.apply_paint_modulators(
                                    pixmap,
                                    PaintModulatorSnapshots {
                                        cmyk_compose: fill_cmyk_compose_snap,
                                        overprint: fill_overprint_snap,
                                        spot: fill_spot_snap,
                                        smask: fill_smask_snap,
                                        smask_spot: fill_smask_spot_snap,
                                    },
                                    fill_cmyk_coverage.as_deref(),
                                    PaintModulatorContext {
                                        gs: &gs_clone,
                                        doc,
                                        page_num,
                                        resources,
                                        base_transform,
                                        fill_side: true,
                                    },
                                )?;
                            }

                            if matches!(op, Operator::FillStrokeEvenOdd) {
                                let stroke_smask_snap = self.smask_snapshot(pixmap, &gs_clone);
                                let stroke_smask_spot_snap = self.smask_spot_snapshot(&gs_clone);
                                let stroke_overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, false);
                                let stroke_cmyk_compose_snap =
                                    self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, false);
                                let stroke_spot_snap = self.spot_paint_snapshot(pixmap, &gs_clone, false);
                                let stroke_cmyk_coverage =
                                    self.rasterise_stroke_coverage(&path, transform, &gs_clone, clip);
                                self.path_rasterizer
                                    .stroke_path_clipped(pixmap, &path, transform, render_gs, clip);
                                self.apply_paint_modulators(
                                    pixmap,
                                    PaintModulatorSnapshots {
                                        cmyk_compose: stroke_cmyk_compose_snap,
                                        overprint: stroke_overprint_snap,
                                        spot: stroke_spot_snap,
                                        smask: stroke_smask_snap,
                                        smask_spot: stroke_smask_spot_snap,
                                    },
                                    stroke_cmyk_coverage.as_deref(),
                                    PaintModulatorContext {
                                        gs: &gs_clone,
                                        doc,
                                        page_num,
                                        resources,
                                        base_transform,
                                        fill_side: false,
                                    },
                                )?;
                            }
                        }
                    } else {
                        let _ = current_path.finish();
                    }
                    current_path = PathBuilder::new();
                }

                // Clipping — suppressed inside an excluded OCG scope. Per PDF
                // spec the clip is a graphics-state side-effect; without
                // gating it, a `W n` issued inside an excluded BDC scope that
                // is not bracketed by `q/Q` would silently restrict subsequent
                // visible content. ~keep
                Operator::ClipNonZero => {
                    if excluded_layer_depth == 0 {
                        if let Some(path) = current_path.clone().finish() {
                            pending_clip = Some((path, tiny_skia::FillRule::Winding));
                        }
                    }
                }
                Operator::ClipEvenOdd => {
                    if excluded_layer_depth == 0 {
                        if let Some(path) = current_path.clone().finish() {
                            pending_clip = Some((path, tiny_skia::FillRule::EvenOdd));
                        }
                    }
                }

                Operator::BeginText => {
                    in_text_object = true;
                    // Start each text object with a clean text-clip path
                    // (§9.4.1: the text clip path is reset at BT and applied
                    // at ET). Any leftover from a malformed/unterminated prior
                    // block is discarded here. ~keep
                    text_clip_accum = None;
                    let gs = gs_stack.current_mut();
                    gs.text_matrix = Matrix::identity();
                    gs.text_line_matrix = Matrix::identity();
                    tracing::trace!("BT (BeginText)");
                }
                Operator::EndText => {
                    in_text_object = false;
                    // WS1.5b — apply the accumulated text clip path (Tr 4–7).
                    // If no clip-mode text was shown the accumulator is None and
                    // ET behaves exactly as before. An all-transparent
                    // accumulator (e.g. every glyph was whitespace or lacked an
                    // outline) is treated as degenerate and leaves the clip
                    // unchanged rather than collapsing it to empty. ~keep
                    if let Some(scratch) = text_clip_accum.take() {
                        let has_coverage = scratch.data().as_chunks::<4>().0.iter().any(|px| px[3] != 0);
                        if has_coverage {
                            let text_mask = tiny_skia::Mask::from_pixmap(scratch.as_ref(), tiny_skia::MaskType::Alpha);
                            // Intersect (logical AND) the glyph silhouette with
                            // the current scope's clip so subsequent content is
                            // confined to the text shape *within* the existing
                            // clip — never widened past it. ~keep
                            if let Some(slot) = clip_stack.last_mut() {
                                let existing = slot.take();
                                *slot = Some(intersect_with_inherited(text_mask, existing.as_ref()));
                            }
                        }
                    }
                }

                Operator::Tc { char_space } => {
                    gs_stack.current_mut().char_space = *char_space;
                }
                Operator::Tw { word_space } => {
                    gs_stack.current_mut().word_space = *word_space;
                }
                Operator::Tz { scale } => {
                    gs_stack.current_mut().horizontal_scaling = *scale;
                }
                Operator::TL { leading } => {
                    gs_stack.current_mut().leading = *leading;
                }
                Operator::Ts { rise } => {
                    gs_stack.current_mut().text_rise = *rise;
                }
                Operator::Tr { render } => {
                    gs_stack.current_mut().render_mode = *render;
                }

                // Text showing — glyphs suppressed inside an excluded OCG layer,
                // but the text matrix still advances so that subsequent visible
                // text inside the same BT/ET paints at the correct X position. ~keep
                Operator::Tj { text } => {
                    if in_text_object {
                        // Type 3 fonts have no outline program; each glyph is a
                        // CharProcs content stream painted under FontMatrix ×
                        // text-space × CTM (ISO 32000-1 §9.6.5). It is handled
                        // here because it re-enters the content-stream renderer. ~keep
                        if self.current_font_is_type3(gs_stack.current()) {
                            let advance = if excluded_layer_depth == 0 {
                                let gs_snap = gs_stack.current().clone();
                                self.render_type3_text(
                                    pixmap,
                                    text,
                                    base_transform,
                                    &gs_snap,
                                    doc,
                                    page_num,
                                    resources,
                                )?
                            } else {
                                self.text_rasterizer.measure_text(text, gs_stack.current(), &self.fonts)
                            };
                            gs_stack.current_mut().advance_text_matrix(advance);
                            continue;
                        }
                        let gs = gs_stack.current();
                        let advance = if excluded_layer_depth == 0 {
                            let clip = clip_stack.last().and_then(|c| c.as_ref());
                            let transform = combine_transforms(base_transform, &gs.ctm);
                            // WS1.5b — modes 4–7 add this show's glyph
                            // outlines to the text clip path (applied at ET).
                            // Gated here so modes 0–3 pay nothing. ~keep
                            if gs.render_mode >= 4 {
                                self.accumulate_text_clip_tj(
                                    &mut text_clip_accum,
                                    pixmap.width(),
                                    pixmap.height(),
                                    text,
                                    transform,
                                    gs,
                                    resources,
                                    doc,
                                );
                            }
                            // Resolve the fill (and/or stroke per Tr mode)
                            // once for the whole `Tj` call and hand the
                            // resolved RGBA to the rasteriser. The rasteriser
                            // already clones `gs` to advance `text_matrix`
                            // per element, so it splices the override into
                            // that clone — no operator-arm-side clone
                            // needed. ~keep
                            let colors = self.pipeline_resolve_text_colors(doc, gs);
                            // §11.4.7 + §11.7.4 + §11.4 cycle: text-
                            // showing is a fill-side paint (modulated by
                            // Tr render mode for stroke). One snapshot
                            // per Tj call brackets the whole string. ~keep
                            let smask_snap = self.smask_snapshot(pixmap, gs);
                            let smask_spot_snap = self.smask_spot_snapshot(gs);
                            let overprint_snap = self.overprint_snapshot(pixmap, gs, true);
                            let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, gs, doc, true);
                            let spot_snap = self.text_fill_spot_snapshot(pixmap, gs);
                            // §9.4 + §11.7.3 + §11.3.3: rasterise the
                            // glyph-outline coverage in parallel with
                            // the visible paint so the spot mirror has
                            // a geometry-true per-pixel coverage mask
                            // (AA-edge fidelity + identical-RGB
                            // collision insulated) instead of a
                            // snapshot-vs-post-paint diff. ~keep
                            let text_coverage = spot_snap.as_ref().and_then(|_| {
                                self.rasterise_text_coverage_render_text(text, transform, gs, resources, doc, clip)
                            });
                            let adv = self.text_rasterizer.render_text(
                                pixmap,
                                text,
                                transform,
                                gs,
                                colors.as_ref(),
                                resources,
                                doc,
                                clip,
                                &self.fonts,
                            )?;
                            let gs_for_apply = gs_stack.current().clone();
                            self.apply_paint_modulators(
                                pixmap,
                                PaintModulatorSnapshots {
                                    cmyk_compose: cmyk_compose_snap,
                                    overprint: overprint_snap,
                                    spot: spot_snap,
                                    smask: smask_snap,
                                    smask_spot: smask_spot_snap,
                                },
                                text_coverage.as_deref(),
                                PaintModulatorContext {
                                    gs: &gs_for_apply,
                                    doc,
                                    page_num,
                                    resources,
                                    base_transform,
                                    fill_side: true,
                                },
                            )?;
                            adv
                        } else {
                            self.text_rasterizer.measure_text(text, gs, &self.fonts)
                        };

                        // The rasterizer returns a scalar magnitude along the
                        // active writing axis. advance_text_matrix routes it
                        // to x (WMode 0) or y (WMode 1), keeping the axis
                        // swap in exactly one place. ~keep
                        gs_stack.current_mut().advance_text_matrix(advance);
                    }
                }
                Operator::Quote { text } => {
                    if in_text_object {
                        // Quote (') is T* followed by Tj — always advance line ~keep
                        let gs_mut = gs_stack.current_mut();
                        let leading = gs_mut.leading;
                        let translation = Matrix::translation(0.0, -leading);
                        gs_mut.text_line_matrix = translation.multiply(&gs_mut.text_line_matrix);
                        gs_mut.text_matrix = gs_mut.text_line_matrix;

                        if self.current_font_is_type3(gs_stack.current()) {
                            let advance = if excluded_layer_depth == 0 {
                                let gs_snap = gs_stack.current().clone();
                                self.render_type3_text(
                                    pixmap,
                                    text,
                                    base_transform,
                                    &gs_snap,
                                    doc,
                                    page_num,
                                    resources,
                                )?
                            } else {
                                self.text_rasterizer.measure_text(text, gs_stack.current(), &self.fonts)
                            };
                            gs_stack.current_mut().advance_text_matrix(advance);
                            continue;
                        }

                        let gs = gs_stack.current();
                        let advance = if excluded_layer_depth == 0 {
                            let clip = clip_stack.last().and_then(|c| c.as_ref());
                            let transform = combine_transforms(base_transform, &gs.ctm);
                            tracing::trace!(
                                "' (Quote): rendering text at Tm=[{}, {}, {}, {}, {}, {}]",
                                gs.text_matrix.a,
                                gs.text_matrix.b,
                                gs.text_matrix.c,
                                gs.text_matrix.d,
                                gs.text_matrix.e,
                                gs.text_matrix.f
                            );
                            if gs.render_mode >= 4 {
                                self.accumulate_text_clip_tj(
                                    &mut text_clip_accum,
                                    pixmap.width(),
                                    pixmap.height(),
                                    text,
                                    transform,
                                    gs,
                                    resources,
                                    doc,
                                );
                            }
                            // Same shape as `Tj`. `'` is `T* Tj` per
                            // ISO 32000-1; the resolved colour depends only
                            // on the prior colour-setting ops, so the resolve
                            // happens here, not inside `T*`. ~keep
                            let colors = self.pipeline_resolve_text_colors(doc, gs);
                            let smask_snap = self.smask_snapshot(pixmap, gs);
                            let smask_spot_snap = self.smask_spot_snapshot(gs);
                            let overprint_snap = self.overprint_snapshot(pixmap, gs, true);
                            let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, gs, doc, true);
                            let spot_snap = self.text_fill_spot_snapshot(pixmap, gs);
                            let text_coverage = spot_snap.as_ref().and_then(|_| {
                                self.rasterise_text_coverage_render_text(text, transform, gs, resources, doc, clip)
                            });
                            let adv = self.text_rasterizer.render_text(
                                pixmap,
                                text,
                                transform,
                                gs,
                                colors.as_ref(),
                                resources,
                                doc,
                                clip,
                                &self.fonts,
                            )?;
                            let gs_for_apply = gs_stack.current().clone();
                            self.apply_paint_modulators(
                                pixmap,
                                PaintModulatorSnapshots {
                                    cmyk_compose: cmyk_compose_snap,
                                    overprint: overprint_snap,
                                    spot: spot_snap,
                                    smask: smask_snap,
                                    smask_spot: smask_spot_snap,
                                },
                                text_coverage.as_deref(),
                                PaintModulatorContext {
                                    gs: &gs_for_apply,
                                    doc,
                                    page_num,
                                    resources,
                                    base_transform,
                                    fill_side: true,
                                },
                            )?;
                            adv
                        } else {
                            self.text_rasterizer.measure_text(text, gs, &self.fonts)
                        };

                        gs_stack.current_mut().advance_text_matrix(advance);
                    }
                }
                Operator::TJ { array } => {
                    if in_text_object {
                        if self.current_font_is_type3(gs_stack.current()) {
                            let advance = if excluded_layer_depth == 0 {
                                let gs_snap = gs_stack.current().clone();
                                self.render_type3_tj_array(
                                    pixmap,
                                    array,
                                    base_transform,
                                    &gs_snap,
                                    doc,
                                    page_num,
                                    resources,
                                )?
                            } else {
                                self.text_rasterizer
                                    .measure_tj_array(array, gs_stack.current(), &self.fonts)
                            };
                            gs_stack.current_mut().advance_text_matrix(advance);
                            continue;
                        }
                        let gs = gs_stack.current();
                        let advance = if excluded_layer_depth == 0 {
                            let clip = clip_stack.last().and_then(|c| c.as_ref());
                            let transform = combine_transforms(base_transform, &gs.ctm);
                            tracing::trace!(
                                "TJ: rendering array at Tm=[{}, {}, {}, {}, {}, {}]",
                                gs.text_matrix.a,
                                gs.text_matrix.b,
                                gs.text_matrix.c,
                                gs.text_matrix.d,
                                gs.text_matrix.e,
                                gs.text_matrix.f
                            );
                            if gs.render_mode >= 4 {
                                self.accumulate_text_clip_tj_array(
                                    &mut text_clip_accum,
                                    pixmap.width(),
                                    pixmap.height(),
                                    array,
                                    transform,
                                    gs,
                                    resources,
                                    doc,
                                );
                            }
                            // Resolve once for the whole `TJ` array — the
                            // numeric offsets inside `array` only adjust
                            // positioning; they cannot alter the active
                            // colour mid-string. The rasteriser threads the
                            // override into the per-element `render_text`
                            // calls so the colour propagates without an
                            // operator-arm-side clone of `gs`. ~keep
                            let colors = self.pipeline_resolve_text_colors(doc, gs);
                            let smask_snap = self.smask_snapshot(pixmap, gs);
                            let smask_spot_snap = self.smask_spot_snapshot(gs);
                            let overprint_snap = self.overprint_snapshot(pixmap, gs, true);
                            let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, gs, doc, true);
                            let spot_snap = self.text_fill_spot_snapshot(pixmap, gs);
                            let text_coverage = spot_snap.as_ref().and_then(|_| {
                                self.rasterise_text_coverage_render_tj_array(array, transform, gs, resources, doc, clip)
                            });
                            let adv = self.text_rasterizer.render_tj_array(
                                pixmap,
                                array,
                                transform,
                                gs,
                                colors.as_ref(),
                                resources,
                                doc,
                                clip,
                                &self.fonts,
                            )?;
                            let gs_for_apply = gs_stack.current().clone();
                            self.apply_paint_modulators(
                                pixmap,
                                PaintModulatorSnapshots {
                                    cmyk_compose: cmyk_compose_snap,
                                    overprint: overprint_snap,
                                    spot: spot_snap,
                                    smask: smask_snap,
                                    smask_spot: smask_spot_snap,
                                },
                                text_coverage.as_deref(),
                                PaintModulatorContext {
                                    gs: &gs_for_apply,
                                    doc,
                                    page_num,
                                    resources,
                                    base_transform,
                                    fill_side: true,
                                },
                            )?;
                            adv
                        } else {
                            self.text_rasterizer.measure_tj_array(array, gs, &self.fonts)
                        };

                        gs_stack.current_mut().advance_text_matrix(advance);
                    }
                }
                Operator::DoubleQuote {
                    word_space,
                    char_space,
                    text,
                } => {
                    if in_text_object {
                        // Double Quote (") always updates state ~keep
                        let gs_mut = gs_stack.current_mut();
                        gs_mut.word_space = *word_space;
                        gs_mut.char_space = *char_space;

                        let leading = gs_mut.leading;
                        let translation = Matrix::translation(0.0, -leading);
                        gs_mut.text_line_matrix = translation.multiply(&gs_mut.text_line_matrix);
                        gs_mut.text_matrix = gs_mut.text_line_matrix;

                        if self.current_font_is_type3(gs_stack.current()) {
                            let advance = if excluded_layer_depth == 0 {
                                let gs_snap = gs_stack.current().clone();
                                self.render_type3_text(
                                    pixmap,
                                    text,
                                    base_transform,
                                    &gs_snap,
                                    doc,
                                    page_num,
                                    resources,
                                )?
                            } else {
                                self.text_rasterizer.measure_text(text, gs_stack.current(), &self.fonts)
                            };
                            gs_stack.current_mut().advance_text_matrix(advance);
                            continue;
                        }

                        let gs = gs_stack.current();
                        let advance = if excluded_layer_depth == 0 {
                            let clip = clip_stack.last().and_then(|c| c.as_ref());
                            let transform = combine_transforms(base_transform, &gs.ctm);
                            tracing::trace!(
                                "\" (DoubleQuote): rendering text at Tm=[{}, {}, {}, {}, {}, {}]",
                                gs.text_matrix.a,
                                gs.text_matrix.b,
                                gs.text_matrix.c,
                                gs.text_matrix.d,
                                gs.text_matrix.e,
                                gs.text_matrix.f
                            );
                            if gs.render_mode >= 4 {
                                self.accumulate_text_clip_tj(
                                    &mut text_clip_accum,
                                    pixmap.width(),
                                    pixmap.height(),
                                    text,
                                    transform,
                                    gs,
                                    resources,
                                    doc,
                                );
                            }
                            // `"` is equivalent to setting Tw, Tc, then
                            // `T* Tj`. Tw/Tc are state-only and don't
                            // influence the resolved colour, so the resolve
                            // happens immediately before painting just like
                            // in `Tj` / `'`. ~keep
                            let colors = self.pipeline_resolve_text_colors(doc, gs);
                            let smask_snap = self.smask_snapshot(pixmap, gs);
                            let smask_spot_snap = self.smask_spot_snapshot(gs);
                            let overprint_snap = self.overprint_snapshot(pixmap, gs, true);
                            let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, gs, doc, true);
                            let spot_snap = self.text_fill_spot_snapshot(pixmap, gs);
                            let text_coverage = spot_snap.as_ref().and_then(|_| {
                                self.rasterise_text_coverage_render_text(text, transform, gs, resources, doc, clip)
                            });
                            let adv = self.text_rasterizer.render_text(
                                pixmap,
                                text,
                                transform,
                                gs,
                                colors.as_ref(),
                                resources,
                                doc,
                                clip,
                                &self.fonts,
                            )?;
                            let gs_for_apply = gs_stack.current().clone();
                            self.apply_paint_modulators(
                                pixmap,
                                PaintModulatorSnapshots {
                                    cmyk_compose: cmyk_compose_snap,
                                    overprint: overprint_snap,
                                    spot: spot_snap,
                                    smask: smask_snap,
                                    smask_spot: smask_spot_snap,
                                },
                                text_coverage.as_deref(),
                                PaintModulatorContext {
                                    gs: &gs_for_apply,
                                    doc,
                                    page_num,
                                    resources,
                                    base_transform,
                                    fill_side: true,
                                },
                            )?;
                            adv
                        } else {
                            self.text_rasterizer.measure_text(text, gs, &self.fonts)
                        };

                        gs_stack.current_mut().advance_text_matrix(advance);
                    }
                }

                Operator::Do { name } => {
                    if excluded_layer_depth == 0 {
                        let gs_clone = gs_stack.current().clone();
                        let transform = combine_transforms(base_transform, &gs_clone.ctm);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        tracing::trace!("Do: rendering XObject '{}'", name);
                        // §11.4.7 + §11.7.4 + §11.4 cycle: the entire
                        // XObject paint (Form or Image) sits inside the
                        // snapshot bracket so a /SMask attached via
                        // ExtGState modulates the cumulative
                        // contribution. Image XObjects always behave as
                        // fill-side paints; Form XObjects honour their
                        // own internal ExtGState changes (the snapshot
                        // captures the page-level state, the Form runs
                        // recursively, and the apply blends the Form's
                        // contribution against the captured backdrop).
                        //
                        // Per-subtype dispatch for the post-Do colour-
                        // lane modulators: Image / ImageMask XObjects do
                        // NOT execute their own paint operators — their
                        // pixel data is painted using the outer
                        // graphics state, so the post-Do CMYK compose,
                        // overprint and spot-lane mirrors are how those
                        // lanes learn about the contribution. Form
                        // XObjects DO execute their own paint operators
                        // (Fill / Stroke / FillStroke / Do / ShowText /
                        // shading), each of which runs its own per-
                        // paint sidecar mirror with the FORM's gs at
                        // the time of the paint. Re-applying the outer
                        // gs's CMYK / overprint / spot mirror after a
                        // Form Do would composite the form's region
                        // again with whatever colour the OUTER gs had,
                        // double-counting (and, when the outer colour
                        // differs from the form's, overwriting the
                        // form's mirror writes — the QA-6 / QA-6-DIAG-2
                        // failure mode where outer /K's iteration 2
                        // /Inner Do lost the inner Form's spot
                        // contribution). SMask attenuation always
                        // applies — an outer /SMask gs in effect at the
                        // Do attaches to the Do's entire region
                        // regardless of how the inner produced its
                        // pixels. ~keep
                        let xobj_subtype = self.xobject_subtype(name, resources, doc);
                        let is_form = matches!(xobj_subtype.as_deref(), Some("Form"));
                        let smask_snap = self.smask_snapshot(pixmap, &gs_clone);
                        let smask_spot_snap = self.smask_spot_snapshot(&gs_clone);
                        let overprint_snap = if is_form {
                            None
                        } else {
                            self.overprint_snapshot(pixmap, &gs_clone, true)
                        };
                        let cmyk_compose_snap = if is_form {
                            None
                        } else {
                            self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, true)
                        };
                        let spot_snap = if is_form {
                            None
                        } else {
                            self.spot_paint_snapshot(pixmap, &gs_clone, true)
                        };
                        // §8.9.5 + §8.9.6.2 + §11.7.3: rasterise the
                        // Image / ImageMask footprint + stencil-bit
                        // coverage so the spot mirror has a geometry-
                        // true per-pixel mask. Skipped for Form
                        // XObjects (their per-paint mirror runs
                        // inside the recursive content stream — the
                        // post-Do mirror for Forms is already
                        // suppressed by round 3's P0 fix). ~keep
                        let image_coverage = spot_snap.as_ref().and_then(|_| {
                            self.rasterise_image_xobject_coverage(name, transform, &gs_clone, resources, doc, clip)
                        });
                        self.render_xobject(pixmap, name, transform, &gs_clone, resources, doc, page_num, clip)?;
                        self.apply_paint_modulators(
                            pixmap,
                            PaintModulatorSnapshots {
                                cmyk_compose: cmyk_compose_snap,
                                overprint: overprint_snap,
                                spot: spot_snap,
                                smask: smask_snap,
                                smask_spot: smask_spot_snap,
                            },
                            image_coverage.as_deref(),
                            PaintModulatorContext {
                                gs: &gs_clone,
                                doc,
                                page_num,
                                resources,
                                base_transform,
                                fill_side: true,
                            },
                        )?;
                    }
                }

                // Inline image (`BI ... ID <data> EI`) — §8.9.7. Unlike a
                // `Do`-invoked image XObject, the pixel data sits directly in
                // the content stream (no indirect object, so no encryption
                // and no `/SMask` or `/Mask`, both of which the spec only
                // allows as indirect references). Expand the abbreviated
                // dictionary keys/values into the same shape
                // `extract_image_from_xobject` expects, wrap it in a
                // synthetic `Object::Stream`, and paint it through the same
                // `render_image`/`render_image_mask` used for `Do` images. ~keep
                Operator::InlineImage { dict, data } => {
                    if excluded_layer_depth == 0 {
                        let gs_clone = gs_stack.current().clone();
                        let transform = combine_transforms(base_transform, &gs_clone.ctm);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        let expanded = crate::extractors::images::expand_inline_image_dict((**dict).clone());
                        let is_image_mask = expanded
                            .get("ImageMask")
                            .map(|o| matches!(o, Object::Boolean(true)))
                            .unwrap_or(false);
                        let synthetic = Object::Stream {
                            dict: expanded,
                            data: bytes::Bytes::from(data.clone()),
                        };
                        if is_image_mask {
                            if let Err(error) =
                                self.render_image_mask(pixmap, &synthetic, None, transform, doc, clip, &gs_clone)
                            {
                                tracing::warn!(
                                    error_code = error.telemetry_code(),
                                    error_offset = ?error.telemetry_offset(),
                                    "skipping unrenderable inline ImageMask"
                                );
                            }
                        } else if let Err(error) =
                            self.render_image(pixmap, &synthetic, None, transform, doc, clip, None, None, &gs_clone)
                        {
                            tracing::warn!(
                                error_code = error.telemetry_code(),
                                error_offset = ?error.telemetry_offset(),
                                "skipping unrenderable inline image"
                            );
                        }
                    }
                }

                Operator::Td { tx, ty } => {
                    if in_text_object {
                        let gs = gs_stack.current_mut();
                        let translation = Matrix::translation(*tx, *ty);
                        gs.text_line_matrix = translation.multiply(&gs.text_line_matrix);
                        gs.text_matrix = gs.text_line_matrix;
                        tracing::trace!("Td: [{}, {}], text_matrix now: {:?}", tx, ty, gs.text_matrix);
                    }
                }
                Operator::TD { tx, ty } => {
                    if in_text_object {
                        let gs = gs_stack.current_mut();
                        gs.leading = -(*ty);
                        let translation = Matrix::translation(*tx, *ty);
                        gs.text_line_matrix = translation.multiply(&gs.text_line_matrix);
                        gs.text_matrix = gs.text_line_matrix;
                        tracing::trace!("TD: [{}, {}], text_matrix now: {:?}", tx, ty, gs.text_matrix);
                    }
                }
                Operator::Tm { a, b, c, d, e, f } => {
                    if in_text_object {
                        let gs = gs_stack.current_mut();
                        gs.text_matrix = Matrix {
                            a: *a,
                            b: *b,
                            c: *c,
                            d: *d,
                            e: *e,
                            f: *f,
                        };
                        gs.text_line_matrix = gs.text_matrix;
                        tracing::trace!(
                            "Tm: [{}, {}, {}, {}, {}, {}], text_matrix now: {:?}",
                            a,
                            b,
                            c,
                            d,
                            e,
                            f,
                            gs.text_matrix
                        );
                    }
                }
                Operator::TStar => {
                    if in_text_object {
                        let gs = gs_stack.current_mut();
                        let leading = gs.leading;
                        let translation = Matrix::translation(0.0, -leading);
                        gs.text_line_matrix = translation.multiply(&gs.text_line_matrix);
                        gs.text_matrix = gs.text_line_matrix;
                        tracing::trace!("T*: text_matrix now: {:?}", gs.text_matrix);
                    }
                }
                Operator::Tf { font, size } => {
                    // Cache the font's writing mode on the graphics state so
                    // the rasterizer hot path can branch on a single
                    // primitive read instead of dereferencing the FontInfo
                    // through the cache for every glyph. ~keep
                    let wmode = self.fonts.get(font).map(|f| f.wmode).unwrap_or(0);
                    let gs = gs_stack.current_mut();
                    gs.font_name = Some(font.clone());
                    gs.font_size = *size;
                    gs.text_wmode = wmode;
                }

                Operator::SetExtGState { dict_name } => {
                    // Fast path: resource dict is already resolved (see top of
                    // this function), so the per-`gs` cost is one HashMap
                    // lookup + one resolve of the small inner state dict. ~keep
                    let entry = ext_g_state_cache.entry(dict_name.clone()).or_insert_with(|| {
                        if let Some(states) = ext_g_states {
                            if let Some(state_obj) = states.get(dict_name) {
                                return parse_ext_g_state_inner(state_obj, doc).unwrap_or_default();
                            }
                        }
                        ParsedExtGState::default()
                    });
                    entry.apply(gs_stack.current_mut());
                }

                // EndPath (n operator): discard current path without painting,
                // but apply any pending clip. Per PDF spec, W n is the standard
                // way to set a clipping path without filling or stroking.
                // Suppress the clip application inside an excluded OCG scope so
                // the clip doesn't leak past EMC into visible content. ~keep
                Operator::EndPath => {
                    if excluded_layer_depth == 0 {
                        apply_pending_clip(&mut pending_clip, &mut clip_stack, pixmap, base_transform, &gs_stack);
                    } else {
                        let _ = pending_clip.take();
                    }
                    current_path = PathBuilder::new();
                }

                Operator::PaintShading { name } => {
                    if excluded_layer_depth == 0 {
                        let mut gs_clone = gs_stack.current().clone();
                        // §8.7.4 + §11.7.3: when the shading's
                        // /ColorSpace is /Separation or non-process
                        // /DeviceN, surface the ink-name list (paired
                        // with the /Function /C0 endpoint tints) onto
                        // `gs_clone.fill_spot_inks` so the spot mirror
                        // sees a non-empty source ink set and fires.
                        // Without this the shading paint silently
                        // bypasses the spot mirror because the gating
                        // (`spot_paint_active`) checks
                        // `gs.fill_spot_inks`, which is otherwise
                        // populated only by `cs`/`scn` colour-set
                        // operators — none of which fire before `sh`. ~keep
                        if !self.spot_paint_active(&gs_clone, true) && self.cmyk_sidecar.is_some() {
                            if let Some(inks) = self.resolve_shading_spot_inks(name, resources, doc) {
                                if !inks.is_empty() {
                                    gs_clone.fill_spot_inks = inks;
                                }
                            }
                        }
                        let transform = combine_transforms(base_transform, &gs_clone.ctm);
                        let clip = clip_stack.last().and_then(|c| c.as_ref());
                        // §11.4.7 + §11.7.4 + §11.4 cycle: shading is
                        // a fill-side paint, so the snapshot/apply
                        // cadence mirrors the path-Fill arm. The
                        // overprint and compose-first paths short-
                        // circuit when the active fill colour is not
                        // CMYK (the shading paint's per-pixel colour
                        // comes from the gradient interpolator, not
                        // `gs.fill_color_cmyk`), so they only fire when
                        // the page set a CMYK fill before invoking
                        // `sh`. ~keep
                        let smask_snap = self.smask_snapshot(pixmap, &gs_clone);
                        let smask_spot_snap = self.smask_spot_snapshot(&gs_clone);
                        let overprint_snap = self.overprint_snapshot(pixmap, &gs_clone, true);
                        let cmyk_compose_snap = self.cmyk_compose_snapshot(pixmap, &gs_clone, doc, true);
                        let spot_snap = self.spot_paint_snapshot(pixmap, &gs_clone, true);
                        // §8.7.4 + §11.7.3: rasterise the shading
                        // geometry (intersected with the active clip)
                        // so the spot mirror sees the geometry-true
                        // per-pixel coverage of the gradient. ~keep
                        let shading_coverage = spot_snap.as_ref().and_then(|_| {
                            self.rasterise_shading_coverage(name, transform, &gs_clone, resources, doc, clip)
                        });
                        self.render_shading(pixmap, name, transform, &gs_clone, resources, doc, clip)?;
                        self.apply_paint_modulators(
                            pixmap,
                            PaintModulatorSnapshots {
                                cmyk_compose: cmyk_compose_snap,
                                overprint: overprint_snap,
                                spot: spot_snap,
                                smask: smask_snap,
                                smask_spot: smask_spot_snap,
                            },
                            shading_coverage.as_deref(),
                            PaintModulatorContext {
                                gs: &gs_clone,
                                doc,
                                page_num,
                                resources,
                                base_transform,
                                fill_side: true,
                            },
                        )?;
                    }
                }

                Operator::BeginMarkedContent { .. } => {
                    marked_content_is_excluded.push(false);
                }
                Operator::BeginMarkedContentDict { tag, properties } => {
                    let mut is_excluded = false;
                    // Tag "OC" scopes can hide content even with empty excluded_layers
                    // when the OCMD uses /VE /Not or /P /AllOff/AnyOff (the
                    // expression evaluates with all OCGs on by default). We can
                    // only short-circuit cheaply for simple OCG refs, which the
                    // optional_content module handles internally. ~keep
                    if tag == "OC" {
                        is_excluded = crate::optional_content::resolve_and_check_ocg_excluded(
                            properties,
                            Some(resources),
                            Some(doc),
                            excluded_layers,
                        );
                    }
                    if is_excluded {
                        excluded_layer_depth += 1;
                    }
                    marked_content_is_excluded.push(is_excluded);
                }
                Operator::EndMarkedContent => {
                    if let Some(was_excluded) = marked_content_is_excluded.pop() {
                        if was_excluded && excluded_layer_depth > 0 {
                            excluded_layer_depth -= 1;
                        }
                    }
                }

                _ => {}
            }
        }

        Ok(())
    }

    /// Returns `true` when the font currently selected in `gs` is a Type 3
    /// font. Type 3 glyphs are user-defined content streams, rendered by
    /// [`Self::render_type3_text`] rather than the outline rasteriser.
    fn current_font_is_type3(&self, gs: &GraphicsState) -> bool {
        gs.font_name
            .as_deref()
            .and_then(|n| self.fonts.get(n))
            .map(|f| f.subtype == "Type3")
            .unwrap_or(false)
    }

    /// Render a Type 3 `TJ` array: each string element paints glyphs and each
    /// numeric element shifts the cursor by `-offset/1000 × Tfs` along the
    /// writing axis. Returns the total text-space advance; the caller applies
    /// it once via `advance_text_matrix`.
    fn render_type3_tj_array(
        &mut self,
        pixmap: &mut Pixmap,
        array: &[crate::content::operators::TextElement],
        base_transform: Transform,
        gs: &GraphicsState,
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
    ) -> Result<f32> {
        use crate::content::operators::TextElement;
        // A local graphics-state copy tracks the cursor across elements; the
        // real text matrix is advanced once by the caller with the returned sum. ~keep
        let mut gs_local = gs.clone();
        let mut total = 0.0f32;
        for element in array {
            match element {
                TextElement::String(text) => {
                    let adv =
                        self.render_type3_text(pixmap, text, base_transform, &gs_local, doc, page_num, resources)?;
                    gs_local.advance_text_matrix(adv);
                    total += adv;
                }
                TextElement::Offset(offset) => {
                    let shift = (-offset / 1000.0) * gs_local.font_size;
                    gs_local.advance_text_matrix(shift);
                    total += shift;
                }
            }
        }
        Ok(total)
    }

    /// Render one Type 3 text string. For each byte code the glyph name is
    /// resolved through the font's `/Encoding` `/Differences`, its `/CharProcs`
    /// content stream is executed under `FontMatrix × text-space × CTM`
    /// (ISO 32000-1 §9.6.5) using the font's own `/Resources`, and the cursor
    /// is advanced by the glyph width. Returns the total text-space advance.
    fn render_type3_text(
        &mut self,
        pixmap: &mut Pixmap,
        text: &[u8],
        base_transform: Transform,
        gs: &GraphicsState,
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
    ) -> Result<f32> {
        let font_name = match gs.font_name.as_deref() {
            Some(n) => n,
            None => return Ok(0.0),
        };
        let font_info = match self.fonts.get(font_name) {
            Some(f) => Arc::clone(f),
            None => return Ok(0.0),
        };

        // Total advance for the whole string. Computed up front so it is
        // applied even when individual glyph descriptions are missing, and
        // shared with the outline text path for a consistent cursor. ~keep
        let string_advance = self.text_rasterizer.measure_text(text, gs, &self.fonts);

        let font_dict_obj = resources
            .as_dict()
            .and_then(|rd| rd.get("Font"))
            .and_then(|f| doc.resolve_object(f).ok())
            .and_then(|fonts| fonts.as_dict().and_then(|fd| fd.get(font_name)).cloned())
            .and_then(|fref| doc.resolve_object(&fref).ok());
        let font_dict = match font_dict_obj.as_ref().and_then(|o| o.as_dict()) {
            Some(d) => d,
            None => return Ok(string_advance),
        };

        // Glyph-space → text-space FontMatrix (default 1/1000 em, Type 1-like). ~keep
        let font_matrix = type3_font_matrix(font_dict);

        let char_procs_obj = font_dict.get("CharProcs").and_then(|o| doc.resolve_object(o).ok());
        let char_procs = match char_procs_obj.as_ref().and_then(|o| o.as_dict()) {
            Some(cp) => cp,
            None => return Ok(string_advance),
        };

        // The font's own /Resources, falling back to the page/form resources. ~keep
        let font_resources = font_dict
            .get("Resources")
            .and_then(|o| doc.resolve_object(o).ok())
            .unwrap_or_else(|| resources.clone());

        // combined_base = base · CTM · Tm  (user→device · text matrix). ~keep
        let transform = combine_transforms(base_transform, &gs.ctm);
        let tm = &gs.text_matrix;
        let combined_base = transform.pre_concat(Transform::from_row(tm.a, tm.b, tm.c, tm.d, tm.e, tm.f));

        let font_size = gs.font_size;
        let h_scale = gs.horizontal_scaling / 100.0;
        // Glyphs are suppressed for the invisible / clip-only render modes. ~keep
        let paint_glyphs = gs.render_mode != 3 && gs.render_mode != 7;

        // Load the Type 3 font's own resources into the font / colour-space
        // caches for the duration of the glyph descriptions (mirrors the Form
        // XObject path so CharProcs that reference fonts / XObjects resolve). ~keep
        let saved_fonts = self.fonts.clone();
        let saved_color_spaces = self.color_spaces.clone();
        let _ = self.load_resources(doc, &font_resources);

        // font_matrix_a converts /Widths glyph-space values to text-space
        // units (FontInfo::font_matrix_a doc comment). Standard fonts use
        // the implicit Type 1 scale 0.001; a Type 3 font declares its own
        // scale via /FontMatrix[0], which can be far from 0.001 (GH#1780:
        // 0.01, i.e. a 100-unit em). Hardcoding /1000.0 here understated
        // this font's advance ~10x, collapsing every glyph onto nearly the
        // same x position. ~keep
        let mut x_cursor = 0.0f32;
        for &code in text {
            let glyph_adv = font_info.get_glyph_width(code as u16) * font_size * font_info.font_matrix_a;

            if paint_glyphs {
                if let Some(name) = font_info.diff_glyph_names.get(&code) {
                    if let Some(stream) = char_procs.get(name) {
                        if let Some(data) = decode_type3_charproc(doc, stream) {
                            // Glyph placement: combined_base · translate(cursor)
                            // · scale(Tfs) · FontMatrix. The cursor is the
                            // un-scaled x position with Th applied at placement,
                            // matching the outline text path. ~keep
                            let px = x_cursor * h_scale;
                            let glyph_transform = combined_base
                                .pre_translate(px, gs.text_rise)
                                .pre_scale(font_size, font_size)
                                .pre_concat(font_matrix);
                            let _ = self.render_type3_glyph(
                                pixmap,
                                &data,
                                glyph_transform,
                                doc,
                                page_num,
                                &font_resources,
                                gs.fill_color_rgb,
                            );
                        }
                    }
                }
            }

            x_cursor += glyph_adv + gs.char_space;
            if code == 0x20 {
                x_cursor += gs.word_space;
            }
        }

        self.fonts = saved_fonts;
        self.color_spaces = saved_color_spaces;
        Ok(string_advance)
    }

    /// Execute a single Type 3 glyph description under `glyph_transform`. The
    /// first glyph operator selects the colour model: `d1` marks a stencil
    /// painted with the current fill colour (all colour operators inside are
    /// ignored), while `d0` lets the glyph set its own colours (ISO 32000-1
    /// §9.6.5.2). Malformed streams and over-deep recursion are skipped.
    fn render_type3_glyph(
        &mut self,
        pixmap: &mut Pixmap,
        data: &[u8],
        glyph_transform: Transform,
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
        fill_rgb: (f32, f32, f32),
    ) -> Result<()> {
        if self.type3_depth >= MAX_TYPE3_DEPTH {
            return Ok(());
        }
        let operators = match parse_content_stream(data) {
            Ok(ops) => ops,
            Err(_) => return Ok(()), // malformed glyph — skip, width already applied ~keep
        };

        // Detect the d0 / d1 metric operator (parsed as `Other`). `d1` locks
        // the fill colour; `d0` leaves the glyph free to set its own colours. ~keep
        let is_d1 = operators
            .iter()
            .find_map(|op| match op {
                Operator::Other { name, .. } if name == "d1" => Some(true),
                Operator::Other { name, .. } if name == "d0" => Some(false),
                _ => None,
            })
            .unwrap_or(false);

        self.type3_depth += 1;
        let prev_lock = self.type3_fill_lock.take();
        if is_d1 {
            self.type3_fill_lock = Some(fill_rgb);
        }
        let result = self.execute_operators(pixmap, glyph_transform, &operators, doc, page_num, resources);
        self.type3_fill_lock = prev_lock;
        self.type3_depth -= 1;
        result
    }

    /// Render a shading pattern (gradient).
    fn render_shading(
        &self,
        pixmap: &mut Pixmap,
        name: &str,
        transform: Transform,
        gs: &GraphicsState,
        resources: &Object,
        doc: &PdfDocument,
        clip_mask: Option<&tiny_skia::Mask>,
    ) -> Result<()> {
        // Look up shading resource. Retain the full resolved object — for
        // mesh shadings (Types 4-7) the geometry lives in the object's
        // stream body, which `as_dict()` alone would drop. ~keep
        let shading_obj = if let Object::Dictionary(res_dict) = resources {
            if let Some(shading_res) = res_dict.get("Shading") {
                let resolved = doc.resolve_object(shading_res)?;
                if let Some(shadings) = resolved.as_dict() {
                    if let Some(sh_obj) = shadings.get(name) {
                        Some(doc.resolve_object(sh_obj)?)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let shading = match shading_obj.as_ref().and_then(|o| o.as_dict()) {
            Some(d) => d.clone(),
            None => {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "render_shading",
                    error_code = "missing_resource",
                    "skipping missing shading resource"
                );
                return Ok(());
            }
        };

        let shading_type = shading.get("ShadingType").and_then(|o| o.as_integer()).unwrap_or(0);

        // Pre-resolve gradient endpoint colours through the resolution
        // pipeline for the shading types we migrate (axial=2, radial=3).
        // For both types the endpoint
        // colours live in the shading's `/Function` (Type 2 exponential
        // interpolation puts the endpoints directly in `/C0` and
        // `/C1`; Type 3 stitching wraps a sub-function whose first /
        // last sub-functions carry them). The current inline path reads
        // `/C0` and `/C1` raw and treats them as already-RGB, which
        // silently truncates DeviceCMYK to its first three components
        // and drops Separation tint-transform evaluation entirely. The
        // pipeline-resolved endpoints respect the shading dict's
        // `/ColorSpace`, so a Type 4 Separation `/C0` becomes the
        // function's actual output rather than a `1 - tint` fall-back.
        //
        // Types 1 (function-based) and 4-7 (mesh) carry per-point /
        // per-vertex colours, not endpoints; this wave does NOT migrate
        // them. They fall straight through to the existing inline path,
        // unmodified. ~keep
        let resolved_endpoints = if shading_type == 2 || shading_type == 3 {
            self.pipeline_resolve_shading_endpoints(&shading, gs, doc)
        } else {
            None
        };

        match shading_type {
            2 => self.render_axial_shading(pixmap, &shading, transform, gs, clip_mask, resolved_endpoints),
            3 => self.render_radial_shading(pixmap, &shading, transform, gs, clip_mask, resolved_endpoints),
            1 | 4 | 5 | 6 | 7 => {
                // Mesh (Types 4-7) and function-based (Type 1) shadings are
                // rasterised by the dedicated hand-written backend — they do
                // not map onto a tiny-skia gradient shader. Colours read from
                // the geometry stream (or produced by the shading's optional
                // `/Function`) are routed back through the standard §8.6
                // colour-space resolution path via this closure so DeviceN /
                // Separation / ICCBased colour spaces resolve identically to
                // the axial/radial endpoints. ~keep
                let shading_obj = match shading_obj.as_ref() {
                    Some(o) => o,
                    None => return Ok(()),
                };
                let resolved_cs = shading.get("ColorSpace").and_then(|o| doc.resolve_object(o).ok());
                let resolve_color = |comps: &[f32]| -> Option<(f32, f32, f32, f32)> {
                    let cs = resolved_cs.as_ref()?;
                    self.pipeline_resolve_components(doc, &self.color_spaces, cs, comps, gs.fill_alpha)
                };
                crate::rendering::mesh_shading::render_mesh_shading(
                    pixmap,
                    &shading,
                    shading_obj,
                    shading_type,
                    transform,
                    doc,
                    clip_mask,
                    &resolve_color,
                )
            }
            _ => {
                tracing::warn!(
                    target: crate::LOG_TARGET_ROOT,
                    operation = "render_shading",
                    error_code = "unsupported_shading_type",
                    shading_type,
                    "skipping unsupported shading"
                );
                Ok(())
            }
        }
    }

    /// Resolve a Type 2 / Type 3 shading dictionary's `/C0` and `/C1`
    /// endpoint colours through the resolution pipeline. The shading
    /// dict's `/ColorSpace` selects the colour space; `/Function` (a
    /// Type 2 exponential or a Type 3 stitching wrapper) carries the
    /// endpoint component arrays. Returns `None` when either endpoint
    /// can't be resolved (missing `/Function`, unsupported sub-function
    /// type, non-RGBA resolver output, etc.) — the caller falls back to
    /// the existing inline behaviour in that case.
    ///
    /// Splits the "what colour" decision (pipeline-resolved) from the
    /// "how to interpolate" decision (still owned by the gradient
    /// backend). The interpolation math is untouched — only the two
    /// fixed endpoint colours are routed through the pipeline.
    fn pipeline_resolve_shading_endpoints(
        &self,
        shading: &std::collections::HashMap<String, Object>,
        gs: &GraphicsState,
        doc: &PdfDocument,
    ) -> Option<((f32, f32, f32, f32), (f32, f32, f32, f32))> {
        // The shading dict's `/ColorSpace` can be a Name (DeviceRGB,
        // CS1, ...) or an inline Array ([/Separation ... funcRef]).
        // Resolve indirect references so the helper sees the final
        // shape. ~keep
        let cs_obj = shading.get("ColorSpace")?;
        let resolved_cs = doc.resolve_object(cs_obj).ok()?;

        // Per ISO 32000-1 §8.7.4.5.3, axial/radial shadings carry a
        // `/Domain` array on the shading dict (default `[0 1]`) that
        // names the parameter range mapped to the gradient axis.
        // Geometric `t=0` evaluates the function at `Domain[0]` and
        // `t=1` evaluates it at `Domain[1]` — the endpoints aren't
        // necessarily `f(0)` and `f(1)`. ~keep
        let (domain0, domain1) = shading
            .get("Domain")
            .and_then(|o| o.as_array())
            .and_then(|arr| {
                let d0 = arr.first()?;
                let d1 = arr.get(1)?;
                let parse = |o: &Object| -> Option<f32> {
                    match o {
                        Object::Real(v) => Some(*v as f32),
                        Object::Integer(v) => Some(*v as f32),
                        _ => None,
                    }
                };
                Some((parse(d0)?, parse(d1)?))
            })
            .unwrap_or((0.0, 1.0));

        // Extract endpoint component arrays from `/Function`. Handles
        // Type 2 (exponential) — where the endpoints are evaluated by
        // applying the shading's `/Domain` to the function's
        // exponential interpolation — and Type 3 (stitching) — where
        // the first sub-function's `/C0` and the last sub-function's
        // `/C1` are taken at face value. Type 3 with non-trivial
        // `/Encode` is not honoured; see the body comment below. ~keep
        let func_obj = shading.get("Function")?;
        let resolved_func = doc.resolve_object(func_obj).ok()?;
        let func_dict = resolved_func.as_dict()?;
        let func_type = func_dict.get("FunctionType").and_then(|o| o.as_integer())?;
        let to_components = |arr: &[Object]| -> Vec<f32> {
            arr.iter()
                .map(|o| match o {
                    Object::Real(v) => *v as f32,
                    Object::Integer(v) => *v as f32,
                    _ => 0.0,
                })
                .collect()
        };
        let (c0_comps, c1_comps) = match func_type {
            2 => {
                // Type 2: exponential interpolation
                // f(x) = C0 + x^N * (C1 - C0).
                // The shading's geometric `t=0` evaluates `f(Domain[0])`
                // and `t=1` evaluates `f(Domain[1])`, so when /Domain
                // is non-default the endpoint colours are NOT raw /C0
                // and /C1. ~keep
                let c0 = to_components(func_dict.get("C0").and_then(|o| o.as_array())?);
                let c1 = to_components(func_dict.get("C1").and_then(|o| o.as_array())?);
                let n = func_dict
                    .get("N")
                    .and_then(|o| match o {
                        Object::Real(v) => Some(*v as f32),
                        Object::Integer(v) => Some(*v as f32),
                        _ => None,
                    })
                    .unwrap_or(1.0);
                let eval = |x: f32| -> Vec<f32> {
                    let p = x.abs().powf(n) * x.signum();
                    c0.iter().zip(c1.iter()).map(|(a, b)| *a + p * (*b - *a)).collect()
                };
                (eval(domain0), eval(domain1))
            }
            3 => {
                // Type 3: stitching. The shading's `/Domain` maps to a
                // sub-function via stitching `/Bounds` and `/Encode`
                // arrays. The current path takes the first
                // sub-function's `/C0` and the last sub-function's
                // `/C1` at face value — correct for the default
                // `Domain [0 1]` with natural `Encode`, but ignores
                // `Encode`-driven sub-domain remapping. Documented gap. ~keep
                let funcs = func_dict.get("Functions").and_then(|o| o.as_array())?;
                let first = funcs.first()?;
                let last = funcs.last().unwrap_or(first);
                let first_resolved = doc.resolve_object(first).ok()?;
                let last_resolved = doc.resolve_object(last).ok()?;
                let first_dict = first_resolved.as_dict()?;
                let last_dict = last_resolved.as_dict()?;
                let c0 = first_dict.get("C0").and_then(|o| o.as_array())?;
                let c1 = last_dict.get("C1").and_then(|o| o.as_array())?;
                (to_components(c0), to_components(c1))
            }
            // Function types 0 (sampled) and 4 (PostScript Type 4
            // calculator) used as the shading's own /Function are
            // out-of-scope for endpoint pre-resolution — they produce
            // colours at intermediate domain points, not at two fixed
            // /C0 / /C1 arrays. Caller falls back to inline. ~keep
            _ => return None,
        };

        // Fold in `gs.fill_alpha` here — it's the alpha the inline
        // code path multiplies into each gradient stop's RGBA when
        // building the tiny-skia LinearGradient / RadialGradient. ~keep
        let c0 = self.pipeline_resolve_components(doc, &self.color_spaces, &resolved_cs, &c0_comps, gs.fill_alpha)?;
        let c1 = self.pipeline_resolve_components(doc, &self.color_spaces, &resolved_cs, &c1_comps, gs.fill_alpha)?;
        Some((c0, c1))
    }

    /// Render axial (linear) gradient shading (Type 2).
    ///
    /// `resolved_endpoints`, when `Some`, supplies pre-resolved RGBA
    /// values for the two gradient stops with `gs.fill_alpha` already
    /// folded in — the resolution-pipeline route produced by
    /// [`Self::pipeline_resolve_shading_endpoints`]. When `None`, the
    /// function falls back to a black-to-white default
    /// (the safety net the legacy inline path used as its outermost
    /// fallback before wave 5).
    fn render_axial_shading(
        &self,
        pixmap: &mut Pixmap,
        shading: &std::collections::HashMap<String, Object>,
        transform: Transform,
        gs: &GraphicsState,
        clip_mask: Option<&tiny_skia::Mask>,
        resolved_endpoints: Option<((f32, f32, f32, f32), (f32, f32, f32, f32))>,
    ) -> Result<()> {
        let coords = shading.get("Coords").and_then(|o| o.as_array());
        let coords = match coords {
            Some(c) if c.len() >= 4 => c,
            _ => return Ok(()),
        };
        let get_f = |i: usize| -> f32 {
            match &coords[i] {
                Object::Real(v) => *v as f32,
                Object::Integer(v) => *v as f32,
                _ => 0.0,
            }
        };
        let (x0, y0, x1, y1) = (get_f(0), get_f(1), get_f(2), get_f(3));

        let extend = shading.get("Extend").and_then(|o| o.as_array());
        let (extend_start, extend_end) = if let Some(ext) = extend {
            let e0 = ext.get(0).map(|o| matches!(o, Object::Boolean(true))).unwrap_or(false);
            let e1 = ext.get(1).map(|o| matches!(o, Object::Boolean(true))).unwrap_or(false);
            (e0, e1)
        } else {
            (false, false)
        };

        // Build the two gradient-stop RGBAs from the pipeline's
        // pre-resolved endpoint pair. When the resolver cannot produce
        // an answer (missing /Function, unsupported sub-function type,
        // non-RGBA resolver output) fall back to the
        // black-to-white default that matches the legacy renderer's
        // safety net — render with sensible defaults rather than
        // panicking or rendering nothing. ~keep
        let (stop0, stop1) = match resolved_endpoints {
            Some(((r0, g0, b0, a0), (r1, g1, b1, a1))) => ((r0, g0, b0, a0), (r1, g1, b1, a1)),
            None => ((0.0, 0.0, 0.0, gs.fill_alpha), (1.0, 1.0, 1.0, gs.fill_alpha)),
        };

        let mut p0 = tiny_skia::Point { x: x0, y: y0 };
        let mut p1 = tiny_skia::Point { x: x1, y: y1 };
        transform.map_point(&mut p0);
        transform.map_point(&mut p1);

        // Per ISO 32000-1 §8.7.4.5.3 the `/Extend` array names whether
        // the gradient paints past its geometric endpoints with the
        // adjacent stop colour. tiny-skia's `SpreadMode::Pad` is the
        // `[true true]` behaviour. For the other three combinations
        // the area past the unwanted side must not be painted at all,
        // so we build an extra clip path from the gradient slab and
        // intersect it with the inherited `clip_mask`. ~keep
        let spread = tiny_skia::SpreadMode::Pad;

        // Build an axis-perpendicular slab clip when at least one side
        // is `false`. The slab is the strip between the two
        // perpendicular lines through `p0` and `p1`; for asymmetric
        // `/Extend`, one side of the strip is the page boundary, the
        // other is the perpendicular. ~keep
        let slab_clip_mask = build_axial_extend_clip(pixmap, p0, p1, extend_start, extend_end, clip_mask);
        let effective_clip = slab_clip_mask.as_ref().or(clip_mask);

        let gradient = tiny_skia::LinearGradient::new(
            tiny_skia::Point { x: p0.x, y: p0.y },
            tiny_skia::Point { x: p1.x, y: p1.y },
            vec![
                tiny_skia::GradientStop::new(
                    0.0,
                    tiny_skia::Color::from_rgba(stop0.0, stop0.1, stop0.2, stop0.3).unwrap_or(tiny_skia::Color::BLACK),
                ),
                tiny_skia::GradientStop::new(
                    1.0,
                    tiny_skia::Color::from_rgba(stop1.0, stop1.1, stop1.2, stop1.3).unwrap_or(tiny_skia::Color::BLACK),
                ),
            ],
            spread,
            Transform::identity(),
        );

        if let Some(shader) = gradient {
            let mut paint = tiny_skia::Paint::default();
            paint.shader = shader;
            paint.anti_alias = true;

            let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, pixmap.width() as f32, pixmap.height() as f32).unwrap();
            let path = PathBuilder::from_rect(rect);
            pixmap.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                effective_clip,
            );
            tracing::trace!(
                "Rendered axial gradient from ({:.1},{:.1}) to ({:.1},{:.1})",
                p0.x,
                p0.y,
                p1.x,
                p1.y
            );
        }

        Ok(())
    }

    /// Render radial gradient shading (Type 3).
    ///
    /// `resolved_endpoints`, when `Some`, supplies pre-resolved RGBA
    /// values for the two gradient stops with `gs.fill_alpha` already
    /// folded in — the resolution-pipeline route produced by
    /// [`Self::pipeline_resolve_shading_endpoints`]. When `None`, the
    /// function falls back to a black-to-white default (the safety net
    /// the legacy inline path used as its outermost fallback before
    /// wave 5).
    fn render_radial_shading(
        &self,
        pixmap: &mut Pixmap,
        shading: &std::collections::HashMap<String, Object>,
        transform: Transform,
        gs: &GraphicsState,
        clip_mask: Option<&tiny_skia::Mask>,
        resolved_endpoints: Option<((f32, f32, f32, f32), (f32, f32, f32, f32))>,
    ) -> Result<()> {
        let coords = shading.get("Coords").and_then(|o| o.as_array());
        let coords = match coords {
            Some(c) if c.len() >= 6 => c,
            _ => return Ok(()),
        };
        let get_f = |i: usize| -> f32 {
            match &coords[i] {
                Object::Real(v) => *v as f32,
                Object::Integer(v) => *v as f32,
                _ => 0.0,
            }
        };
        let (x0, y0, r0, x1, y1, r1) = (get_f(0), get_f(1), get_f(2), get_f(3), get_f(4), get_f(5));

        let extend = shading.get("Extend").and_then(|o| o.as_array());
        let (extend_start, extend_end) = if let Some(ext) = extend {
            let e0 = ext.first().map(|o| matches!(o, Object::Boolean(true))).unwrap_or(false);
            let e1 = ext.get(1).map(|o| matches!(o, Object::Boolean(true))).unwrap_or(false);
            (e0, e1)
        } else {
            (false, false)
        };

        let (stop0, stop1) = match resolved_endpoints {
            Some(((r0c, g0, b0, a0), (r1c, g1, b1, a1))) => ((r0c, g0, b0, a0), (r1c, g1, b1, a1)),
            None => ((0.0, 0.0, 0.0, gs.fill_alpha), (1.0, 1.0, 1.0, gs.fill_alpha)),
        };

        // Per ISO 32000-1 §8.7.4.5.4, the radial gradient interpolates
        // between two circles `(x0, y0, r0)` (the inner / start circle,
        // mapped to the function value at the gradient's `Domain[0]`)
        // and `(x1, y1, r1)` (the outer / end circle, mapped to
        // `Domain[1]`). When `(x0, y0) == (x1, y1)` and `r0 == 0` the
        // result is a familiar centred radial; non-concentric inputs
        // produce off-centre / cone gradients that real PDFs use for
        // highlight, spotlight, and lens effects. ~keep
        let mut center0 = tiny_skia::Point { x: x0, y: y0 };
        let mut edge0 = tiny_skia::Point { x: x0 + r0, y: y0 };
        let mut center1 = tiny_skia::Point { x: x1, y: y1 };
        let mut edge1 = tiny_skia::Point { x: x1 + r1, y: y1 };
        transform.map_point(&mut center0);
        transform.map_point(&mut edge0);
        transform.map_point(&mut center1);
        transform.map_point(&mut edge1);
        let radius0 = ((edge0.x - center0.x).powi(2) + (edge0.y - center0.y).powi(2)).sqrt();
        let radius1 = ((edge1.x - center1.x).powi(2) + (edge1.y - center1.y).powi(2)).sqrt();

        // Per ISO 32000-1 §8.7.4.5.4 the `/Extend` array names whether
        // the gradient paints past the start (inner) and end (outer)
        // circles with the adjacent stop colour. tiny-skia's
        // `SpreadMode::Pad` is the `[true true]` behaviour; for any
        // `false` side we need an explicit clip. For the common
        // `r0 < r1` case `Extend[1]=false` clips outside the outer
        // circle and `Extend[0]=false` clips inside the inner circle. ~keep
        let radial_clip_mask = build_radial_extend_clip(
            pixmap,
            (center0, radius0),
            (center1, radius1),
            extend_start,
            extend_end,
            clip_mask,
        );
        let effective_clip = radial_clip_mask.as_ref().or(clip_mask);

        let gradient = tiny_skia::RadialGradient::new(
            tiny_skia::Point {
                x: center0.x,
                y: center0.y,
            },
            radius0, // start_radius (inner circle, in device space) ~keep
            tiny_skia::Point {
                x: center1.x,
                y: center1.y,
            },
            radius1, // end_radius (outer circle, in device space) ~keep
            vec![
                tiny_skia::GradientStop::new(
                    0.0,
                    tiny_skia::Color::from_rgba(stop0.0, stop0.1, stop0.2, stop0.3).unwrap_or(tiny_skia::Color::BLACK),
                ),
                tiny_skia::GradientStop::new(
                    1.0,
                    tiny_skia::Color::from_rgba(stop1.0, stop1.1, stop1.2, stop1.3).unwrap_or(tiny_skia::Color::BLACK),
                ),
            ],
            tiny_skia::SpreadMode::Pad,
            Transform::identity(),
        );

        if let Some(shader) = gradient {
            let mut paint = tiny_skia::Paint::default();
            paint.shader = shader;
            paint.anti_alias = true;
            let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, pixmap.width() as f32, pixmap.height() as f32).unwrap();
            let path = PathBuilder::from_rect(rect);
            pixmap.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                effective_clip,
            );
            tracing::trace!(
                "Rendered radial gradient from ({:.1},{:.1}) r={:.1} to ({:.1},{:.1}) r={:.1}",
                center0.x,
                center0.y,
                radius0,
                center1.x,
                center1.y,
                radius1,
            );
        }

        Ok(())
    }

    /// Render an XObject (image or form).
    /// Resolve the `/Subtype` name of the named XObject in the active
    /// resources without rendering it. Returns `Some("Form")`,
    /// `Some("Image")`, etc., or `None` when the lookup fails or the
    /// XObject lacks a `/Subtype`. Used by the `Do` operator dispatcher
    /// to pick the correct post-Do colour-lane modulators per ISO
    /// 32000-1 §11.4.7 (Image XObjects paint with outer gs; Form
    /// XObjects run their own operators with their own gs).
    fn xobject_subtype(&self, name: &str, resources: &Object, doc: &PdfDocument) -> Option<String> {
        let res_dict = resources.as_dict()?;
        let xobj_entry = res_dict.get("XObject")?;
        let xobjects_obj = doc.resolve_object(xobj_entry).ok()?;
        let xobjects = xobjects_obj.as_dict()?;
        let xobj_ref_obj = xobjects.get(name)?;
        let xobj = doc.resolve_object(xobj_ref_obj).ok()?;
        if let Object::Stream { ref dict, .. } = xobj {
            return dict.get("Subtype").and_then(|o| o.as_name()).map(String::from);
        }
        None
    }

    fn render_xobject(
        &mut self,
        pixmap: &mut Pixmap,
        name: &str,
        transform: Transform,
        gs: &GraphicsState,
        resources: &Object,
        doc: &PdfDocument,
        page_num: usize,
        clip_mask: Option<&tiny_skia::Mask>,
    ) -> Result<()> {
        if let Object::Dictionary(res_dict) = resources {
            // PDF spec uses "XObject" (singular) ~keep
            if let Some(xobj_entry) = res_dict.get("XObject") {
                let xobjects_obj = doc.resolve_object(xobj_entry)?;
                if let Some(xobjects) = xobjects_obj.as_dict() {
                    if let Some(xobj_ref_obj) = xobjects.get(name) {
                        let xobj = doc.resolve_object(xobj_ref_obj)?;
                        let xobj_ref = xobj_ref_obj.as_reference();
                        tracing::trace!("Resolved XObject '{}' type: {:?}", name, xobj);

                        if let Object::Stream { ref dict, .. } = xobj {
                            if let Some(smask) = dict.get("SMask") {
                                tracing::trace!("Image has SMask: {:?}", smask);
                            }
                            if let Some(mask) = dict.get("Mask") {
                                tracing::trace!("Image has Mask: {:?}", mask);
                            }
                            if let Some(imask) = dict.get("ImageMask") {
                                tracing::trace!("Image is ImageMask: {:?}", imask);
                            }
                            if let Some(subtype) = dict.get("Subtype").and_then(|o| o.as_name()) {
                                match subtype {
                                    "Image" => {
                                        // ImageMask XObjects (1-bit stencil painted with
                                        // the current fill colour) take their fill from
                                        // graphics state, not from the pixel data. Route
                                        // that fill through the resolution pipeline so a
                                        // Type 4 Separation fill paints the mask with the
                                        // function-evaluated tint rather than the legacy
                                        // `1 - tint` fallback.
                                        //
                                        // Standard images (`/ImageMask` absent or false)
                                        // carry their colour in the pixel data and do
                                        // not interact with the pipeline; they pass
                                        // straight through to `render_image`. ~keep
                                        let is_image_mask = dict
                                            .get("ImageMask")
                                            .map(|o| matches!(o, Object::Boolean(true)))
                                            .unwrap_or(false);
                                        if is_image_mask {
                                            let spliced =
                                                self.pipeline_resolve_paint_gs(doc, gs, PipelinePaintKind::ImageMask);
                                            let render_gs: &GraphicsState = spliced.as_ref().unwrap_or(gs);
                                            if let Err(error) = self.render_image_mask(
                                                pixmap, &xobj, xobj_ref, transform, doc, clip_mask, render_gs,
                                            ) {
                                                tracing::warn!(
                                                    error_code = error.telemetry_code(),
                                                    error_offset = ?error.telemetry_offset(),
                                                    "skipping unrenderable ImageMask XObject"
                                                );
                                            }
                                        } else {
                                            let smask = dict.get("SMask").cloned();
                                            let mask = dict.get("Mask").cloned();
                                            if let Err(error) = self.render_image(
                                                pixmap, &xobj, xobj_ref, transform, doc, clip_mask, smask, mask, gs,
                                            ) {
                                                tracing::warn!(
                                                    error_code = error.telemetry_code(),
                                                    error_offset = ?error.telemetry_offset(),
                                                    "skipping unrenderable image XObject"
                                                );
                                            }
                                        }
                                    }
                                    "Form" => {
                                        tracing::trace!("XObject '{}' is a Form", name);
                                        let stream_data = if let Some(r) = xobj_ref {
                                            doc.decode_stream_with_encryption(&xobj, r)?
                                        } else {
                                            xobj.decode_stream_data()?
                                        };

                                        let form_resources = dict.get("Resources").unwrap_or(resources);

                                        let old_fonts = self.fonts.clone();
                                        let old_cs = self.color_spaces.clone();
                                        self.load_resources(doc, form_resources)?;

                                        if let Err(error) = self.render_form_xobject(
                                            pixmap,
                                            &dict,
                                            &stream_data,
                                            transform,
                                            doc,
                                            page_num,
                                            form_resources,
                                        ) {
                                            tracing::warn!(
                                                error_code = error.telemetry_code(),
                                                error_offset = ?error.telemetry_offset(),
                                                "skipping malformed Form XObject"
                                            );
                                        }

                                        self.fonts = old_fonts;
                                        self.color_spaces = old_cs;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Render an image XObject.
    fn render_image(
        &mut self,
        pixmap: &mut Pixmap,
        xobject: &Object,
        obj_ref: Option<ObjectRef>,
        transform: Transform,
        doc: &PdfDocument,
        clip_mask: Option<&tiny_skia::Mask>,
        smask_obj: Option<Object>,
        mask_obj: Option<Object>,
        gs: &GraphicsState,
    ) -> Result<()> {
        use crate::extractors::images::extract_image_from_xobject;

        let color_space_map = self.color_spaces.clone();
        let pdf_image = extract_image_from_xobject(Some(doc), xobject, obj_ref, Some(&color_space_map))?;
        let dynamic_image = pdf_image.to_dynamic_image()?;
        let mut rgba_image = dynamic_image.to_rgba8();

        // A referenced ImageMask uses stencil polarity: default /Decode [0 1]
        // makes sample 0 opaque, while [1 0] reverses it. ~keep
        if let Some(mask_ref) = mask_obj {
            if let Some(ref_obj) = mask_ref.as_reference() {
                if let Ok(mask_stream) = doc.load_object(ref_obj) {
                    match extract_image_from_xobject(Some(doc), &mask_stream, Some(ref_obj), Some(&color_space_map)) {
                        Ok(mask_image) => {
                            if let Ok(mask_dyn) = mask_image.to_dynamic_image() {
                                let mask_gray = mask_dyn.to_luma8();
                                let is_image_mask = mask_stream
                                    .as_dict()
                                    .and_then(|dict| dict.get("ImageMask"))
                                    .is_some_and(|value| matches!(value, Object::Boolean(true)));
                                let mw = mask_gray.width();
                                let mh = mask_gray.height();
                                let iw = rgba_image.width();
                                let ih = rgba_image.height();
                                if mw == 0 || mh == 0 {
                                    // No mask sample to test: leave the base
                                    // image fully opaque rather than guessing. ~keep
                                    tracing::warn!(
                                        "Ignoring image Mask: it is {mw}x{mh} and carries \
                                         no sample to test"
                                    );
                                } else {
                                    for y in 0..ih {
                                        let my = ((y as u64 * mh as u64 / ih as u64) as u32).min(mh - 1);
                                        for x in 0..iw {
                                            let mx = ((x as u64 * mw as u64 / iw as u64) as u32).min(mw - 1);
                                            let sample = mask_gray.get_pixel(mx, my)[0];
                                            // Extracted ImageMask luma is black for the painting sample,
                                            // including after an inverted /Decode is folded in. Alpha has
                                            // the opposite polarity: black ink must be fully opaque. ~keep
                                            let mask_val = if is_image_mask { 255 - sample } else { sample };
                                            let pixel = rgba_image.get_pixel_mut(x, y);
                                            pixel[3] = ((pixel[3] as u32 * mask_val as u32) / 255) as u8;
                                        }
                                    }
                                    tracing::trace!("Applied image Mask ({}x{}) to image ({}x{})", mw, mh, iw, ih);
                                }
                            }
                        }
                        Err(_) => {
                            if let Object::Stream { ref dict, .. } = mask_stream {
                                let mask_dict = dict;
                                let is_image_mask = mask_dict
                                    .get("ImageMask")
                                    .map(|o| matches!(o, Object::Boolean(true)))
                                    .unwrap_or(false);
                                if is_image_mask {
                                    // /Width and /Height may be indirect references
                                    // (ISO 32000-1 §7.3.10); resolve them the same way
                                    // `extract_image_from_xobject` does, or an indirect
                                    // dimension reads as 0 here, the `mw > 0 && mh > 0`
                                    // guard below fails, and the mask is silently
                                    // skipped with no warning at all. ~keep
                                    let resolve_int = |o: &Object| -> Option<i64> {
                                        match o.as_reference() {
                                            Some(r) => doc.load_object(r).ok().and_then(|v| v.as_integer()),
                                            None => o.as_integer(),
                                        }
                                    };
                                    // `as u32` turns a negative /Width into a
                                    // near-u32::MAX stride that passes the
                                    // check below. ~keep
                                    let mw = mask_dict
                                        .get("Width")
                                        .and_then(resolve_int)
                                        .and_then(|v| u32::try_from(v).ok())
                                        .unwrap_or(0);
                                    let mh = mask_dict
                                        .get("Height")
                                        .and_then(resolve_int)
                                        .and_then(|v| u32::try_from(v).ok())
                                        .unwrap_or(0);
                                    if mw > 0 && mh > 0 {
                                        if let Ok(raw_mask_data) =
                                            doc.decode_stream_with_encryption(&mask_stream, ref_obj)
                                        {
                                            // CCITT data may be pass-through (not decompressed).
                                            // Check if we need to decompress Group 4 CCITT. ~keep
                                            let expected_bytes = ((mw as usize + 7) / 8) * mh as usize;
                                            let mask_data = if raw_mask_data.len() < expected_bytes / 2 {
                                                let k = mask_dict
                                                    .get("DecodeParms")
                                                    .and_then(|o| o.as_dict())
                                                    .and_then(|d| d.get("K"))
                                                    .and_then(|o| o.as_integer())
                                                    .unwrap_or(0);
                                                if k == -1 {
                                                    #[allow(deprecated)]
                                                    let ccitt_result =
                                                        crate::extractors::ccitt_bilevel::decompress_ccitt_group4(
                                                            &raw_mask_data,
                                                            mw,
                                                            mh,
                                                        );
                                                    match ccitt_result {
                                                        Ok(decompressed) => {
                                                            tracing::trace!(
                                                                "CCITT Group4 decompressed mask: {} → {} bytes",
                                                                raw_mask_data.len(),
                                                                decompressed.len()
                                                            );
                                                            decompressed
                                                        }
                                                        Err(error) => {
                                                            trace_ccitt_mask_decode_failure(&error);
                                                            raw_mask_data
                                                        }
                                                    }
                                                } else {
                                                    raw_mask_data
                                                }
                                            } else {
                                                raw_mask_data
                                            };
                                            // 1-bit mask: each byte has 8 pixels, MSB first ~keep
                                            let iw = rgba_image.width();
                                            let ih = rgba_image.height();
                                            let row_bytes = (mw as usize + 7) / 8;
                                            let inverted_decode = mask_dict
                                                .get("Decode")
                                                .and_then(Object::as_array)
                                                .and_then(|decode| decode.first())
                                                .is_some_and(|first| match first {
                                                    Object::Integer(value) => *value > 0,
                                                    Object::Real(value) => *value > 0.5,
                                                    _ => false,
                                                });
                                            for y in 0..ih {
                                                let my =
                                                    ((y as u64 * mh as u64 / ih as u64) as u32).min(mh - 1) as usize;
                                                for x in 0..iw {
                                                    let mx = ((x as u64 * mw as u64 / iw as u64) as u32).min(mw - 1)
                                                        as usize;
                                                    let byte_idx = my * row_bytes + mx / 8;
                                                    let bit_idx = 7 - (mx % 8);
                                                    // ImageMask default /Decode maps sample 0 to paint;
                                                    // [1 0] reverses that polarity. ~keep
                                                    let mask_val = if byte_idx < mask_data.len() {
                                                        let bit = (mask_data[byte_idx] >> bit_idx) & 1 == 1;
                                                        if if inverted_decode { bit } else { !bit } {
                                                            255u8
                                                        } else {
                                                            0u8
                                                        }
                                                    } else {
                                                        255u8
                                                    };
                                                    let pixel = rgba_image.get_pixel_mut(x, y);
                                                    pixel[3] = ((pixel[3] as u32 * mask_val as u32) / 255) as u8;
                                                }
                                            }
                                            tracing::trace!(
                                                "Applied stencil ImageMask ({}x{}) to image ({}x{})",
                                                mw,
                                                mh,
                                                iw,
                                                ih
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if let Object::Array(mask_array) = &mask_ref {
                // Colour-key masking (ISO 32000-1 §8.9.6.4): the /Mask is an
                // array of 2 × ncomp integers [min1 max1 min2 max2 ...] in the
                // image's pre-Decode colour-component space. A source pixel whose
                // raw component samples all fall within their [min,max] range is
                // made fully transparent. ~keep
                let ncomp = pdf_image.color_space().components();
                if *pdf_image.color_space() == crate::extractors::images::ColorSpace::Indexed {
                    // An Indexed image's colour-key range is stated in its raw
                    // INDEX space (ncomp=1), not the palette-expanded RGB
                    // `samples_are_raw` tracks -- that flag is always false here
                    // once the palette lookup runs, which used to make this
                    // masking unreachable. The extractor keeps the index plane
                    // alongside the RGB precisely so this can read it. (GH#1899) ~keep
                    match (pdf_image.raw_indexed_samples(), parse_color_key_mask(mask_array, ncomp)) {
                        (Some(indices), Some(ranges)) => {
                            apply_color_key_mask_to_indices(indices, &ranges, &mut rgba_image);
                        }
                        (None, _) => {
                            tracing::warn!(
                                "Skipping colour-key /Mask: no raw index plane recorded for this \
                                 Indexed image (JPX-coded Indexed images are not yet covered)"
                            );
                        }
                        (_, None) => {
                            tracing::warn!("Ignoring malformed color-key /Mask array (ncomp={})", ncomp);
                        }
                    }
                } else if pdf_image.color_key_samples().is_none() && !pdf_image.samples_are_raw() {
                    // The extractor mapped a non-default /Decode into the
                    // stored samples, so they are no longer in the space the
                    // /Mask ranges are expressed in. Masking them here would
                    // hide the wrong pixels; skip and say so rather than
                    // silently inverting the transparent region. ~keep
                    tracing::warn!(
                        "Skipping colour-key /Mask: the stored samples are no longer in the \
                         raw sample space the mask ranges are expressed in"
                    );
                } else {
                    match parse_color_key_mask(mask_array, ncomp) {
                        Some(ranges) => {
                            apply_color_key_mask(&pdf_image, &ranges, &mut rgba_image);
                        }
                        None => {
                            tracing::warn!("Ignoring malformed color-key /Mask array (ncomp={})", ncomp);
                        }
                    }
                }
            }
        }

        if let Some(smask_ref) = smask_obj {
            if let Ok(resolved_smask) = doc.resolve_object(&smask_ref) {
                let smask_obj_ref = smask_ref.as_reference();
                if let Ok(smask_image) =
                    extract_image_from_xobject(Some(doc), &resolved_smask, smask_obj_ref, Some(&color_space_map))
                {
                    if let Ok(smask_dyn) = smask_image.to_dynamic_image() {
                        let smask_gray = smask_dyn.to_luma8();

                        // Apply SMask to alpha channel
                        // Rescale smask if dimensions don't match (simplification) ~keep
                        let sw = smask_gray.width();
                        let sh = smask_gray.height();
                        let iw = rgba_image.width();
                        let ih = rgba_image.height();

                        for y in 0..ih {
                            for x in 0..iw {
                                let sx = (x * sw / iw).min(sw - 1);
                                let sy = (y * sh / ih).min(sh - 1);
                                let alpha = smask_gray.get_pixel(sx, sy)[0];

                                let pixel = rgba_image.get_pixel_mut(x, y);
                                pixel[3] = ((pixel[3] as u32 * alpha as u32) / 255) as u8;
                            }
                        }
                    }
                }
            }
        }

        // The extractor keeps the opacity channel only when the image has no /SMask, so the
        // two soft masks never both apply. ~keep
        let mut color_is_premultiplied = false;
        if let Some(soft_mask) = pdf_image.soft_mask_in_data() {
            let opacity = soft_mask.samples();
            if opacity.len() == rgba_image.width() as usize * rgba_image.height() as usize {
                for (pixel, &alpha) in rgba_image.pixels_mut().zip(opacity) {
                    if soft_mask.is_premultiplied() {
                        let prior_alpha = u32::from(pixel[3]);
                        for component in &mut pixel.0[..3] {
                            *component = ((u32::from(*component) * prior_alpha) / 255) as u8;
                        }
                    }
                    pixel[3] = ((u32::from(pixel[3]) * u32::from(alpha)) / 255) as u8;
                }
                color_is_premultiplied = soft_mask.is_premultiplied();
            } else {
                tracing::warn!("Ignoring /SMaskInData: the opacity channel does not match the image size");
            }
        }

        // A tiny_skia pixmap holds premultiplied RGBA. `/SMaskInData 2` already supplies colour
        // premultiplied by its opacity; every other decoded image needs the conversion here. ~keep
        if !color_is_premultiplied {
            premultiply_rgba(&mut rgba_image);
        }

        let src_w = rgba_image.width();
        let src_h = rgba_image.height();

        let image_transform = image_unit_square_transform(transform, src_w, src_h);
        let mut paint = pixmap_paint_for_image_blit(image_transform, gs.fill_alpha, &gs.blend_mode);

        // Fast path: SIMD pre-resize when the transform is a pure scale+translate and
        // the image is being downscaled.  fast_image_resize (AVX2/SSE4.1/NEON) resizes
        // to exact output dimensions; we then blit the already-correct pixels at the
        // right position with a translate-only transform and Nearest quality (no second
        // resampling pass).  For rotated/sheared transforms or upscaling, fall through
        // to the tiny-skia bilinear/bicubic path (already selected by the helper above). ~keep
        let use_fast = image_transform.kx.abs() <= 1e-4
            && image_transform.ky.abs() <= 1e-4
            && image_transform.sx > 0.0
            && image_transform.sy > 0.0
            && (image_transform.sx < 0.9 || image_transform.sy < 0.9);

        let (blit_w, blit_h, blit_data, blit_transform) = if use_fast {
            let dst_w = ((image_transform.sx * src_w as f32).round() as u32).max(1);
            let dst_h = ((image_transform.sy * src_h as f32).round() as u32).max(1);
            let resized = resize_rgba(rgba_image.as_raw(), src_w, src_h, dst_w, dst_h);
            if let Some(pixels) = resized {
                // SIMD pre-resize produced the exact output dimensions —
                // the subsequent blit is 1:1, so override to Nearest to
                // skip a second resampling pass. ~keep
                paint.quality = tiny_skia::FilterQuality::Nearest;
                let t = Transform::from_translate(image_transform.tx, image_transform.ty);
                (dst_w, dst_h, pixels, t)
            } else {
                // fast_image_resize failed; fall back to tiny_skia
                // resampling with the helper's chosen quality. ~keep
                (src_w, src_h, rgba_image.into_raw(), image_transform)
            }
        } else {
            (src_w, src_h, rgba_image.into_raw(), image_transform)
        };

        if let Some(img_pixmap) =
            tiny_skia::IntSize::from_wh(blit_w, blit_h).and_then(|size| Pixmap::from_vec(blit_data, size))
        {
            pixmap.draw_pixmap(0, 0, img_pixmap.as_ref(), &paint, blit_transform, clip_mask);
        }

        Ok(())
    }

    /// Return CCITT parameters for an ImageMask stream, if present.
    ///
    /// `/DecodeParms` arrays are positionally aligned with `/Filter`
    /// arrays. CCITT must be the final filter: earlier ASCII/Flate wrappers
    /// have already been removed by `decode_stream_with_encryption`, while a
    /// later filter would incorrectly receive still-compressed fax data from
    /// the stream decoder's intentional CCITT pass-through.
    /// Is this ImageMask's final filter `JBIG2Decode`?
    ///
    /// Mirrors `image_mask_ccitt_params`' filter handling: the codec must be last, since
    /// anything after it would operate on decoded pixels rather than the bitstream.
    fn image_mask_is_jbig2(dict: &HashMap<String, Object>) -> bool {
        match dict.get("Filter") {
            Some(Object::Name(name)) => name == "JBIG2Decode",
            Some(Object::Array(filters)) => filters
                .last()
                .and_then(Object::as_name)
                .is_some_and(|name| name == "JBIG2Decode"),
            _ => false,
        }
    }

    fn image_mask_ccitt_params(
        dict: &HashMap<String, Object>,
        width: u32,
        height: u32,
        doc: &PdfDocument,
    ) -> Result<Option<crate::decoders::CcittParams>> {
        let (ccitt_index, filter_count) = match dict.get("Filter") {
            Some(Object::Name(name)) if Self::is_ccitt_filter(name) => (0, 1),
            Some(Object::Array(filters)) => {
                let Some(index) = filters
                    .iter()
                    .position(|filter| filter.as_name().is_some_and(Self::is_ccitt_filter))
                else {
                    return Ok(None);
                };
                (index, filters.len())
            }
            _ => return Ok(None),
        };

        if ccitt_index + 1 != filter_count {
            return Err(Error::Image(
                "CCITTFaxDecode must be the final ImageMask filter".to_string(),
            ));
        }
        if width > u16::MAX as u32 {
            return Err(Error::Image(format!(
                "CCITT ImageMask width {width} exceeds decoder limit {}",
                u16::MAX
            )));
        }

        let resolved_params = dict
            .get("DecodeParms")
            .map(|params| {
                let resolved = doc.resolve_object(params).map_err(|error| {
                    Error::Image(format!("Unable to resolve CCITT ImageMask /DecodeParms: {error}"))
                })?;
                if params.as_reference().is_some() && matches!(&resolved, Object::Null) {
                    return Err(Error::Image(
                        "Unable to resolve CCITT ImageMask /DecodeParms: reference resolved to null".to_string(),
                    ));
                }
                Ok(resolved)
            })
            .transpose()?;
        let selected_params = match resolved_params {
            Some(Object::Array(params)) => params
                .get(ccitt_index)
                .map(|params| {
                    let resolved = doc.resolve_object(params).map_err(|error| {
                        Error::Image(format!(
                            "Unable to resolve CCITT ImageMask /DecodeParms array entry: {error}"
                        ))
                    })?;
                    if params.as_reference().is_some() && matches!(&resolved, Object::Null) {
                        return Err(Error::Image(
                            "Unable to resolve CCITT ImageMask /DecodeParms array entry: reference resolved to null"
                                .to_string(),
                        ));
                    }
                    Ok(resolved)
                })
                .transpose()?,
            other => other,
        };
        let params_obj = selected_params.as_ref();
        let params_dict = match params_obj {
            None | Some(Object::Null) => None,
            Some(params) => Some(
                params
                    .as_dict()
                    .ok_or_else(|| Error::Image("CCITT ImageMask /DecodeParms must be a dictionary".to_string()))?,
            ),
        };
        if params_dict
            .and_then(|decode_params| decode_params.get("K"))
            .is_some_and(|value| value.as_integer().is_none())
        {
            return Err(Error::Image(
                "CCITT ImageMask /DecodeParms /K must be an integer".to_string(),
            ));
        }
        Self::validate_ccitt_mask_dimension(params_dict, "Columns", width)?;
        Self::validate_ccitt_mask_dimension(params_dict, "Rows", height)?;

        let mut params = crate::object::extract_ccitt_params_with_width(params_obj, Some(width)).unwrap_or_else(|| {
            crate::decoders::CcittParams {
                // ISO 32000-1 Table 11: absent /K defaults to Group 3 1-D. ~keep
                k: 0,
                columns: width,
                rows: Some(height),
                ..Default::default()
            }
        });
        let has_explicit_k = params_obj
            .and_then(Object::as_dict)
            .is_some_and(|decode_params| decode_params.contains_key("K"));
        if !has_explicit_k {
            // The shared parser currently carries a historical Group 4
            // default; enforce the PDF filter default at this renderer
            // boundary without changing other image-extraction behaviour. ~keep
            params.k = 0;
        }
        // Width and Height are authoritative for an Image XObject. Explicit
        // mismatches were rejected above; normalise omitted values here. ~keep
        params.columns = width;
        params.rows = Some(height);
        Ok(Some(params))
    }

    fn validate_ccitt_mask_dimension(params: Option<&HashMap<String, Object>>, key: &str, expected: u32) -> Result<()> {
        let Some(value) = params.and_then(|decode_params| decode_params.get(key)) else {
            return Ok(());
        };
        let integer = value
            .as_integer()
            .ok_or_else(|| Error::Image(format!("CCITT ImageMask /DecodeParms /{key} must be an integer")))?;
        let actual = u32::try_from(integer)
            .map_err(|_| Error::Image(format!("CCITT ImageMask /DecodeParms /{key} must be positive")))?;
        if actual == 0 {
            return Err(Error::Image(format!(
                "CCITT ImageMask /DecodeParms /{key} must be positive"
            )));
        }
        if actual != expected {
            return Err(Error::Image(format!(
                "CCITT ImageMask /DecodeParms /{key} {actual} does not match image dimension {expected}"
            )));
        }
        Ok(())
    }

    fn is_ccitt_filter(name: &str) -> bool {
        name.eq_ignore_ascii_case("CCITTFaxDecode") || name.eq_ignore_ascii_case("CCF")
    }

    /// `(width, height, row_bytes, packed_len, rgba_len)` for a 1-bpc stencil,
    /// rejecting geometry that does not fit or that no stream could back —
    /// before anything is allocated from it.
    ///
    /// /Width and /Height may be indirect references (ISO 32000-1 §7.3.10
    /// permits any dict entry to be one), so `dict.get(key).as_integer()`
    /// alone reads `None` for them; resolve the same way
    /// `extract_image_from_xobject` does for the non-mask image path. ~keep
    pub(crate) fn image_mask_layout(
        dict: &HashMap<String, Object>,
        doc: &PdfDocument,
    ) -> Result<(u32, u32, usize, usize, usize)> {
        let dimension = |key: &str| -> Result<u32> {
            let raw = dict
                .get(key)
                .ok_or_else(|| Error::Image(format!("ImageMask missing /{key}")))?;
            let value = match raw.as_reference() {
                Some(r) => doc.load_object(r).ok().and_then(|o| o.as_integer()),
                None => raw.as_integer(),
            }
            .ok_or_else(|| Error::Image(format!("ImageMask missing /{key}")))?;
            let dimension =
                u32::try_from(value).map_err(|_| Error::Image(format!("ImageMask /{key} must be positive")))?;
            if dimension == 0 {
                return Err(Error::Image(format!("ImageMask /{key} must be positive")));
            }
            Ok(dimension)
        };

        let width = dimension("Width")?;
        let height = dimension("Height")?;
        let width_usize =
            usize::try_from(width).map_err(|_| Error::Image("ImageMask /Width exceeds platform limits".to_string()))?;
        let height_usize = usize::try_from(height)
            .map_err(|_| Error::Image("ImageMask /Height exceeds platform limits".to_string()))?;
        let pixel_count = width_usize
            .checked_mul(height_usize)
            .ok_or_else(|| Error::Image("ImageMask pixel count overflow".to_string()))?;
        let rgba_len = pixel_count
            .checked_mul(4)
            .ok_or_else(|| Error::Image("ImageMask RGBA size overflow".to_string()))?;
        let row_bytes = width_usize
            .checked_add(7)
            .ok_or_else(|| Error::Image("ImageMask row size overflow".to_string()))?
            / 8;
        let expected = row_bytes
            .checked_mul(height_usize)
            .ok_or_else(|| Error::Image("ImageMask packed size overflow".to_string()))?;
        Ok((width, height, row_bytes, expected, rgba_len))
    }

    /// Render an Image XObject with `/ImageMask true` — a 1-bit stencil
    /// painted with the current fill colour.
    ///
    /// Per ISO 32000-1 §8.9.6.4, under the default `/Decode [0 1]` a
    /// sample value of `0` paints the destination with the current
    /// nonstroking colour and `1` leaves it unaffected; `/Decode [1 0]`
    /// reverses the polarity. There is no `/ColorSpace`; the colour
    /// comes from `gs.fill_color_rgb` / `gs.fill_alpha`. The caller (the
    /// `Do` arm in `render_page_with_options`) is responsible for
    /// routing that fill through the resolution pipeline, so this
    /// helper consumes whatever `gs` it is handed without re-resolving.
    ///
    /// Supports raw 1-bit samples and CCITT Group 3/4 streams, default
    /// and inverted `/Decode` polarities, and bilinear/bicubic resampling
    /// chosen by the image-space-to-user-space scale (matches
    /// `render_image`).
    fn render_image_mask(
        &mut self,
        pixmap: &mut Pixmap,
        xobject: &Object,
        obj_ref: Option<ObjectRef>,
        transform: Transform,
        doc: &PdfDocument,
        clip_mask: Option<&tiny_skia::Mask>,
        gs: &GraphicsState,
    ) -> Result<()> {
        let dict = xobject
            .as_dict()
            .ok_or_else(|| Error::Image("ImageMask XObject is not a stream".to_string()))?;

        let (width, height, row_bytes, expected, rgba_len) = Self::image_mask_layout(dict, doc)?;

        // PDF §8.9.6.4: ImageMask BitsPerComponent must be 1 when present.
        // Some producers omit it; default to 1. Others write it as an
        // indirect reference (ISO 32000-1 §7.3.10), which the plain
        // `as_integer()` read below would silently treat as absent rather
        // than validate — resolve it the same way /Width and /Height are. ~keep
        let resolve_int = |obj: &Object| -> Option<i64> {
            match obj.as_reference() {
                Some(r) => doc.load_object(r).ok().and_then(|o| o.as_integer()),
                None => obj.as_integer(),
            }
        };
        let bpc = dict.get("BitsPerComponent").and_then(resolve_int).unwrap_or(1);
        if bpc != 1 {
            return Err(Error::Image(format!(
                "ImageMask requires BitsPerComponent 1, got {bpc}"
            )));
        }

        // /Decode array: [0 1] means sample 0 paints (default); [1 0]
        // means sample 1 paints. Other forms are spec-illegal for ImageMask. ~keep
        let invert = match dict.get("Decode") {
            Some(Object::Array(arr)) if arr.len() >= 2 => {
                let first = match &arr[0] {
                    Object::Real(v) => *v as f32,
                    Object::Integer(v) => *v as f32,
                    _ => 0.0,
                };
                first > 0.5
            }
            _ => false,
        };

        let mut raw = if let Some(r) = obj_ref {
            doc.decode_stream_with_encryption(xobject, r)?
        } else {
            xobject.decode_stream_data()?
        };
        if Self::image_mask_is_jbig2(dict) {
            // JBIG2 is a compressed bitstream, so `raw` here is an order of magnitude
            // smaller than the packed bitmap the stencil loop expects; without decoding it
            // first, the length check below rejects every JBIG2 mask as "stream too short". ~keep
            raw = crate::extractors::images::decode_jbig2_image_mask_bits(
                xobject,
                obj_ref,
                dict,
                Some(doc),
                width,
                height,
            )?;
        } else if let Some(params) = Self::image_mask_ccitt_params(dict, width, height, doc)? {
            raw = crate::extractors::ccitt_bilevel::decompress_ccitt(&raw, &params)?;

            // The shared CCITT image decoder normalises packed rows to
            // 0=white, 1=black. An ImageMask needs the actual PDF sample
            // values instead: with the CCITT default `/BlackIs1 false`,
            // 0 means black and 1 means white; `/BlackIs1 true` reverses
            // that mapping. `decompress_ccitt` already applies BlackIs1
            // while normalising, so one final inversion recovers the sample
            // values consumed by the independent `/Decode` mapping below. ~keep
            for byte in &mut raw {
                *byte = !*byte;
            }
        }

        // Stencil pixels → premultiplied RGBA, applying the fill colour
        // to each opaque sample. Rows are packed MSB-first; each row is
        // padded to the next byte boundary. ~keep
        let (fr, fg, fb) = gs.fill_color_rgb;
        let fa = gs.fill_alpha.clamp(0.0, 1.0);
        let pa = (fa * 255.0).round().clamp(0.0, 255.0) as u8;
        // Premultiplied opaque sample: tiny-skia's Pixmap is
        // premultiplied; build the channels accordingly so blends and
        // SMask composition stay correct. ~keep
        let pr = ((fr.clamp(0.0, 1.0) * fa) * 255.0).round().clamp(0.0, 255.0) as u8;
        let pg = ((fg.clamp(0.0, 1.0) * fa) * 255.0).round().clamp(0.0, 255.0) as u8;
        let pb = ((fb.clamp(0.0, 1.0) * fa) * 255.0).round().clamp(0.0, 255.0) as u8;

        if raw.len() < expected {
            return Err(Error::Image(format!(
                "ImageMask stream too short: {} bytes for {}x{} (expected {})",
                raw.len(),
                width,
                height,
                expected
            )));
        }

        let mut rgba = Vec::new();
        rgba.try_reserve_exact(rgba_len)
            .map_err(|_| Error::Image(format!("Unable to allocate {rgba_len} ImageMask bytes")))?;
        rgba.resize(rgba_len, 0);
        for y in 0..height {
            let row_off = (y as usize) * row_bytes;
            for x in 0..width {
                let byte_idx = row_off + (x / 8) as usize;
                let bit_idx = 7 - (x % 8);
                let bit = (raw[byte_idx] >> bit_idx) & 1 == 1;
                let opaque = if invert { bit } else { !bit };
                if opaque {
                    let off = ((y as usize * width as usize) + x as usize) * 4;
                    rgba[off] = pr;
                    rgba[off + 1] = pg;
                    rgba[off + 2] = pb;
                    rgba[off + 3] = pa;
                }
            }
        }

        let image_transform = image_unit_square_transform(transform, width, height);
        // Opacity is 1.0 because fill_alpha is already baked into the
        // stencil pixels by the loop above; blend mode + scale-driven
        // quality come from the shared helper. ~keep
        let paint = pixmap_paint_for_image_blit(image_transform, 1.0, &gs.blend_mode);

        if let Some(stencil_pixmap) = Pixmap::from_vec(
            rgba,
            tiny_skia::IntSize::from_wh(width, height)
                .ok_or_else(|| Error::Image("ImageMask invalid dimensions".to_string()))?,
        ) {
            pixmap.draw_pixmap(0, 0, stencil_pixmap.as_ref(), &paint, image_transform, clip_mask);
        }

        Ok(())
    }

    /// Render a Form XObject by parsing its content stream recursively.
    ///
    /// Per PDF spec §8.10, a Form XObject contains its own content stream,
    /// optional /Matrix transform, and optional /Resources dictionary.
    fn render_form_xobject(
        &mut self,
        pixmap: &mut Pixmap,
        dict: &std::collections::HashMap<String, Object>,
        data: &[u8],
        parent_transform: Transform,
        doc: &PdfDocument,
        page_num: usize,
        parent_resources: &Object,
    ) -> Result<()> {
        let form_matrix = if let Some(Object::Array(arr)) = dict.get("Matrix") {
            let get_f32 = |i: usize| -> f32 {
                match arr.get(i) {
                    Some(Object::Real(v)) => *v as f32,
                    Some(Object::Integer(v)) => *v as f32,
                    _ => {
                        if i == 0 || i == 3 {
                            1.0
                        } else {
                            0.0
                        }
                    }
                }
            };
            Transform::from_row(get_f32(0), get_f32(1), get_f32(2), get_f32(3), get_f32(4), get_f32(5))
        } else {
            Transform::identity()
        };

        let combined_transform = parent_transform.pre_concat(form_matrix);

        let is_transparency_group = dict
            .get("Group")
            .and_then(|g| g.as_dict())
            .map(|gd| gd.get("S").and_then(|s| s.as_name()) == Some("Transparency"))
            .unwrap_or(false);

        let form_resources = if let Some(res) = dict.get("Resources") {
            doc.resolve_object(res)?
        } else {
            parent_resources.clone()
        };

        let operators = match parse_content_stream(data) {
            Ok(ops) => ops,
            Err(e) => {
                return Err(e);
            }
        };

        if is_transparency_group {
            // Per PDF spec 11.6.6: Render transparency group to a separate pixmap,
            // then composite onto the parent. For isolated groups (I=true), the
            // initial backdrop is fully transparent. ~keep
            let is_isolated = dict
                .get("Group")
                .and_then(|g| g.as_dict())
                .and_then(|gd| gd.get("I"))
                .map(|i| match i {
                    Object::Boolean(b) => *b,
                    _ => false,
                })
                .unwrap_or(false);

            // ISO 32000-1:2008 §11.4.6.2 — knockout flag. A knockout group
            // composites each element against the group's initial backdrop
            // rather than against the accumulated paint from earlier
            // elements. Later elements override earlier ones in regions
            // where both contribute. ~keep
            let is_knockout = dict
                .get("Group")
                .and_then(|g| g.as_dict())
                .and_then(|gd| gd.get("K"))
                .map(|k| match k {
                    Object::Boolean(b) => *b,
                    _ => false,
                })
                .unwrap_or(false);

            tracing::trace!(
                "Rendering transparency group (isolated={}, knockout={})",
                is_isolated,
                is_knockout
            );

            let mut group_pixmap = Pixmap::new(pixmap.width(), pixmap.height())
                .ok_or_else(|| crate::error::Error::InvalidPdf("Failed to create group pixmap".into()))?;

            if !is_isolated {
                // Non-isolated: copy parent content as initial backdrop ~keep
                group_pixmap.data_mut().copy_from_slice(pixmap.data());
            }
            // Isolated groups start fully transparent (default Pixmap state) ~keep

            if is_knockout {
                // §11.4.6.2: snapshot the initial backdrop, then composite
                // each element separately against it. The accumulator
                // starts as the backdrop; each paint operator's result is
                // merged in so later paints override earlier ones in
                // overlap regions. ~keep
                self.execute_knockout_group(
                    &mut group_pixmap,
                    combined_transform,
                    &operators,
                    doc,
                    page_num,
                    &form_resources,
                )?;
            } else {
                self.execute_operators(
                    &mut group_pixmap,
                    combined_transform,
                    &operators,
                    doc,
                    page_num,
                    &form_resources,
                )?;
            }

            if is_isolated {
                pixmap.draw_pixmap(
                    0,
                    0,
                    group_pixmap.as_ref(),
                    &tiny_skia::PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
            } else {
                // Non-isolated: the group pixmap IS the result (it started with parent content)
                // ~keep
                pixmap.data_mut().copy_from_slice(group_pixmap.data());
            }
        } else {
            self.execute_operators(pixmap, combined_transform, &operators, doc, page_num, &form_resources)?;
        }

        Ok(())
    }

    /// Rasterise a `/PatternType 1` tiling pattern into the current fill
    /// region (ISO 32000-1:2008 §8.7.3).
    ///
    /// A tiling pattern paints a small cell — its own content stream
    /// clipped to `/BBox` — repeated on a lattice spaced by `/XStep` ×
    /// `/YStep` in pattern space. The pattern `/Matrix` maps pattern
    /// space to the default (initial) coordinate system of the pattern's
    /// parent content stream, here taken as `base_transform` (the device
    /// transform in effect before the current CTM), NOT the CTM active at
    /// fill time.
    ///
    /// `/PaintType 1` (coloured) cells supply their own colour; `/PaintType 2`
    /// (uncoloured) cells are painted in the current fill colour
    /// (`gs.fill_color_rgb`).
    ///
    /// Returns `Ok(true)` when the region was painted — either tiled, or
    /// (on a perf/geometry guard) flooded with the cell's average colour —
    /// and `Ok(false)` when the referenced pattern is not a usable tiling
    /// pattern (`/PatternType 2` shading, missing/malformed dict, or an
    /// over-large cell), so the caller paints its normal solid fill.
    /// Never panics and never loops unboundedly.
    #[allow(clippy::too_many_arguments)]
    fn fill_with_tiling_pattern(
        &mut self,
        pixmap: &mut Pixmap,
        path: &tiny_skia::Path,
        base_transform: Transform,
        path_transform: Transform,
        fill_rule: tiny_skia::FillRule,
        clip: Option<&tiny_skia::Mask>,
        gs: &GraphicsState,
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
    ) -> Result<bool> {
        // Cap the offscreen cell raster and the tile count so a pathological
        // pattern cannot exhaust memory or spin. Beyond these limits we fall
        // back to a solid flood (average colour) or defer to the caller. ~keep
        const MAX_CELL_PX: u32 = 4096;
        const MAX_TILES: i64 = 1_000_000;

        let Some(pattern_name) = gs.fill_pattern_name.as_deref() else {
            return Ok(false);
        };

        let Some(res_dict) = resources.as_dict() else {
            return Ok(false);
        };
        let Some(pattern_group) = res_dict.get("Pattern") else {
            return Ok(false);
        };
        let pattern_group = doc.resolve_object(pattern_group)?;
        let Some(pattern_map) = pattern_group.as_dict() else {
            return Ok(false);
        };
        let Some(pattern_entry) = pattern_map.get(pattern_name) else {
            return Ok(false);
        };
        let pattern_ref = pattern_entry.as_reference();
        let pattern_obj = doc.resolve_object(pattern_entry)?;
        let Some(pdict) = pattern_obj.as_dict() else {
            return Ok(false);
        };

        // Only tiling patterns (PatternType 1) are handled here; shading
        // patterns (PatternType 2) are left to the caller's solid fallback. ~keep
        if pdict.get("PatternType").and_then(|o| o.as_integer()).unwrap_or(1) != 1 {
            return Ok(false);
        }
        let paint_type = pdict.get("PaintType").and_then(|o| o.as_integer()).unwrap_or(1);

        let num = |o: &Object| -> Option<f32> {
            o.as_integer()
                .map(|i| i as f32)
                .or_else(|| o.as_real().map(|r| r as f32))
        };
        let read_array = |key: &str, n: usize| -> Option<Vec<f32>> {
            let arr = pdict.get(key)?.as_array()?;
            if arr.len() < n {
                return None;
            }
            arr.iter().take(n).map(&num).collect()
        };

        let Some(bbox) = read_array("BBox", 4) else {
            return Ok(false);
        };
        let x_step = pdict.get("XStep").and_then(num).unwrap_or(bbox[2] - bbox[0]);
        let y_step = pdict.get("YStep").and_then(num).unwrap_or(bbox[3] - bbox[1]);
        let m = read_array("Matrix", 6).unwrap_or_else(|| vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let pattern_matrix = Transform::from_row(m[0], m[1], m[2], m[3], m[4], m[5]);

        // Own the pattern's resource dict before releasing the borrow on
        // `pattern_obj` (needed again to decode the stream). ~keep
        let pattern_resources = match pdict.get("Resources") {
            Some(r) => doc.resolve_object(r)?,
            None => resources.clone(),
        };

        let t = base_transform.pre_concat(pattern_matrix);
        let map = |x: f32, y: f32| -> (f32, f32) { (x * t.sx + y * t.kx + t.tx, x * t.ky + y * t.sy + t.ty) };

        let corners = [
            map(bbox[0], bbox[1]),
            map(bbox[2], bbox[1]),
            map(bbox[2], bbox[3]),
            map(bbox[0], bbox[3]),
        ];
        let (mut cminx, mut cminy, mut cmaxx, mut cmaxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (x, y) in corners {
            cminx = cminx.min(x);
            cminy = cminy.min(y);
            cmaxx = cmaxx.max(x);
            cmaxy = cmaxy.max(y);
        }
        if ![cminx, cminy, cmaxx, cmaxy].iter().all(|v| v.is_finite()) {
            return Ok(false);
        }
        let cw = (cmaxx - cminx).ceil();
        let ch = (cmaxy - cminy).ceil();
        if !(1.0..=MAX_CELL_PX as f32).contains(&cw) || !(1.0..=MAX_CELL_PX as f32).contains(&ch) {
            return Ok(false);
        }
        let (cw, ch) = (cw as u32, ch as u32);

        // Device step vectors (linear part of `t` applied to the pattern-space
        // step vectors). For an axis-aligned matrix the cross terms are ~0. ~keep
        let step_x = (x_step * t.sx, x_step * t.ky);
        let step_y = (y_step * t.kx, y_step * t.sy);
        let scale = t.sx.abs().max(t.sy.abs()).max(t.kx.abs()).max(t.ky.abs()).max(1e-6);
        let axis_aligned = t.kx.abs() <= 1e-3 * scale && t.ky.abs() <= 1e-3 * scale;
        let step_x_len = step_x.0.hypot(step_x.1);
        let step_y_len = step_y.0.hypot(step_y.1);

        let stream_data = if let Some(r) = pattern_ref {
            doc.decode_stream_with_encryption(&pattern_obj, r)?
        } else {
            pattern_obj.decode_stream_data()?
        };
        let cell_ops = match parse_content_stream(&stream_data) {
            Ok(ops) => ops,
            Err(_) => return Ok(false),
        };
        let mut cell = match Pixmap::new(cw, ch) {
            Some(p) => p,
            None => return Ok(false),
        };
        // Map pattern space into the cell pixmap: `t`, shifted so the cell's
        // device min-corner lands at the pixmap origin. ~keep
        let cell_transform = Transform::from_translate(-cminx, -cminy).pre_concat(t);
        // Render the cell with a fresh resource scope and no CMYK sidecar
        // (the sidecar is sized to the page pixmap, not this cell). ~keep
        let saved_sidecar = self.cmyk_sidecar.take();
        let saved_fonts = self.fonts.clone();
        let saved_cs = self.color_spaces.clone();
        let _ = self.load_resources(doc, &pattern_resources);
        let render_res =
            self.execute_operators(&mut cell, cell_transform, &cell_ops, doc, page_num, &pattern_resources);
        self.fonts = saved_fonts;
        self.color_spaces = saved_cs;
        self.cmyk_sidecar = saved_sidecar;
        if render_res.is_err() {
            return Ok(false);
        }

        // /PaintType 2 (uncoloured): recolour the cell coverage with the
        // current fill colour, preserving the rendered alpha. ~keep
        if paint_type == 2 {
            let (fr, fg, fb) = gs.fill_color_rgb;
            let (fr, fg, fb) = (
                (fr.clamp(0.0, 1.0) * 255.0) as u32,
                (fg.clamp(0.0, 1.0) * 255.0) as u32,
                (fb.clamp(0.0, 1.0) * 255.0) as u32,
            );
            for px in cell.data_mut().as_chunks_mut::<4>().0 {
                let a = px[3] as u32;
                px[0] = (fr * a / 255) as u8;
                px[1] = (fg * a / 255) as u8;
                px[2] = (fb * a / 255) as u8;
            }
        }

        // Average (premultiplied) cell colour, used both for the geometry
        // fallback and to skip fully-transparent cells. ~keep
        let (mut sr, mut sg, mut sb, mut sa) = (0u64, 0u64, 0u64, 0u64);
        for px in cell.data().as_chunks::<4>().0 {
            sr += px[0] as u64;
            sg += px[1] as u64;
            sb += px[2] as u64;
            sa += px[3] as u64;
        }
        let npix = (cw as u64) * (ch as u64);
        let avg_a = (sa / npix) as u8;
        if avg_a == 0 && paint_type == 1 {
            return Ok(true);
        }
        // Un-premultiply the average to a straight colour for the flood path. ~keep
        let unpremul = |sum: u64| -> u8 {
            if avg_a == 0 {
                0
            } else {
                (((sum / npix) as f32) * 255.0 / avg_a as f32).min(255.0) as u8
            }
        };
        let avg_color = tiny_skia::Color::from_rgba8(unpremul(sr), unpremul(sg), unpremul(sb), avg_a);

        let b = path.bounds();
        let pm = |x: f32, y: f32| -> (f32, f32) {
            (
                x * path_transform.sx + y * path_transform.kx + path_transform.tx,
                x * path_transform.ky + y * path_transform.sy + path_transform.ty,
            )
        };
        let pcorners = [
            pm(b.left(), b.top()),
            pm(b.right(), b.top()),
            pm(b.right(), b.bottom()),
            pm(b.left(), b.bottom()),
        ];
        let (mut rx0, mut ry0, mut rx1, mut ry1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (x, y) in pcorners {
            rx0 = rx0.min(x);
            ry0 = ry0.min(y);
            rx1 = rx1.max(x);
            ry1 = ry1.max(y);
        }
        let (w, h) = (pixmap.width() as f32, pixmap.height() as f32);
        rx0 = rx0.max(0.0);
        ry0 = ry0.max(0.0);
        rx1 = rx1.min(w);
        ry1 = ry1.min(h);
        if rx1 <= rx0 || ry1 <= ry0 {
            return Ok(true); // fill region off-screen — nothing to paint ~keep
        }

        // Geometry guards: rotated/sheared matrix, degenerate or too-dense
        // steps, or an unusable tile count -> flood the path with the cell's
        // average colour instead of tiling. ~keep
        let mut base_paint = tiny_skia::Paint::default();
        base_paint.anti_alias = true;
        let flood = |pixmap: &mut Pixmap| {
            let mut p = base_paint.clone();
            p.set_color(avg_color);
            guarded_fill_path(pixmap, path, &p, fill_rule, path_transform, clip);
        };
        if !axis_aligned
            || x_step.abs() <= f32::EPSILON
            || y_step.abs() <= f32::EPSILON
            || step_x_len < 0.5
            || step_y_len < 0.5
        {
            flood(pixmap);
            return Ok(true);
        }

        let (i_lo, i_hi) = axis_tile_range(rx0, rx1, cminx, cw as f32, step_x.0);
        let (j_lo, j_hi) = axis_tile_range(ry0, ry1, cminy, ch as f32, step_y.1);
        let tile_count = (i_hi as i64 - i_lo as i64 + 1) * (j_hi as i64 - j_lo as i64 + 1);
        if tile_count <= 0 || tile_count > MAX_TILES {
            flood(pixmap);
            return Ok(true);
        }

        // Build the fill-region mask (path coverage ∩ active clip) once and
        // blit the cell into every lattice position under it. ~keep
        let mut mask = match tiny_skia::Mask::new(pixmap.width(), pixmap.height()) {
            Some(m) => m,
            None => {
                flood(pixmap);
                return Ok(true);
            }
        };
        guarded_mask_fill_path(&mut mask, path, fill_rule, true, path_transform);
        if let Some(c) = clip {
            for (mv, cv) in mask.data_mut().iter_mut().zip(c.data().iter()) {
                *mv = (*mv).min(*cv);
            }
        }

        let blit = PixmapPaint {
            opacity: gs.fill_alpha.clamp(0.0, 1.0),
            // Nearest keeps tile seams crisp for axis-aligned integer-ish steps. ~keep
            quality: tiny_skia::FilterQuality::Nearest,
            ..PixmapPaint::default()
        };
        for j in j_lo..=j_hi {
            for i in i_lo..=i_hi {
                let px = cminx + i as f32 * step_x.0 + j as f32 * step_y.0;
                let py = cminy + i as f32 * step_x.1 + j as f32 * step_y.1;
                pixmap.draw_pixmap(
                    0,
                    0,
                    cell.as_ref(),
                    &blit,
                    Transform::from_translate(px, py),
                    Some(&mask),
                );
            }
        }
        Ok(true)
    }

    /// Take a snapshot of `pixmap` if the graphics state has an active
    /// `/SMask`. The caller paints normally, then calls
    /// [`Self::apply_smask_after_paint`] with the snapshot to modulate
    /// the painted contribution by the soft mask. Returns `None` when
    /// the gs has no soft mask, so the caller takes the no-op branch.
    fn smask_snapshot(&self, pixmap: &Pixmap, gs: &GraphicsState) -> Option<Vec<u8>> {
        if gs.smask.is_some() {
            Some(pixmap.data().to_vec())
        } else {
            None
        }
    }

    /// Companion to [`Self::smask_snapshot`] for the spot-lane sidecar.
    /// When the graphics state has an active `/SMask` AND the sidecar
    /// is allocated, return a flat snapshot of every spot plane so the
    /// SMask attenuation path can blend `m·post_mirror + (1-m)·pre`
    /// per pixel per lane.
    ///
    /// ISO 32000-1 §11.3.3 + §11.7.3: "Only a single shape value and
    /// opacity value shall be maintained at each point in the computed
    /// group results; they shall apply to both process and spot colour
    /// components." The pixmap's RGB lanes receive the SMask alpha
    /// attenuation via [`Self::apply_smask_after_paint`]; the spot
    /// lanes need the same attenuation against their pre-paint state so
    /// the lane composes at the spec-correct effective alpha.
    fn smask_spot_snapshot(&self, gs: &GraphicsState) -> Option<Vec<u8>> {
        gs.smask.as_ref()?;
        let sidecar = self.cmyk_sidecar.as_ref()?;
        Some(sidecar.spots_all().to_vec())
    }

    /// Predicate: should the CMYK compose-before-convert path fire for
    /// the current paint operator? Per ISO 32000-1:2008 §11.4 + Annex G,
    /// transparency compositing happens in the source colour space and
    /// the OutputIntent ICC conversion happens at display. When all of
    /// the following hold, the spec-correct rendering requires composing
    /// in CMYK before converting through the ICC profile:
    ///
    /// * The active colour on the relevant side is genuine CMYK
    ///   (`gs.fill_color_cmyk` / `gs.stroke_color_cmyk` populated).
    /// * The graphics state declares non-trivial transparency: alpha
    ///   below 1.0, a non-Normal blend mode, or an active soft mask.
    /// * A CMYK OutputIntent ICC profile is available (otherwise the
    ///   additive-clamp fallback is linear, so convert-first and
    ///   compose-first are byte-identical and we save the work).
    ///
    /// Returns `true` only when every condition is met so the no-op
    /// branch is the cheapest possible test: a single ICC-profile
    /// lookup + a few `gs` field reads.
    fn cmyk_compose_active(&self, gs: &GraphicsState, doc: &PdfDocument, fill_side: bool) -> bool {
        let has_cmyk = if fill_side {
            gs.fill_color_cmyk.is_some()
        } else {
            gs.stroke_color_cmyk.is_some()
        };
        if !has_cmyk {
            return false;
        }
        // ISO 32000-1 §11.7.4.3: when overprint is active the
        // CompatibleOverprint blend function takes over the per-channel
        // composition (`α · B(c_b, c_s) + (1 - α) · c_b`). Running the
        // compose-first helper additionally would double-touch the
        // sidecar and corrupt the OPM=1 preserve-on-zero rule (compose
        // would write `(1-α)·c_b`, then overprint would read that as
        // the new backdrop). The overprint helper handles compose
        // itself for overprint paints. ~keep
        let overprint = if fill_side {
            gs.fill_overprint
        } else {
            gs.stroke_overprint
        };
        if overprint {
            return false;
        }
        let alpha = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };
        let non_trivial = alpha < 1.0 || gs.blend_mode != "Normal" || gs.smask.is_some();
        if !non_trivial {
            return false;
        }
        doc.output_intent_cmyk_profile().is_some()
    }

    /// Snapshot the pixmap when [`Self::cmyk_compose_active`] returns
    /// true. The caller paints normally with the tiny_skia rasteriser
    /// (which renders CMYK→RGB-via-ICC then alpha-blends in RGB — the
    /// convert-first path), then hands the snapshot to
    /// [`Self::apply_cmyk_compose_after_paint`] to overwrite the
    /// painted region with the compose-first result.
    fn cmyk_compose_snapshot(
        &self,
        pixmap: &Pixmap,
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) -> Option<Vec<u8>> {
        if self.cmyk_compose_active(gs, doc, fill_side) {
            Some(pixmap.data().to_vec())
        } else {
            None
        }
    }

    /// Snapshot the pixmap when the spot-lane mirror is about to fire.
    /// Returns `Some(pixmap_bytes)` when the sidecar is allocated AND
    /// the active side has at least one spot ink in the sidecar's
    /// discovered spot set; `None` otherwise. The mirror helper
    /// (`mirror_spot_paint_into_sidecar_with_coverage`) uses the
    /// snapshot to recover painted-pixel positions via a snapshot-vs-
    /// post-paint diff when the caller has no pre-rasterised coverage
    /// mask. Path-paint callers pass the pre-rasterised coverage
    /// directly and ignore the snapshot's diff role.
    fn spot_paint_snapshot(&self, pixmap: &Pixmap, gs: &GraphicsState, fill_side: bool) -> Option<Vec<u8>> {
        if !self.spot_paint_active(gs, fill_side) {
            return None;
        }
        Some(pixmap.data().to_vec())
    }

    /// Fill-side spot snapshot for a text show, additionally gated on the
    /// fill-producing text render modes (`Tr` 0/2/4/6). ISO 32000-1 §9.3.6
    /// Table 106: modes 1/3/5/7 lay down no visible *fill* mark — mode 3 is
    /// fully invisible, 1/5 stroke only, 7 clip only. The spot mirror derives
    /// its coverage from [`Self::coverage_only_gs`], which force-overrides the
    /// render mode to 0 so the coverage scratch always paints; without this
    /// gate an invisible (`3 Tr`) or stroke-only show would still write the
    /// spot/InkA lane where nothing was painted. `spot_paint_active` cannot
    /// carry this check because it is shared with path paints, for which the
    /// text render mode is meaningless.
    fn text_fill_spot_snapshot(&self, pixmap: &Pixmap, gs: &GraphicsState) -> Option<Vec<u8>> {
        if !matches!(gs.render_mode, 0 | 2 | 4 | 6) {
            return None;
        }
        self.spot_paint_snapshot(pixmap, gs, true)
    }

    /// Snapshot the pixmap when the CMYK sidecar plane is present and
    /// the paint side carries a CMYK colour. The plane mirror runs at
    /// every CMYK paint (opaque or transparent) so the sidecar stays
    /// in sync with the page's plate state. The mirror helper
    /// `mirror_cmyk_paint_into_sidecar` consumes the snapshot + post-
    /// paint pixmap to identify the painted region and writes updated
    /// CMYK quadruples at those pixels.
    fn cmyk_sidecar_snapshot(&self, pixmap: &Pixmap, gs: &GraphicsState, fill_side: bool) -> Option<Vec<u8>> {
        self.cmyk_sidecar.as_ref()?;
        let has_cmyk = if fill_side {
            gs.fill_color_cmyk.is_some()
        } else {
            gs.stroke_color_cmyk.is_some()
        };
        if !has_cmyk {
            return None;
        }
        Some(pixmap.data().to_vec())
    }

    /// After a CMYK paint (opaque or transparent), write updated CMYK
    /// quadruples to the sidecar plane at painted pixels. The
    /// effective coverage is recovered from the snapshot vs post-paint
    /// pixmap diff so AA-edge pixels carry the correct partial CMYK.
    /// Skipped silently when the sidecar is None (detection-OFF) or
    /// when the painted-pixel-recovery cannot proceed (e.g. the
    /// rasteriser produced no observable diff).
    ///
    /// Called only when the paint is OPAQUE (no transparency
    /// composition needed). For transparent paints, the compose-first
    /// path is the source of truth for sidecar updates — it already
    /// mirrors the composed quadruple after compositing.
    ///
    /// For overprint paints, sidecar update happens inside
    /// [`Self::apply_overprint_after_paint`] which handles plate
    /// merging.
    fn mirror_cmyk_paint_into_sidecar(
        &mut self,
        pixmap: &Pixmap,
        snapshot: &[u8],
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) {
        let (sc, sm, sy, sk) = if fill_side {
            match gs.fill_color_cmyk {
                Some(v) => v,
                None => return,
            }
        } else {
            match gs.stroke_color_cmyk {
                Some(v) => v,
                None => return,
            }
        };

        // Skip when compose-first or overprint paths handle the
        // sidecar update themselves. Those paths run within their
        // own `apply_*_after_paint` helpers and write composed /
        // merged CMYK directly. ~keep
        let alpha = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };
        let overprint = if fill_side {
            gs.fill_overprint
        } else {
            gs.stroke_overprint
        };
        let transparent = alpha < 1.0 || gs.blend_mode != "Normal" || gs.smask.is_some();
        if transparent || overprint {
            return;
        }

        // For opaque CMYK paints the post-paint RGB came through the
        // ICC convert-first (or additive-clamp fallback) path. To
        // detect painted pixels we look at the snapshot vs post-paint
        // diff; for AA-edge pixels we need to recover the effective
        // coverage so the sidecar carries the right partial-coverage
        // CMYK. ~keep
        let src_rgb_ic = {
            let c_u8 = (sc.clamp(0.0, 1.0) * 255.0).round() as u8;
            let m_u8 = (sm.clamp(0.0, 1.0) * 255.0).round() as u8;
            let y_u8 = (sy.clamp(0.0, 1.0) * 255.0).round() as u8;
            let k_u8 = (sk.clamp(0.0, 1.0) * 255.0).round() as u8;
            if let Some(profile) = doc.output_intent_cmyk_profile() {
                let intent = crate::color::RenderingIntent::from_pdf_name(&gs.rendering_intent);
                let transform = self.icc_transform_cache.get_or_build(&profile, intent);
                let rgb = transform.convert_cmyk_pixel(c_u8, m_u8, y_u8, k_u8);
                [rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0]
            } else {
                let (r, g, b) = cmyk_to_rgb(sc, sm, sy, sk);
                [r, g, b]
            }
        };

        let post = pixmap.data();
        let plane = match self.cmyk_sidecar.as_mut() {
            Some(s) => s.cmyk_mut(),
            None => return,
        };
        debug_assert_eq!(post.len(), snapshot.len());
        debug_assert_eq!(post.len(), plane.len());

        for px in 0..(post.len() / 4) {
            let off = px * 4;
            let painted = post[off] != snapshot[off]
                || post[off + 1] != snapshot[off + 1]
                || post[off + 2] != snapshot[off + 2]
                || post[off + 3] != snapshot[off + 3];
            if !painted {
                continue;
            }

            // Recover effective coverage c from the source-over blend
            // on the channel with maximum |snap - src|. ~keep
            let snap_r = snapshot[off] as f32 / 255.0;
            let snap_g = snapshot[off + 1] as f32 / 255.0;
            let snap_b = snapshot[off + 2] as f32 / 255.0;
            let post_r = post[off] as f32 / 255.0;
            let post_g = post[off + 1] as f32 / 255.0;
            let post_b = post[off + 2] as f32 / 255.0;

            let diffs = [
                (snap_r - src_rgb_ic[0]).abs(),
                (snap_g - src_rgb_ic[1]).abs(),
                (snap_b - src_rgb_ic[2]).abs(),
            ];
            let (max_idx, max_diff) = diffs
                .iter()
                .enumerate()
                .fold((0usize, 0.0_f32), |acc, (i, &v)| if v > acc.1 { (i, v) } else { acc });
            let coverage = if max_diff > 1.0 / 255.0 {
                let (snap_ch, post_ch, src_ch) = match max_idx {
                    0 => (snap_r, post_r, src_rgb_ic[0]),
                    1 => (snap_g, post_g, src_rgb_ic[1]),
                    _ => (snap_b, post_b, src_rgb_ic[2]),
                };
                ((snap_ch - post_ch) / (snap_ch - src_ch)).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let dc = plane[off] as f32 / 255.0;
            let dm = plane[off + 1] as f32 / 255.0;
            let dy = plane[off + 2] as f32 / 255.0;
            let dk = plane[off + 3] as f32 / 255.0;

            let mc = coverage * sc + (1.0 - coverage) * dc;
            let mm = coverage * sm + (1.0 - coverage) * dm;
            let my = coverage * sy + (1.0 - coverage) * dy;
            let mk = coverage * sk + (1.0 - coverage) * dk;

            plane[off] = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 1] = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 2] = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 3] = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    /// Recompute every painted pixel through the §11.4 compose-first
    /// rule. The naive paint path converted CMYK→RGB through the
    /// OutputIntent ICC before alpha-blending; under a non-linear ICC
    /// (input curves != identity), `ICC(α·A + (1-α)·B) ≠ α·ICC(A) +
    /// (1-α)·ICC(B)`, so the convert-first result diverges from the
    /// spec-correct compose-first value. This helper recovers the
    /// effective coverage from the post-paint RGB (using the convert-
    /// first source RGB the rasteriser actually wrote) and replaces the
    /// pixel with `ICC(α·source_cmyk + (1-α)·snapshot_cmyk)`, where
    /// `snapshot_cmyk` comes from inverting the snapshot RGB through
    /// the additive-clamp formula. The inversion is exact when the
    /// snapshot was produced by an additive-clamp paint (the
    /// no-transparency baseline) and is the same lossy approximation
    /// the composite overprint path admits when the backdrop went
    /// through a non-trivial ICC.
    ///
    /// Alpha channel is preserved from the post-paint pixmap because
    /// the alpha composition rule is the same in either ordering
    /// (`α_out = c·α_src + (1-c·α_src)·α_dst`).
    /// Rasterise a fill path to a coverage byte buffer when the CMYK
    /// sidecar is active. Returns `None` when the sidecar is
    /// detection-OFF — the diff-driven compose-first path is the
    /// only one used in that case and a coverage mask would be
    /// unused work.
    fn rasterise_fill_coverage(
        &self,
        path: &tiny_skia::Path,
        transform: Transform,
        fill_rule: tiny_skia::FillRule,
        clip: Option<&tiny_skia::Mask>,
    ) -> Option<Vec<u8>> {
        let sidecar = self.cmyk_sidecar.as_ref()?;
        let (w, h) = sidecar.dims();
        let mut mask = tiny_skia::Mask::new(w, h)?;
        guarded_mask_fill_path(&mut mask, path, fill_rule, true, transform);
        let mut buf = mask.data().to_vec();
        // Intersect with the active clip mask. tiny_skia's clip mask
        // is per-pixel coverage; pixel-wise min gives the
        // intersection. ~keep
        if let Some(c) = clip {
            for (b, cv) in buf.iter_mut().zip(c.data().iter()) {
                *b = (*b).min(*cv);
            }
        }
        Some(buf)
    }

    /// Rasterise a stroke path to a coverage byte buffer. Mirror of
    /// [`Self::rasterise_fill_coverage`] for the stroke-side compose-
    /// first / overprint paths. tiny_skia's `Mask` does not expose
    /// `stroke_path` directly, so this routes through a scratch
    /// alpha-only `Pixmap`: paint the stroke with full-alpha black,
    /// then extract the alpha channel as the coverage buffer.
    fn rasterise_stroke_coverage(
        &self,
        path: &tiny_skia::Path,
        transform: Transform,
        gs: &GraphicsState,
        clip: Option<&tiny_skia::Mask>,
    ) -> Option<Vec<u8>> {
        let sidecar = self.cmyk_sidecar.as_ref()?;
        let (w, h) = sidecar.dims();
        let mut scratch = Pixmap::new(w, h)?;
        let dash = if !gs.dash_pattern.0.is_empty() {
            tiny_skia::StrokeDash::new(gs.dash_pattern.0.clone(), gs.dash_pattern.1)
        } else {
            None
        };
        let stroke = tiny_skia::Stroke {
            width: gs.line_width,
            line_cap: match gs.line_cap {
                1 => tiny_skia::LineCap::Round,
                2 => tiny_skia::LineCap::Square,
                _ => tiny_skia::LineCap::Butt,
            },
            line_join: match gs.line_join {
                1 => tiny_skia::LineJoin::Round,
                2 => tiny_skia::LineJoin::Bevel,
                _ => tiny_skia::LineJoin::Miter,
            },
            miter_limit: gs.miter_limit,
            dash,
        };
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba8(0, 0, 0, 255));
        paint.anti_alias = true;
        guarded_stroke_path(&mut scratch, path, &paint, &stroke, transform, clip);
        let buf: Vec<u8> = scratch.data().as_chunks::<4>().0.iter().map(|px| px[3]).collect();
        Some(buf)
    }

    /// Build a coverage-only `GraphicsState` clone from `gs`. The clone
    /// forces full opacity (`fill_alpha` / `stroke_alpha` = 1.0),
    /// `/Normal` blend, and opaque-black fill colour. Re-running a paint
    /// with this gs into a fresh transparent scratch pixmap produces an
    /// alpha channel that equals geometry coverage at every pixel — the
    /// same per-pixel coverage `tiny_skia::Mask::fill_path` and the
    /// stroke-side scratch-Pixmap helper produce for path-side coverage.
    /// The caller extracts the alpha channel via
    /// [`Self::extract_alpha_as_coverage`].
    ///
    /// `gs.render_mode` is preserved verbatim. ISO 32000-1 §9.3.6 text
    /// rendering mode 3 ("neither fill nor stroke; add to path for
    /// clipping") produces no visible mark, and under the §11.3.3
    /// single shape/opacity per pixel rule the spot lane must see no
    /// mark either (§11.7.3 composes the spot lane with the same shape
    /// / opacity as the page). The text rasteriser already collapses
    /// the paint to fully transparent for `render_mode == 3` (see
    /// `text_rasterizer.rs` — `paint.set_color(rgba 0,0,0,0)`), so the
    /// scratch alpha channel correctly resolves to zero coverage and no
    /// spot lane write fires. Overriding `render_mode` to 0 here would
    /// paint visible glyphs into the coverage scratch while the visible
    /// pixmap shows nothing, leaking a spurious spot-lane write.
    fn coverage_only_gs(gs: &GraphicsState) -> GraphicsState {
        let mut cov = gs.clone();
        cov.fill_alpha = 1.0;
        cov.stroke_alpha = 1.0;
        cov.blend_mode = "Normal".to_string();
        cov.fill_color_rgb = (0.0, 0.0, 0.0);
        cov.stroke_color_rgb = (0.0, 0.0, 0.0);
        // Strip SMask so the scratch render doesn't kick off a
        // recursive SMask compose with a different geometry. ~keep
        cov.smask = None;
        // Force a fill-producing render mode. The visible mode may be 7
        // (clip-only) or 3 (invisible), both of which the text rasteriser
        // deliberately paints with transparent paint (WS1.5) — routing the
        // coverage render through those modes would yield an empty silhouette
        // and silently drop the clip. Mode 0 fills the glyph body opaquely,
        // which is exactly the coverage the clip accumulation needs. ~keep
        cov.render_mode = 0;
        cov
    }

    /// Extract the alpha channel from a pixmap as a byte buffer. The
    /// alpha encodes per-pixel coverage when the pixmap was painted
    /// with opaque-black paint and `BlendMode::SourceOver` on a fresh
    /// transparent backdrop — both glyph fills, image blits, and
    /// shading paints obey that contract through the existing
    /// rasterisers when the gs has `fill_alpha = 1.0` and
    /// `blend_mode = "Normal"`. Per pixel: `alpha == 255` is fully
    /// covered, `alpha == 0` is uncovered, intermediate values carry
    /// AA-edge partial coverage. The buffer is then handed to the
    /// spot-mirror's coverage-aware path verbatim.
    fn extract_alpha_as_coverage(pixmap: &Pixmap) -> Vec<u8> {
        pixmap.data().as_chunks::<4>().0.iter().map(|px| px[3]).collect()
    }

    /// WS1.5b — union a clip-mode (`Tr` 4–7) `Tj` / `'` / `"` show's glyph
    /// outlines into the text-clip accumulator.
    ///
    /// `accum` is a page-sized scratch pixmap whose alpha channel holds the
    /// accumulated glyph silhouette for the enclosing `BT`…`ET` block; it is
    /// created lazily on the first clip-mode show so modes 0–3 never allocate
    /// it. Glyphs are laid down with [`Self::coverage_only_gs`] (opaque black,
    /// `SourceOver`), so each show's outlines union with the previous ones in
    /// place — exactly the "add to the current clip path" semantics of ISO
    /// 32000-1 §9.4.1. [`Self::coverage_only_gs`] forces fill mode 0 so the
    /// glyph bodies rasterise opaquely even when the visible mode is 7
    /// (clip-only) or 3 (invisible), which the rasteriser paints transparent.
    /// The inherited clip is intentionally *not* applied here; the final `ET`
    /// intersection folds the silhouette into the live clip.
    #[allow(clippy::too_many_arguments)]
    fn accumulate_text_clip_tj(
        &self,
        accum: &mut Option<Pixmap>,
        width: u32,
        height: u32,
        text: &[u8],
        transform: Transform,
        gs: &GraphicsState,
        resources: &Object,
        doc: &PdfDocument,
    ) {
        if accum.is_none() {
            *accum = Pixmap::new(width, height);
        }
        let Some(scratch) = accum.as_mut() else {
            return;
        };
        let cov_gs = Self::coverage_only_gs(gs);
        // Coverage raster is permitted to fail silently — the visible-paint
        // call for the same show already surfaces any real error, and a
        // missing silhouette simply means no clip contribution. ~keep
        let _ = self.text_rasterizer.render_text(
            scratch,
            text,
            transform,
            &cov_gs,
            None,
            resources,
            doc,
            None,
            &self.fonts,
        );
    }

    /// WS1.5b — `TJ` positioning-array counterpart of
    /// [`Self::accumulate_text_clip_tj`]. Same contract.
    #[allow(clippy::too_many_arguments)]
    fn accumulate_text_clip_tj_array(
        &self,
        accum: &mut Option<Pixmap>,
        width: u32,
        height: u32,
        array: &[crate::content::operators::TextElement],
        transform: Transform,
        gs: &GraphicsState,
        resources: &Object,
        doc: &PdfDocument,
    ) {
        if accum.is_none() {
            *accum = Pixmap::new(width, height);
        }
        let Some(scratch) = accum.as_mut() else {
            return;
        };
        let cov_gs = Self::coverage_only_gs(gs);
        let _ = self.text_rasterizer.render_tj_array(
            scratch,
            array,
            transform,
            &cov_gs,
            None,
            resources,
            doc,
            None,
            &self.fonts,
        );
    }

    /// Rasterise the text-show coverage for a single `Tj` / `'` / `"`
    /// string by running the same `text_rasterizer.render_text` path
    /// the visible paint uses, but with [`Self::coverage_only_gs`] so
    /// the alpha channel encodes per-glyph AA-edge coverage exactly.
    /// Returns `None` when the sidecar is detection-OFF (coverage
    /// would be unused work).
    ///
    /// Per ISO 32000-1 §9.4 text-showing operators + §9.6 simple-font
    /// glyph rasterisation: every glyph in the run is laid into the
    /// scratch pixmap via the same tt-parser / harfrust / ttf-outline
    /// path the visible paint uses, so the coverage mask is geometry-
    /// identical (including font-fallback substitutions) to the
    /// visible glyph bodies.
    fn rasterise_text_coverage_render_text(
        &self,
        text: &[u8],
        base_transform: Transform,
        gs: &GraphicsState,
        resources: &Object,
        doc: &PdfDocument,
        clip_mask: Option<&tiny_skia::Mask>,
    ) -> Option<Vec<u8>> {
        let sidecar = self.cmyk_sidecar.as_ref()?;
        let (w, h) = sidecar.dims();
        let mut scratch = Pixmap::new(w, h)?;
        let cov_gs = Self::coverage_only_gs(gs);
        let _ = self.text_rasterizer.render_text(
            &mut scratch,
            text,
            base_transform,
            &cov_gs,
            None,
            resources,
            doc,
            clip_mask,
            &self.fonts,
        );
        Some(Self::extract_alpha_as_coverage(&scratch))
    }

    /// Rasterise the text-show coverage for a `TJ` array. Mirror of
    /// [`Self::rasterise_text_coverage_render_text`] for the
    /// positioning-adjustment form. Same §9.4 + §9.6 contract.
    fn rasterise_text_coverage_render_tj_array(
        &self,
        array: &[crate::content::operators::TextElement],
        base_transform: Transform,
        gs: &GraphicsState,
        resources: &Object,
        doc: &PdfDocument,
        clip_mask: Option<&tiny_skia::Mask>,
    ) -> Option<Vec<u8>> {
        let sidecar = self.cmyk_sidecar.as_ref()?;
        let (w, h) = sidecar.dims();
        let mut scratch = Pixmap::new(w, h)?;
        let cov_gs = Self::coverage_only_gs(gs);
        let _ = self.text_rasterizer.render_tj_array(
            &mut scratch,
            array,
            base_transform,
            &cov_gs,
            None,
            resources,
            doc,
            clip_mask,
            &self.fonts,
        );
        Some(Self::extract_alpha_as_coverage(&scratch))
    }

    /// Rasterise the coverage for an Image / ImageMask Do by re-running
    /// the same image / stencil paint path into a fresh transparent
    /// scratch pixmap with [`Self::coverage_only_gs`] (fill_alpha = 1,
    /// /Normal BM). The resulting alpha channel folds the unit-square
    /// device-space footprint (§8.9.5) with the per-pixel stencil bit
    /// (§8.9.6.2 /Decode default) for ImageMasks AND with the per-
    /// pixel alpha of the source image for sampled images.
    ///
    /// Returns `None` when the sidecar is detection-OFF or when the
    /// XObject is a Form (Form Do is handled by the per-paint mirror
    /// inside the form's recursive content stream — the post-Do mirror
    /// for Form XObjects is suppressed by round 3's P0 fix).
    fn rasterise_image_xobject_coverage(
        &mut self,
        name: &str,
        transform: Transform,
        gs: &GraphicsState,
        resources: &Object,
        doc: &PdfDocument,
        clip_mask: Option<&tiny_skia::Mask>,
    ) -> Option<Vec<u8>> {
        let sidecar = self.cmyk_sidecar.as_ref()?;
        let (w, h) = sidecar.dims();
        let mut scratch = Pixmap::new(w, h)?;
        let cov_gs = Self::coverage_only_gs(gs);
        // Resolve the XObject reference + subtype dispatch the same
        // way the visible-paint Do arm does, but only for Image and
        // ImageMask subtypes. Form XObjects are excluded because
        // their post-Do mirror is suppressed (round 3 P0 fix), and
        // because re-running a Form Do here would invoke its own
        // nested content stream recursively — work that has nothing
        // to do with coverage extraction on the OUTER Do site. ~keep
        let xobj_dict_resources = resources;
        if let Object::Dictionary(res_dict) = xobj_dict_resources {
            if let Some(xobj_entry) = res_dict.get("XObject") {
                let xobjects_obj = doc.resolve_object(xobj_entry).ok()?;
                if let Some(xobjects) = xobjects_obj.as_dict() {
                    if let Some(xobj_ref_obj) = xobjects.get(name) {
                        let xobj = doc.resolve_object(xobj_ref_obj).ok()?;
                        let xobj_ref = xobj_ref_obj.as_reference();
                        if let Object::Stream { ref dict, .. } = xobj {
                            if let Some(subtype) = dict.get("Subtype").and_then(|o| o.as_name()) {
                                if subtype == "Image" {
                                    let is_image_mask = dict
                                        .get("ImageMask")
                                        .map(|o| matches!(o, Object::Boolean(true)))
                                        .unwrap_or(false);
                                    if is_image_mask {
                                        let _ = self.render_image_mask(
                                            &mut scratch,
                                            &xobj,
                                            xobj_ref,
                                            transform,
                                            doc,
                                            clip_mask,
                                            &cov_gs,
                                        );
                                    } else {
                                        let smask = dict.get("SMask").cloned();
                                        let mask = dict.get("Mask").cloned();
                                        let _ = self.render_image(
                                            &mut scratch,
                                            &xobj,
                                            xobj_ref,
                                            transform,
                                            doc,
                                            clip_mask,
                                            smask,
                                            mask,
                                            &cov_gs,
                                        );
                                    }
                                } else {
                                    // Form XObject (or other): no
                                    // coverage from this site —
                                    // returning all-zero coverage
                                    // would over-suppress the spot
                                    // mirror's diff fallback. Instead
                                    // signal "no coverage produced"
                                    // by returning None; the spot
                                    // mirror falls back to the diff
                                    // branch. ~keep
                                    return None;
                                }
                            }
                        }
                    }
                }
            }
        }
        Some(Self::extract_alpha_as_coverage(&scratch))
    }

    /// Resolve the shading dict's spot-ink list. Returns
    /// `Some(non_empty)` when the shading's `/ColorSpace` is
    /// `/Separation` or a non-process `/DeviceN`, with the tints taken
    /// from the function's `/C0` endpoint (correct for constant
    /// gradients; for varying gradients the C0 tint is the LANE write
    /// the §11.3.3 compose will see — a single tint per ink is the
    /// most the current spot-mirror representation supports).
    ///
    /// Returns `None` when the shading isn't found, has no
    /// `/ColorSpace`, or its CS is a process colour space.
    fn resolve_shading_spot_inks(
        &self,
        name: &str,
        resources: &Object,
        doc: &PdfDocument,
    ) -> Option<Vec<(String, f32)>> {
        let res_dict = resources.as_dict()?;
        let shadings_obj = res_dict.get("Shading")?;
        let shadings = doc.resolve_object(shadings_obj).ok()?;
        let shadings_dict = shadings.as_dict()?;
        let sh_obj = shadings_dict.get(name)?;
        let shading = doc.resolve_object(sh_obj).ok()?;
        let shading_dict = shading.as_dict()?;

        let cs_obj = shading_dict.get("ColorSpace")?;
        let cs_resolved = doc.resolve_object(cs_obj).ok()?;

        // The CS might be a Name pointing into the page Resources
        // ColorSpace dict. Walk it to its array form so
        // `extract_paint_spot_inks` can match against the
        // `/Separation` / `/DeviceN` head. ~keep
        let cs_array_object: Object = if let Some(cs_name) = cs_resolved.as_name() {
            let cs_dict_obj = res_dict.get("ColorSpace")?;
            let cs_dict_resolved = doc.resolve_object(cs_dict_obj).ok()?;
            let cs_dict = cs_dict_resolved.as_dict()?;
            let named = cs_dict.get(cs_name)?;
            doc.resolve_object(named).ok()?
        } else {
            cs_resolved
        };

        // Extract the function's /C0 endpoint (used for constant
        // gradients; for Type 2 functions this is the value at
        // /Domain[0]). ~keep
        let func_obj = shading_dict.get("Function")?;
        let func_resolved = doc.resolve_object(func_obj).ok()?;
        let func_dict = func_resolved.as_dict()?;
        let c0_obj = func_dict.get("C0")?;
        let c0_arr = c0_obj.as_array()?;
        let c0_components: Vec<f32> = c0_arr
            .iter()
            .map(|o| match o {
                Object::Real(v) => *v as f32,
                Object::Integer(v) => *v as f32,
                _ => 0.0,
            })
            .collect();

        let inks = crate::rendering::sidecar::extract_paint_spot_inks(&cs_array_object, &c0_components, doc);
        if inks.is_empty() { None } else { Some(inks) }
    }

    /// Rasterise the coverage for a shading paint (`sh` operator) by
    /// re-running `render_shading` into a fresh transparent scratch
    /// pixmap with [`Self::coverage_only_gs`] (fill_alpha = 1, /Normal
    /// BM). The shading interpolator paints its gradient colour into
    /// the scratch, and the alpha channel records per-pixel coverage
    /// of the gradient geometry intersected with the active clip
    /// (§8.7.4).
    ///
    /// Returns `None` when the sidecar is detection-OFF.
    fn rasterise_shading_coverage(
        &self,
        name: &str,
        transform: Transform,
        gs: &GraphicsState,
        resources: &Object,
        doc: &PdfDocument,
        clip_mask: Option<&tiny_skia::Mask>,
    ) -> Option<Vec<u8>> {
        let sidecar = self.cmyk_sidecar.as_ref()?;
        let (w, h) = sidecar.dims();
        let mut scratch = Pixmap::new(w, h)?;
        let cov_gs = Self::coverage_only_gs(gs);
        let _ = self.render_shading(&mut scratch, name, transform, &cov_gs, resources, doc, clip_mask);
        Some(Self::extract_alpha_as_coverage(&scratch))
    }

    /// Coverage-aware compose-first that takes a pre-rasterised path
    /// coverage mask. Used when the CMYK sidecar is allocated so the
    /// "painted region" is identified independent of the snap-vs-dest
    /// diff (which fails when source and backdrop ICC-RGB collide,
    /// producing painted=false at pixels that the path actually
    /// covered). Falls through to the standard
    /// [`Self::apply_cmyk_compose_after_paint`] when the sidecar is
    /// None.
    fn apply_cmyk_compose_after_paint_with_coverage(
        &mut self,
        pixmap: &mut Pixmap,
        snapshot: &[u8],
        coverage: Option<&[u8]>,
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) {
        if self.cmyk_sidecar.is_none() || coverage.is_none() {
            // Fall back to the diff-driven path. Detection-OFF
            // byte-identical behaviour. ~keep
            self.apply_cmyk_compose_after_paint(pixmap, snapshot, gs, doc, fill_side);
            return;
        }

        let (sc, sm, sy, sk) = if fill_side {
            match gs.fill_color_cmyk {
                Some(v) => v,
                None => return,
            }
        } else {
            match gs.stroke_color_cmyk {
                Some(v) => v,
                None => return,
            }
        };
        let alpha_g = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };
        let profile = match doc.output_intent_cmyk_profile() {
            Some(p) => p,
            None => return,
        };
        let intent = crate::color::RenderingIntent::from_pdf_name(&gs.rendering_intent);
        let coverage = coverage.expect("checked above");
        // Hoist the ICC transform out of the per-pixel loop. The cache key
        // includes `profile.content_hash()`, which hashes every byte of the
        // ICC profile blob — a per-pixel lookup on a full-page transparency
        // fill ran tens of GB of hash work for the same (profile, intent)
        // tuple every paint. The sibling diff-driven path
        // (`apply_cmyk_compose_after_paint`) hoists the same way. ~keep
        let transform = self.icc_transform_cache.get_or_build(&profile, intent);
        let dest = pixmap.data_mut();

        for px in 0..(dest.len() / 4) {
            let off = px * 4;
            let cov = coverage[px];
            if cov == 0 {
                continue;
            }
            let coverage_frac = cov as f32 / 255.0;
            let c_alpha = (coverage_frac * alpha_g).clamp(0.0, 1.0);

            let plane = self.cmyk_sidecar.as_ref().expect("checked above").cmyk();
            let dc = plane[off] as f32 / 255.0;
            let dm = plane[off + 1] as f32 / 255.0;
            let dy = plane[off + 2] as f32 / 255.0;
            let dk = plane[off + 3] as f32 / 255.0;

            let mc = c_alpha * sc + (1.0 - c_alpha) * dc;
            let mm = c_alpha * sm + (1.0 - c_alpha) * dm;
            let my = c_alpha * sy + (1.0 - c_alpha) * dy;
            let mk = c_alpha * sk + (1.0 - c_alpha) * dk;

            let mc_u8 = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
            let mm_u8 = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
            let my_u8 = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
            let mk_u8 = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;

            let rgb = transform.convert_cmyk_pixel(mc_u8, mm_u8, my_u8, mk_u8);

            dest[off] = rgb[0];
            dest[off + 1] = rgb[1];
            dest[off + 2] = rgb[2];

            let plane = self.cmyk_sidecar.as_mut().expect("re-borrow").cmyk_mut();
            plane[off] = mc_u8;
            plane[off + 1] = mm_u8;
            plane[off + 2] = my_u8;
            plane[off + 3] = mk_u8;
        }
        let _ = snapshot; // diff-path no longer consults the snapshot ~keep
    }

    fn apply_cmyk_compose_after_paint(
        &mut self,
        pixmap: &mut Pixmap,
        snapshot: &[u8],
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) {
        let (sc, sm, sy, sk) = if fill_side {
            match gs.fill_color_cmyk {
                Some(v) => v,
                None => return,
            }
        } else {
            match gs.stroke_color_cmyk {
                Some(v) => v,
                None => return,
            }
        };
        let alpha_g = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };
        let profile = match doc.output_intent_cmyk_profile() {
            Some(p) => p,
            None => return,
        };

        // Build a single ICC transform for this call. The renderer's
        // per-page IccTransformCache holds the compiled qcms transform
        // across the many paint operators on the page; we look it up
        // ONCE here and reuse the Arc<Transform> for every pixel in the
        // loop below. The cache key includes `profile.content_hash()`,
        // which hashes every byte of the profile blob (SipHash over
        // hundreds of KB on a typical CMYK profile); a per-pixel lookup
        // would re-hash the same blob on every paint. ~keep
        let intent = crate::color::RenderingIntent::from_pdf_name(&gs.rendering_intent);
        let transform = self.icc_transform_cache.get_or_build(&profile, intent);

        // Compute the convert-first source RGB the rasteriser actually
        // wrote into the pixmap. We need this to recover the effective
        // coverage `c·α` from the post-paint pixel:
        //   post = (c·α)·src_rgb_ic + (1 - c·α)·snap_rgb
        // The recovery picks the channel with maximum |snap - src| for
        // numerical stability and skips channels where the difference
        // is below a threshold. ~keep
        let src_rgb_ic = {
            let c_u8 = (sc.clamp(0.0, 1.0) * 255.0).round() as u8;
            let m_u8 = (sm.clamp(0.0, 1.0) * 255.0).round() as u8;
            let y_u8 = (sy.clamp(0.0, 1.0) * 255.0).round() as u8;
            let k_u8 = (sk.clamp(0.0, 1.0) * 255.0).round() as u8;
            let rgb = transform.convert_cmyk_pixel(c_u8, m_u8, y_u8, k_u8);
            [rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0]
        };

        let dest = pixmap.data_mut();
        debug_assert_eq!(dest.len(), snapshot.len());

        for px in 0..(dest.len() / 4) {
            let off = px * 4;

            // Detect "this pixel was painted": any RGBA byte differs
            // between snapshot and current pixmap. ~keep
            let painted = dest[off] != snapshot[off]
                || dest[off + 1] != snapshot[off + 1]
                || dest[off + 2] != snapshot[off + 2]
                || dest[off + 3] != snapshot[off + 3];
            if !painted {
                continue;
            }

            let snap_r = snapshot[off] as f32 / 255.0;
            let snap_g = snapshot[off + 1] as f32 / 255.0;
            let snap_b = snapshot[off + 2] as f32 / 255.0;
            let post_r = dest[off] as f32 / 255.0;
            let post_g = dest[off + 1] as f32 / 255.0;
            let post_b = dest[off + 2] as f32 / 255.0;

            // Recover effective coverage c·α by inverting the source-
            // over alpha-blend on the channel with maximum |snap -
            // src_rgb_ic| (most numerically stable). Default to the
            // graphics-state alpha when the source RGB matches the
            // snapshot exactly on every channel — in that case the
            // pixel's RGB contribution is zero so any coverage value
            // produces the same result. ~keep
            let diffs = [
                (snap_r - src_rgb_ic[0]).abs(),
                (snap_g - src_rgb_ic[1]).abs(),
                (snap_b - src_rgb_ic[2]).abs(),
            ];
            let (max_idx, max_diff) = diffs
                .iter()
                .enumerate()
                .fold((0usize, 0.0_f32), |acc, (i, &v)| if v > acc.1 { (i, v) } else { acc });

            let c_alpha = if max_diff > 1.0 / 255.0 {
                let (snap_ch, post_ch, src_ch) = match max_idx {
                    0 => (snap_r, post_r, src_rgb_ic[0]),
                    1 => (snap_g, post_g, src_rgb_ic[1]),
                    _ => (snap_b, post_b, src_rgb_ic[2]),
                };
                ((snap_ch - post_ch) / (snap_ch - src_ch)).clamp(0.0, 1.0)
            } else {
                // Source RGB ≈ snapshot RGB — coverage is moot, but use
                // the graphics-state alpha as a sensible fallback so a
                // non-Normal blend mode still gets the right magnitude. ~keep
                alpha_g
            };

            // Backdrop CMYK source. Two paths:
            //
            //  (a) Sidecar plane present — read CMYK quadruple directly
            //      from the page-resident plate buffer. This is the
            //      press-accurate path; under a non-linear ICC the
            //      additive-clamp inversion below is lossy.
            //  (b) No sidecar — fall back to §10.3.5 additive-clamp
            //      inversion of the snapshot RGB. Exact for the
            //      baseline-white backdrop and the additive-clamp
            //      fallback OutputIntent path; bounded-loss when the
            //      backdrop went through a non-linear ICC. Documented
            //      gap, kept for the detection-OFF path. ~keep
            let (dc, dm, dy, dk) = if let Some(plane) = self.cmyk_sidecar.as_ref().map(CmykSidecar::cmyk) {
                (
                    plane[off] as f32 / 255.0,
                    plane[off + 1] as f32 / 255.0,
                    plane[off + 2] as f32 / 255.0,
                    plane[off + 3] as f32 / 255.0,
                )
            } else {
                (
                    (1.0 - snap_r).max(0.0),
                    (1.0 - snap_g).max(0.0),
                    (1.0 - snap_b).max(0.0),
                    0.0_f32,
                )
            };

            let mc = c_alpha * sc + (1.0 - c_alpha) * dc;
            let mm = c_alpha * sm + (1.0 - c_alpha) * dm;
            let my = c_alpha * sy + (1.0 - c_alpha) * dy;
            let mk = c_alpha * sk + (1.0 - c_alpha) * dk;

            // Convert the composed CMYK through the OutputIntent ICC,
            // reusing the loop-hoisted `transform`. ~keep
            let mc_u8 = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
            let mm_u8 = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
            let my_u8 = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
            let mk_u8 = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;
            let rgb = transform.convert_cmyk_pixel(mc_u8, mm_u8, my_u8, mk_u8);

            dest[off] = rgb[0];
            dest[off + 1] = rgb[1];
            dest[off + 2] = rgb[2];
            // Alpha unchanged — the source-over alpha rule is identical
            // in convert-first vs compose-first, so the tiny_skia
            // rasteriser's alpha output is correct as-is. ~keep

            // Mirror the composed CMYK into the sidecar so subsequent
            // paints see the press-accurate backdrop. The mirror is
            // bypassed when the sidecar is None (detection-OFF
            // byte-identical path). ~keep
            if let Some(plane) = self.cmyk_sidecar.as_mut().map(CmykSidecar::cmyk_mut) {
                plane[off] = mc_u8;
                plane[off + 1] = mm_u8;
                plane[off + 2] = my_u8;
                plane[off + 3] = mk_u8;
            }
        }
    }

    /// Take a snapshot of `pixmap` when the graphics state has the
    /// overprint parameter active for the targeted side. Used by
    /// [`Self::apply_overprint_after_paint`] to recover the pre-paint
    /// pixel state in the painted region so the §11.7.4.3
    /// CompatibleOverprint blend function can be applied.
    ///
    /// The snapshot fires for every source colour space class
    /// classified by [`source_for_overprint`] — DeviceCMYK direct,
    /// DeviceGray/RGB/CIE/ICCBased process spaces, and
    /// Separation/DeviceN. The per-channel blend function dispatches
    /// on the source class; without the snapshot the painted region
    /// could not be identified for compositing.
    fn overprint_snapshot(&self, pixmap: &Pixmap, gs: &GraphicsState, fill_side: bool) -> Option<Vec<u8>> {
        if source_for_overprint(gs, fill_side).is_some() {
            Some(pixmap.data().to_vec())
        } else {
            None
        }
    }

    /// Apply §11.7.4 composite overprint correction to the painted
    /// region. For each pixel where the paint contributed (snapshot
    /// differs from the post-paint pixmap), read the *snapshot's* RGB,
    /// invert to CMYK, and per-plate compose with the new paint's CMYK
    /// quadruple under the active OPM rule:
    ///
    ///   - OPM=0 (standard): non-source plates are knocked out to 0
    ///     except where overprint preserves them; for the composite
    ///     preview the simplest implementation honours "non-zero
    ///     source plate replaces dest" and "zero source plate is
    ///     transparent for that plate, dest preserved".
    ///   - OPM=1 (nonzero): zero source components are transparent for
    ///     their plate (dest preserved); non-zero replace dest plate.
    ///
    /// The merged CMYK is converted back to RGB and written to the
    /// destination pixel, replacing the naïve over-paint result.
    /// Coverage-aware overprint correction. Like
    /// [`Self::apply_cmyk_compose_after_paint_with_coverage`] but for
    /// the §11.7.4 plate merge. Reads backdrop CMYK from the sidecar
    /// instead of the additive-clamp inversion of the snapshot RGB.
    /// Falls back to [`Self::apply_overprint_after_paint`] when the
    /// sidecar is None.
    fn apply_overprint_after_paint_with_coverage(
        &mut self,
        pixmap: &mut Pixmap,
        snapshot: &[u8],
        coverage: Option<&[u8]>,
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) {
        if self.cmyk_sidecar.is_none() || coverage.is_none() {
            self.apply_overprint_after_paint(pixmap, snapshot, gs, doc, fill_side);
            return;
        }

        let Some(source) = source_for_overprint(gs, fill_side) else {
            return;
        };
        let opm = gs.overprint_mode;
        let alpha_g = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };
        let (sc, sm, sy, sk) = source.cmyk;
        let coverage = coverage.expect("checked above");

        let icc_path = doc.output_intent_cmyk_profile().is_some();
        let icc_profile = if icc_path {
            doc.output_intent_cmyk_profile()
        } else {
            None
        };
        let icc_intent = if icc_path {
            Some(crate::color::RenderingIntent::from_pdf_name(&gs.rendering_intent))
        } else {
            None
        };
        let icc_transform = match (icc_profile.as_ref(), icc_intent) {
            (Some(profile), Some(intent)) => Some(self.icc_transform_cache.get_or_build(profile, intent)),
            _ => None,
        };

        let dest = pixmap.data_mut();
        for px in 0..(dest.len() / 4) {
            let off = px * 4;
            let cov = coverage[px];
            if cov == 0 {
                continue;
            }
            // Effective alpha for this pixel — §11.3.3's α'. ~keep
            let c_alpha = ((cov as f32 / 255.0) * alpha_g).clamp(0.0, 1.0);

            let plane = self.cmyk_sidecar.as_ref().expect("checked above").cmyk();
            let dc = plane[off] as f32 / 255.0;
            let dm = plane[off + 1] as f32 / 255.0;
            let dy = plane[off + 2] as f32 / 255.0;
            let dk_existing = plane[off + 3] as f32 / 255.0;

            // §11.7.4.3 per-channel CompatibleOverprint composed with α. ~keep
            let mc = compose_overprint_channel(source.class, ProcessChannel::C, sc, dc, opm, c_alpha);
            let mm = compose_overprint_channel(source.class, ProcessChannel::M, sm, dm, opm, c_alpha);
            let my = compose_overprint_channel(source.class, ProcessChannel::Y, sy, dy, opm, c_alpha);
            let mk = compose_overprint_channel(source.class, ProcessChannel::K, sk, dk_existing, opm, c_alpha);

            let (r_byte, g_byte, b_byte) = if let Some(transform) = icc_transform.as_ref() {
                let mc_u8 = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
                let mm_u8 = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
                let my_u8 = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
                let mk_u8 = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;
                let rgb = transform.convert_cmyk_pixel(mc_u8, mm_u8, my_u8, mk_u8);
                (rgb[0], rgb[1], rgb[2])
            } else {
                let (rr, rg, rb) = cmyk_to_rgb(mc, mm, my, mk);
                (
                    (rr * 255.0).round().clamp(0.0, 255.0) as u8,
                    (rg * 255.0).round().clamp(0.0, 255.0) as u8,
                    (rb * 255.0).round().clamp(0.0, 255.0) as u8,
                )
            };

            dest[off] = r_byte;
            dest[off + 1] = g_byte;
            dest[off + 2] = b_byte;

            let plane = self.cmyk_sidecar.as_mut().expect("re-borrow").cmyk_mut();
            plane[off] = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 1] = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 2] = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 3] = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        let _ = snapshot;
    }

    /// Snapshot the pixmap when the sidecar is active AND the current
    /// paint is an RGB-source paint (DeviceRGB / DeviceGray / CalGray /
    /// RGB ICCBased — i.e. `fill_color_cmyk` is None on the active
    /// side). ISO 32000-1 §11.3.4 defines the §11.3.3 blend / composite
    /// computation that operates inside a single colour space; the
    /// "ONE blend space" mandate itself is §11.4.5.1's `/Group /CS`
    /// definition. On a CMYK OutputIntents page the group blend space
    /// IS CMYK (§11.4.5.1 default for a page-level transparency group
    /// derived from the document's OutputIntent), so an RGB-source
    /// paint must be converted to CMYK at paint-resolution time and
    /// mirrored into the sidecar. The companion helper
    /// [`Self::mirror_rgb_paint_into_sidecar`] runs the conversion +
    /// per-pixel composition.
    fn cmyk_sidecar_snapshot_for_rgb_paint(
        &self,
        pixmap: &Pixmap,
        gs: &GraphicsState,
        fill_side: bool,
    ) -> Option<Vec<u8>> {
        self.cmyk_sidecar.as_ref()?;
        let has_cmyk = if fill_side {
            gs.fill_color_cmyk.is_some()
        } else {
            gs.stroke_color_cmyk.is_some()
        };
        if has_cmyk {
            // The CMYK mirror path handles this paint; the RGB mirror
            // must NOT double-touch the sidecar. ~keep
            return None;
        }
        Some(pixmap.data().to_vec())
    }

    /// Convert the active side's RGB colour to a CMYK quadruple using
    /// the document's OutputIntent CMYK profile when available, or the
    /// §10.3.5 inverse `(C, M, Y) = (1-R, 1-G, 1-B)` with `K = 0`
    /// fallback when the active backend has no CMYK output path. The
    /// fallback loses ink-coverage information in the K plane —
    /// documented behaviour, observable only when the destination
    /// press carries non-zero K under the converted RGB region.
    fn resolve_rgb_paint_to_cmyk(
        &mut self,
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) -> (f32, f32, f32, f32) {
        let (r, g, b) = if fill_side {
            gs.fill_color_rgb
        } else {
            gs.stroke_color_rgb
        };
        let r = r.clamp(0.0, 1.0);
        let g = g.clamp(0.0, 1.0);
        let b = b.clamp(0.0, 1.0);
        if let Some(profile) = doc.output_intent_cmyk_profile() {
            let intent = crate::color::RenderingIntent::from_pdf_name(&gs.rendering_intent);
            if let Some(transform) = self.icc_transform_cache.get_or_build_srgb_to_cmyk(&profile, intent) {
                let cmyk = transform.convert_pixel([r, g, b]);
                return (cmyk[0], cmyk[1], cmyk[2], cmyk[3]);
            }
        }
        // Process-ink separation for the qcms / no-CMM backends: the inverse of
        // the tetralinear `crate::color::cmyk_to_rgb`, so a pure-RGB paint
        // mirrored into the CMYK sidecar and composited back round-trips within
        // the process gamut (an out-of-gamut sRGB paint gamut-compresses). K
        // stays 0 (no black generation). Replaces the additive `(1-R,1-G,1-B)`.
        //
        // When the document catalog DECLARES an /OutputIntents array
        // but `output_intent_cmyk_profile()` returns `None`, the
        // producer asked for a press conversion that we couldn't honour
        // (e.g. profile bytes failed to parse, or no entry carried a
        // /N=4 /DestOutputProfile). Falling through to the K=0 inverse
        // silently degrades press output — the K plane goes empty
        // where the OutputIntent profile would have allocated black
        // ink. Log a one-shot warning so this is observable until
        // upstream lands the proper profile-parse-error diagnostic.
        // When no /OutputIntents declaration is present the K=0
        // fallback is the documented device-RGB behaviour and stays
        // silent. ~keep
        if doc.has_output_intents_declaration() && !self.k_zero_warning_emitted {
            tracing::warn!(
                "rgb→cmyk fallback fired with K=0 while document declares \
                 /OutputIntents. Profile lookup returned None (likely an \
                 unparseable /DestOutputProfile stream); press output \
                 will degrade in the K plane. Tracked upstream as \
                 upstream issue #712."
            );
            self.k_zero_warning_emitted = true;
        }
        crate::color::rgb_to_cmyk(r, g, b)
    }

    /// Mirror an RGB-source paint into the CMYK sidecar via §11.3.4 +
    /// §11.4.5.1 blend-space conversion (§11.4.5.1 defines the group's
    /// /CS as the single blend colour space; §11.3.4 is the per-pixel
    /// compositing computation that runs inside it). Diff-driven
    /// variant for paints with no pre-rasterised coverage; the
    /// with-coverage variant is the hot path under transparency.
    fn mirror_rgb_paint_into_sidecar(
        &mut self,
        pixmap: &Pixmap,
        snapshot: &[u8],
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) {
        if self.cmyk_sidecar.is_none() {
            return;
        }
        let has_cmyk = if fill_side {
            gs.fill_color_cmyk.is_some()
        } else {
            gs.stroke_color_cmyk.is_some()
        };
        if has_cmyk {
            return;
        }
        // Skip overprint paints — overprint is meaningful only on
        // process-channel CMYK sources per §11.7.4.3 Table 149, and
        // the RGB source has no plate assignment to merge. ~keep
        let overprint = if fill_side {
            gs.fill_overprint
        } else {
            gs.stroke_overprint
        };
        if overprint {
            return;
        }

        let alpha = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };
        let (sc, sm, sy, sk) = self.resolve_rgb_paint_to_cmyk(gs, doc, fill_side);

        let post = pixmap.data();
        let plane = match self.cmyk_sidecar.as_mut() {
            Some(s) => s.cmyk_mut(),
            None => return,
        };
        debug_assert_eq!(post.len(), snapshot.len());
        debug_assert_eq!(post.len(), plane.len());

        for px in 0..(post.len() / 4) {
            let off = px * 4;
            let painted = post[off] != snapshot[off]
                || post[off + 1] != snapshot[off + 1]
                || post[off + 2] != snapshot[off + 2]
                || post[off + 3] != snapshot[off + 3];
            if !painted {
                continue;
            }
            // Effective coverage from the alpha-channel delta. For
            // opaque RGB paints the post-alpha is 255 against any
            // backdrop, so coverage = 1. For transparent paints we
            // bound via the alpha; the visible pixmap diff carries
            // alpha edge contributions, but for the §11.3.4 +
            // §11.4.5.1 sidecar mirror the conservative choice is to
            // mirror at the paint's nominal alpha — over-mirroring at
            // an AA-edge pixel still produces a smoothly-graded CMYK
            // backdrop and the next paint's coverage mask defines the
            // final composite. ~keep
            let eff = alpha.clamp(0.0, 1.0);
            let dc = plane[off] as f32 / 255.0;
            let dm = plane[off + 1] as f32 / 255.0;
            let dy = plane[off + 2] as f32 / 255.0;
            let dk = plane[off + 3] as f32 / 255.0;
            let mc = eff * sc + (1.0 - eff) * dc;
            let mm = eff * sm + (1.0 - eff) * dm;
            let my = eff * sy + (1.0 - eff) * dy;
            let mk = eff * sk + (1.0 - eff) * dk;
            plane[off] = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 1] = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 2] = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 3] = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    /// Coverage-aware mirror of RGB-source paints into the CMYK
    /// sidecar. Pattern matches [`Self::mirror_cmyk_paint_into_sidecar_with_coverage`].
    fn mirror_rgb_paint_into_sidecar_with_coverage(
        &mut self,
        pixmap: &Pixmap,
        snapshot: &[u8],
        coverage: Option<&[u8]>,
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) {
        if self.cmyk_sidecar.is_none() || coverage.is_none() {
            self.mirror_rgb_paint_into_sidecar(pixmap, snapshot, gs, doc, fill_side);
            return;
        }
        let has_cmyk = if fill_side {
            gs.fill_color_cmyk.is_some()
        } else {
            gs.stroke_color_cmyk.is_some()
        };
        if has_cmyk {
            return;
        }
        let overprint = if fill_side {
            gs.fill_overprint
        } else {
            gs.stroke_overprint
        };
        if overprint {
            return;
        }
        let alpha = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };
        let (sc, sm, sy, sk) = self.resolve_rgb_paint_to_cmyk(gs, doc, fill_side);

        let coverage = coverage.expect("checked above");
        let plane = self.cmyk_sidecar.as_mut().expect("checked above").cmyk_mut();
        for px in 0..(plane.len() / 4) {
            let cov = coverage[px];
            if cov == 0 {
                continue;
            }
            // Effective alpha at this pixel = path coverage · paint alpha. ~keep
            let eff = (cov as f32 / 255.0) * alpha.clamp(0.0, 1.0);
            let off = px * 4;
            let dc = plane[off] as f32 / 255.0;
            let dm = plane[off + 1] as f32 / 255.0;
            let dy = plane[off + 2] as f32 / 255.0;
            let dk = plane[off + 3] as f32 / 255.0;
            let mc = eff * sc + (1.0 - eff) * dc;
            let mm = eff * sm + (1.0 - eff) * dm;
            let my = eff * sy + (1.0 - eff) * dy;
            let mk = eff * sk + (1.0 - eff) * dk;
            plane[off] = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 1] = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 2] = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 3] = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        let _ = snapshot;
    }

    /// Coverage-aware mirror of opaque CMYK paints into the sidecar.
    /// Like [`Self::mirror_cmyk_paint_into_sidecar`] but uses the
    /// pre-rasterised coverage instead of the snap-vs-dest diff.
    fn mirror_cmyk_paint_into_sidecar_with_coverage(
        &mut self,
        pixmap: &Pixmap,
        snapshot: &[u8],
        coverage: Option<&[u8]>,
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) {
        if self.cmyk_sidecar.is_none() || coverage.is_none() {
            self.mirror_cmyk_paint_into_sidecar(pixmap, snapshot, gs, doc, fill_side);
            return;
        }

        let (sc, sm, sy, sk) = if fill_side {
            match gs.fill_color_cmyk {
                Some(v) => v,
                None => return,
            }
        } else {
            match gs.stroke_color_cmyk {
                Some(v) => v,
                None => return,
            }
        };
        // Skip when the paint is transparent or overprint — those
        // paths handle the sidecar update themselves. ~keep
        let alpha = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };
        let overprint = if fill_side {
            gs.fill_overprint
        } else {
            gs.stroke_overprint
        };
        let transparent = alpha < 1.0 || gs.blend_mode != "Normal" || gs.smask.is_some();
        if transparent || overprint {
            return;
        }

        let coverage = coverage.expect("checked above");
        let plane = self.cmyk_sidecar.as_mut().expect("checked above").cmyk_mut();
        for px in 0..(plane.len() / 4) {
            let cov = coverage[px];
            if cov == 0 {
                continue;
            }
            let cov_f = cov as f32 / 255.0;
            let off = px * 4;
            let dc = plane[off] as f32 / 255.0;
            let dm = plane[off + 1] as f32 / 255.0;
            let dy = plane[off + 2] as f32 / 255.0;
            let dk = plane[off + 3] as f32 / 255.0;
            let mc = cov_f * sc + (1.0 - cov_f) * dc;
            let mm = cov_f * sm + (1.0 - cov_f) * dm;
            let my = cov_f * sy + (1.0 - cov_f) * dy;
            let mk = cov_f * sk + (1.0 - cov_f) * dk;
            plane[off] = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 1] = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 2] = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
            plane[off + 3] = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        let _ = snapshot;
        let _ = doc;
    }

    /// Predicate: should the spot-lane mirror fire for the current paint?
    ///
    /// Returns `true` when:
    /// 1. The sidecar is allocated (page declares transparency / overprint
    ///    AND a CMYK OutputIntent is present).
    /// 2. The active side declares spot inks via `gs.fill_spot_inks` /
    ///    `gs.stroke_spot_inks` (populated by SetFillColorN /
    ///    SetStrokeColorN when the colour space is /Separation or
    ///    /DeviceN per ISO 32000-1 §8.6.6.4 / §8.6.6.5).
    /// 3. At least one of those inks has a corresponding plane in
    ///    `sidecar.spot_names()`. An ink with no plane is the §8.6.6.3
    ///    "device has no plate for this colorant" branch — the
    ///    alternate colour space's CMYK decomposition lands on the
    ///    process plane via the existing CMYK mirror, so there is no
    ///    spot-lane work for this paint.
    fn spot_paint_active(&self, gs: &GraphicsState, fill_side: bool) -> bool {
        let Some(sidecar) = self.cmyk_sidecar.as_ref() else {
            return false;
        };
        let inks = if fill_side {
            &gs.fill_spot_inks
        } else {
            &gs.stroke_spot_inks
        };
        if inks.is_empty() {
            return false;
        }
        inks.iter().any(|(name, _)| sidecar.spot_index(name).is_some())
    }

    /// Apply per-pixel spot lane composition for the most recent paint.
    ///
    /// Composition follows ISO 32000-1 §11.3.3 (basic compositing
    /// formula) + §11.7.4.2 (per-lane BM dispatch). For each active
    /// source spot ink whose plane exists on the page:
    ///
    /// 1. Classify the requested `gs.blend_mode` via
    ///    [`BlendModeClass::from_name`]. The §11.6.3 unknown-name
    ///    fallback keeps unrecognised modes on the /Normal path.
    /// 2. Read the spot's per-lane dispatch
    ///    ([`BlendModeClass::spot_dispatch`]) — for
    ///    [`SpotBlendDispatch::SubstituteNormal`] the §11.7.4.2 rule
    ///    forces /Normal on the spot lane regardless of the requested
    ///    mode.
    /// 3. Compose the new tint per pixel:
    ///    `t_r = (1 - α') · t_b + α' · B(t_b, t_s)` where
    ///    `α' = coverage · gs_alpha`, `t_b` is the backdrop tint,
    ///    `t_s` is the source tint, and `B(·, ·)` is the dispatched
    ///    blend function on subtractive tints. Per §11.3.5.2 Table 136
    ///    the separable formulas operate on additive components — for
    ///    /Normal and the white-preserving modes the subtractive form
    ///    is mathematically equivalent (the formulas are component-wise
    ///    monotonic), so we apply them directly on tint values without
    ///    the additive↔subtractive conversion round-trip.
    ///
    /// Spot inks active on the source but with no plane in the sidecar
    /// (device does not carry the colorant per §8.6.6.3) are silently
    /// skipped — the composite RGB pixmap already received the
    /// alternate-CS approximation through the rasteriser.
    ///
    /// Other spot inks (in `sidecar.spot_names()` but NOT in the
    /// source's `gs.fill_spot_inks` / `gs.stroke_spot_inks`) are NOT
    /// touched. Per §11.7.3, every paint conceptually hits every
    /// component; for unsourced components the spec assigns "additive
    /// 1.0 / subtractive 0.0". Under /Normal: result = source 0.0
    /// composed against backdrop t_b gives `(1 - α') · t_b + α' · 0 =
    /// (1 - α') · t_b` — which for opaque paints `(α' = 1)` would
    /// ERASE the backdrop. Per §11.7.4.3 CompatibleOverprint, when
    /// overprint is enabled the spec instead preserves the backdrop on
    /// unsourced channels (B(c_b, c_s) = c_b). We adopt the
    /// overprint-preserving semantics unconditionally for unsourced
    /// spot lanes: real-world PDFs that target spot inks almost always
    /// expect "paint only what I said to paint" (the CompatibleOverprint
    /// behaviour), and the erase-on-unsourced policy under /Normal
    /// without overprint produces visually wrong output that no
    /// prepress workflow desires. This is pinned as
    /// [`HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP`] in the
    /// probes.
    fn mirror_spot_paint_into_sidecar_with_coverage(
        &mut self,
        pixmap: &Pixmap,
        snapshot: &[u8],
        coverage: Option<&[u8]>,
        gs: &GraphicsState,
        fill_side: bool,
    ) {
        if !self.spot_paint_active(gs, fill_side) {
            return;
        }

        let source_inks: Vec<(String, f32)> = if fill_side {
            gs.fill_spot_inks.clone()
        } else {
            gs.stroke_spot_inks.clone()
        };
        let gs_alpha = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };

        // §11.7.4.2 dispatch: classify the requested BM once. ~keep
        let class = crate::rendering::sidecar::BlendModeClass::from_name(&gs.blend_mode);
        // Per §11.7.4.2 the spot lane either uses the requested BM
        // unchanged, or substitutes /Normal. SubstituteNormal returns
        // "Normal" so the separable_blend helper takes the c_s path
        // identically. ~keep
        let effective_bm: &str = match class.spot_dispatch() {
            crate::rendering::sidecar::SpotBlendDispatch::UseRequested => gs.blend_mode.as_str(),
            crate::rendering::sidecar::SpotBlendDispatch::SubstituteNormal => "Normal",
        };

        // Build a coverage source. Two shapes:
        // * `coverage`: pre-rasterised path coverage from the path-paint
        //   helpers (`rasterise_fill_coverage` / `rasterise_stroke_coverage`).
        //   Bytes are 0..255 effective coverage per pixel.
        // * `None`: paint sites that don't have a separate rasteriser
        //   call (FillStroke combos, text, shading, Do). Fall back to a
        //   snapshot-vs-post diff: any pixel that changed is treated as
        //   "fully painted" (coverage = 255). This loses partial-coverage
        //   fidelity at AA edges; interior pixels are byte-exact. ~keep
        let post = pixmap.data();
        let computed_coverage: Vec<u8>;
        let cov_slice: &[u8] = if let Some(c) = coverage {
            c
        } else {
            debug_assert_eq!(post.len(), snapshot.len());
            computed_coverage = (0..post.len() / 4)
                .map(|px| {
                    let off = px * 4;
                    let changed = post[off] != snapshot[off]
                        || post[off + 1] != snapshot[off + 1]
                        || post[off + 2] != snapshot[off + 2]
                        || post[off + 3] != snapshot[off + 3];
                    if changed { 255 } else { 0 }
                })
                .collect();
            &computed_coverage
        };

        let sidecar = match self.cmyk_sidecar.as_mut() {
            Some(s) => s,
            None => return,
        };

        for (name, tint) in source_inks {
            // §8.6.6.3: ink not in the device's plate set → no spot
            // lane to write. The composite RGB pixmap already carries
            // the alternate-CS approximation. ~keep
            let Some(idx) = sidecar.spot_index(&name) else {
                continue;
            };
            let Some(plane) = sidecar.spot_plane_mut(idx) else {
                continue;
            };
            // The `tint` value is the operator's component for this
            // colorant — already subtractive per §8.6.6.4 / §8.6.6.5. ~keep
            let c_s = tint.clamp(0.0, 1.0);
            debug_assert_eq!(plane.len(), cov_slice.len());

            for (px, cov) in cov_slice.iter().enumerate() {
                let cov = *cov;
                if cov == 0 {
                    continue;
                }
                // Effective coverage·alpha — §11.3.3's α_s. ~keep
                let alpha = (cov as f32 / 255.0) * gs_alpha;
                let alpha = alpha.clamp(0.0, 1.0);
                let t_b = plane[px] as f32 / 255.0;
                let blended = crate::rendering::sidecar::separable_blend(effective_bm, t_b, c_s);
                let t_r = (1.0 - alpha) * t_b + alpha * blended;
                plane[px] = (t_r.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }

    /// Apply ISO 32000-1 §11.7.4.3 CompatibleOverprint to every painted
    /// pixel.
    ///
    /// The §11.7.4.3 blend function `B(c_b, c_s)` returns a subtractive
    /// tint per Table 149, dispatched on source colour space × OP × OPM:
    ///
    /// |Source CS                            |Component          |OP=true OPM=0|OP=true OPM=1                |
    /// |-------------------------------------|-------------------|-------------|-----------------------------|
    /// |DeviceCMYK direct                    |C, M, Y, K         |c_s          |c_s if c_s≠0 else c_b        |
    /// |DeviceCMYK direct                    |Process not in CMYK|c_s          |c_s                          |
    /// |DeviceCMYK direct                    |Spot               |c_b          |c_b                          |
    /// |Any other process CS (e.g. DeviceGray|Process            |c_s          |c_s                          |
    /// |  DeviceRGB, ICCBased, DeviceCMYK    |Spot               |c_b          |c_b                          |
    /// |  via sampled image)                 |                   |             |                             |
    /// |Separation / DeviceN                 |Process            |c_b          |c_b                          |
    /// |                                     |Named spot         |c_s          |c_s                          |
    /// |                                     |Unnamed spot       |c_b          |c_b                          |
    ///
    /// The OPM=1 zero-source-preserve rule is specific to row 1
    /// (DeviceCMYK directly specified). §11.7.4.5 makes this explicit:
    /// "Nonzero overprint mode shall apply only to painting operations
    /// that use the current colour in the graphics state when the
    /// current colour space is DeviceCMYK".
    ///
    /// Each painted pixel composes per §11.3.3 as
    /// `c_r = α · B(c_b, c_s) + (1 − α) · c_b`, where α is the effective
    /// shape×opacity at the pixel. This helper recovers α from the
    /// snapshot-vs-post-paint diff like the coverage-less compose path
    /// does; the coverage-aware variant
    /// ([`Self::apply_overprint_after_paint_with_coverage`]) reads α
    /// directly from the path coverage mask + `gs` alpha.
    ///
    /// The process lanes (CMYK) are written to the sidecar plane and
    /// converted to RGB via the OutputIntent ICC (falling back to the
    /// additive-clamp `cmyk_to_rgb` round-trip when no profile is
    /// available). Spot lanes are handled separately by
    /// [`Self::mirror_spot_paint_into_sidecar_with_coverage`] — for
    /// Separation / DeviceN sources the named spot lane carries c_s; for
    /// all other source classes the spot lane is preserved (no write),
    /// matching Table 149's spot row.
    fn apply_overprint_after_paint(
        &mut self,
        pixmap: &mut Pixmap,
        snapshot: &[u8],
        gs: &GraphicsState,
        doc: &PdfDocument,
        fill_side: bool,
    ) {
        let Some(source) = source_for_overprint(gs, fill_side) else {
            return;
        };
        let opm = gs.overprint_mode;
        let alpha_g = if fill_side { gs.fill_alpha } else { gs.stroke_alpha };
        let (sc, sm, sy, sk) = source.cmyk;
        // ICC path active when the CMYK sidecar plane is present AND an
        // OutputIntent CMYK profile is available. The merged CMYK then
        // runs through the ICC; otherwise the additive-clamp
        // `cmyk_to_rgb` round-trip stays in place. ~keep
        let icc_path = self.cmyk_sidecar.is_some() && doc.output_intent_cmyk_profile().is_some();
        let icc_profile = if icc_path {
            doc.output_intent_cmyk_profile()
        } else {
            None
        };
        let icc_intent = if icc_path {
            Some(crate::color::RenderingIntent::from_pdf_name(&gs.rendering_intent))
        } else {
            None
        };
        let icc_transform = match (icc_profile.as_ref(), icc_intent) {
            (Some(profile), Some(intent)) => Some(self.icc_transform_cache.get_or_build(profile, intent)),
            _ => None,
        };

        // Pre-compute the convert-first source RGB the rasteriser
        // actually wrote. Used to invert the source-over alpha blend
        // and recover effective coverage·alpha per pixel. Mirrors the
        // `apply_cmyk_compose_after_paint` recovery for byte-identity
        // with the compose-first path. ~keep
        let src_rgb_ic = if let Some(transform) = icc_transform.as_ref() {
            let c_u8 = (sc.clamp(0.0, 1.0) * 255.0).round() as u8;
            let m_u8 = (sm.clamp(0.0, 1.0) * 255.0).round() as u8;
            let y_u8 = (sy.clamp(0.0, 1.0) * 255.0).round() as u8;
            let k_u8 = (sk.clamp(0.0, 1.0) * 255.0).round() as u8;
            let rgb = transform.convert_cmyk_pixel(c_u8, m_u8, y_u8, k_u8);
            [rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0]
        } else {
            let (r, g, b) = cmyk_to_rgb(sc, sm, sy, sk);
            [r, g, b]
        };

        let dest = pixmap.data_mut();
        debug_assert_eq!(dest.len(), snapshot.len());

        for px in 0..(dest.len() / 4) {
            let off = px * 4;

            let painted = dest[off] != snapshot[off]
                || dest[off + 1] != snapshot[off + 1]
                || dest[off + 2] != snapshot[off + 2]
                || dest[off + 3] != snapshot[off + 3];
            if !painted {
                continue;
            }

            let snap_r = snapshot[off] as f32 / 255.0;
            let snap_g = snapshot[off + 1] as f32 / 255.0;
            let snap_b = snapshot[off + 2] as f32 / 255.0;
            let post_r = dest[off] as f32 / 255.0;
            let post_g = dest[off + 1] as f32 / 255.0;
            let post_b = dest[off + 2] as f32 / 255.0;
            let diffs = [
                (snap_r - src_rgb_ic[0]).abs(),
                (snap_g - src_rgb_ic[1]).abs(),
                (snap_b - src_rgb_ic[2]).abs(),
            ];
            let (max_idx, max_diff) = diffs
                .iter()
                .enumerate()
                .fold((0usize, 0.0_f32), |acc, (i, &v)| if v > acc.1 { (i, v) } else { acc });
            let c_alpha = if max_diff > 1.0 / 255.0 {
                let (snap_ch, post_ch, src_ch) = match max_idx {
                    0 => (snap_r, post_r, src_rgb_ic[0]),
                    1 => (snap_g, post_g, src_rgb_ic[1]),
                    _ => (snap_b, post_b, src_rgb_ic[2]),
                };
                ((snap_ch - post_ch) / (snap_ch - src_ch)).clamp(0.0, 1.0)
            } else {
                alpha_g
            };

            // Backdrop CMYK from sidecar; additive-clamp fallback when
            // the sidecar is None. ~keep
            let (dc, dm, dy, dk_existing) = if let Some(plane) = self.cmyk_sidecar.as_ref().map(CmykSidecar::cmyk) {
                (
                    plane[off] as f32 / 255.0,
                    plane[off + 1] as f32 / 255.0,
                    plane[off + 2] as f32 / 255.0,
                    plane[off + 3] as f32 / 255.0,
                )
            } else {
                let dr = snapshot[off] as f32 / 255.0;
                let dg = snapshot[off + 1] as f32 / 255.0;
                let db = snapshot[off + 2] as f32 / 255.0;
                ((1.0 - dr).max(0.0), (1.0 - dg).max(0.0), (1.0 - db).max(0.0), 0.0_f32)
            };

            // Per-channel §11.7.4.3 CompatibleOverprint blend function,
            // then §11.3.3 composition with effective alpha. ~keep
            let mc = compose_overprint_channel(source.class, ProcessChannel::C, sc, dc, opm, c_alpha);
            let mm = compose_overprint_channel(source.class, ProcessChannel::M, sm, dm, opm, c_alpha);
            let my = compose_overprint_channel(source.class, ProcessChannel::Y, sy, dy, opm, c_alpha);
            let mk = compose_overprint_channel(source.class, ProcessChannel::K, sk, dk_existing, opm, c_alpha);

            // CMYK → RGB conversion. ICC path for the press-accurate
            // case; additive-clamp `cmyk_to_rgb` for the fallback. ~keep
            let (r_byte, g_byte, b_byte) = if let Some(transform) = icc_transform.as_ref() {
                let mc_u8 = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
                let mm_u8 = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
                let my_u8 = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
                let mk_u8 = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;
                let rgb = transform.convert_cmyk_pixel(mc_u8, mm_u8, my_u8, mk_u8);
                (rgb[0], rgb[1], rgb[2])
            } else {
                let (rr, rg, rb) = cmyk_to_rgb(mc, mm, my, mk);
                (
                    (rr * 255.0).round().clamp(0.0, 255.0) as u8,
                    (rg * 255.0).round().clamp(0.0, 255.0) as u8,
                    (rb * 255.0).round().clamp(0.0, 255.0) as u8,
                )
            };

            // Preserve the painted pixel's alpha (post-paint alpha
            // already accounts for the paint's contribution); just
            // overwrite RGB with the per-channel composed value. ~keep
            dest[off] = r_byte;
            dest[off + 1] = g_byte;
            dest[off + 2] = b_byte;

            // Mirror the composed CMYK into the sidecar so subsequent
            // paints see the post-overprint backdrop. ~keep
            if let Some(plane) = self.cmyk_sidecar.as_mut().map(CmykSidecar::cmyk_mut) {
                plane[off] = (mc.clamp(0.0, 1.0) * 255.0).round() as u8;
                plane[off + 1] = (mm.clamp(0.0, 1.0) * 255.0).round() as u8;
                plane[off + 2] = (my.clamp(0.0, 1.0) * 255.0).round() as u8;
                plane[off + 3] = (mk.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }

    /// Modulate the destination pixmap's painted contribution by the
    /// soft mask declared on `gs`. The mask is rendered once per call
    /// from the referenced Form XObject; on rendering failure the
    /// snapshot is restored (the paint is suppressed entirely — safer
    /// than leaving the unmodulated paint, which would mis-render
    /// content the author intended to hide).
    ///
    /// Per ISO 32000-1:2008 §11.4.7, for each pixel:
    ///
    /// - `S=Alpha`: `mask_value = form_pixmap.alpha[px]`
    /// - `S=Luminosity`: `mask_value = 0.30 R + 0.59 G + 0.11 B` of form_pixmap
    ///
    /// Optional `/TR` transfer is evaluated on the mask value before
    /// modulation. The destination pixel is updated as a linear blend
    /// between `snapshot` and `pixmap` weighted by the mask:
    /// `dest = mask * pixmap + (1 - mask) * snapshot`.
    fn apply_smask_after_paint(
        &mut self,
        pixmap: &mut Pixmap,
        snapshot: &[u8],
        spot_snapshot: Option<&[u8]>,
        gs: &GraphicsState,
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
        base_transform: Transform,
    ) -> Result<()> {
        let smask = match gs.smask.as_ref() {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        // Defend against adversarial cyclic /SMask /G chains: the form
        // referenced by /G can itself declare /SMask on its own
        // content, re-entering this materialisation path. Without a
        // cap recursion is unbounded. At the cap the paint is left
        // unmodulated (the pre-paint snapshot is NOT restored — the
        // caller's paint stays visible) and the recursion unwinds. ~keep
        if self.smask_depth >= MAX_SMASK_DEPTH {
            tracing::warn!(
                "SMask materialisation reached MAX_SMASK_DEPTH={}; \
                 likely cyclic /SMask /G chain. Skipping further \
                 modulation on this paint.",
                MAX_SMASK_DEPTH
            );
            return Ok(());
        }
        self.smask_depth += 1;
        let result = self.apply_smask_after_paint_inner(
            pixmap,
            snapshot,
            spot_snapshot,
            &smask,
            doc,
            page_num,
            resources,
            base_transform,
        );
        self.smask_depth -= 1;
        result
    }

    fn apply_smask_after_paint_inner(
        &mut self,
        pixmap: &mut Pixmap,
        snapshot: &[u8],
        spot_snapshot: Option<&[u8]>,
        smask: &crate::content::graphics_state::SoftMaskForm,
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
        base_transform: Transform,
    ) -> Result<()> {
        // Render the Form XObject into a fresh pixmap. The pixmap
        // starts fully transparent for /S /Alpha (the spec default
        // backdrop is the black point, which projects to alpha=0).
        // For /S /Luminosity the optional /BC backdrop pre-fills with
        // the declared colour; absent /BC the spec default is the
        // colour space's black point (also fills with zeros). ~keep
        let w = pixmap.width();
        let h = pixmap.height();
        let mut mask_pixmap = match Pixmap::new(w, h) {
            Some(p) => p,
            None => {
                // Allocation failed — restore the snapshot to avoid
                // emitting an unmasked paint. ~keep
                pixmap.data_mut().copy_from_slice(snapshot);
                return Ok(());
            }
        };

        // Resolve the Form XObject. We load it before the /BC pre-fill
        // so the pre-fill can consult the Form's /Group /CS for
        // 5+ component DeviceN backdrops (the n=1/3/4 device-family
        // cases don't need the Group CS — array length disambiguates). ~keep
        let form_obj = match doc.load_object(smask.form_ref) {
            Ok(o) => o,
            Err(_) => {
                pixmap.data_mut().copy_from_slice(snapshot);
                return Ok(());
            }
        };

        let (form_dict, form_data) = match &form_obj {
            Object::Stream { dict, .. } => {
                // Decode through the encryption layer if present, the
                // same way render_form_xobject does at the main
                // dispatch site (page_renderer:2320). ~keep
                let data = doc.decode_stream_with_encryption(&form_obj, smask.form_ref)?;
                (dict.clone(), data)
            }
            _ => {
                pixmap.data_mut().copy_from_slice(snapshot);
                return Ok(());
            }
        };

        // For /S /Luminosity, pre-fill with the /BC backdrop if
        // present. The backdrop is in the Group colour space:
        //  - n=1   → /DeviceGray
        //  - n=3   → /DeviceRGB
        //  - n=4   → /DeviceCMYK
        //  - n>=5  → /DeviceN (or /NChannel) declared on the Form's
        //           /Group /CS. Evaluating an /DeviceN backdrop
        //           requires walking the Group /CS tint transform
        //           and projecting the alternate-space colour through
        //           the same path the renderer uses for /Separation /
        //           /DeviceN paints. The helper below handles that. ~keep
        if smask.subtype == crate::content::graphics_state::SoftMaskSubtype::Luminosity {
            if let Some(ref bc) = smask.backdrop {
                let (r, g, b) = match bc.len() {
                    1 => {
                        let v = (bc[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                        (v, v, v)
                    }
                    3 => (
                        (bc[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                        (bc[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                        (bc[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                    ),
                    4 => {
                        let (rf, gf, bf) = cmyk_to_rgb(bc[0], bc[1], bc[2], bc[3]);
                        (
                            (rf * 255.0).round() as u8,
                            (gf * 255.0).round() as u8,
                            (bf * 255.0).round() as u8,
                        )
                    }
                    n if n >= 5 => {
                        // §11.6.5.2 Table 144 + §8.6.6.5: when the
                        // Form group declares DeviceN / NChannel as
                        // its /CS, /BC carries n tints. Evaluate the
                        // group's tint transform on the BC tints and
                        // project the resulting alternate-space colour
                        // to RGB. Falls to (0, 0, 0) (the spec's
                        // black-point default) if the group's CS is
                        // not a recognised DeviceN. ~keep
                        evaluate_devicen_bc_to_rgb(&form_dict, bc, doc).unwrap_or((0, 0, 0))
                    }
                    _ => (0, 0, 0),
                };
                let data = mask_pixmap.data_mut();
                for px in 0..(w * h) as usize {
                    let off = px * 4;
                    data[off] = r;
                    data[off + 1] = g;
                    data[off + 2] = b;
                    data[off + 3] = 255;
                }
            }
        }

        let form_resources_obj = form_dict
            .get("Resources")
            .and_then(|r| doc.resolve_object(r).ok())
            .unwrap_or_else(|| resources.clone());

        // Render the form using the page's base transform: §11.6.5.2
        // mandates the mask be evaluated in the device space in effect
        // at the host paint, which carries both the DPI scale and the
        // PDF→device y-flip. Using `Transform::identity()` here would
        // leave the mask at PDF user-space (72 dpi, y-up) — mis-scaled
        // and y-flipped relative to the pixmap whenever DPI ≠ 72.
        // The form's /Matrix is still composed on top of `base_transform`
        // by `render_form_xobject`, so the mask remains positioned by
        // its own matrix within the page-aligned device frame. ~keep
        let _ = self.render_form_xobject(
            &mut mask_pixmap,
            &form_dict,
            &form_data,
            base_transform,
            doc,
            page_num,
            &form_resources_obj,
        );

        // Resolve /TR transfer function once. The audit fixture uses
        // a Type-2 power function (`N=2` squares the input); the
        // helper below covers Type 2 and falls through to identity
        // for unsupported types. PDF spec §11.4.7 requires identity
        // as the default when /TR is absent. ~keep
        let transfer = smask
            .transfer
            .as_ref()
            .and_then(|tr_obj| doc.resolve_object(tr_obj).ok())
            .and_then(|resolved| parse_transfer_function(doc, &resolved));

        // Apply the mask: pixmap = mask * pixmap + (1 - mask) * snapshot. ~keep
        let mask_data = mask_pixmap.data();
        let dest = pixmap.data_mut();
        debug_assert_eq!(mask_data.len(), dest.len());
        debug_assert_eq!(snapshot.len(), dest.len());

        // §11.3.3 + §11.7.3: the SMask alpha is a single shape/opacity
        // value per pixel that applies to BOTH process and spot colour
        // components. Compute the per-pixel mask alpha once, then
        // attenuate the visible pixmap (RGB+α) AND, when the sidecar
        // is allocated, every spot lane against its pre-mirror
        // snapshot. ~keep
        let pixel_count = dest.len() / 4;
        let mut mask_alpha: Vec<f32> = Vec::with_capacity(pixel_count);
        for px in 0..pixel_count {
            let off = px * 4;
            let mut m = match smask.subtype {
                crate::content::graphics_state::SoftMaskSubtype::Alpha => mask_data[off + 3] as f32 / 255.0,
                crate::content::graphics_state::SoftMaskSubtype::Luminosity => {
                    let r = mask_data[off] as f32 / 255.0;
                    let g = mask_data[off + 1] as f32 / 255.0;
                    let b = mask_data[off + 2] as f32 / 255.0;
                    0.30 * r + 0.59 * g + 0.11 * b
                }
            };

            if let Some(ref tf) = transfer {
                m = tf.eval(m).clamp(0.0, 1.0);
            }
            mask_alpha.push(m);

            let inv_m = 1.0 - m;
            for c in 0..4 {
                let painted = dest[off + c] as f32;
                let backed = snapshot[off + c] as f32;
                let blended = m * painted + inv_m * backed;
                dest[off + c] = blended.clamp(0.0, 255.0).round() as u8;
            }
        }

        // Spot lanes: apply the same SMask alpha attenuation to every
        // spot plane against its pre-mirror snapshot. Per §11.7.3, the
        // soft mask's alpha modulates the spot lane the same way it
        // modulates process channels — a single (shape, opacity) per
        // pixel applies to every lane class. Skipping this step (or
        // applying the SMask only to the pixmap) leaves the spot lanes
        // composed at α=1 while the visible pixmap is attenuated, so
        // the press plate output would over-deposit ink relative to
        // the visible composite by exactly the SMask attenuation
        // factor. ~keep
        if let (Some(pre_spots), Some(sidecar)) = (spot_snapshot, self.cmyk_sidecar.as_mut()) {
            let spots = sidecar.spots_all_mut();
            // The snapshot length tracks the page's spot plane count.
            // If the sidecar's plane count changed mid-paint (it
            // doesn't — fixed at page setup) the comparison would be
            // unsafe; debug-assert it stays in sync. ~keep
            debug_assert_eq!(spots.len(), pre_spots.len());
            let plane_size = pixel_count;
            let plane_count = spots.len() / plane_size;
            for plane_idx in 0..plane_count {
                let base = plane_idx * plane_size;
                for px in 0..plane_size {
                    let m = mask_alpha[px];
                    let inv_m = 1.0 - m;
                    let post = spots[base + px] as f32;
                    let pre = pre_spots[base + px] as f32;
                    let blended = m * post + inv_m * pre;
                    spots[base + px] = blended.clamp(0.0, 255.0).round() as u8;
                }
            }
        }

        Ok(())
    }

    /// Render a knockout transparency group per ISO 32000-1:2008 §11.4.6.2.
    ///
    /// The group's initial backdrop is `pixmap` on entry. Each painted
    /// element composites against that backdrop (not against earlier
    /// elements in the group), and later elements override earlier ones
    /// in overlap regions.
    ///
    /// Implementation: segment the operator stream at paint operators
    /// (Fill / Stroke / FillStroke / PaintShading / DrawObject /
    /// ShowText / inline image). For each paint boundary `i`, render
    /// the cumulative slice `operators[0..=i]` into a fresh
    /// backdrop-copy scratch pixmap. The cumulative replay preserves
    /// graphics-state side effects (color, CTM, clip) across paint
    /// boundaries while keeping each paint's pixel contribution
    /// referenced to the original backdrop. The scratch pixmap's
    /// differences from the backdrop identify the pixels this element
    /// touched, which then overwrite the accumulator.
    ///
    /// Cost: O(N · K) operator executions where N is total operators
    /// and K is paint operators. Knockout groups are rare in practice
    /// so the quadratic factor is acceptable.
    fn execute_knockout_group(
        &mut self,
        pixmap: &mut Pixmap,
        base_transform: Transform,
        operators: &[Operator],
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
    ) -> Result<()> {
        // Backdrop is the pixmap state at group entry. ~keep
        let width = pixmap.width();
        let height = pixmap.height();
        let backdrop_data: Vec<u8> = pixmap.data().to_vec();

        // Sidecar backdrop snapshot. ISO 32000-1 §11.3.3 + §11.4.6.2:
        // a knockout group composes each element against the group's
        // INITIAL backdrop, and the single (shape, opacity) the spec
        // maintains per pixel applies to BOTH process and spot lanes.
        // So the CMYK plane and every spot plane must be reset to the
        // group's backdrop before each element's cumulative replay,
        // exactly like the RGB pixmap is. Without this reset the
        // round-2 spot mirror's per-paint writes would compose against
        // the previous element's lane state — that is non-isolated
        // group semantics, NOT knockout. The brief calls this out as
        // the round-2 gap the secondary scope of round 3 closes. ~keep
        let sidecar_backdrop_cmyk: Option<Vec<u8>> = self.cmyk_sidecar.as_ref().map(|s| s.cmyk().to_vec());
        let sidecar_backdrop_spots: Option<Vec<u8>> = self.cmyk_sidecar.as_ref().map(|s| s.spots_all().to_vec());

        // Identify paint-operator indices. These define element
        // boundaries. ~keep
        let paint_indices: Vec<usize> = operators
            .iter()
            .enumerate()
            .filter_map(|(i, op)| if is_paint_operator(op) { Some(i) } else { None })
            .collect();

        if paint_indices.is_empty() {
            // No paint ops — still execute for state side effects (rare). ~keep
            return self.execute_operators(pixmap, base_transform, operators, doc, page_num, resources);
        }

        // Accumulator starts as the backdrop. Each element's painted
        // pixels overwrite the accumulator. ~keep
        let mut accumulator: Vec<u8> = backdrop_data.clone();
        // Sidecar accumulators parallel `accumulator` for the process
        // and spot lanes. They start at the group's initial backdrop
        // and absorb per-element scratch-vs-backdrop diffs. ~keep
        let mut sidecar_accum_cmyk: Option<Vec<u8>> = sidecar_backdrop_cmyk.clone();
        let mut sidecar_accum_spots: Option<Vec<u8>> = sidecar_backdrop_spots.clone();

        for &end_idx in &paint_indices {
            // Cumulative replay: graphics-state operators 0..end_idx
            // plus the paint at end_idx, with all PRIOR paint operators
            // filtered out. Filtering keeps the state side effects
            // (CTM, fill color, ExtGState, clip path construction) that
            // the current paint depends on, while ensuring no earlier
            // element's pixel contribution reaches the scratch. The
            // scratch is initialised to the backdrop so the paint
            // composites against the group's initial backdrop only. ~keep
            let mut scratch = Pixmap::new(width, height)
                .ok_or_else(|| crate::error::Error::InvalidPdf("knockout scratch pixmap alloc failed".into()))?;
            scratch.data_mut().copy_from_slice(&backdrop_data);

            // Reset sidecar lanes to the group's backdrop before this
            // element's replay so the per-paint mirror writes compose
            // against the BACKDROP (knockout rule), not against earlier
            // elements' lane state. The §11.4.6.2 spec is explicit: the
            // group's "constituent objects ... shall be composited with
            // the group's initial backdrop rather than with each
            // other". This restoration extends that rule to the
            // process / spot lanes the round-1/2 sidecar carries. ~keep
            if let (Some(sidecar), Some(cmyk_b)) = (self.cmyk_sidecar.as_mut(), sidecar_backdrop_cmyk.as_ref()) {
                sidecar.restore_cmyk(cmyk_b);
            }
            if let (Some(sidecar), Some(spots_b)) = (self.cmyk_sidecar.as_mut(), sidecar_backdrop_spots.as_ref()) {
                sidecar.restore_spots(spots_b);
            }

            let element_ops: Vec<Operator> = operators[..=end_idx]
                .iter()
                .enumerate()
                .filter_map(|(i, op)| {
                    if i < end_idx && is_paint_operator(op) {
                        None
                    } else {
                        Some(op.clone())
                    }
                })
                .collect();

            self.execute_operators(&mut scratch, base_transform, &element_ops, doc, page_num, resources)?;

            // Merge: where scratch differs from backdrop, this element
            // touched the pixel — its value overrides the accumulator.
            // Comparing scratch vs backdrop (not vs accumulator) is the
            // key knockout semantic: each element sees only the
            // backdrop, never the accumulated paint from earlier
            // elements. ~keep
            let scratch_data = scratch.data();
            debug_assert_eq!(scratch_data.len(), backdrop_data.len());
            debug_assert_eq!(accumulator.len(), backdrop_data.len());

            for px in 0..(scratch_data.len() / 4) {
                let off = px * 4;
                let same = scratch_data[off] == backdrop_data[off]
                    && scratch_data[off + 1] == backdrop_data[off + 1]
                    && scratch_data[off + 2] == backdrop_data[off + 2]
                    && scratch_data[off + 3] == backdrop_data[off + 3];
                if !same {
                    accumulator[off] = scratch_data[off];
                    accumulator[off + 1] = scratch_data[off + 1];
                    accumulator[off + 2] = scratch_data[off + 2];
                    accumulator[off + 3] = scratch_data[off + 3];
                }
            }

            // Merge sidecar lanes: any byte that differs from the
            // backdrop snapshot was written by this element's paint
            // mirror. Pull the post-element value into the accumulator
            // so later replay iterations see only the backdrop on
            // restore, but the merged group result preserves every
            // element's contribution (last-paint wins on per-byte
            // collision, mirroring the pixmap merge). ~keep
            if let (Some(sidecar), Some(accum), Some(backdrop)) = (
                self.cmyk_sidecar.as_ref(),
                sidecar_accum_cmyk.as_mut(),
                sidecar_backdrop_cmyk.as_ref(),
            ) {
                let post = sidecar.cmyk();
                debug_assert_eq!(post.len(), backdrop.len());
                debug_assert_eq!(accum.len(), backdrop.len());
                for i in 0..post.len() {
                    if post[i] != backdrop[i] {
                        accum[i] = post[i];
                    }
                }
            }
            if let (Some(sidecar), Some(accum), Some(backdrop)) = (
                self.cmyk_sidecar.as_ref(),
                sidecar_accum_spots.as_mut(),
                sidecar_backdrop_spots.as_ref(),
            ) {
                let post = sidecar.spots_all();
                debug_assert_eq!(post.len(), backdrop.len());
                debug_assert_eq!(accum.len(), backdrop.len());
                for i in 0..post.len() {
                    if post[i] != backdrop[i] {
                        accum[i] = post[i];
                    }
                }
            }
        }

        // Replay any trailing non-paint operators (state side effects
        // that follow the last paint) onto the accumulator. The group's
        // visible output IS the accumulator, so we install it before
        // returning. ~keep
        pixmap.data_mut().copy_from_slice(&accumulator);

        // Install the merged sidecar accumulators back into the
        // sidecar. The group's spot and process lanes are now the
        // accumulated knockout result — later operators (outside the
        // group) compose against this state. ~keep
        if let (Some(sidecar), Some(cmyk_a)) = (self.cmyk_sidecar.as_mut(), sidecar_accum_cmyk.as_ref()) {
            sidecar.restore_cmyk(cmyk_a);
        }
        if let (Some(sidecar), Some(spots_a)) = (self.cmyk_sidecar.as_mut(), sidecar_accum_spots.as_ref()) {
            sidecar.restore_spots(spots_a);
        }
        Ok(())
    }

    /// Render annotations for a page.
    fn render_annotations(
        &mut self,
        pixmap: &mut Pixmap,
        base_transform: Transform,
        doc: &PdfDocument,
        page_num: usize,
    ) -> Result<()> {
        let annotations = doc.get_annotations(page_num)?;
        // Reuse the per-render snapshot so we don't deep-clone the HashSet here. ~keep
        let excluded_snapshot: Option<Arc<HashSet<String>>> = self.excluded_layers_snapshot.clone();
        for annot in annotations {
            // Per ISO 32000-1 §12.5.2, an annotation dict may carry an /OC
            // entry referencing the OCG/OCMD the annotation belongs to. Skip
            // the annotation entirely if its layer is excluded. ~keep
            if let Some(ref excluded_layers) = excluded_snapshot {
                if let Some(oc_obj) = annot.raw_dict.as_ref().and_then(|d| d.get("OC")) {
                    if crate::optional_content::annotation_is_excluded(oc_obj, doc, excluded_layers) {
                        continue;
                    }
                }
            }
            if let Some(ap_obj) = annot.raw_dict.as_ref().and_then(|d| d.get("AP")) {
                let ap_stream_obj = doc.resolve_object(ap_obj)?;

                // ISO 32000-1 §12.5.5: /N is the normal appearance; /D and /R
                // show only under pointer press or hover, so an /AP without /N
                // draws nothing in a static render. ~keep
                if let Object::Dictionary(ap_dict) = ap_stream_obj {
                    if let Some(n_entry) = ap_dict.get("N") {
                        let n_stream_obj = doc.resolve_object(n_entry)?;
                        if let Object::Stream { ref dict, .. } = n_stream_obj {
                            let ap_data = if let Some(r) = n_entry.as_reference() {
                                doc.decode_stream_with_encryption(&n_stream_obj, r)?
                            } else {
                                n_stream_obj.decode_stream_data()?
                            };

                            if let Some(rect) = annot.rect {
                                let x = rect[0] as f32;
                                let y = rect[1] as f32;
                                let annot_transform = base_transform.pre_translate(x, y);

                                let old_fonts = self.fonts.clone();
                                let old_cs = self.color_spaces.clone();
                                if let Some(res) = dict.get("Resources") {
                                    if let Ok(res_obj) = doc.resolve_object(res) {
                                        self.load_resources(doc, &res_obj)?;
                                    }
                                }

                                self.render_form_xobject(
                                    pixmap,
                                    &dict,
                                    &ap_data,
                                    annot_transform,
                                    doc,
                                    page_num,
                                    &Object::Dictionary(std::collections::HashMap::new()),
                                )?;

                                self.fonts = old_fonts;
                                self.color_spaces = old_cs;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Encode Pixmap to JPEG format.
    fn encode_jpeg(&self, pixmap: &Pixmap) -> Result<Vec<u8>> {
        let width = pixmap.width();
        let height = pixmap.height();
        let data = pixmap.data();

        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for i in 0..(width * height) as usize {
            let r = data[i * 4] as f32;
            let g = data[i * 4 + 1] as f32;
            let b = data[i * 4 + 2] as f32;
            let a = data[i * 4 + 3] as f32 / 255.0;

            if a > 0.0 {
                rgb_data.push((r / a).min(255.0) as u8);
                rgb_data.push((g / a).min(255.0) as u8);
                rgb_data.push((b / a).min(255.0) as u8);
            } else {
                rgb_data.push(0);
                rgb_data.push(0);
                rgb_data.push(0);
            }
        }

        let img = image::ImageBuffer::<image::Rgb<u8>, _>::from_raw(width, height, rgb_data)
            .ok_or_else(|| Error::InvalidPdf("Failed to create image buffer".to_string()))?;

        let mut output = std::io::Cursor::new(Vec::new());
        img.write_to(&mut output, image::ImageFormat::Jpeg)
            .map_err(|e| Error::InvalidPdf(format!("JPEG encoding failed: {}", e)))?;

        Ok(output.into_inner())
    }

    /// Resolve the colours a path operator needs through the resolution
    /// pipeline and return a `GraphicsState` clone with the resolved RGBA
    /// spliced into the fields the rasteriser reads. Returns `None` when
    /// no side produced an RGBA the composite backend can consume
    /// directly — letting the caller borrow the original `gs` without
    /// allocating a clone.
    ///
    /// Path-fill (`f`/`F`/`f*`), path-stroke (`S`), and path
    /// fill-stroke combos (`B`/`b`/`B*`/`b*`) all flow through this;
    /// each variant of [`PipelinePaintKind`] decides which side(s) to
    /// resolve. Both sides resolve independently — the pipeline keys
    /// all of its side-specific behaviour off `intent.side`, so a Type 4
    /// Separation on the fill side and a plain DeviceRGB on the stroke
    /// side route correctly without contaminating each other.
    ///
    /// Text operators use the sibling
    /// [`Self::pipeline_resolve_text_colors`] — the text rasteriser
    /// already clones `gs` to advance `text_matrix`, so handing it
    /// colour overrides rather than a pre-cloned `GraphicsState` keeps
    /// the text path to one clone per operator instead of two.
    pub(crate) fn pipeline_resolve_paint_gs(
        &self,
        doc: &PdfDocument,
        gs: &GraphicsState,
        kind: PipelinePaintKind,
    ) -> Option<GraphicsState> {
        let (fills, strokes) = match kind {
            // ImageMask paints the stencil with the current fill colour
            // and never reads the stroke side; at this helper layer it
            // is semantically equivalent to PathFill. The variant is
            // kept distinct so the wave-5 separation-backend split can
            // dispatch on it without churning callers. ~keep
            PipelinePaintKind::PathFill | PipelinePaintKind::ImageMask => (true, false),
            PipelinePaintKind::PathStroke => (false, true),
            PipelinePaintKind::PathFillStroke => (true, true),
        };
        // Resolve, then short-circuit when the resolved RGBA already
        // equals the GS field that would supply it inline. For
        // Device-family inputs the resolver always returns Some but
        // the answer is the same colour the inline path would read,
        // so a clone here is wasted work. Skipping it keeps the
        // Device-family case allocation-free — the common path most
        // PDFs take. ~keep
        let fill_rgba = if fills {
            self.pipeline_resolve_rgba(doc, gs, PaintSide::Fill)
                .filter(|c| !rgba_matches(*c, gs.fill_color_rgb, gs.fill_alpha))
        } else {
            None
        };
        let stroke_rgba = if strokes {
            self.pipeline_resolve_rgba(doc, gs, PaintSide::Stroke)
                .filter(|c| !rgba_matches(*c, gs.stroke_color_rgb, gs.stroke_alpha))
        } else {
            None
        };
        if fill_rgba.is_none() && stroke_rgba.is_none() {
            return None;
        }
        let mut spliced = gs.clone();
        if let Some((r, g, b, a)) = fill_rgba {
            spliced.fill_color_rgb = (r, g, b);
            spliced.fill_alpha = a;
        }
        if let Some((r, g, b, a)) = stroke_rgba {
            spliced.stroke_color_rgb = (r, g, b);
            spliced.stroke_alpha = a;
        }
        Some(spliced)
    }

    /// Resolve the text-painting colours through the resolution
    /// pipeline and return them as side-tagged RGBA tuples for the text
    /// rasteriser to splice into its own `current_gs` clone. Returns
    /// `None` when the active `Tr` mode does not require any resolved
    /// side, or when neither side produced an RGBA the composite backend
    /// can consume directly — letting the caller hand the rasteriser
    /// the unmodified `gs` reference.
    ///
    /// Mirrors the side-selection logic of
    /// [`Self::pipeline_resolve_paint_gs`] but returns colours rather
    /// than a `GraphicsState` clone: the text rasteriser already clones
    /// `gs` to walk `text_matrix` per glyph (or per `TJ` element), so
    /// it splices the overrides into that clone — eliminating the
    /// operator-arm-side clone we would otherwise pay on every `Tj` /
    /// `TJ` / `'` / `"`.
    ///
    /// `Tr`-mode handling (ISO 32000-1 §9.3.6 Table 106):
    /// * `0`, `2`, `4`, `6` fill the glyph → resolve fill side.
    /// * `1`, `2`, `5`, `6` stroke the glyph → resolve stroke side.
    /// * `3` is invisible (no painting); skip resolution entirely so
    ///   PDFs that emit text-as-OCR-overlay don't pay any pipeline
    ///   cost.
    pub(crate) fn pipeline_resolve_text_colors(&self, doc: &PdfDocument, gs: &GraphicsState) -> Option<ResolvedColors> {
        if gs.render_mode == 3 {
            return None;
        }
        // Same short-circuit as the path helper: a resolved RGBA that
        // matches the GS field the rasteriser would read inline is a
        // no-op override. Filtering it out lets the operator arm pass
        // `None` straight through and skip the per-element
        // `paint.set_color` write inside `render_text`. ~keep
        let fill = if matches!(gs.render_mode, 0 | 2 | 4 | 6) {
            self.pipeline_resolve_rgba(doc, gs, PaintSide::Fill)
                .filter(|c| !rgba_matches(*c, gs.fill_color_rgb, gs.fill_alpha))
        } else {
            None
        };
        let stroke = if matches!(gs.render_mode, 1 | 2 | 5 | 6) {
            self.pipeline_resolve_rgba(doc, gs, PaintSide::Stroke)
                .filter(|c| !rgba_matches(*c, gs.stroke_color_rgb, gs.stroke_alpha))
        } else {
            None
        };
        let colors = ResolvedColors { fill, stroke };
        if colors.is_empty() { None } else { Some(colors) }
    }

    /// Resolve the active colour for `side` through the resolution pipeline.
    /// Returns `None` when the resolver produces a non-RGBA variant the
    /// composite backend cannot consume directly (per-channel outputs
    /// reserved for separation backends).
    ///
    /// Routes the current colour through [`ResolutionPipeline`], which
    /// handles `Separation`/`DeviceN` colour spaces backed by PostScript
    /// Type 4 tint transforms — the case the inline match arms used to
    /// evaluate as `1.0 - tint` before wave 5 deleted the fallback.
    ///
    /// Fill and stroke share one helper because the only differences are
    /// which `gs` fields supply the colour and which `PaintSide` the
    /// pipeline routes against. The pipeline's colour stage already
    /// keys all of its side-specific behaviour (e.g. alpha fold) off
    /// `intent.side`.
    fn pipeline_resolve_rgba(
        &self,
        doc: &PdfDocument,
        gs: &GraphicsState,
        side: PaintSide,
    ) -> Option<(f32, f32, f32, f32)> {
        let (space_name, components) = match side {
            PaintSide::Fill => (gs.fill_color_space.as_str(), &gs.fill_color_components),
            PaintSide::Stroke => (gs.stroke_color_space.as_str(), &gs.stroke_color_components),
        };
        let resolved_space_obj = self.color_spaces.get(space_name);
        let logical = build_logical_color(space_name, components, resolved_space_obj);
        self.run_pipeline_for_logical(doc, &self.color_spaces, logical, gs, side)
    }

    /// `gs`-free overload of the colour-resolution path: route an
    /// explicit colour-space + components tuple through the pipeline and
    /// return the resolved RGBA.
    ///
    /// The path/text/image-mask helpers above read their colour inputs
    /// from `gs.fill_color_space` / `gs.fill_color_components` (or the
    /// stroke equivalents). Shading endpoint colours don't live there —
    /// they sit in the shading dictionary's `/Function /C0` and `/C1`
    /// arrays, alongside the shading dictionary's own `/ColorSpace`. The
    /// dispatcher needs to resolve those two endpoints independently
    /// of `gs` so the gradient backend can hand them to the
    /// interpolator as fixed stops. This helper is that hook: caller
    /// supplies the shading's `/ColorSpace` object directly and the
    /// per-endpoint component list; the helper builds the logical
    /// colour, runs it through the pipeline against a synthesised
    /// graphics state carrying only the requested alpha (every other
    /// `gs` field — blend mode, overprint — is irrelevant for endpoint
    /// resolution because the gradient is composited as a single Source
    /// Over fill by the caller), and returns the RGBA.
    ///
    /// Returns `None` only when the resolver produces a non-RGBA variant
    /// (per-channel outputs reserved for separation backends). The
    /// caller is then expected to fall back to its inline behaviour.
    pub(crate) fn pipeline_resolve_components(
        &self,
        doc: &PdfDocument,
        color_spaces: &HashMap<String, Object>,
        space: &Object,
        components: &[f32],
        alpha: f32,
    ) -> Option<(f32, f32, f32, f32)> {
        // Two shapes appear in real PDFs for a shading dict's
        // `/ColorSpace`: a Name (either a Device alias like
        // `/DeviceRGB` or a per-page resource name like `/CS1`), or an
        // inline Array (e.g. `[/Separation /MagentaSpot /DeviceCMYK
        // funcRef]`). `build_logical_color` already handles both via
        // its name + `Option<&Object>` arguments, so this wrapper just
        // dispatches into it; inline arrays get the empty name so the
        // Device-family fast-path doesn't fire. ~keep
        let (space_name, resolved_space): (&str, Option<&Object>) = match space {
            Object::Name(n) => (n.as_str(), color_spaces.get(n.as_str())),
            other => ("", Some(other)),
        };
        let logical = build_logical_color(space_name, components, resolved_space);

        // The pipeline reads `gs.fill_alpha` for fill-side alpha fold.
        // A synthesised default `GraphicsState` patched with `alpha`
        // produces the correct RGBA; overprint / blend plans on the
        // synth gs are produced but discarded — only the colour is
        // returned. ~keep
        let mut synth_gs = GraphicsState::new();
        synth_gs.fill_alpha = alpha;
        self.run_pipeline_for_logical(doc, color_spaces, logical, &synth_gs, PaintSide::Fill)
    }

    /// Core resolver step shared between [`Self::pipeline_resolve_rgba`]
    /// (gs-bound path-side resolution) and
    /// [`Self::pipeline_resolve_components`] (gs-free shading-endpoint
    /// resolution). Builds the [`PaintIntent`], runs the pipeline, and
    /// projects the resolved colour down to an RGBA tuple — returning
    /// `None` for non-RGBA variants the composite backend cannot
    /// consume directly.
    fn run_pipeline_for_logical(
        &self,
        doc: &PdfDocument,
        color_spaces: &HashMap<String, Object>,
        logical: LogicalColor<'_>,
        gs: &GraphicsState,
        side: PaintSide,
    ) -> Option<(f32, f32, f32, f32)> {
        let pipeline = ResolutionPipeline::new();
        // Document /OutputIntents CMYK profile + page-level
        // /Default[Gray|RGB|CMYK] (§8.6.5.6) + graphics-state rendering
        // intent (§10.7.3) feed the colour stage's ICC dispatch. The
        // `output_intent_cmyk_profile()` accessor already filters for
        // /N=4 and parses the embedded stream; we just hand the Arc
        // (when present) to the context. ~keep
        let output_intent = doc.output_intent_cmyk_profile();
        // Hand the per-page CMYK transform cache to the resolver. The
        // cache lives on `Self` (cleared at render start in
        // `render_page_with_options`); threading it here is what
        // turns the 1000-paint same-colour case from "rebuild qcms
        // transform 1000×" into "cache miss once, hit 999×". ~keep
        let ctx = ResolutionContext::new(doc, color_spaces)
            .with_output_intent(output_intent.as_ref())
            .with_rendering_intent(crate::color::RenderingIntent::from_pdf_name(&gs.rendering_intent))
            .with_defaults(
                color_spaces.get("DefaultGray"),
                color_spaces.get("DefaultRGB"),
                color_spaces.get("DefaultCMYK"),
            )
            .with_icc_transform_cache(Some(&self.icc_transform_cache));
        // No geometry is needed: the colour stage only reads `color`
        // (and reads `gs` for the alpha fold). `ColorOnly` lets the
        // intent express that without conjuring a placeholder path. ~keep
        let intent = PaintIntent {
            kind: PaintKind::ColorOnly,
            side,
            gs,
            color: logical,
            ctm: gs.ctm,
        };
        let cmd = pipeline.resolve(&intent, &ctx, None).ok()?;
        match cmd.color {
            ResolvedColor::Rgba { r, g, b, a } => Some((r, g, b, a)),
            // Genuine DeviceCMYK sources, plus Separation and DeviceN
            // with a DeviceCMYK alternate, emit `Cmyk` so the per-plate
            // backend has the channel decomposition. Project to RGBA
            // via the context-aware CMYK→RGB path: consult the
            // document's /OutputIntents CMYK profile when present, fall
            // back to the process-ink conversion otherwise. ~keep
            ResolvedColor::Cmyk { c, m, y, k, a } => {
                let (r, g, b) = crate::rendering::resolution::color::cmyk_to_rgb_via_intent(c, m, y, k, &ctx);
                Some((r, g, b, a))
            }
            // /ICCBased N=4 with a parseable embedded profile that
            // compiled a usable CMM. Per §8.6.5.5 the embedded profile
            // is THE conversion source for this colour space — it
            // overrides the document /OutputIntents — so the RGB on
            // this variant is already the right composite output. The
            // CMYK side-payload is for the per-plate router only. ~keep
            ResolvedColor::IccCmyk { r, g, b, a, .. } => Some((r, g, b, a)),
            _ => None,
        }
    }
}

/// Per-channel `f32` comparison tolerance used by [`rgba_matches`]. The
/// resolver folds Device-family inputs through the same RGB encoding the
/// inline path uses, so an exact match is the expected case; the
/// epsilon is sized to absorb single-ulp drift from intermediate
/// computations (alpha fold, CMYK → RGB) without admitting an actual
/// colour change. Anything coarser would risk dropping subtle overrides
/// the renderer needs to honour.
const RGBA_MATCH_EPSILON: f32 = 1.0e-6;

/// Single-input single-output transfer function used by `/SMask /TR`.
/// `Identity` is the spec default when `/TR` is absent.
#[derive(Clone, Debug)]
pub(crate) enum SMaskTransfer {
    /// Identity transfer.
    Identity,
    /// `f(x) = C0 + x^N * (C1 - C0)` per §7.10.3 Type 2 functions.
    Type2 {
        /// Lower endpoint of the codomain.
        c0: f32,
        /// Upper endpoint of the codomain.
        c1: f32,
        /// Exponent.
        n: f32,
    },
    /// Type 0 sampled function (§7.10.2). One-dimensional unit-interval
    /// lookup table — the parser materialises the sampled stream into
    /// a `Vec<f32>` so per-pixel evaluation is a single bounded
    /// allocation-free read.
    Type0 {
        /// One sample per /Size[0] entry, decoded to the [0, 1]
        /// output range. Linear interpolation between adjacent entries
        /// evaluates the function at intermediate inputs.
        samples: Vec<f32>,
    },
    /// Type 4 PostScript calculator (§7.10.5). The compiled program
    /// is reused per pixel; `Program` carries no mutable state so
    /// concurrent calls are safe.
    Type4 {
        /// Compiled PostScript program. The caller routes one f64
        /// input through `evaluate` and reads one f64 output.
        program: crate::functions::Program,
    },
    /// Type 3 stitching function (§7.10.4). Combines `k` subfunctions
    /// over disjoint subintervals of `/Domain`. For an SMask /TR the
    /// outer function is 1-input 1-output; each subfunction must also
    /// be 1-input 1-output (verified at parse time). Subfunctions can
    /// themselves be any function type the parser accepts, including
    /// Type 3 — recursive stitching is unusual but spec-legal.
    Type3 {
        /// Subfunctions in domain order. The `Vec`'s heap allocation
        /// breaks the recursive type's would-be infinite size; no
        /// extra `Box` is required (clippy `vec_box`). Length is `k`,
        /// where `k = bounds.len() + 1`.
        subfunctions: Vec<SMaskTransfer>,
        /// `k - 1` boundary values dividing `/Domain` into `k`
        /// subintervals. The i-th subinterval per §7.10.4 step 2 is
        /// `[x0, b0)`, ..., `[b(k-2), x1]` — a boundary value belongs
        /// to the subinterval on its right.
        bounds: Vec<f32>,
        /// `k` pairs of `(e_lo, e_hi)` that linearly remap each
        /// subinterval onto the corresponding subfunction's native
        /// input range. Indexed by subfunction position.
        encode: Vec<(f32, f32)>,
        /// `/Domain` as `(x0, x1)`. Inputs outside this range are
        /// clipped to the nearest endpoint before dispatch.
        domain: (f32, f32),
    },
}

impl SMaskTransfer {
    /// Evaluate the transfer at `x` clamped to its domain `[0, 1]`.
    pub(crate) fn eval(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        match self {
            SMaskTransfer::Identity => x,
            SMaskTransfer::Type2 { c0, c1, n } => {
                let p = x.powf(*n);
                c0 + p * (c1 - c0)
            }
            SMaskTransfer::Type0 { samples } => {
                // §7.10.2 Type-0 sampled: clamp x to [0, 1] (the
                // domain), encode to sample-index space, linearly
                // interpolate between the two nearest entries. ~keep
                let n = samples.len();
                if n == 0 {
                    return x;
                }
                if n == 1 {
                    return samples[0];
                }
                let pos = x * (n as f32 - 1.0);
                let lo = pos.floor() as usize;
                let hi = (lo + 1).min(n - 1);
                let frac = pos - lo as f32;
                let v = samples[lo] * (1.0 - frac) + samples[hi] * frac;
                v.clamp(0.0, 1.0)
            }
            SMaskTransfer::Type4 { program } => {
                // §7.10.5 PostScript calculator. The compiled program
                // takes one f64 input and emits one f64 output for a
                // /TR function (1→1 per §11.6.5.2 Table 144). Failure
                // modes (stack underflow, runtime budget) fall back
                // to identity rather than panicking; the transfer
                // function is a rendering-time concern and a malformed
                // program should not break the page render. ~keep
                match program.evaluate(&[x as f64]) {
                    Ok(out) if !out.is_empty() => (out[0] as f32).clamp(0.0, 1.0),
                    _ => x,
                }
            }
            SMaskTransfer::Type3 {
                subfunctions,
                bounds,
                encode,
                domain,
            } => {
                // §7.10.4 Type 3 stitching. Steps follow the spec:
                //   1. Clip input to `/Domain` (the outer clamp to
                //      [0, 1] at the top of `eval` already constrains
                //      the SMask /TR input to its [0, 1] range; this
                //      tighter clip enforces the function's own
                //      declared /Domain).
                //   2. Find the subinterval index `i` such that
                //      `b(i-1) <= x < b(i)`, with the convention that
                //      a boundary value belongs to the subinterval on
                //      its right and the final subinterval is
                //      half-open at its upper end (`x >= b(k-2)` →
                //      `i = k-1`).
                //   3. Compute the subinterval bounds and linearly
                //      remap `x` from `[lo_i, hi_i]` to the
                //      subfunction's native input range
                //      `[encode_lo_i, encode_hi_i]`.
                //   4. Evaluate the i-th subfunction at the encoded
                //      input; the result is the function's output.
                //
                // Malformed-input policy: an empty subfunctions vec
                // (which the parser rejects, but defensively guarded
                // here) returns the clipped input unchanged. A
                // zero-width subinterval — possible if a /Bounds entry
                // equals one of its neighbouring endpoints — degenerates
                // the linear remap (division by zero); in that case we
                // use the subfunction's `encode_lo` directly, which is
                // the only well-defined point in the remap. ~keep
                let (x0, x1) = *domain;
                let x_clipped = x.clamp(x0, x1);
                let k = subfunctions.len();
                if k == 0 {
                    return x_clipped;
                }
                // Step 2: locate subinterval index via the half-open
                // convention. `partition_point` returns the count of
                // bounds strictly ≤ x_clipped; that count IS the
                // subinterval index because every boundary belongs to
                // the right subinterval. ~keep
                let i = bounds.iter().copied().filter(|b| x_clipped >= *b).count().min(k - 1);
                let lo_i = if i == 0 { x0 } else { bounds[i - 1] };
                let hi_i = if i == k - 1 { x1 } else { bounds[i] };
                let (e_lo, e_hi) = encode.get(i).copied().unwrap_or((0.0, 1.0));
                let encoded = if (hi_i - lo_i).abs() <= f32::EPSILON {
                    // Zero-width subinterval — use the encode-lo
                    // endpoint directly. Any input that falls into a
                    // collapsed subinterval is the boundary point
                    // itself, so this is the only spec-coherent choice. ~keep
                    e_lo
                } else {
                    e_lo + (x_clipped - lo_i) * (e_hi - e_lo) / (hi_i - lo_i)
                };
                subfunctions[i].eval(encoded)
            }
        }
    }
}

/// Parse a `/SMask /TR` function. Type 0 (sampled), Type 2 (exponential
/// interpolation), Type 3 (stitching), and Type 4 (PostScript calculator)
/// are recognised per ISO 32000-1:2008 §7.10. Unrecognised function
/// types fall to Identity, the spec default for an absent or
/// unrecognised /TR per §11.4.7.
fn parse_transfer_function(doc: &PdfDocument, obj: &Object) -> Option<SMaskTransfer> {
    // Identity is a Name `/Identity` per Table 109. Anything else
    // should be a function dictionary. ~keep
    if let Some("Identity") = obj.as_name() {
        return Some(SMaskTransfer::Identity);
    }
    let dict = obj.as_dict()?;
    let ft = dict.get("FunctionType").and_then(Object::as_integer)?;
    match ft {
        0 => parse_type0_transfer_function(obj, dict).or(Some(SMaskTransfer::Identity)),
        2 => {
            let c0 = dict
                .get("C0")
                .and_then(|o| o.as_array())
                .and_then(|a| a.first())
                .and_then(|v| {
                    v.as_real()
                        .map(|r| r as f32)
                        .or_else(|| v.as_integer().map(|i| i as f32))
                })
                .unwrap_or(0.0);
            let c1 = dict
                .get("C1")
                .and_then(|o| o.as_array())
                .and_then(|a| a.first())
                .and_then(|v| {
                    v.as_real()
                        .map(|r| r as f32)
                        .or_else(|| v.as_integer().map(|i| i as f32))
                })
                .unwrap_or(1.0);
            let n = dict
                .get("N")
                .and_then(|v| {
                    v.as_real()
                        .map(|r| r as f32)
                        .or_else(|| v.as_integer().map(|i| i as f32))
                })
                .unwrap_or(1.0);
            Some(SMaskTransfer::Type2 { c0, c1, n })
        }
        3 => parse_type3_transfer_function(doc, dict).or(Some(SMaskTransfer::Identity)),
        4 => parse_type4_transfer_function(obj).or(Some(SMaskTransfer::Identity)),
        _ => Some(SMaskTransfer::Identity),
    }
}

/// Decode a Type 0 sampled-function stream into a unit-interval lookup
/// table over the 1-input 1-output domain. Returns `None` for any
/// shape the SMask /TR contract doesn't accept (multi-input or
/// multi-output) so the caller can fall back to Identity. Per
/// §7.10.2:
///  - `/Domain` is a 2-element array `[lo hi]` defining the input
///    range; for /TR this is `[0 1]` by construction.
///  - `/Range` is a 2-element array defining the output range; for
///    /TR this is `[0 1]` by construction.
///  - `/Size` is a 1-element array `[N]` — N sample positions.
///  - `/BitsPerSample` is the bit count per packed sample (1/2/4/8/
///    12/16/24/32). We accept the canonical 8-bit case the SMask /TR
///    samples-as-LUT pattern uses; deeper depths fall to None.
///  - `/Encode` defaults to `[0 Size[0]-1]` and `/Decode` defaults to
///    `/Range`. We honour the defaults; explicit overrides for /TR
///    are rare but supported via the standard linear remap.
fn parse_type0_transfer_function(
    obj: &Object,
    dict: &std::collections::HashMap<String, Object>,
) -> Option<SMaskTransfer> {
    // Single-input single-output only. /TR per §11.6.5.2 Table 144 is
    // a 1→1 function; reject anything else so we don't silently
    // mishandle a malformed N→M sampled function. ~keep
    let domain_len = dict
        .get("Domain")
        .and_then(|o| o.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let range_len = dict
        .get("Range")
        .and_then(|o| o.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    if domain_len != 2 || range_len != 2 {
        return None;
    }
    let size_arr = dict.get("Size").and_then(|o| o.as_array())?;
    if size_arr.len() != 1 {
        return None;
    }
    let size = size_arr.first().and_then(Object::as_integer)? as usize;
    if size == 0 || size > 65_536 {
        return None;
    }
    let bps = dict.get("BitsPerSample").and_then(Object::as_integer).unwrap_or(8);
    if bps != 8 {
        // Only the 8-bit packing is honoured. Other depths land at
        // Identity to keep the parser simple; a real-world /TR rarely
        // uses anything other than 8-bit samples. ~keep
        return None;
    }
    let stream_bytes = match obj {
        Object::Stream { .. } => obj.decode_stream_data().ok()?,
        _ => return None,
    };
    if stream_bytes.len() < size {
        return None;
    }
    // /Decode default = /Range; /Encode default = [0 Size-1]. For the
    // canonical /TR shape both defaults apply, so the raw sample byte
    // /255 IS the unit-interval LUT value. ~keep
    let dec_lo;
    let dec_hi;
    if let Some(arr) = dict.get("Decode").and_then(|o| o.as_array()) {
        if arr.len() != 2 {
            return None;
        }
        dec_lo = obj_to_f32(arr.first()?)?;
        dec_hi = obj_to_f32(arr.get(1)?)?;
    } else {
        let r = dict.get("Range").and_then(|o| o.as_array())?;
        dec_lo = obj_to_f32(r.first()?)?;
        dec_hi = obj_to_f32(r.get(1)?)?;
    }
    let max_sample_value = 255.0; // bps=8 above ~keep
    let mut samples: Vec<f32> = Vec::with_capacity(size);
    for i in 0..size {
        let raw = stream_bytes[i] as f32;
        let v = dec_lo + (raw / max_sample_value) * (dec_hi - dec_lo);
        samples.push(v.clamp(0.0, 1.0));
    }
    Some(SMaskTransfer::Type0 { samples })
}

/// Compile a Type 4 PostScript calculator stream as a transfer
/// function. The /SMask /TR contract is 1-input 1-output per
/// §11.6.5.2 Table 144; we route through the existing crate-private
/// `Program` evaluator which already serves Separation / DeviceN tint
/// transforms. Returns `None` when the stream isn't a Stream object,
/// the parse fails (orphan procedure body, unknown operator), or the
/// program advertises a multi-input/multi-output shape that doesn't
/// match a transfer function.
fn parse_type4_transfer_function(obj: &Object) -> Option<SMaskTransfer> {
    let dict = obj.as_dict()?;
    let domain_len = dict
        .get("Domain")
        .and_then(|o| o.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let range_len = dict
        .get("Range")
        .and_then(|o| o.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    // §7.10.5: Type 4 requires Domain and Range. /TR is 1→1. ~keep
    if domain_len != 2 || range_len != 2 {
        return None;
    }
    let stream_bytes = match obj {
        Object::Stream { .. } => obj.decode_stream_data().ok()?,
        _ => return None,
    };
    let program = crate::functions::Program::compile(&stream_bytes).ok()?;
    Some(SMaskTransfer::Type4 { program })
}

/// Parse a Type 3 stitching function (§7.10.4) as a transfer function.
/// A stitching function combines `k` subfunctions over disjoint
/// subintervals of `/Domain`, dispatching the input through whichever
/// subfunction's subinterval contains it after a linear remap. The
/// SMask /TR contract is 1-input 1-output (§11.6.5.2 Table 144), so
/// the outer function's `/Domain` is a 2-element array and each
/// subfunction must itself parse as a 1-input 1-output transfer.
///
/// Required entries per Table 39:
///  - `/Domain [x0 x1]` — 2-element array.
///  - `/Functions [f0 ... f(k-1)]` — array of `k` subfunctions, each
///    parsed recursively (any type the dispatcher accepts is valid).
///  - `/Bounds [b0 ... b(k-2)]` — `k - 1` boundary values dividing
///    `/Domain` into `k` subintervals; per §7.10.4 the spec requires
///    `x0 < b0 < b1 < ... < b(k-2) < x1`. We do NOT enforce strict
///    monotonicity here: a zero-width subinterval (e.g. `b(j-1) ==
///    b(j)`, or a boundary equal to an endpoint) is malformed but
///    spec-permitted; the `eval` arm handles the zero-width case by
///    using the subfunction's `encode_lo` directly.
///  - `/Encode [e0_lo e0_hi ... e(k-1)_lo e(k-1)_hi]` — `2k` values
///    mapping each subinterval to its subfunction's native input range.
///
/// Returns `None` for any shape the /TR contract rejects:
/// multi-input outer function, mismatched `/Bounds` or `/Encode`
/// arity, a subfunction that fails to parse, or zero subfunctions.
/// The caller falls back to Identity on `None`.
fn parse_type3_transfer_function(
    doc: &PdfDocument,
    dict: &std::collections::HashMap<String, Object>,
) -> Option<SMaskTransfer> {
    // Outer /Domain must be 1-input (2 values) for a /TR function. ~keep
    let domain_arr = dict.get("Domain").and_then(|o| o.as_array())?;
    if domain_arr.len() != 2 {
        return None;
    }
    let x0 = obj_to_f32(domain_arr.first()?)?;
    let x1 = obj_to_f32(domain_arr.get(1)?)?;

    // /Functions — recursively parse each subfunction. Subfunctions
    // can be indirect refs so we resolve before recursing. ~keep
    let funcs_arr = dict.get("Functions").and_then(|o| o.as_array())?;
    if funcs_arr.is_empty() {
        return None;
    }
    let k = funcs_arr.len();
    let mut subfunctions: Vec<SMaskTransfer> = Vec::with_capacity(k);
    for f in funcs_arr {
        let resolved = doc.resolve_object(f).ok()?;
        let parsed = parse_transfer_function(doc, &resolved)?;
        subfunctions.push(parsed);
    }

    // /Bounds — k-1 entries. ~keep
    let bounds_arr = dict.get("Bounds").and_then(|o| o.as_array())?;
    if bounds_arr.len() != k - 1 {
        return None;
    }
    let mut bounds: Vec<f32> = Vec::with_capacity(k - 1);
    for b in bounds_arr {
        bounds.push(obj_to_f32(b)?);
    }

    // /Encode — 2k entries (k pairs of (lo, hi)). ~keep
    let encode_arr = dict.get("Encode").and_then(|o| o.as_array())?;
    if encode_arr.len() != 2 * k {
        return None;
    }
    let mut encode: Vec<(f32, f32)> = Vec::with_capacity(k);
    for i in 0..k {
        let lo = obj_to_f32(encode_arr.get(2 * i)?)?;
        let hi = obj_to_f32(encode_arr.get(2 * i + 1)?)?;
        encode.push((lo, hi));
    }

    Some(SMaskTransfer::Type3 {
        subfunctions,
        bounds,
        encode,
        domain: (x0, x1),
    })
}

fn obj_to_f32(o: &Object) -> Option<f32> {
    o.as_real()
        .map(|r| r as f32)
        .or_else(|| o.as_integer().map(|i| i as f32))
}

/// Evaluate a /BC backdrop colour whose component count is 5 or more,
/// against the Form XObject's /Group /CS = /DeviceN (or /NChannel).
/// Returns the RGB byte triple after the DeviceN tint transform runs
/// and the alternate-space result projects to RGB.
///
/// Per ISO 32000-1:2008 §11.6.5.2 Table 144 + §8.6.6.5 (DeviceN colour
/// spaces): the BC entry consists of `n` numbers (one per group CS
/// component), and the renderer must evaluate the group's tint
/// transform to project the BC tints into the alternate colour space
/// before any further conversion.
///
/// Returns `None` when:
///  - the Form has no /Group dict, or
///  - the Group has no /CS entry, or
///  - the CS is not a /DeviceN array, or
///  - the tint transform evaluator fails to produce a result.
fn evaluate_devicen_bc_to_rgb(
    form_dict: &std::collections::HashMap<String, Object>,
    bc: &[f32],
    doc: &PdfDocument,
) -> Option<(u8, u8, u8)> {
    let group_obj = form_dict.get("Group")?;
    let group_resolved = doc.resolve_object(group_obj).ok()?;
    let group_dict = group_resolved.as_dict()?;
    let cs_obj = group_dict.get("CS")?;
    let cs_resolved = doc.resolve_object(cs_obj).ok()?;
    let cs_arr = match &cs_resolved {
        Object::Array(arr) => arr,
        _ => return None,
    };
    let type_name = cs_arr.first().and_then(|o| o.as_name())?;
    if type_name != "DeviceN" && type_name != "NChannel" {
        return None;
    }
    let alt_cs_obj = cs_arr.get(2)?;
    let func_obj = cs_arr.get(3)?;
    let func_resolved = doc.resolve_object(func_obj).ok()?;
    let func_dict = func_resolved.as_dict()?;

    let altspace_values: Vec<f32> = evaluate_bc_tint_function(doc, &func_resolved, func_dict, bc)?;

    // Resolve the alternate space (Name → fast path, Array → typed
    // closed-form projection per §8.6.5.2-5 / §8.6.5.5). ~keep
    let alt_resolved = doc.resolve_object(alt_cs_obj).ok()?;
    let (r, g, b) = project_bc_altspace_to_rgb(doc, &alt_resolved, &altspace_values)?;

    Some((
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
    ))
}

/// Evaluate a DeviceN tint-transform function for /BC backdrop
/// resolution, dispatching across PDF function types 0/2/3/4.
///
/// Per ISO 32000-1:2008 §7.10:
///  - **Type 0** (sampled, §7.10.2) — n-dimensional sampled function;
///    evaluated by N-linear interpolation of the surrounding 2^n
///    nearest samples in the packed CLUT stream.
///  - **Type 2** (exponential, §7.10.3) — 1→m; only `bc[0]` reaches the
///    function (Type 2 inputs are scalar by spec).
///  - **Type 3** (stitching, §7.10.4) — 1→m; only `bc[0]` reaches the
///    outer function; the per-subinterval dispatch recurses into any
///    subfunction type the parser accepts.
///  - **Type 4** (PostScript calculator, §7.10.5) — n→m via the
///    crate-private `Program` evaluator.
fn evaluate_bc_tint_function(
    doc: &PdfDocument,
    func_resolved: &Object,
    func_dict: &std::collections::HashMap<String, Object>,
    bc: &[f32],
) -> Option<Vec<f32>> {
    let func_type = func_dict.get("FunctionType").and_then(Object::as_integer)?;
    match func_type {
        0 => evaluate_type0_multi(func_resolved, func_dict, bc),
        2 => Some(evaluate_type2_multi(func_dict, bc.first().copied().unwrap_or(0.0))),
        3 => evaluate_type3_multi(doc, func_dict, bc.first().copied().unwrap_or(0.0)),
        4 => {
            let bytes = match func_resolved {
                Object::Stream { .. } => func_resolved.decode_stream_data().ok()?,
                _ => return None,
            };
            let program = crate::functions::Program::compile(&bytes).ok()?;
            let inputs: Vec<f64> = bc.iter().map(|&v| v as f64).collect();
            let result = program.evaluate(&inputs).ok()?;
            Some(result.into_iter().map(|v| v as f32).collect())
        }
        _ => None,
    }
}

/// Evaluate a Type 2 (exponential interpolation) function with scalar
/// input `x`, returning the per-output samples per §7.10.3:
/// `y_j = C0_j + x^N · (C1_j - C0_j)`.
fn evaluate_type2_multi(dict: &std::collections::HashMap<String, Object>, x: f32) -> Vec<f32> {
    let n_pow = dict
        .get("N")
        .and_then(|o| o.as_real().or_else(|| o.as_integer().map(|i| i as f64)))
        .unwrap_or(1.0) as f32;
    let c0 = dict.get("C0").and_then(|o| o.as_array());
    let c1 = dict.get("C1").and_then(|o| o.as_array());
    let len = c0.map(|a| a.len()).max(c1.map(|a| a.len())).unwrap_or(1);
    let x_pow = if n_pow == 1.0 { x } else { x.powf(n_pow) };
    let mut out = Vec::with_capacity(len);
    for j in 0..len {
        let c0j = c0.and_then(|a| a.get(j)).and_then(obj_to_f32).unwrap_or(0.0);
        let c1j = c1.and_then(|a| a.get(j)).and_then(obj_to_f32).unwrap_or(1.0);
        out.push(c0j + x_pow * (c1j - c0j));
    }
    out
}

/// Evaluate a Type 3 (stitching) function with scalar input `x` per
/// §7.10.4. Recursively evaluates the subfunction containing `x` and
/// returns its per-output samples. Subfunctions can be any function
/// type the dispatcher accepts (Type 0/2/3/4); cyclic Type 3 ⊃ Type 3
/// chains are unusual but spec-legal and supported via the recursive
/// call back into `evaluate_bc_tint_function`.
fn evaluate_type3_multi(
    doc: &PdfDocument,
    dict: &std::collections::HashMap<String, Object>,
    x: f32,
) -> Option<Vec<f32>> {
    let domain_arr = dict.get("Domain").and_then(|o| o.as_array())?;
    if domain_arr.len() != 2 {
        return None;
    }
    let x0 = obj_to_f32(domain_arr.first()?)?;
    let x1 = obj_to_f32(domain_arr.get(1)?)?;

    let funcs_arr = dict.get("Functions").and_then(|o| o.as_array())?;
    if funcs_arr.is_empty() {
        return None;
    }
    let k = funcs_arr.len();

    let bounds_arr = dict.get("Bounds").and_then(|o| o.as_array())?;
    if bounds_arr.len() != k - 1 {
        return None;
    }
    let mut bounds: Vec<f32> = Vec::with_capacity(k - 1);
    for b in bounds_arr {
        bounds.push(obj_to_f32(b)?);
    }

    let encode_arr = dict.get("Encode").and_then(|o| o.as_array())?;
    if encode_arr.len() != 2 * k {
        return None;
    }

    let x_clipped = x.clamp(x0, x1);
    let i = bounds.iter().copied().filter(|b| x_clipped >= *b).count().min(k - 1);
    let lo_i = if i == 0 { x0 } else { bounds[i - 1] };
    let hi_i = if i == k - 1 { x1 } else { bounds[i] };
    let e_lo = obj_to_f32(encode_arr.get(2 * i)?)?;
    let e_hi = obj_to_f32(encode_arr.get(2 * i + 1)?)?;
    let encoded = if (hi_i - lo_i).abs() <= f32::EPSILON {
        e_lo
    } else {
        e_lo + (x_clipped - lo_i) * (e_hi - e_lo) / (hi_i - lo_i)
    };

    let sub_obj = funcs_arr.get(i)?;
    let sub_resolved = doc.resolve_object(sub_obj).ok()?;
    let sub_dict = sub_resolved.as_dict()?;
    evaluate_bc_tint_function(doc, &sub_resolved, sub_dict, &[encoded])
}

/// Cap on the number of sampled-function input dimensions we'll evaluate.
/// Real-world DeviceN colorant counts top out around 8 (the same ceiling
/// `resolution/color.rs` applies to its own Type 0 evaluator); `2^8 = 256`
/// corner samples per evaluation keeps multilinear interpolation cheap while
/// still covering every DeviceN tint transform seen in practice. A `/Domain`
/// array longer than this is rejected rather than looping `2^N` corners for
/// an attacker-controlled `N` — the `/Size` and stream-length guards do not
/// bound `N` on their own, because an all-ones `/Size` makes a one-byte
/// sample stream satisfy them at any dimension count, and at `N >= 64` the
/// `1usize << N` corner count overflows outright.
const MAX_SAMPLED_FUNCTION_DIMS: usize = 8;

/// Evaluate a Type 0 (sampled) function with n-dimensional input `bc`
/// per §7.10.2.
///
/// The sampled function is stored as a packed stream of m·∏Size_i
/// samples; each sample is a `BitsPerSample`-bit unsigned value laid
/// out in row-major order with input dimension 0 varying fastest. We
/// linearly remap each input via `Encode` to a continuous sample index,
/// then n-linearly interpolate among the 2^n surrounding integer-grid
/// samples and finally remap the per-output samples through `Decode`
/// into the function's output range.
///
/// Returns `None` for any shape the evaluator cannot satisfy: missing
/// /Size or /Range, /BitsPerSample outside the canonical 8-bit case
/// (other depths are spec-legal but rare for tint transforms; rejecting
/// the call lets the caller report unsupported), input arity mismatch,
/// more input dimensions than [`MAX_SAMPLED_FUNCTION_DIMS`], stream too
/// short, or any malformed array.
fn evaluate_type0_multi(
    obj: &Object,
    dict: &std::collections::HashMap<String, Object>,
    bc: &[f32],
) -> Option<Vec<f32>> {
    let domain_arr = dict.get("Domain").and_then(|o| o.as_array())?;
    let range_arr = dict.get("Range").and_then(|o| o.as_array())?;
    if domain_arr.len() % 2 != 0 || range_arr.len() % 2 != 0 {
        return None;
    }
    let n_in = domain_arr.len() / 2;
    let n_out = range_arr.len() / 2;
    if n_in == 0 || n_out == 0 || bc.len() < n_in || n_in > MAX_SAMPLED_FUNCTION_DIMS {
        return None;
    }

    let size_arr = dict.get("Size").and_then(|o| o.as_array())?;
    if size_arr.len() != n_in {
        return None;
    }
    let mut sizes: Vec<usize> = Vec::with_capacity(n_in);
    let mut total_samples: usize = 1;
    for s in size_arr {
        let v = s.as_integer()? as usize;
        if v == 0 {
            return None;
        }
        sizes.push(v);
        total_samples = total_samples.checked_mul(v)?;
    }
    total_samples = total_samples.checked_mul(n_out)?;

    let bps = dict.get("BitsPerSample").and_then(Object::as_integer).unwrap_or(8);
    if bps != 8 {
        // §7.10.2 admits 1/2/4/8/12/16/24/32. We accept the canonical
        // 8-bit case used by every tint-transform PDF observed in the
        // wild. Wider depths fall through to None so the caller can
        // record the unsupported case (currently the only consumer is
        // /BC, which records via parent dispatch). ~keep
        return None;
    }
    let max_sample = 255.0_f32;

    let bytes = match obj {
        Object::Stream { .. } => obj.decode_stream_data().ok()?,
        _ => return None,
    };
    if bytes.len() < total_samples {
        return None;
    }

    // Encode: linearly remap each domain input to a continuous index
    // in `[0, Size_i - 1]`. Defaults to `[0 Size_i - 1]` per spec. ~keep
    let encode_arr = dict.get("Encode").and_then(|o| o.as_array());
    let mut encoded_idx: Vec<f32> = Vec::with_capacity(n_in);
    for i in 0..n_in {
        let d_lo = obj_to_f32(domain_arr.get(2 * i)?)?;
        let d_hi = obj_to_f32(domain_arr.get(2 * i + 1)?)?;
        let (e_lo, e_hi) = if let Some(arr) = encode_arr {
            if arr.len() == 2 * n_in {
                (obj_to_f32(arr.get(2 * i)?)?, obj_to_f32(arr.get(2 * i + 1)?)?)
            } else {
                (0.0, (sizes[i] - 1) as f32)
            }
        } else {
            (0.0, (sizes[i] - 1) as f32)
        };
        let x = bc[i].clamp(d_lo, d_hi);
        let mapped = if (d_hi - d_lo).abs() <= f32::EPSILON {
            e_lo
        } else {
            e_lo + (x - d_lo) * (e_hi - e_lo) / (d_hi - d_lo)
        };
        let clamped = mapped.clamp(0.0, (sizes[i] - 1) as f32);
        encoded_idx.push(clamped);
    }

    // N-linear interpolation among the 2^n surrounding integer-grid
    // points. `lo_i` is the floor index per dimension, `frac_i` is the
    // fractional offset toward the next grid point. ~keep
    let mut lo: Vec<usize> = Vec::with_capacity(n_in);
    let mut frac: Vec<f32> = Vec::with_capacity(n_in);
    for i in 0..n_in {
        let v = encoded_idx[i];
        let lo_i = (v.floor() as isize).max(0) as usize;
        let lo_i = lo_i.min(sizes[i] - 1);
        let f_i = if lo_i + 1 >= sizes[i] { 0.0 } else { v - lo_i as f32 };
        lo.push(lo_i);
        frac.push(f_i);
    }

    // Stride per dimension. Dimension 0 varies fastest: stride[0] = n_out,
    // stride[i] = stride[i-1] * sizes[i-1]. ~keep
    let mut strides: Vec<usize> = Vec::with_capacity(n_in);
    let mut acc = n_out;
    for size in &sizes {
        strides.push(acc);
        acc = acc.checked_mul(*size)?;
    }

    // Decode: per-output `[lo hi]` mapping the [0, 255] sample byte to
    // the function's output range. Defaults to `Range`. ~keep
    let decode_arr = dict.get("Decode").and_then(|o| o.as_array());

    let mut out = Vec::with_capacity(n_out);
    let combinations = 1usize << n_in;
    for j in 0..n_out {
        let (d_lo, d_hi) = if let Some(arr) = decode_arr {
            if arr.len() == 2 * n_out {
                (obj_to_f32(arr.get(2 * j)?)?, obj_to_f32(arr.get(2 * j + 1)?)?)
            } else {
                (
                    obj_to_f32(range_arr.get(2 * j)?)?,
                    obj_to_f32(range_arr.get(2 * j + 1)?)?,
                )
            }
        } else {
            (
                obj_to_f32(range_arr.get(2 * j)?)?,
                obj_to_f32(range_arr.get(2 * j + 1)?)?,
            )
        };
        let r_lo = obj_to_f32(range_arr.get(2 * j)?)?;
        let r_hi = obj_to_f32(range_arr.get(2 * j + 1)?)?;

        let mut accum = 0.0_f32;
        for c in 0..combinations {
            // For each combination of {lo, lo+1} across the n_in dims,
            // compute the offset into the packed stream and the
            // multi-linear weight (product of per-dim weights). ~keep
            let mut offset = j;
            let mut weight = 1.0_f32;
            for i in 0..n_in {
                let upper = (c >> i) & 1 == 1;
                let idx_i = if upper { (lo[i] + 1).min(sizes[i] - 1) } else { lo[i] };
                offset += idx_i * strides[i];
                let w_i = if upper { frac[i] } else { 1.0 - frac[i] };
                weight *= w_i;
            }
            let raw = bytes[offset] as f32;
            let decoded = d_lo + (raw / max_sample) * (d_hi - d_lo);
            accum += weight * decoded;
        }
        out.push(accum.clamp(r_lo, r_hi));
    }
    Some(out)
}

/// Project a DeviceN /BC alternate-space tuple into RGB per §8.6.5.
///
/// Supports `DeviceGray` / `DeviceRGB` / `DeviceCMYK` (Name forms and
/// short names), `[/CalGray <<dict>>]`, `[/CalRGB <<dict>>]`,
/// `[/Lab <<dict>>]`, and `[/ICCBased <stream>]` of any N. Cal* and
/// Lab use closed-form §8.6.5.2-4 projections; ICCBased delegates to
/// the linked CMM (lcms2 or qcms) — when no CMM is linked in we fall
/// back to the embedded `/Alternate` colour space recursively, per
/// §8.6.5.5.
fn project_bc_altspace_to_rgb(doc: &PdfDocument, alt_resolved: &Object, values: &[f32]) -> Option<(f32, f32, f32)> {
    if let Some(name) = alt_resolved.as_name() {
        return match name {
            "DeviceCMYK" | "CMYK" if values.len() >= 4 => Some(cmyk_to_rgb(values[0], values[1], values[2], values[3])),
            "DeviceRGB" | "RGB" if values.len() >= 3 => Some((values[0], values[1], values[2])),
            "DeviceGray" | "G" if !values.is_empty() => {
                let v = values[0];
                Some((v, v, v))
            }
            _ => None,
        };
    }

    let arr = match alt_resolved {
        Object::Array(a) => a,
        _ => return None,
    };
    let family = arr.first().and_then(|o| o.as_name())?;
    match family {
        "DeviceCMYK" | "CMYK" if values.len() >= 4 => Some(cmyk_to_rgb(values[0], values[1], values[2], values[3])),
        "DeviceRGB" | "RGB" if values.len() >= 3 => Some((values[0], values[1], values[2])),
        "DeviceGray" | "G" if !values.is_empty() => {
            let v = values[0];
            Some((v, v, v))
        }
        "CalGray" => project_cal_gray_to_rgb(arr.get(1)?, values),
        "CalRGB" => project_cal_rgb_to_rgb(arr.get(1)?, values),
        "Lab" => project_lab_to_rgb(arr.get(1)?, values),
        "ICCBased" => {
            let stream_obj = arr.get(1)?;
            let stream_resolved = doc.resolve_object(stream_obj).ok()?;
            project_iccbased_to_rgb(doc, &stream_resolved, values)
        }
        _ => None,
    }
}

/// §8.6.5.2 CalGray → linear XYZ → sRGB. The /Gamma exponent applies
/// to the input value; the result is multiplied by /WhitePoint and
/// then converted to sRGB through the standard D65 sRGB transform.
fn project_cal_gray_to_rgb(dict_obj: &Object, values: &[f32]) -> Option<(f32, f32, f32)> {
    let dict = dict_obj.as_dict()?;
    let g = values.first().copied().unwrap_or(0.0).clamp(0.0, 1.0);
    let gamma = dict
        .get("Gamma")
        .and_then(|o| o.as_real().or_else(|| o.as_integer().map(|i| i as f64)))
        .unwrap_or(1.0) as f32;
    let wp = read_whitepoint(dict);

    // §8.6.5.2: A_g = a^gamma; X = X_w · A_g; Y = Y_w · A_g; Z = Z_w · A_g. ~keep
    let a_g = g.powf(gamma);
    let x = wp.0 * a_g;
    let y = wp.1 * a_g;
    let z = wp.2 * a_g;
    Some(xyz_to_srgb(x, y, z))
}

/// Parse a Cal* / Lab `/WhitePoint` entry, defaulting to D65
/// (0.9505, 1.0, 1.0890) per the standard sRGB / Cal* convention when
/// the entry is missing or malformed.
fn read_whitepoint(dict: &std::collections::HashMap<String, Object>) -> (f32, f32, f32) {
    let arr = match dict.get("WhitePoint").and_then(|o| o.as_array()) {
        Some(a) if a.len() == 3 => a,
        _ => return (0.9505, 1.0, 1.0890),
    };
    let xw = obj_to_f32(&arr[0]).unwrap_or(0.9505);
    let yw = obj_to_f32(&arr[1]).unwrap_or(1.0);
    let zw = obj_to_f32(&arr[2]).unwrap_or(1.0890);
    (xw, yw, zw)
}

/// §8.6.5.3 CalRGB → linear XYZ → sRGB. Per-channel /Gamma applied to
/// the per-channel input, then the /Matrix multiplies the gamma-applied
/// tuple into linear XYZ; XYZ → sRGB closes the chain.
fn project_cal_rgb_to_rgb(dict_obj: &Object, values: &[f32]) -> Option<(f32, f32, f32)> {
    let dict = dict_obj.as_dict()?;
    if values.len() < 3 {
        return None;
    }
    let a = values[0].clamp(0.0, 1.0);
    let b = values[1].clamp(0.0, 1.0);
    let c = values[2].clamp(0.0, 1.0);
    let (g_r, g_g, g_b) = match dict.get("Gamma").and_then(|o| o.as_array()) {
        Some(arr) if arr.len() == 3 => (
            obj_to_f32(&arr[0]).unwrap_or(1.0),
            obj_to_f32(&arr[1]).unwrap_or(1.0),
            obj_to_f32(&arr[2]).unwrap_or(1.0),
        ),
        _ => (1.0_f32, 1.0_f32, 1.0_f32),
    };
    let mat = match dict.get("Matrix").and_then(|o| o.as_array()) {
        Some(arr) if arr.len() == 9 => {
            let mut m = [0.0_f32; 9];
            for (i, slot) in m.iter_mut().enumerate() {
                *slot = obj_to_f32(&arr[i]).unwrap_or(0.0);
            }
            m
        }
        _ => [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    };

    // §8.6.5.3: A = a^g_a, B = b^g_b, C = c^g_c; XYZ = Matrix · (A B C)^T.
    // The matrix is stored column-major per spec (Table 64): the first
    // three entries are the X column [X_a, Y_a, Z_a], the next three
    // are the Y column, the last three are the Z column. ~keep
    let a_p = a.powf(g_r);
    let b_p = b.powf(g_g);
    let c_p = c.powf(g_b);
    let x = mat[0] * a_p + mat[3] * b_p + mat[6] * c_p;
    let y = mat[1] * a_p + mat[4] * b_p + mat[7] * c_p;
    let z = mat[2] * a_p + mat[5] * b_p + mat[8] * c_p;
    Some(xyz_to_srgb(x, y, z))
}

/// §8.6.5.4 Lab → XYZ → sRGB via the standard CIELab inverse. The
/// dictionary's /WhitePoint sets the reference white; the function
/// `f^-1(t) = t^3 if t > 6/29, else 3·(6/29)^2·(t - 4/29)`.
fn project_lab_to_rgb(dict_obj: &Object, values: &[f32]) -> Option<(f32, f32, f32)> {
    let dict = dict_obj.as_dict()?;
    if values.len() < 3 {
        return None;
    }
    let l = values[0];
    let a = values[1];
    let b = values[2];

    let wp = read_whitepoint(dict);

    // §8.6.5.4: M = (L* + 16) / 116; L_X = M + a*/500; L_Z = M - b*/200. ~keep
    let m = (l + 16.0) / 116.0;
    let l_x = m + a / 500.0;
    let l_z = m - b / 200.0;

    fn inv_f(t: f32) -> f32 {
        let cutoff = 6.0_f32 / 29.0;
        if t > cutoff {
            t * t * t
        } else {
            3.0 * cutoff * cutoff * (t - 4.0 / 29.0)
        }
    }

    let x = wp.0 * inv_f(l_x);
    let y = wp.1 * inv_f(m);
    let z = wp.2 * inv_f(l_z);
    Some(xyz_to_srgb(x, y, z))
}

/// Linear XYZ → sRGB via the standard ITU-R BT.709 / sRGB primaries
/// matrix and the §IEC 61966-2-1 piecewise transfer function. Inputs
/// are CIE XYZ tristimulus values normalised so Y_white = 1.
fn xyz_to_srgb(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    // sRGB primaries matrix (D65 reference). The PDF Cal* /Lab specs
    // express XYZ tristimulus values; sRGB is the canonical output. ~keep
    let r = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
    let g = -0.969_266 * x + 1.8760108 * y + 0.041_556 * z;
    let b = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;
    fn gamma_compress(u: f32) -> f32 {
        let u = u.clamp(0.0, 1.0);
        if u <= 0.0031308 {
            12.92 * u
        } else {
            1.055 * u.powf(1.0 / 2.4) - 0.055
        }
    }
    (gamma_compress(r), gamma_compress(g), gamma_compress(b))
}

/// §8.6.5.5 ICCBased projection. Under a linked CMM (lcms2 or qcms),
/// build a source-profile → sRGB transform and apply it. Without a
/// linked CMM, fall back to the embedded `/Alternate` space and
/// recurse. Without a /Alternate, fall back to the device family
/// inferred from the stream's /N (DeviceGray for N=1, DeviceRGB for
/// N=3, DeviceCMYK for N=4) per §8.6.5.5.
fn project_iccbased_to_rgb(doc: &PdfDocument, stream_resolved: &Object, values: &[f32]) -> Option<(f32, f32, f32)> {
    let dict = stream_resolved.as_dict()?;
    let n = dict.get("N").and_then(|o| o.as_integer()).unwrap_or(3);

    {
        if let Ok(bytes) = stream_resolved.decode_stream_data() {
            if let Some(profile) = crate::color::IccProfile::parse(bytes, n.clamp(0, 255) as u8) {
                let profile = std::sync::Arc::new(profile);
                let intent = crate::color::RenderingIntent::default();
                let transform = crate::color::Transform::new_srgb_target(std::sync::Arc::clone(&profile), intent);
                if transform.has_cmm() {
                    match n {
                        4 if values.len() >= 4 => {
                            let c_u8 = (values[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                            let m_u8 = (values[1].clamp(0.0, 1.0) * 255.0).round() as u8;
                            let y_u8 = (values[2].clamp(0.0, 1.0) * 255.0).round() as u8;
                            let k_u8 = (values[3].clamp(0.0, 1.0) * 255.0).round() as u8;
                            let rgb = transform.convert_cmyk_pixel(c_u8, m_u8, y_u8, k_u8);
                            return Some((rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0));
                        }
                        3 if values.len() >= 3 => {
                            let r_u8 = (values[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                            let g_u8 = (values[1].clamp(0.0, 1.0) * 255.0).round() as u8;
                            let b_u8 = (values[2].clamp(0.0, 1.0) * 255.0).round() as u8;
                            let rgb = transform.convert_rgb_buffer(&[r_u8, g_u8, b_u8]);
                            if rgb.len() >= 3 {
                                return Some((rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0));
                            }
                        }
                        1 if !values.is_empty() => {
                            let g_u8 = (values[0].clamp(0.0, 1.0) * 255.0).round() as u8;
                            let rgb = transform.convert_gray_buffer(&[g_u8]);
                            if rgb.len() >= 3 {
                                return Some((rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // No CMM (or CMM declined the profile) — recurse into /Alternate. ~keep
    if let Some(alt_obj) = dict.get("Alternate") {
        let alt_resolved = doc.resolve_object(alt_obj).ok()?;
        return project_bc_altspace_to_rgb(doc, &alt_resolved, values);
    }
    // No /Alternate — synthesise the device family per /N (§8.6.5.5). ~keep
    match n {
        4 if values.len() >= 4 => Some(cmyk_to_rgb(values[0], values[1], values[2], values[3])),
        3 if values.len() >= 3 => Some((values[0], values[1], values[2])),
        1 if !values.is_empty() => Some((values[0], values[0], values[0])),
        _ => None,
    }
}

/// Returns `true` when the operator paints pixels into the pixmap.
///
/// Used by the knockout-group renderer to segment the operator stream
/// at element boundaries. Per ISO 32000-1:2008 §11.4.6.2 each "element"
/// in a knockout group is delimited by a paint operator and composites
/// independently against the group's initial backdrop.
fn is_paint_operator(op: &Operator) -> bool {
    matches!(
        op,
        Operator::Fill
            | Operator::FillEvenOdd
            | Operator::Stroke
            | Operator::CloseStroke
            | Operator::FillStroke
            | Operator::FillStrokeEvenOdd
            | Operator::CloseFillStroke
            | Operator::CloseFillStrokeEvenOdd
            | Operator::PaintShading { .. }
            | Operator::Do { .. }
            | Operator::InlineImage { .. }
            | Operator::Tj { .. }
            | Operator::TJ { .. }
            | Operator::Quote { .. }
            | Operator::DoubleQuote { .. }
    )
}

/// Returns `true` when the resolved `(r, g, b, a)` matches the supplied
/// rgb triple and alpha within [`RGBA_MATCH_EPSILON`] on every channel.
///
/// Used by the resolution-pipeline helpers to detect no-op overrides:
/// for Device-family inputs the pipeline always produces an RGBA, but
/// the value is the same one the inline path would have read from
/// `gs.*_color_rgb` directly. Skipping the splice in that case keeps
/// the resolution path allocation-free for the common case where no
/// Separation/DeviceN colour space is in play.
fn rgba_matches(resolved: (f32, f32, f32, f32), rgb: (f32, f32, f32), alpha: f32) -> bool {
    let (r, g, b, a) = resolved;
    let (gr, gg, gb) = rgb;
    (r - gr).abs() <= RGBA_MATCH_EPSILON
        && (g - gg).abs() <= RGBA_MATCH_EPSILON
        && (b - gb).abs() <= RGBA_MATCH_EPSILON
        && (a - alpha).abs() <= RGBA_MATCH_EPSILON
}

/// Build a [`LogicalColor`] from the dispatcher's view of the active colour:
/// the fill colour space name, the raw components on the stack, and (when the
/// space is non-Device) the resolved space object from the resources map.
fn build_logical_color<'a>(
    space_name: &str,
    components: &[f32],
    resolved_space: Option<&'a Object>,
) -> LogicalColor<'a> {
    // Device families fold directly into `LogicalColor::Device` — the
    // resolver's spec-conformance for these is verified by colour-stage
    // unit tests; routing through the same Device path keeps the
    // pipeline's behaviour identical to the inline path for the
    // non-Separation cases.
    //
    // Component-count mismatch (e.g. `/ColorSpace /DeviceCMYK` with only
    // 1 component on the stack) falls through to the `_ =>` arm below,
    // which routes through the resolver's gray fallback. Output happens
    // to match the inline `parse_color_array` single-element-array
    // expansion `(g, g, g)` — both paths paint the gray value across
    // all three RGB channels. ~keep
    match space_name {
        "DeviceGray" | "G" if !components.is_empty() => LogicalColor::Device(DeviceColor::Gray(components[0])),
        "DeviceRGB" | "RGB" if components.len() >= 3 => {
            LogicalColor::Device(DeviceColor::Rgb(components[0], components[1], components[2]))
        }
        "DeviceCMYK" | "CMYK" if components.len() >= 4 => LogicalColor::Device(DeviceColor::Cmyk(
            components[0],
            components[1],
            components[2],
            components[3],
        )),
        _ => {
            // Non-device space: hand the resolver the space object so it
            // can dispatch on Separation / DeviceN / ICCBased / Indexed.
            // Fall back to `DeviceGray` as a logical-colour shape if the
            // resources map didn't carry an entry for this name — the
            // resolver's gray fallback then matches the inline path.
            //
            // Use a thread-local static name object to satisfy the
            // `'a` lifetime on the fallback arm without cloning. ~keep
            use std::sync::OnceLock;
            static GRAY_FALLBACK: OnceLock<Object> = OnceLock::new();
            let space =
                resolved_space.unwrap_or_else(|| GRAY_FALLBACK.get_or_init(|| Object::Name("DeviceGray".to_string())));
            LogicalColor::Spaced {
                space,
                components: components.iter().copied().collect(),
            }
        }
    }
}

/// Resize a premultiplied RGBA byte buffer using SIMD-accelerated bilinear filtering.
///
/// The resizer's default treats alpha as straight and multiplies every pixel by it before
/// filtering, then divides it out after; on a premultiplied buffer that leaves a soft edge with
/// more colour than its opacity allows, so the alpha pass is turned off. (GH#1905)
///
/// Returns `None` on failure (zero dimensions, SIMD dispatch error) so callers
/// can fall back to tiny_skia's own resampling path.
fn resize_rgba(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Option<Vec<u8>> {
    use fast_image_resize::images::Image;
    use fast_image_resize::pixels::PixelType;
    use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};

    // from_slice_u8 needs a mutable slice; copy into a local buffer. ~keep
    let mut buf = src.to_vec();
    let src_img = Image::from_slice_u8(src_w, src_h, &mut buf, PixelType::U8x4).ok()?;
    let mut dst_img = Image::new(dst_w, dst_h, PixelType::U8x4);
    Resizer::new()
        .resize(
            &src_img,
            &mut dst_img,
            &ResizeOptions::new()
                .resize_alg(ResizeAlg::Convolution(FilterType::Bilinear))
                .use_alpha(false),
        )
        .ok()?;
    Some(dst_img.into_vec())
}

/// Encode a tiny_skia `Pixmap` to PNG.
///
/// Uses fdeflate (ultra-fast) compression via the `image` crate instead of
/// tiny_skia's built-in `encode_png`, which defaults to flate2 level 6 and is
/// 3–5× slower on typical page images.
fn encode_png(pixmap: &Pixmap) -> Result<Vec<u8>> {
    let w = pixmap.width();
    let h = pixmap.height();

    // Demultiply: tiny_skia stores premultiplied RGBA; PNG expects straight alpha. ~keep
    let src = pixmap.data();
    let mut data = src.to_vec();
    for chunk in data.as_chunks_mut::<4>().0 {
        let a = chunk[3];
        if a != 0 && a != 255 {
            let a32 = a as u32;
            chunk[0] = ((chunk[0] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
            chunk[1] = ((chunk[1] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
            chunk[2] = ((chunk[2] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
        }
    }

    use image::ImageEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut output = Vec::new();
    PngEncoder::new_with_quality(&mut output, CompressionType::Fast, FilterType::Sub)
        .write_image(&data, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| Error::InvalidPdf(format!("PNG encoding failed: {}", e)))?;
    Ok(output)
}

/// Combine two transformations.
fn combine_transforms(base: Transform, ctm: &Matrix) -> Transform {
    base.pre_concat(Transform::from_row(ctm.a, ctm.b, ctm.c, ctm.d, ctm.e, ctm.f))
}

/// Parse a Type 3 font's `/FontMatrix` into a glyph-space → text-space
/// transform. Defaults to the Type 1 matrix `[0.001 0 0 0.001 0 0]` when the
/// entry is missing or malformed (ISO 32000-1 §9.6.5).
fn type3_font_matrix(font_dict: &HashMap<String, Object>) -> Transform {
    if let Some(arr) = font_dict.get("FontMatrix").and_then(|o| o.as_array()) {
        if arr.len() == 6 {
            let f = |i: usize| -> Option<f32> {
                arr[i]
                    .as_real()
                    .map(|r| r as f32)
                    .or_else(|| arr[i].as_integer().map(|v| v as f32))
            };
            if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(g)) = (f(0), f(1), f(2), f(3), f(4), f(5)) {
                if [a, b, c, d, e, g].iter().all(|v| v.is_finite()) {
                    return Transform::from_row(a, b, c, d, e, g);
                }
            }
        }
    }
    Transform::from_row(0.001, 0.0, 0.0, 0.001, 0.0, 0.0)
}

/// Decode a `/CharProcs` glyph stream. Resolves an indirect reference (with
/// encryption support) or decodes a direct stream. Returns `None` for a
/// non-stream object or on decode failure, so the caller skips the glyph.
fn decode_type3_charproc(doc: &PdfDocument, obj: &Object) -> Option<Vec<u8>> {
    if let Some(obj_ref) = obj.as_reference() {
        let resolved = doc.load_object(obj_ref).ok()?;
        if matches!(resolved, Object::Stream { .. }) {
            return doc.decode_stream_with_encryption(&resolved, obj_ref).ok();
        }
        return None;
    }
    if matches!(obj, Object::Stream { .. }) {
        return obj.decode_stream_data().ok();
    }
    None
}

/// Inclusive tile-index range `[lo, hi]` (along one axis) whose cells
/// intersect the device interval `[region_lo, region_hi]`.
///
/// Tile `i` occupies device coordinates `[cell_min + i·step,
/// cell_min + i·step + cell_extent]`. Solving for the indices whose cell
/// interval overlaps the region gives the two bounds; `step` may be
/// negative (a flipped pattern matrix), so the candidates are ordered by
/// `min`/`max` rather than assuming a sign. The range is deliberately
/// over-inclusive by up to one tile on each side (the per-tile clip mask
/// discards any cell that falls entirely outside the fill path), which
/// keeps the arithmetic branch-free.
///
/// `step` must be non-zero — callers guard `|step| >= 0.5` device px
/// before calling — so this never divides by zero.
fn axis_tile_range(region_lo: f32, region_hi: f32, cell_min: f32, cell_extent: f32, step: f32) -> (i32, i32) {
    let a = (region_lo - cell_extent - cell_min) / step;
    let b = (region_hi - cell_min) / step;
    let lo = a.min(b).floor();
    let hi = a.max(b).ceil();
    // Clamp to i32 so an absurd (but guard-passing) region cannot overflow;
    // the caller additionally caps the total tile count. ~keep
    (lo.max(i32::MIN as f32) as i32, hi.min(i32::MAX as f32) as i32)
}

/// Build the image-space → user-space transform for a PDF image blit.
///
/// Per ISO 32000-1 §8.9.5, PDF images live in a unit square in the user
/// coordinate system; image rows are top-to-bottom (opposite of PDF's
/// bottom-to-top y axis). The pre-translate-by-1-in-y + pre-scale-by
/// `1/src_w, -1/src_h` flips the rows AND normalises the source-pixel
/// extent to the unit square, so the caller's `parent` CTM places the
/// image where the PDF demands.
///
/// Shared by `render_image` and `render_image_mask`.
fn image_unit_square_transform(parent: Transform, src_w: u32, src_h: u32) -> Transform {
    parent
        .pre_translate(0.0, 1.0)
        .pre_scale(1.0 / src_w as f32, -1.0 / src_h as f32)
}

/// Build the `PixmapPaint` used to blit an already-flipped image into
/// the page pixmap.
///
/// `image_transform` must already be the output of
/// [`image_unit_square_transform`] (or the SIMD fast path's
/// translate-only equivalent); the helper reads its scale to pick
/// Bicubic when the blit is an upscale or 1:1 and Bilinear when it is a
/// downscale — the same heuristic both `render_image` and
/// `render_image_mask` used independently before this consolidation.
/// `opacity` is the source's alpha (the std-image path passes
/// `gs.fill_alpha`; the ImageMask path bakes alpha into the stencil
/// pixels and passes `1.0`). `blend_mode_pdf` is the PDF blend-mode
/// name from `gs.blend_mode`.
///
/// Shared by `render_image` and `render_image_mask`.
fn pixmap_paint_for_image_blit(image_transform: Transform, opacity: f32, blend_mode_pdf: &str) -> PixmapPaint {
    let mut paint = PixmapPaint::default();
    paint.opacity = opacity;
    paint.blend_mode = crate::rendering::pdf_blend_mode_to_skia(blend_mode_pdf);
    let (xs, ys) = image_transform.get_scale();
    paint.quality = if xs >= 1.0 || ys >= 1.0 {
        tiny_skia::FilterQuality::Bicubic
    } else {
        tiny_skia::FilterQuality::Bilinear
    };
    paint
}

/// Convert DeviceCMYK (0.0-1.0) to DeviceRGB (0.0-1.0) using the PROCESS-INK
/// conversion (`crate::color::cmyk_to_rgb`, tetralinear over the 16 measured ink
/// corners), NOT the naive additive-clamp `R = 1 - min(1, C+K)`. This unifies
/// the renderer's DeviceCMYK display with the text/extraction and image paths so
/// the same CMYK value resolves to the same RGB everywhere (100% K is `#231F20`,
/// 100% cyan `#00ADEF`). The RGB->CMYK sidecar inverse is
/// `crate::color::rgb_to_cmyk`, which keeps the overprint round-trip consistent
/// within the process gamut. A real ICC/OutputIntent CMM still takes precedence
/// when a profile is available.
fn cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> (f32, f32, f32) {
    crate::color::cmyk_to_rgb(c, m, y, k)
}

/// Parse a colour-key `/Mask` array (ISO 32000-1 §8.9.6.4) into per-component
/// `(min, max)` sample ranges. The array is `[min1 max1 min2 max2 ...]` with one
/// pair per colour component, in the image's pre-Decode component space.
///
/// Returns `None` for any malformed array — wrong length for `ncomp`, a
/// non-integer entry, a negative bound, or `min > max` — so the caller can fall
/// back to no masking rather than guess.
fn parse_color_key_mask(arr: &[Object], ncomp: usize) -> Option<Vec<(u32, u32)>> {
    if ncomp == 0 || arr.len() != ncomp * 2 {
        return None;
    }
    let mut ranges = Vec::with_capacity(ncomp);
    for pair in arr.as_chunks::<2>().0 {
        let lo = pair[0].as_integer()?;
        let hi = pair[1].as_integer()?;
        if lo < 0 || hi < 0 || lo > hi {
            return None;
        }
        ranges.push((lo as u32, hi as u32));
    }
    Some(ranges)
}

/// Returns `true` when a pixel's raw component samples all fall within their
/// corresponding colour-key `(min, max)` ranges, meaning the pixel must be made
/// fully transparent (ISO 32000-1 §8.9.6.4). Returns `false` on any length
/// mismatch so a bad range set never masks.
fn color_key_pixel_masked(components: &[u8], ranges: &[(u32, u32)]) -> bool {
    if components.is_empty() || components.len() != ranges.len() {
        return false;
    }
    components
        .iter()
        .zip(ranges.iter())
        .all(|(&c, &(lo, hi))| (c as u32) >= lo && (c as u32) <= hi)
}

/// Convert a straight-alpha RGBA image to the premultiplied form a tiny_skia pixmap holds.
fn premultiply_rgba(rgba: &mut image::RgbaImage) {
    for pixel in rgba.pixels_mut().filter(|pixel| pixel[3] != u8::MAX) {
        let [r, g, b, a] = pixel.0;
        let premultiplied = tiny_skia::ColorU8::from_rgba(r, g, b, a).premultiply();
        pixel.0 = [premultiplied.red(), premultiplied.green(), premultiplied.blue(), a];
    }
}

/// Apply a colour-key `/Mask` to an already-decoded RGBA image by zeroing the
/// alpha of every source pixel whose raw component samples all fall within the
/// mask ranges.
///
/// Colour-key masking is defined against the raw pre-Decode samples. Those are
/// the raw samples the extractor kept when it moved the stored samples out of
/// that space, or else an 8-bit `ImageData::Raw` buffer whose per-pixel byte
/// count matches `ranges.len()`. For anything else (JPEG, 16-bit depths) the
/// ranges cannot be mapped onto the decoded pixels, so masking is skipped
/// rather than applied incorrectly.
fn apply_color_key_mask(
    image: &crate::extractors::images::PdfImage,
    ranges: &[(u32, u32)],
    rgba: &mut image::RgbaImage,
) {
    use crate::extractors::images::ImageData;

    let ncomp = ranges.len();
    let pixels = if let Some(samples) = image.color_key_samples() {
        if samples.len() != image.width() as usize * image.height() as usize * ncomp {
            tracing::warn!("color-key /Mask: {ncomp} ranges do not fit the image's raw samples, skipping");
            return;
        }
        samples
    } else {
        let ImageData::Raw { pixels, format } = image.data() else {
            tracing::warn!("color-key /Mask: non-raw (e.g. JPEG) image, skipping");
            return;
        };
        if image.bits_per_component() != 8 || format.bytes_per_pixel() != ncomp {
            tracing::warn!(
                "color-key /Mask: unsupported layout (bpc={}, bpp={}, ncomp={}), skipping",
                image.bits_per_component(),
                format.bytes_per_pixel(),
                ncomp
            );
            return;
        }
        pixels.as_slice()
    };
    let w = rgba.width() as usize;
    let h = rgba.height() as usize;
    if pixels.len() < w * h * ncomp {
        tracing::warn!("color-key /Mask: sample buffer too small, skipping");
        return;
    }
    for y in 0..h {
        for x in 0..w {
            let base = (y * w + x) * ncomp;
            if color_key_pixel_masked(&pixels[base..base + ncomp], ranges) {
                rgba.get_pixel_mut(x as u32, y as u32)[3] = 0;
            }
        }
    }
}

/// Apply a colour-key `/Mask` to an `/Indexed` image's raw index plane (one
/// byte per pixel -- ISO 32000-1 §8.9.6.4 gives an Indexed colour space
/// exactly one component), zeroing alpha for every pixel whose index falls in
/// the masked range. Reads the index the extractor kept alongside the
/// palette-expanded RGB `apply_color_key_mask` reads, because the mask's
/// range is stated in index space, not in that RGB. (GH#1899)
fn apply_color_key_mask_to_indices(indices: &[u8], ranges: &[(u32, u32)], rgba: &mut image::RgbaImage) {
    let w = rgba.width() as usize;
    let h = rgba.height() as usize;
    if indices.len() < w * h {
        tracing::warn!("color-key /Mask: index buffer too small, skipping");
        return;
    }
    for y in 0..h {
        for x in 0..w {
            let idx = indices[y * w + x];
            if color_key_pixel_masked(std::slice::from_ref(&idx), ranges) {
                rgba.get_pixel_mut(x as u32, y as u32)[3] = 0;
            }
        }
    }
}

// Test-only counter that records how many `apply_pending_clip` calls actually
// materialized a clip (i.e. did real rasterization work). Used by the
// regression probe below to lock in the per-paint-op fast path. ~keep
#[cfg(test)]
pub(crate) static APC_MATERIALIZED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// ISO 32000-1 §11.7.4.3 / Table 149 source colour space classes.
///
/// The CompatibleOverprint blend function `B(c_b, c_s)` selects between
/// source replace (`c_s`) and backdrop preserve (`c_b`) per-channel
/// based on (a) which source CS class the paint operator uses and (b)
/// whether OPM=1's zero-source-preserve rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceCsClass {
    /// `DeviceCMYK` specified directly via `k` / `K` / `sc` / `scn` on
    /// a `/DeviceCMYK` colour space. This is Table 149 row 1 — the only
    /// class for which the OPM=1 zero-source-preserve rule applies. The
    /// process colour components (C, M, Y, K) of the group colour space
    /// receive `B = c_s` under OPM=0 and `B = (c_s if c_s≠0 else c_b)`
    /// under OPM=1.
    DeviceCmykDirect,
    /// Any other process colour space — `DeviceGray`, `DeviceRGB`,
    /// `CalGray`, `CalRGB`, `ICCBased` of any N, or `DeviceCMYK`
    /// not-directly-specified (e.g. a sampled image's pixel colours).
    /// Table 149 row 2: all process colour components of the group CS
    /// get `B = c_s` regardless of OPM. The OPM=1 zero-source-preserve
    /// rule does not apply (§11.7.4.5: "Nonzero overprint mode shall
    /// apply only to painting operations that use the current colour
    /// in the graphics state when the current colour space is
    /// DeviceCMYK").
    OtherProcess,
    /// `Separation` or non-process `DeviceN`. Table 149 row 3: process
    /// colour components preserve backdrop (`B = c_b`); the named-spot
    /// lanes carry `c_s`; unnamed spot lanes preserve backdrop. The
    /// process-side override is the dispositive difference from the
    /// process-CS classes — a Separation paint must NOT mark process
    /// plates even when its alternate colour space rasterised an RGB
    /// approximation into the composite buffer.
    SeparationOrDeviceN,
}

/// One of the four DeviceCMYK process channels. Used by
/// [`compose_overprint_channel`] to identify which channel index of the
/// `Source` CMYK quadruple a per-channel call concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessChannel {
    C,
    M,
    Y,
    K,
}

/// Resolved source colour for the §11.7.4.3 CompatibleOverprint path.
///
/// The CMYK quadruple is the source colour expressed in DeviceCMYK
/// regardless of the original colour space — for DeviceGray it is
/// `(0, 0, 0, 1-g)`, for DeviceRGB it is the §10.3.5 additive-clamp
/// inverse, and for Separation/DeviceN it is the alternate-space
/// evaluation (or `(0, 0, 0, 0)` when the alternate path produces
/// nothing — in that case the process-lane preserve rule does the work).
#[derive(Debug, Clone, Copy)]
struct OverprintSource {
    class: SourceCsClass,
    cmyk: (f32, f32, f32, f32),
}

/// Determine the §11.7.4.3 source colour for an overprint paint.
///
/// Returns `None` when no `B(c_b, c_s)` would fire — the caller should
/// skip the per-channel pass.
///
/// The dispatch reads `gs.fill_color_space` / `gs.stroke_color_space`
/// to classify the source. For DeviceCMYK direct we also require
/// `fill_color_cmyk` / `stroke_color_cmyk` populated; if it is missing
/// (e.g. a stale state where the colour space name is "DeviceCMYK" but
/// the components vector is empty) we degrade gracefully to
/// `OtherProcess` so the source CMYK is recovered from the RGB
/// fallback below.
fn source_for_overprint(gs: &GraphicsState, fill_side: bool) -> Option<OverprintSource> {
    let (space_name, color_cmyk, color_rgb, components, spot_inks) = if fill_side {
        (
            gs.fill_color_space.as_str(),
            gs.fill_color_cmyk,
            gs.fill_color_rgb,
            &gs.fill_color_components,
            &gs.fill_spot_inks,
        )
    } else {
        (
            gs.stroke_color_space.as_str(),
            gs.stroke_color_cmyk,
            gs.stroke_color_rgb,
            &gs.stroke_color_components,
            &gs.stroke_spot_inks,
        )
    };
    let overprint_active = if fill_side {
        gs.fill_overprint
    } else {
        gs.stroke_overprint
    };
    if !overprint_active {
        return None;
    }

    match space_name {
        "DeviceCMYK" | "CMYK" => {
            // Table 149 row 1: DeviceCMYK specified directly. The
            // graphics-state CMYK quadruple is the source. When the
            // colour space is named DeviceCMYK but no component vector
            // landed yet (initial-colour edge case after a `cs` without
            // an `scn`), fall back to (0, 0, 0, 1) — the spec's §8.6.8
            // initial colour for DeviceCMYK. ~keep
            let cmyk = color_cmyk.unwrap_or((0.0, 0.0, 0.0, 1.0));
            Some(OverprintSource {
                class: SourceCsClass::DeviceCmykDirect,
                cmyk,
            })
        }
        "DeviceGray" | "G" | "CalGray" => {
            // Table 149 row 2: DeviceGray maps to CMYK as (0, 0, 0, 1-g)
            // per the standard gray→CMYK conversion (used by the
            // device-space paint pipeline and §10.3.5). ~keep
            let g = components.first().copied().unwrap_or(color_rgb.0);
            let k = (1.0 - g).clamp(0.0, 1.0);
            Some(OverprintSource {
                class: SourceCsClass::OtherProcess,
                cmyk: (0.0, 0.0, 0.0, k),
            })
        }
        "DeviceRGB" | "RGB" | "CalRGB" => {
            // Table 149 row 2: DeviceRGB maps to CMYK via the §10.3.5
            // additive-clamp inverse `C = 1 - R`, `M = 1 - G`,
            // `Y = 1 - B`, `K = 0`. ~keep
            let r = components.first().copied().unwrap_or(color_rgb.0);
            let g = components.get(1).copied().unwrap_or(color_rgb.1);
            let b = components.get(2).copied().unwrap_or(color_rgb.2);
            let c = (1.0 - r).clamp(0.0, 1.0);
            let m = (1.0 - g).clamp(0.0, 1.0);
            let y = (1.0 - b).clamp(0.0, 1.0);
            Some(OverprintSource {
                class: SourceCsClass::OtherProcess,
                cmyk: (c, m, y, 0.0),
            })
        }
        _ => {
            // Composite-named space — Separation, DeviceN, ICCBased,
            // Indexed, Pattern. The spot lanes (if any) are mirrored
            // separately by `mirror_spot_paint_into_sidecar_with_coverage`;
            // here we only need to know the process-side rule for the
            // four CMYK channels.
            //
            // Dispatch precedence:
            //
            // 1. `color_cmyk` populated — DeviceN /Process attribution
            //    (§8.6.6.5) is in play and the source CMYK was
            //    reconstructed in `SetFillColorN`. Process lanes follow
            //    Table 149 row 2 "any other process colour space"
            //    regardless of whether a spot tail is also present:
            //    the spot tail's tints land via the spot mirror, but
            //    the process tail's tints still drive the process
            //    channels via `B = c_s`. Mixed DeviceN /Process+spot
            //    must NOT preserve backdrop on the process lanes — the
            //    process tints are sourced from the same `scn` and
            //    contribute to the C/M/Y/K plates.
            //
            // 2. `spot_inks` non-empty (no process CMYK) — pure
            //    Separation or DeviceN with NO process attribution.
            //    Process lanes preserve backdrop per Table 149 row 3;
            //    the named spot lanes are handled by the spot mirror.
            //
            // 3. Otherwise — ICCBased / Pattern / Indexed / DeviceN
            //    /Process whose /Process /ColorSpace the dispatcher
            //    could not resolve (CalRGB / CalGray array forms,
            //    malformed /Components per
            //    HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES). Falls
            //    under Table 149 row 2; recover CMYK from the
            //    convert-from-RGB additive-clamp inverse so the
            //    per-process-channel `B = c_s` rule has a defensible
            //    source value. ~keep
            if let Some(cmyk) = color_cmyk {
                Some(OverprintSource {
                    class: SourceCsClass::OtherProcess,
                    cmyk,
                })
            } else if !spot_inks.is_empty() {
                Some(OverprintSource {
                    class: SourceCsClass::SeparationOrDeviceN,
                    cmyk: (0.0, 0.0, 0.0, 0.0),
                })
            } else {
                let (r, g, b) = color_rgb;
                let c = (1.0 - r).clamp(0.0, 1.0);
                let m = (1.0 - g).clamp(0.0, 1.0);
                let y = (1.0 - b).clamp(0.0, 1.0);
                Some(OverprintSource {
                    class: SourceCsClass::OtherProcess,
                    cmyk: (c, m, y, 0.0),
                })
            }
        }
    }
}

/// ISO 32000-1 §11.7.4.3 + §11.3.3 per-channel composed result.
///
/// Computes `c_r = α · B(c_b, c_s) + (1 − α) · c_b` for one process
/// channel, where `B` is the CompatibleOverprint blend function per
/// Table 149. The dispatch closely follows Table 149's rows; see the
/// docstring on [`PageRenderer::apply_overprint_after_paint`] for the
/// table layout.
///
/// - `class` — which Table 149 row applies.
/// - `channel` — the C/M/Y/K identity of this call.
/// - `c_s`, `c_b` — source and backdrop subtractive tints for this
///   channel.
/// - `opm` — graphics-state `/OPM` value (0 or 1).
/// - `alpha` — effective shape × opacity for the pixel.
fn compose_overprint_channel(
    class: SourceCsClass,
    _channel: ProcessChannel,
    c_s: f32,
    c_b: f32,
    opm: u8,
    alpha: f32,
) -> f32 {
    let b = match class {
        SourceCsClass::DeviceCmykDirect => {
            // Table 149 row 1: B = c_s for C/M/Y/K under OPM=0 or when
            // c_s ≠ 0 under OPM=1; B = c_b for c_s == 0 under OPM=1.
            // The §11.7.4.5 NOTE 1 explicitly restricts the OPM=1
            // preserve rule to the directly-specified-DeviceCMYK case. ~keep
            if opm == 1 && c_s == 0.0 { c_b } else { c_s }
        }
        SourceCsClass::OtherProcess => {
            // Table 149 row 2: B = c_s for every process colour
            // component of the group CS regardless of OPM. ~keep
            c_s
        }
        SourceCsClass::SeparationOrDeviceN => {
            // Table 149 row 3: process colour components preserve
            // backdrop. The named-spot lanes are handled by the spot
            // sidecar mirror, not by this per-process-channel pass. ~keep
            c_b
        }
    };
    let alpha = alpha.clamp(0.0, 1.0);
    alpha * b + (1.0 - alpha) * c_b
}

fn apply_pending_clip(
    pending_clip: &mut Option<(tiny_skia::Path, tiny_skia::FillRule)>,
    clip_stack: &mut Vec<Option<tiny_skia::Mask>>,
    pixmap: &Pixmap,
    base_transform: Transform,
    gs_stack: &GraphicsStateStack,
) {
    if let Some((path, fill_rule)) = pending_clip.take() {
        #[cfg(test)]
        APC_MATERIALIZED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let gs = gs_stack.current();
        let transform = combine_transforms(base_transform, &gs.ctm);

        // A clip path beyond f32 device precision cannot be rasterized.
        // Drop the clip rather than materialize an empty mask — an empty
        // mask would erase every subsequent draw on the page. ~keep
        if !device_bounds_rasterizable(&path, transform) {
            tracing::debug!("skipping clip beyond f32 device precision: {:?}", path.bounds());
            return;
        }

        let Some(slot) = clip_stack.last_mut() else {
            return;
        };
        match slot {
            // Intersect the new clip path into the current scope's mask in
            // place. tiny_skia::Mask::intersect_path allocates one submask,
            // rasterizes the path into it, then folds it into `self` via the
            // library's rounded `(a*b)/255` premultiply — replacing the
            // previous code path which additionally cloned the current mask
            // (a full page-sized memcpy) before running an equivalent scalar
            // multiply loop. The clone was redundant: every `q` already pushes
            // a cloned mask onto `clip_stack`, so the top-of-stack mask at the
            // current depth is already this scope's private copy and may be
            // mutated in place. ~keep
            Some(existing_mask) => {
                existing_mask.intersect_path(&path, fill_rule, true, transform);
            }
            None => {
                let mut new_mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height()).unwrap();
                new_mask.fill_path(&path, fill_rule, true, transform);
                *slot = Some(new_mask);
            }
        }
    }
}

/// Build a `tiny_skia::Mask` that clips an axial shading to the
/// gradient slab defined by `/Extend`. Returns `None` for the
/// `[true true]` case (no clipping needed beyond the inherited
/// `clip_mask`, which the caller handles directly).
///
/// The slab is the strip between the two lines perpendicular to the
/// axis through `p0` and `p1`. Asymmetric extends paint the strip
/// plus one half-plane past the extended end. The returned mask is
/// the intersection of the slab with the inherited `clip_mask`.
fn build_axial_extend_clip(
    pixmap: &Pixmap,
    p0: tiny_skia::Point,
    p1: tiny_skia::Point,
    extend_start: bool,
    extend_end: bool,
    inherited: Option<&tiny_skia::Mask>,
) -> Option<tiny_skia::Mask> {
    if extend_start && extend_end {
        return None;
    }

    let w = pixmap.width() as f32;
    let h = pixmap.height() as f32;

    // Axis vector (device-space) and unit-normal perpendicular. A
    // degenerate axis (p0 ≈ p1) collapses to a zero-area gradient; no
    // valid slab can be constructed, so skip the extra clip and let
    // the inherited mask carry through. ~keep
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let len = (dx * dx + dy * dy).sqrt();
    if !len.is_finite() || len < 1.0e-6 {
        return None;
    }
    let ux = dx / len;
    let uy = dy / len;
    let px = -uy;
    let py = ux;

    // Far perpendicular extent — large enough to cover the pixmap
    // diagonal from any axis position. Using 4× the diagonal stays
    // robust against off-page axis endpoints. ~keep
    let diag = (w * w + h * h).sqrt();
    let far_perp = 4.0 * diag;

    // The "axis-direction" extent must reach past the pixmap from
    // either endpoint when /Extend on that side is true. Same 4×
    // diagonal margin keeps the test robust. ~keep
    let far_axis_start = if extend_start { 4.0 * diag } else { 0.0 };
    let far_axis_end = if extend_end { 4.0 * diag } else { 0.0 };

    // Four corners of the slab polygon, walking
    // (start_minus_perp, start_plus_perp, end_plus_perp, end_minus_perp)
    // so the polygon is convex / non-self-intersecting. ~keep
    let start_x = p0.x - far_axis_start * ux;
    let start_y = p0.y - far_axis_start * uy;
    let end_x = p1.x + far_axis_end * ux;
    let end_y = p1.y + far_axis_end * uy;
    let mut pb = PathBuilder::new();
    pb.move_to(start_x - far_perp * px, start_y - far_perp * py);
    pb.line_to(start_x + far_perp * px, start_y + far_perp * py);
    pb.line_to(end_x + far_perp * px, end_y + far_perp * py);
    pb.line_to(end_x - far_perp * px, end_y - far_perp * py);
    pb.close();
    let path = pb.finish()?;

    let mut mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height())?;
    guarded_mask_fill_path(
        &mut mask,
        &path,
        tiny_skia::FillRule::Winding,
        true,
        Transform::identity(),
    );
    Some(intersect_with_inherited(mask, inherited))
}

/// Build a `tiny_skia::Mask` that clips a radial shading to the
/// gradient region defined by `/Extend`. Returns `None` for the
/// `[true true]` case.
///
/// Strategy for the common `r0 < r1` case:
/// * `Extend[1] = false` → exclude pixels outside the outer circle.
/// * `Extend[0] = false` → exclude pixels inside the inner circle
///   (forms an annulus when combined with the outer exclusion).
fn build_radial_extend_clip(
    pixmap: &Pixmap,
    start: (tiny_skia::Point, f32),
    end: (tiny_skia::Point, f32),
    extend_start: bool,
    extend_end: bool,
    inherited: Option<&tiny_skia::Mask>,
) -> Option<tiny_skia::Mask> {
    if extend_start && extend_end {
        return None;
    }

    let (c0, r0) = start;
    let (c1, r1) = end;

    // For non-concentric circles the spec's family-of-circles cone
    // shape is more complex than a simple annulus; the best-effort
    // approximation here is the union of the disks at each end. This
    // captures the common "spotlight" pattern (small inner point,
    // large outer circle) without painting outside the outer circle.
    //
    // When `Extend[0] = false` we also exclude the inner disk
    // (subtract it via an even-odd fill rule). ~keep
    let mut mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height())?;

    let outer_path = {
        let mut pb = PathBuilder::new();
        if !extend_end {
            // Outer boundary is the outer circle plus the inner
            // circle padded outward (for the inner-padded extend-true
            // case we just use the outer circle). ~keep
            pb.push_circle(c1.x, c1.y, r1.max(1.0e-3));
        } else {
            // No outer-side clip: the outer boundary is the full
            // pixmap rectangle. ~keep
            let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, pixmap.width() as f32, pixmap.height() as f32)?;
            pb.push_rect(rect);
        }
        pb.finish()?
    };
    guarded_mask_fill_path(
        &mut mask,
        &outer_path,
        tiny_skia::FillRule::Winding,
        true,
        Transform::identity(),
    );

    if !extend_start && r0 > 1.0e-3 {
        // Subtract the inner disk by painting black into the mask.
        // tiny-skia's `Mask` is a single-channel u8 buffer; "subtract"
        // by filling the inner path into a fresh inner-mask and then
        // multiplying mask by (1 - inner_mask). ~keep
        let mut inner_mask = tiny_skia::Mask::new(pixmap.width(), pixmap.height())?;
        let mut pb = PathBuilder::new();
        pb.push_circle(c0.x, c0.y, r0);
        if let Some(inner_path) = pb.finish() {
            guarded_mask_fill_path(
                &mut inner_mask,
                &inner_path,
                tiny_skia::FillRule::Winding,
                true,
                Transform::identity(),
            );
            let outer_data = mask.data_mut();
            let inner_data = inner_mask.data();
            for i in 0..outer_data.len() {
                let outside_inner = 255u32 - inner_data[i] as u32;
                outer_data[i] = ((outer_data[i] as u32 * outside_inner) / 255) as u8;
            }
        }
    }

    Some(intersect_with_inherited(mask, inherited))
}

/// Multiply the per-pixel coverage of `mask` by the inherited
/// `clip_mask` so the gradient is bounded by both at once.
fn intersect_with_inherited(mut mask: tiny_skia::Mask, inherited: Option<&tiny_skia::Mask>) -> tiny_skia::Mask {
    if let Some(existing) = inherited {
        let data = mask.data_mut();
        let other = existing.data();
        // Both masks are sized to the pixmap, so the buffers match. ~keep
        let n = data.len().min(other.len());
        for i in 0..n {
            data[i] = ((data[i] as u32 * other[i] as u32) / 255) as u8;
        }
    }
    mask
}

fn trace_ccitt_mask_decode_failure(error: &Error) {
    crate::error::trace_recovery("decode_ccitt_image_mask", error);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::Object;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use tracing::Level;
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::SubscriberExt as _;

    #[derive(Clone, Debug)]
    struct CapturedEvent {
        level: Level,
        target: String,
        fields: BTreeMap<String, String>,
    }

    #[derive(Clone, Default)]
    struct EventCapture(Arc<Mutex<Vec<CapturedEvent>>>);

    impl<S: tracing::Subscriber> Layer<S> for EventCapture {
        fn on_event(&self, event: &tracing::Event<'_>, _context: tracing_subscriber::layer::Context<'_, S>) {
            let mut fields = FieldCapture::default();
            event.record(&mut fields);
            self.0.lock().unwrap().push(CapturedEvent {
                level: *event.metadata().level(),
                target: event.metadata().target().to_string(),
                fields: fields.0,
            });
        }
    }

    #[derive(Default)]
    struct FieldCapture(BTreeMap<String, String>);

    impl tracing::field::Visit for FieldCapture {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0.insert(field.name().to_string(), format!("{value:?}"));
        }

        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            self.0.insert(field.name().to_string(), value.to_string());
        }

        fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
            self.0.insert(field.name().to_string(), value.to_string());
        }
    }

    fn capture_events(operation: impl FnOnce()) -> Vec<CapturedEvent> {
        let capture = EventCapture::default();
        let subscriber = tracing_subscriber::registry().with(capture.clone());
        tracing::subscriber::with_default(subscriber, operation);
        capture.0.lock().unwrap().clone()
    }

    fn pdf_doc_with_extra_object(extra: Option<&[u8]>) -> PdfDocument {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        let mut append = |number: usize, body: &[u8]| {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
            pdf.extend_from_slice(body);
            pdf.extend_from_slice(b"\nendobj\n");
        };
        append(1, b"<< /Type /Catalog /Pages 2 0 R >>");
        append(2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
        append(3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>");
        if let Some(body) = extra {
            append(4, body);
        }
        let xref = pdf.len();
        let object_count = offsets.len() + 1;
        pdf.extend_from_slice(format!("xref\n0 {object_count}\n0000000000 65535 f \n").as_bytes());
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!("trailer\n<< /Size {object_count} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        PdfDocument::from_bytes(pdf).expect("open minimal PDF")
    }

    fn minimal_pdf_doc() -> PdfDocument {
        pdf_doc_with_extra_object(None)
    }

    #[test]
    fn renderer_recovery_warnings_exclude_untrusted_names_and_errors() {
        const SECRET_NAME: &str = "CONFIDENTIAL_SHADING_d1a2";
        const SECRET_ERROR: &str = "CONFIDENTIAL_CCITT_8b4f";
        let doc = minimal_pdf_doc();
        let renderer = PageRenderer::new(RenderOptions::default());
        let mut pixmap = Pixmap::new(1, 1).unwrap();
        let graphics_state = GraphicsState::new();
        let missing_resources = Object::Dictionary(HashMap::new());
        let unsupported_resources = Object::Dictionary(HashMap::from([(
            "Shading".to_string(),
            Object::Dictionary(HashMap::from([(
                SECRET_NAME.to_string(),
                Object::Dictionary(HashMap::from([("ShadingType".to_string(), Object::Integer(99))])),
            )])),
        )]));

        let events = capture_events(|| {
            renderer
                .render_shading(
                    &mut pixmap,
                    SECRET_NAME,
                    Transform::identity(),
                    &graphics_state,
                    &missing_resources,
                    &doc,
                    None,
                )
                .unwrap();
            renderer
                .render_shading(
                    &mut pixmap,
                    SECRET_NAME,
                    Transform::identity(),
                    &graphics_state,
                    &unsupported_resources,
                    &doc,
                    None,
                )
                .unwrap();
            trace_ccitt_mask_decode_failure(&Error::Decode(SECRET_ERROR.to_string()));
        });

        let warnings: Vec<_> = events
            .iter()
            .filter(|event| event.level == Level::WARN && event.target == crate::LOG_TARGET_ROOT)
            .collect();
        assert_eq!(
            warnings.len(),
            3,
            "expected exact renderer recovery events: {events:#?}"
        );
        assert_eq!(
            warnings[0].fields.get("operation").map(String::as_str),
            Some("render_shading")
        );
        assert_eq!(
            warnings[0].fields.get("error_code").map(String::as_str),
            Some("missing_resource")
        );
        assert_eq!(
            warnings[1].fields.get("operation").map(String::as_str),
            Some("render_shading")
        );
        assert_eq!(
            warnings[1].fields.get("error_code").map(String::as_str),
            Some("unsupported_shading_type")
        );
        assert_eq!(warnings[1].fields.get("shading_type").map(String::as_str), Some("99"));
        assert_eq!(
            warnings[2].fields.get("operation").map(String::as_str),
            Some("decode_ccitt_image_mask")
        );
        assert_eq!(
            warnings[2].fields.get("error_code").map(String::as_str),
            Some("decode_error")
        );
        let rendered = format!("{events:?}");
        assert!(!rendered.contains(SECRET_NAME));
        assert!(!rendered.contains(SECRET_ERROR));
    }

    fn image_mask_dict(width: i64, height: i64) -> HashMap<String, Object> {
        HashMap::from([
            ("Width".to_string(), Object::Integer(width)),
            ("Height".to_string(), Object::Integer(height)),
        ])
    }

    fn ccitt_mask_dict(width: i64, height: i64, columns: i64, rows: i64) -> HashMap<String, Object> {
        let mut dict = image_mask_dict(width, height);
        dict.insert("Filter".to_string(), Object::Name("CCITTFaxDecode".to_string()));
        dict.insert(
            "DecodeParms".to_string(),
            Object::Dictionary(HashMap::from([
                ("K".to_string(), Object::Integer(-1)),
                ("Columns".to_string(), Object::Integer(columns)),
                ("Rows".to_string(), Object::Integer(rows)),
            ])),
        );
        dict
    }

    /// A JBIG2 `/ImageMask` must be routed to the JBIG2 decoder rather than treated as
    /// packed 1-bit rows. Scanners emitting Mixed Raster Content put ALL page text in
    /// these masks, and the compressed bitstream is an order of magnitude smaller than
    /// the bitmap it expands to -- so the un-decoded path always failed the stencil
    /// length check with "ImageMask stream too short", the error was swallowed as a
    /// warning, and the page rendered as its blank background. A 16-page scanned
    /// ordinance produced 123 such warnings and OCR'd to 174 characters instead of
    /// ~12,000.
    #[test]
    fn image_mask_is_jbig2_detects_the_codec_as_a_bare_name_and_as_the_final_array_filter() {
        let bare = HashMap::from([("Filter".to_string(), Object::Name("JBIG2Decode".to_string()))]);
        assert!(PageRenderer::image_mask_is_jbig2(&bare));

        let array = HashMap::from([(
            "Filter".to_string(),
            Object::Array(vec![
                Object::Name("FlateDecode".to_string()),
                Object::Name("JBIG2Decode".to_string()),
            ]),
        )]);
        assert!(PageRenderer::image_mask_is_jbig2(&array));
    }

    /// The codec must be LAST: a filter applied after it would operate on decoded pixels,
    /// not the bitstream, so decoding here would be wrong. Mirrors the CCITT rule.
    #[test]
    fn image_mask_is_jbig2_rejects_non_final_and_unrelated_filters() {
        let not_final = HashMap::from([(
            "Filter".to_string(),
            Object::Array(vec![
                Object::Name("JBIG2Decode".to_string()),
                Object::Name("FlateDecode".to_string()),
            ]),
        )]);
        assert!(!PageRenderer::image_mask_is_jbig2(&not_final));

        let ccitt = HashMap::from([("Filter".to_string(), Object::Name("CCITTFaxDecode".to_string()))]);
        assert!(!PageRenderer::image_mask_is_jbig2(&ccitt));

        assert!(!PageRenderer::image_mask_is_jbig2(&HashMap::new()));
    }

    #[test]
    fn image_mask_layout_rejects_non_positive_dimensions_before_allocation() {
        let doc = minimal_pdf_doc();
        let negative = PageRenderer::image_mask_layout(&image_mask_dict(-1, 1), &doc);
        assert!(matches!(negative, Err(Error::Image(message)) if message.contains("positive")));

        let zero = PageRenderer::image_mask_layout(&image_mask_dict(1, 0), &doc);
        assert!(matches!(zero, Err(Error::Image(message)) if message.contains("positive")));
    }

    #[test]
    fn image_mask_layout_accepts_large_valid_dimensions_without_allocating() {
        let doc = minimal_pdf_doc();
        let layout = PageRenderer::image_mask_layout(&image_mask_dict(5_000, 4_000), &doc).expect("valid layout");
        assert_eq!(layout, (5_000, 4_000, 625, 2_500_000, 80_000_000));
    }

    /// GH#1444 residual: `/Height` (or `/Width`) written as an indirect reference
    /// (ISO 32000-1 §7.3.10 permits this for any dict entry) must resolve the same
    /// way the non-mask image path resolves it, instead of reading as "missing".
    #[test]
    fn image_mask_layout_resolves_an_indirect_height() {
        let doc = pdf_doc_with_extra_object(Some(b"600"));
        let mut dict = image_mask_dict(400, 1);
        dict.insert("Height".to_string(), Object::Reference(ObjectRef::new(4, 0)));

        let layout = PageRenderer::image_mask_layout(&dict, &doc).expect("indirect /Height resolves");
        assert_eq!(layout, (400, 600, 50, 30_000, 960_000));
    }

    #[test]
    fn ccitt_image_mask_rejects_width_beyond_decoder_limit() {
        let dict = ccitt_mask_dict(65_536, 1, 65_536, 1);
        let doc = minimal_pdf_doc();
        let result = PageRenderer::image_mask_ccitt_params(&dict, 65_536, 1, &doc);
        assert!(matches!(result, Err(Error::Image(message)) if message.contains("decoder limit")));
    }

    #[test]
    fn ccitt_image_mask_rejects_dimension_mismatches() {
        let columns = ccitt_mask_dict(8, 1, 7, 1);
        let doc = minimal_pdf_doc();
        let result = PageRenderer::image_mask_ccitt_params(&columns, 8, 1, &doc);
        assert!(matches!(result, Err(Error::Image(message)) if message.contains("/Columns 7")));

        let rows = ccitt_mask_dict(8, 1, 8, 2);
        let result = PageRenderer::image_mask_ccitt_params(&rows, 8, 1, &doc);
        assert!(matches!(result, Err(Error::Image(message)) if message.contains("/Rows 2")));
    }

    #[test]
    fn ccitt_image_mask_rejects_invalid_k_and_accepts_all_negative_k_values() {
        let doc = minimal_pdf_doc();
        let mut invalid = ccitt_mask_dict(8, 1, 8, 1);
        let Some(Object::Dictionary(invalid_params)) = invalid.get_mut("DecodeParms") else {
            panic!("decode parameters must be a dictionary");
        };
        invalid_params.insert("K".to_string(), Object::Name("invalid".to_string()));
        let result = PageRenderer::image_mask_ccitt_params(&invalid, 8, 1, &doc);
        assert!(matches!(result, Err(Error::Image(message)) if message.contains("/K must be an integer")));

        let mut negative = ccitt_mask_dict(8, 1, 8, 1);
        let Some(Object::Dictionary(negative_params)) = negative.get_mut("DecodeParms") else {
            panic!("decode parameters must be a dictionary");
        };
        negative_params.insert("K".to_string(), Object::Integer(-2));
        let params = PageRenderer::image_mask_ccitt_params(&negative, 8, 1, &doc)
            .expect("valid parameters")
            .expect("CCITT parameters");
        assert!(params.is_group_4());
        assert_eq!(params.k, -2);
    }

    #[test]
    fn ccitt_image_mask_rejects_dangling_and_wrong_type_decode_params_references() {
        let mut dangling = ccitt_mask_dict(8, 1, 8, 1);
        dangling.insert("DecodeParms".to_string(), Object::Reference(ObjectRef::new(99, 0)));
        let error = PageRenderer::image_mask_ccitt_params(&dangling, 8, 1, &minimal_pdf_doc())
            .expect_err("dangling reference must fail");
        assert!(
            matches!(&error, Error::Image(message) if message.contains("Unable to resolve")),
            "unexpected dangling-reference error: {error:?}"
        );

        let mut dangling_array = ccitt_mask_dict(8, 1, 8, 1);
        dangling_array.insert(
            "Filter".to_string(),
            Object::Array(vec![
                Object::Name("ASCIIHexDecode".to_string()),
                Object::Name("CCITTFaxDecode".to_string()),
            ]),
        );
        dangling_array.insert(
            "DecodeParms".to_string(),
            Object::Array(vec![Object::Null, Object::Reference(ObjectRef::new(99, 0))]),
        );
        let error = PageRenderer::image_mask_ccitt_params(&dangling_array, 8, 1, &minimal_pdf_doc())
            .expect_err("dangling array entry must fail");
        assert!(
            matches!(&error, Error::Image(message) if message.contains("array entry")),
            "unexpected dangling array-entry error: {error:?}"
        );

        let mut wrong_type = ccitt_mask_dict(8, 1, 8, 1);
        wrong_type.insert("DecodeParms".to_string(), Object::Reference(ObjectRef::new(4, 0)));
        let doc = pdf_doc_with_extra_object(Some(b"42"));
        let result = PageRenderer::image_mask_ccitt_params(&wrong_type, 8, 1, &doc);
        assert!(matches!(result, Err(Error::Image(message)) if message.contains("must be a dictionary")));
    }

    #[test]
    fn ccitt_image_mask_rejects_non_final_filter() {
        let mut dict = ccitt_mask_dict(8, 1, 8, 1);
        dict.insert(
            "Filter".to_string(),
            Object::Array(vec![
                Object::Name("CCITTFaxDecode".to_string()),
                Object::Name("ASCIIHexDecode".to_string()),
            ]),
        );
        let result = PageRenderer::image_mask_ccitt_params(&dict, 8, 1, &minimal_pdf_doc());
        assert!(matches!(result, Err(Error::Image(message)) if message.contains("final")));
    }

    #[test]
    fn test_color_key_mask_range_logic() {
        let ranges = [(10u32, 20u32), (100u32, 150u32)];

        assert!(color_key_pixel_masked(&[15, 120], &ranges));
        // Boundaries are inclusive. ~keep
        assert!(color_key_pixel_masked(&[10, 100], &ranges));
        assert!(color_key_pixel_masked(&[20, 150], &ranges));
        assert!(!color_key_pixel_masked(&[9, 120], &ranges));
        assert!(!color_key_pixel_masked(&[15, 151], &ranges));
        // Length mismatch never masks. ~keep
        assert!(!color_key_pixel_masked(&[15], &ranges));
        assert!(!color_key_pixel_masked(&[], &ranges));

        // RGB color-key: only pixels equal to the exact keyed colour drop out. ~keep
        let rgb = [(255u32, 255u32), (0, 0), (0, 0)]; // pure red is transparent ~keep
        assert!(color_key_pixel_masked(&[255, 0, 0], &rgb));
        assert!(!color_key_pixel_masked(&[254, 0, 0], &rgb));
        assert!(!color_key_pixel_masked(&[255, 1, 0], &rgb));
    }

    #[test]
    fn test_parse_color_key_mask() {
        let arr = vec![
            Object::Integer(0),
            Object::Integer(10),
            Object::Integer(20),
            Object::Integer(30),
            Object::Integer(40),
            Object::Integer(50),
        ];
        assert_eq!(parse_color_key_mask(&arr, 3), Some(vec![(0, 10), (20, 30), (40, 50)]));

        assert_eq!(parse_color_key_mask(&arr, 2), None);
        assert_eq!(parse_color_key_mask(&arr, 0), None);
        let bad = vec![Object::Integer(30), Object::Integer(10)];
        assert_eq!(parse_color_key_mask(&bad, 1), None);
        let neg = vec![Object::Integer(-1), Object::Integer(10)];
        assert_eq!(parse_color_key_mask(&neg, 1), None);
        let non_int = vec![Object::Real(1.5), Object::Integer(10)];
        assert_eq!(parse_color_key_mask(&non_int, 1), None);
    }

    #[test]
    fn tiling_pattern_axis_tile_range_covers_region() {
        // A 10-px cell anchored at device x=0, stepping every 10 px, must
        // cover the region [5, 45] with tiles i = 0..=4. ~keep
        let (lo, hi) = axis_tile_range(5.0, 45.0, 0.0, 10.0, 10.0);
        assert!(lo <= 0, "lo {lo} should include tile 0");
        assert!(hi >= 4, "hi {hi} should include tile 4");
        for i in 0..=4 {
            assert!(lo <= i && i <= hi, "tile {i} must be in [{lo},{hi}]");
        }
        assert!(hi < 100 && lo > -100);
    }

    #[test]
    fn tiling_pattern_axis_tile_range_negative_step() {
        // A flipped pattern axis (negative device step) must still yield a
        // valid, region-covering range. ~keep
        let (lo, hi) = axis_tile_range(0.0, 30.0, 0.0, 10.0, -10.0);
        assert!(lo <= hi);
        assert!(lo <= -3 && hi >= 0, "range [{lo},{hi}] must cover i in [-3,0]");
    }

    #[test]
    fn tiling_pattern_axis_tile_range_offset_anchor() {
        // Non-zero cell anchor: cell width 20, anchored at x=100, step 20,
        // region [130, 175] → tiles i=1,2,3 all included. ~keep
        let (lo, hi) = axis_tile_range(130.0, 175.0, 100.0, 20.0, 20.0);
        for i in 1..=3 {
            assert!(lo <= i && i <= hi, "tile {i} must be in [{lo},{hi}]");
        }
    }

    #[test]
    fn test_cmyk_to_rgb_white() {
        let (r, g, b) = cmyk_to_rgb(0.0, 0.0, 0.0, 0.0);
        assert!((r - 1.0).abs() < 0.001);
        assert!((g - 1.0).abs() < 0.001);
        assert!((b - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cmyk_to_rgb_black() {
        // Process inks (not additive): 100% K is the K ink #231F20, NOT #000000. ~keep
        let (r, g, b) = cmyk_to_rgb(0.0, 0.0, 0.0, 1.0);
        let q = |v: f32| (v * 255.0).round() as u8;
        assert_eq!([q(r), q(g), q(b)], [0x23, 0x1F, 0x20]);
    }

    #[test]
    fn test_cmyk_to_rgb_pure_cyan() {
        // Process inks (not additive): 100% cyan is #00ADEF, NOT #00FFFF. ~keep
        let (r, g, b) = cmyk_to_rgb(1.0, 0.0, 0.0, 0.0);
        let q = |v: f32| (v * 255.0).round() as u8;
        assert_eq!([q(r), q(g), q(b)], [0x00, 0xAD, 0xEF]);
    }

    #[test]
    fn test_negative_rect_normalization() {
        // Negative height: re 100 200 50 -30 → should normalize to (100, 170, 50, 30) ~keep
        let x: f32 = 100.0;
        let y: f32 = 200.0;
        let w: f32 = 50.0;
        let h: f32 = -30.0;
        let (nx, nw) = if w < 0.0 { (x + w, -w) } else { (x, w) };
        let (ny, nh) = if h < 0.0 { (y + h, -h) } else { (y, h) };
        assert!((nx - 100.0).abs() < 0.001);
        assert!((ny - 170.0).abs() < 0.001);
        assert!((nw - 50.0).abs() < 0.001);
        assert!((nh - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_negative_rect_both_negative() {
        let x: f32 = 100.0;
        let y: f32 = 200.0;
        let w: f32 = -50.0;
        let h: f32 = -30.0;
        let (nx, nw) = if w < 0.0 { (x + w, -w) } else { (x, w) };
        let (ny, nh) = if h < 0.0 { (y + h, -h) } else { (y, h) };
        assert!((nx - 50.0).abs() < 0.001);
        assert!((ny - 170.0).abs() < 0.001);
        assert!((nw - 50.0).abs() < 0.001);
        assert!((nh - 30.0).abs() < 0.001);
    }

    /// The crux of the ET application: the accumulated glyph silhouette is
    /// converted to an alpha `Mask` and AND-ed into the current clip, so
    /// subsequent content survives only where it falls inside BOTH the text
    /// shape and the pre-existing clip. This exercises the exact
    /// `Mask::from_pixmap(Alpha)` + `intersect_with_inherited` path the `ET`
    /// arm runs, minus glyph shaping (which the coverage rasteriser handles).
    #[test]
    fn text_clip_intersects_glyph_silhouette_within_existing_clip() {
        use tiny_skia::{Color, FillRule, Mask, MaskType, Paint, PathBuilder, Pixmap, Rect, Transform};

        let w = 20u32;
        let h = 20u32;

        // Simulated accumulated text-clip silhouette: an opaque-black square
        // covering the page's centre (x,y in 5..15). This is what
        // `accumulate_text_clip_*` leaves in the scratch pixmap after a
        // mode-≥4 show. ~keep
        let mut scratch = Pixmap::new(w, h).unwrap();
        let mut paint = Paint::default();
        paint.set_color(Color::BLACK);
        paint.anti_alias = false;
        let sil = Rect::from_xywh(5.0, 5.0, 10.0, 10.0).unwrap();
        scratch.fill_rect(sil, &paint, Transform::identity(), None);

        let has_coverage = scratch.data().as_chunks::<4>().0.iter().any(|px| px[3] != 0);
        assert!(has_coverage, "painted silhouette must report coverage");

        let mut existing = Mask::new(w, h).unwrap();
        let mut pb = PathBuilder::new();
        pb.push_rect(Rect::from_xywh(0.0, 0.0, 20.0, 10.0).unwrap());
        existing.fill_path(&pb.finish().unwrap(), FillRule::Winding, false, Transform::identity());

        let text_mask = Mask::from_pixmap(scratch.as_ref(), MaskType::Alpha);
        let result = super::intersect_with_inherited(text_mask, Some(&existing));

        let at = |x: u32, y: u32| result.data()[(y * w + x) as usize];
        assert_eq!(at(7, 7), 255, "kept where text ∩ clip");
        assert_eq!(at(7, 12), 0, "clip must not be widened past its bound");
        assert_eq!(at(2, 2), 0, "content outside the glyph shape is clipped away");
        assert_eq!(at(18, 18), 0, "corner outside the glyph stays background");
    }

    /// An accumulator that saw only whitespace / outline-less glyphs is fully
    /// transparent; the `ET` arm must treat that as degenerate and leave the
    /// clip untouched rather than collapsing it to an empty region.
    #[test]
    fn text_clip_empty_accumulator_is_degenerate() {
        use tiny_skia::Pixmap;
        let scratch = Pixmap::new(16, 16).unwrap(); // fresh -> fully transparent ~keep
        let has_coverage = scratch.data().as_chunks::<4>().0.iter().any(|px| px[3] != 0);
        assert!(!has_coverage, "empty accumulator must be treated as no clip change");
    }

    /// The coverage graphics state used to rasterise the clip silhouette must
    /// force fill mode 0 (so clip-only mode-7 / invisible mode-3 glyphs still
    /// rasterise their outline — the text rasteriser paints those modes with
    /// transparent paint, which would otherwise yield an empty silhouette and
    /// silently drop the clip) while forcing opaque paint (so alpha == coverage).
    #[test]
    fn coverage_gs_forces_fill_mode_for_clip_silhouette() {
        use crate::content::graphics_state::GraphicsState;
        for visible_mode in [3u8, 4, 5, 6, 7] {
            let mut gs = GraphicsState::default();
            gs.render_mode = visible_mode;
            gs.fill_alpha = 0.3;
            let cov = super::PageRenderer::coverage_only_gs(&gs);
            assert_eq!(
                cov.render_mode, 0,
                "coverage must fill regardless of visible mode {visible_mode}"
            );
            assert_eq!(cov.fill_alpha, 1.0, "coverage must be opaque");
            assert!(cov.smask.is_none(), "coverage must strip SMask");
        }
    }

    // ---------------------------------------------------------------------
    // Helper-level pins for the text-resolution splice.
    //
    // The text-side integration tests in
    // `tests/test_render_resolution_pipeline_qa_wave*.rs` exercise the
    // full renderer end-to-end, but two properties are not directly
    // observable from there today:
    //
    //   * Stroke-side resolution. The text rasteriser does not currently
    //     paint stroked glyphs, so the spliced stroke colour never reaches
    //     the pixmap. We probe it here by inspecting the
    //     `GraphicsState` the helper returns.
    //
    //   * Helper-returns-`None` on the no-op-splice path. The
    //     integration test asserts the rendered output is unchanged when
    //     the resolved RGBA equals the GS field already set, which holds
    //     whether the helper returns `None` or `Some(clone)`. We probe
    //     the return value directly here.
    //
    // Both probes call `pipeline_resolve_text_colors` directly. The
    // wider integration coverage stays untouched.
    // --------------------------------------------------------------------- ~keep

    use crate::content::graphics_state::GraphicsState;
    use crate::rendering::resolution::test_support::fixture_doc;
    use smallvec::smallvec;
    use std::collections::HashMap;

    fn type4_magenta_separation_space() -> Object {
        // `{ 0.0 exch 0.0 0.0 }` — at full tint this yields CMYK(0,1,0,0),
        // which the colour resolver converts to RGB ≈ (1, 0, 1) (magenta).
        // Same shape as the colour-stage and pipeline regression tests. ~keep
        let program = b"{ 0.0 exch 0.0 0.0 }";
        let mut func_dict: HashMap<String, Object> = HashMap::new();
        func_dict.insert("FunctionType".into(), Object::Integer(4));
        func_dict.insert(
            "Domain".into(),
            Object::Array(vec![Object::Integer(0), Object::Integer(1)]),
        );
        func_dict.insert(
            "Range".into(),
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(1),
            ]),
        );
        let func_obj = Object::Stream {
            dict: func_dict,
            data: program.to_vec().into(),
        };
        Object::Array(vec![
            Object::Name("Separation".into()),
            Object::Name("MagentaSpot".into()),
            Object::Name("DeviceCMYK".into()),
            func_obj,
        ])
    }

    #[test]
    fn pipeline_resolve_text_colors_strokes_magenta_under_tr1() {
        // T-1 stroke-side resolution probe.
        //
        // Construct a `PageRenderer` with a Separation/DeviceCMYK/Type-4
        // colour space attached to the stroke side. Under Tr=1 the
        // helper must resolve the stroke side through the pipeline and
        // yield the Type-4-evaluated RGB on the `stroke` channel of the
        // returned `ResolvedColors`. The legacy `1.0 - tint = 0`
        // fallback would put black on the stroke channel; the pipeline
        // must produce magenta (R high, G low, B high). ~keep
        let mut renderer = PageRenderer::new(RenderOptions::default());
        renderer
            .color_spaces
            .insert("SpotMagenta".to_string(), type4_magenta_separation_space());

        let mut gs = GraphicsState::new();
        gs.render_mode = 1; // Stroke-only text. ~keep
        gs.stroke_color_space = "SpotMagenta".to_string();
        gs.stroke_color_components = smallvec![1.0]; // full tint ~keep
        // Leave fill side at the GraphicsState default (DeviceGray, no
        // components) so a stray fill-side resolve attempt would fail
        // out — keeping the assertion focused on the stroke channel. ~keep

        let doc = fixture_doc();
        let colors = renderer
            .pipeline_resolve_text_colors(&doc, &gs)
            .expect("Tr=1 stroke side must produce ResolvedColors");

        let (r, g, b, _a) = colors.stroke.expect("Tr=1 must populate the stroke side");
        // Process-ink magenta corner #EC008C = (0.9255, 0, 0.5490); the
        // legacy 1-tint=0 fallback would put black on the stroke channel. ~keep
        assert!(
            (r - 0.9255).abs() < 0.02 && g < 0.02 && (b - 0.5490).abs() < 0.02,
            "stroke side must be process-ink magenta (Type-4 evaluated), \
             not the legacy 1-tint=0 black; got ({r}, {g}, {b})"
        );
        // The fill channel must not have been resolved — the helper
        // selects only the side(s) the Tr mode names. ~keep
        assert!(colors.fill.is_none(), "Tr=1 must not touch the fill side");
    }

    #[test]
    fn pipeline_resolve_paint_gs_short_circuits_when_resolved_matches_gs() {
        // D-3 short-circuit. With a DeviceRGB fill already set on `gs`,
        // the pipeline resolves to the same (r, g, b, alpha) as
        // `gs.fill_color_rgb` / `gs.fill_alpha`. The helper must skip
        // the GraphicsState clone in that case and return `None` — the
        // caller borrows `gs` directly. This keeps the Device-family
        // path (the common case) allocation-free. ~keep
        let renderer = PageRenderer::new(RenderOptions::default());

        let mut gs = GraphicsState::new();
        gs.fill_color_space = "DeviceRGB".to_string();
        gs.fill_color_components = smallvec![0.25, 0.5, 0.75];
        // The dispatcher's inline path keeps `gs.fill_color_rgb` in
        // sync with the components; mirror that here so the
        // short-circuit comparison sees a true no-op. ~keep
        gs.fill_color_rgb = (0.25, 0.5, 0.75);
        gs.fill_alpha = 1.0;

        let doc = fixture_doc();
        assert!(
            renderer
                .pipeline_resolve_paint_gs(&doc, &gs, PipelinePaintKind::PathFill)
                .is_none(),
            "Device-family fill that resolves to the same RGBA as gs must short-circuit"
        );
    }

    #[test]
    fn pipeline_resolve_paint_gs_image_mask_short_circuits_same_as_path_fill() {
        // Wave 3 pin. `PipelinePaintKind::ImageMask` must follow the
        // same fill-only resolve-and-short-circuit rules as
        // `PipelinePaintKind::PathFill`: a Device-family fill whose
        // resolved RGBA already matches `gs.fill_color_rgb` returns
        // `None` (no clone), and the stroke side is never touched. ~keep
        let renderer = PageRenderer::new(RenderOptions::default());

        let mut gs = GraphicsState::new();
        gs.fill_color_space = "DeviceRGB".to_string();
        gs.fill_color_components = smallvec![0.25, 0.5, 0.75];
        gs.fill_color_rgb = (0.25, 0.5, 0.75);
        gs.fill_alpha = 1.0;

        let doc = fixture_doc();
        assert!(
            renderer
                .pipeline_resolve_paint_gs(&doc, &gs, PipelinePaintKind::ImageMask)
                .is_none(),
            "ImageMask Device-family fill matching gs must short-circuit"
        );
    }

    #[test]
    fn pipeline_resolve_paint_gs_image_mask_resolves_type4_separation_fill() {
        // ImageMask capability pin. With a Separation/DeviceCMYK Type 4
        // colour space on the fill side, the `ImageMask` variant must
        // produce a spliced `GraphicsState` whose `fill_color_rgb` is
        // the Type 4 program output (magenta), NOT the legacy
        // `1 - tint = 0` black. Same helper, same colour-stage path,
        // just driven by the ImageMask variant. ~keep
        let mut renderer = PageRenderer::new(RenderOptions::default());
        renderer
            .color_spaces
            .insert("SpotMagenta".to_string(), type4_magenta_separation_space());

        let mut gs = GraphicsState::new();
        gs.fill_color_space = "SpotMagenta".to_string();
        gs.fill_color_components = smallvec![1.0]; // full tint ~keep
        gs.fill_color_rgb = (0.0, 0.0, 0.0); // legacy 1-tint=0 black ~keep
        gs.fill_alpha = 1.0;

        let doc = fixture_doc();
        let spliced = renderer
            .pipeline_resolve_paint_gs(&doc, &gs, PipelinePaintKind::ImageMask)
            .expect("Type 4 Separation fill must splice through ImageMask variant");

        let (r, g, b) = spliced.fill_color_rgb;
        // Process-ink magenta corner #EC008C = (0.9255, 0, 0.5490). ~keep
        assert!(
            (r - 0.9255).abs() < 0.02 && g < 0.02 && (b - 0.5490).abs() < 0.02,
            "ImageMask fill must be process-ink magenta (Type 4 evaluated), not legacy black; got ({r}, {g}, {b})"
        );
        // Stroke side must remain untouched — the variant is fill-only. ~keep
        assert_eq!(
            spliced.stroke_color_rgb, gs.stroke_color_rgb,
            "ImageMask variant must not touch the stroke channel"
        );
    }

    #[test]
    fn pipeline_resolve_text_colors_short_circuits_when_resolved_matches_gs() {
        // Same short-circuit on the text-side helper, Tr=0 fill-only:
        // a DeviceRGB whose resolved value equals the current gs fields
        // must produce no override (no per-element paint.set_color in
        // the rasteriser). ~keep
        let renderer = PageRenderer::new(RenderOptions::default());

        let mut gs = GraphicsState::new();
        gs.render_mode = 0;
        gs.fill_color_space = "DeviceRGB".to_string();
        gs.fill_color_components = smallvec![0.1, 0.2, 0.3];
        gs.fill_color_rgb = (0.1, 0.2, 0.3);
        gs.fill_alpha = 1.0;

        let doc = fixture_doc();
        assert!(
            renderer.pipeline_resolve_text_colors(&doc, &gs).is_none(),
            "Device-family text fill that resolves to the same RGBA as gs must short-circuit"
        );
    }

    #[test]
    fn rgba_matches_within_epsilon() {
        // The tolerance must absorb single-ulp drift from intermediate
        // computations but reject any real colour change. ~keep
        assert!(rgba_matches((0.25, 0.5, 0.75, 1.0), (0.25, 0.5, 0.75), 1.0));
        let drift = RGBA_MATCH_EPSILON * 0.5;
        assert!(rgba_matches(
            (0.25 + drift, 0.5 + drift, 0.75 + drift, 1.0 + drift),
            (0.25, 0.5, 0.75),
            1.0
        ));
        // Anything beyond the epsilon is a real change and must not
        // short-circuit — single-channel mismatch is enough. ~keep
        assert!(!rgba_matches((0.26, 0.5, 0.75, 1.0), (0.25, 0.5, 0.75), 1.0));
        assert!(!rgba_matches((0.25, 0.5, 0.75, 0.5), (0.25, 0.5, 0.75), 1.0));
    }

    // ---------------------------------------------------------------------
    // `pipeline_resolve_components` helper unit pins.
    //
    // The shading integration tests in
    // `tests/test_render_resolution_pipeline_qa_wave*.rs` probe the
    // helper through the renderer. These unit pins probe the helper's
    // own contract directly, so a regression in routing (e.g.
    // Device-family short-circuit vs Spaced dispatch) shows up at the
    // helper level before any pixel-comparison machinery is involved.
    // --------------------------------------------------------------------- ~keep

    #[test]
    fn pipeline_resolve_components_resolves_type4_separation_to_correct_rgba() {
        // Capability pin. The Separation/DeviceCMYK/Type-4 space at
        // full tint must come out as magenta after the pipeline runs
        // the PostScript program — the same regression case the
        // colour-stage and full-pipeline unit tests pin at lower
        // levels, here verified via the wave-4 shading-endpoint
        // overload. ~keep
        let renderer = PageRenderer::new(RenderOptions::default());

        let space = type4_magenta_separation_space();
        let doc = fixture_doc();
        let color_spaces: HashMap<String, Object> = HashMap::new();

        let rgba = renderer
            .pipeline_resolve_components(&doc, &color_spaces, &space, &[1.0], 1.0)
            .expect("Type 4 Separation full-tint must resolve to Some(rgba)");
        let (r, g, b, a) = rgba;
        assert!(
            (r - 0.9255).abs() < 1.0e-3 && g.abs() < 1.0e-3 && (b - 0.5490).abs() < 1.0e-3 && (a - 1.0).abs() < 1.0e-3,
            "Type 4 Separation at tint=1 must produce process-ink magenta RGBA \
             (#EC008C ≈ 0.9255, 0, 0.5490, 1); got ({r}, {g}, {b}, {a})"
        );
    }

    #[test]
    fn pipeline_resolve_components_short_circuits_for_device_families() {
        // Parity pin. For DeviceRGB / DeviceGray / DeviceCMYK the
        // pipeline must produce the same RGBA the inline shading
        // path would compute (modulo the inline path's
        // long-standing DeviceCMYK truncation bug, which is the
        // entire reason wave 4 exists). The pin here is on the
        // resolver's behaviour, not on the inline path: for each
        // device family the resolved RGBA must equal the
        // mathematically-correct device→RGB conversion. ~keep
        let renderer = PageRenderer::new(RenderOptions::default());
        let doc = fixture_doc();
        let color_spaces: HashMap<String, Object> = HashMap::new();

        let rgb_space = Object::Name("DeviceRGB".to_string());
        let rgba = renderer
            .pipeline_resolve_components(&doc, &color_spaces, &rgb_space, &[0.5, 0.25, 0.75], 0.8)
            .expect("DeviceRGB must resolve");
        let (r, g, b, a) = rgba;
        assert!(
            (r - 0.5).abs() < 1.0e-6
                && (g - 0.25).abs() < 1.0e-6
                && (b - 0.75).abs() < 1.0e-6
                && (a - 0.8).abs() < 1.0e-6,
            "DeviceRGB must pass components through verbatim with alpha folded; got ({r}, {g}, {b}, {a})"
        );

        let gray_space = Object::Name("DeviceGray".to_string());
        let rgba = renderer
            .pipeline_resolve_components(&doc, &color_spaces, &gray_space, &[0.42], 1.0)
            .expect("DeviceGray must resolve");
        let (r, g, b, _a) = rgba;
        assert!(
            (r - 0.42).abs() < 1.0e-6 && (g - 0.42).abs() < 1.0e-6 && (b - 0.42).abs() < 1.0e-6,
            "DeviceGray must expand the single component to (g, g, g); got ({r}, {g}, {b})"
        );

        // DeviceCMYK: process-ink conversion (tetralinear over the 16
        // measured ink corners). Pure cyan (1, 0, 0, 0) lands on the
        // measured cyan corner #00ADEF = (0, 0.6784, 0.9373). ~keep
        let cmyk_space = Object::Name("DeviceCMYK".to_string());
        let rgba = renderer
            .pipeline_resolve_components(&doc, &color_spaces, &cmyk_space, &[1.0, 0.0, 0.0, 0.0], 1.0)
            .expect("DeviceCMYK must resolve");
        let (r, g, b, _a) = rgba;
        assert!(
            r.abs() < 1.0e-3 && (g - 0.6784).abs() < 1.0e-3 && (b - 0.9373).abs() < 1.0e-3,
            "DeviceCMYK pure cyan must map to process-ink #00ADEF (0, 0.6784, 0.9373); got ({r}, {g}, {b})"
        );
    }

    // Perf-regression probe: apply_pending_clip's only expensive work is the
    // path-rasterization branch that runs when `pending_clip` is Some. A naive
    // refactor (e.g. dropping the `Option::take` short-circuit, or treating
    // every paint op as a fresh clip) would explode the materialization count
    // to O(paint ops). This test pins the contract by driving the function
    // directly with K paint-op-style invocations and N clip-state changes, and
    // asserting the materialization count equals N — not K.
    //
    // The probe is serialized via `APC_PROBE_LOCK` because it reads / resets a
    // process-wide AtomicU64. No other test in this mod calls
    // `apply_pending_clip`, but the lock keeps the contract safe under future
    // additions and under `cargo test -- --test-threads=1` parity. ~keep
    static APC_PROBE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn apply_pending_clip_materializes_only_per_clip_state_change() {
        use crate::content::GraphicsStateStack;
        use std::sync::atomic::Ordering;
        use tiny_skia::{FillRule, PathBuilder, Pixmap, Rect, Transform};

        let _guard = APC_PROBE_LOCK.lock().unwrap();

        let pixmap = Pixmap::new(200, 200).expect("pixmap");
        let gs_stack = GraphicsStateStack::new();
        let base_transform = Transform::identity();

        let make_clip_path = || {
            let mut pb = PathBuilder::new();
            pb.push_rect(Rect::from_xywh(10.0, 10.0, 50.0, 50.0).unwrap());
            pb.finish().unwrap()
        };

        // Scenario A: 1 clip-state change followed by K paint-op-style calls
        // with no pending clip. Only the first call should materialize. ~keep
        const K: usize = 100;
        APC_MATERIALIZED.store(0, Ordering::Relaxed);
        let mut clip_stack: Vec<Option<tiny_skia::Mask>> = vec![None];
        let mut pending: Option<(tiny_skia::Path, FillRule)> = Some((make_clip_path(), FillRule::Winding));
        for _ in 0..K {
            apply_pending_clip(&mut pending, &mut clip_stack, &pixmap, base_transform, &gs_stack);
        }
        let after_one_clip = APC_MATERIALIZED.load(Ordering::Relaxed);
        assert_eq!(
            after_one_clip, 1,
            "1 W operator followed by {K} paint ops must materialize the clip \
             mask exactly once (got {after_one_clip})"
        );

        // Scenario B: N clip-state changes each followed by K paint ops.
        // Materialization count must equal N, not K*N. ~keep
        const N: usize = 5;
        APC_MATERIALIZED.store(0, Ordering::Relaxed);
        let mut clip_stack: Vec<Option<tiny_skia::Mask>> = vec![None];
        for _ in 0..N {
            let mut pending: Option<(tiny_skia::Path, FillRule)> = Some((make_clip_path(), FillRule::Winding));
            for _ in 0..K {
                apply_pending_clip(&mut pending, &mut clip_stack, &pixmap, base_transform, &gs_stack);
            }
        }
        let after_n_clips = APC_MATERIALIZED.load(Ordering::Relaxed);
        assert_eq!(
            after_n_clips, N as u64,
            "{N} W operators each followed by {K} paint ops must materialize \
             exactly {N} times (got {after_n_clips})"
        );
    }

    /// `type3_font_matrix` returns the explicit `/FontMatrix` when well-formed,
    /// and falls back to the Type 1 default for missing / malformed entries.
    #[test]
    fn type3_font_matrix_parse() {
        let mut d: HashMap<String, Object> = HashMap::new();
        d.insert(
            "FontMatrix".into(),
            Object::Array(vec![
                Object::Real(0.01),
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(0.02),
                Object::Integer(5),
                Object::Integer(6),
            ]),
        );
        let m = type3_font_matrix(&d);
        assert!((m.sx - 0.01).abs() < 1e-9 && (m.sy - 0.02).abs() < 1e-9);
        assert!((m.tx - 5.0).abs() < 1e-6 && (m.ty - 6.0).abs() < 1e-6);

        // Missing entry → 1/1000 default. ~keep
        let empty: HashMap<String, Object> = HashMap::new();
        let def = type3_font_matrix(&empty);
        assert!((def.sx - 0.001).abs() < 1e-9 && (def.sy - 0.001).abs() < 1e-9);

        let mut bad: HashMap<String, Object> = HashMap::new();
        bad.insert("FontMatrix".into(), Object::Array(vec![Object::Real(0.5)]));
        let badm = type3_font_matrix(&bad);
        assert!((badm.sx - 0.001).abs() < 1e-9);
    }

    /// Build a minimal single-page PDF with a Type 3 font whose only glyph
    /// (`/rect`, code 65) is a `d1` stencil that fills a 700×700 glyph-space
    /// rectangle. The page shows it once, at font size 100, after setting the
    /// fill colour to red.
    fn build_type3_rect_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        let mut offsets: Vec<usize> = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"3 0 obj\n\
              << /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100]\n\
                 /Contents 4 0 R\n\
                 /Resources << /Font << /T3 5 0 R >> >>\n\
              >>\nendobj\n\n",
        );

        let content = b"BT /T3 100 Tf 1 0 0 rg 10 10 Td (A) Tj ET";
        offsets.push(pdf.len());
        let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
        pdf.extend_from_slice(hdr.as_bytes());
        pdf.extend_from_slice(content);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"5 0 obj\n\
              << /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750]\n\
                 /FontMatrix [0.001 0 0 0.001 0 0]\n\
                 /FirstChar 65 /LastChar 65 /Widths [700]\n\
                 /Encoding 6 0 R /CharProcs 7 0 R >>\nendobj\n\n",
        );

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"6 0 obj\n<< /Type /Encoding /Differences [65 /rect] >>\nendobj\n\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"7 0 obj\n<< /rect 8 0 R >>\nendobj\n\n");

        let glyph = b"700 0 0 0 700 700 d1 0 0 700 700 re f";
        offsets.push(pdf.len());
        let ghdr = format!("8 0 obj\n<< /Length {} >>\nstream\n", glyph.len());
        pdf.extend_from_slice(ghdr.as_bytes());
        pdf.extend_from_slice(glyph);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        let xref_offset = pdf.len();
        let n_obj = offsets.len() + 1;
        let mut xref = format!("xref\n0 {}\n", n_obj);
        xref.push_str("0000000000 65535 f \n");
        for off in &offsets {
            xref.push_str(&format!("{:010} 00000 n \n", off));
        }
        pdf.extend_from_slice(xref.as_bytes());
        let trailer = format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            n_obj, xref_offset
        );
        pdf.extend_from_slice(trailer.as_bytes());
        pdf
    }

    /// The Type 3 `d1` glyph paints a filled rectangle that takes the current
    /// (red) fill colour, producing non-blank red pixels in the glyph cell.
    #[test]
    fn type3_d1_glyph_renders_filled_rect() {
        use crate::document::PdfDocument;

        let pdf = build_type3_rect_pdf();
        let doc = PdfDocument::from_bytes(pdf).expect("parse Type3 PDF");

        let opts = RenderOptions {
            format: ImageFormat::RawRgba8,
            ..RenderOptions::with_dpi(150)
        };
        let mut renderer = PageRenderer::new(opts);
        let img = renderer.render_page(&doc, 0).expect("render page");

        assert_eq!(img.format, ImageFormat::RawRgba8);
        assert_eq!(img.data.len(), (img.width * img.height * 4) as usize);

        // Count red pixels: R high, G/B low. A blank page (glyph not painted)
        // yields zero; the d1 stencil taking the current fill colour yields a
        // solid red rectangle. ~keep
        let mut red = 0usize;
        for px in img.data.as_chunks::<4>().0 {
            if px[0] > 200 && px[1] < 80 && px[2] < 80 {
                red += 1;
            }
        }
        assert!(
            red > 200,
            "expected a red Type3 d1 glyph rectangle, found {red} red pixels \
             in a {}x{} image",
            img.width,
            img.height
        );
    }

    /// Build a minimal single-page PDF whose only content is a `Do` of an
    /// image XObject declaring `/Height 0`. The image decodes to an empty
    /// buffer, so the blit dimensions computed by `render_image` are
    /// degenerate.
    fn build_zero_height_image_pdf() -> Vec<u8> {
        let mut pdf = Vec::new();
        let mut offsets: Vec<usize> = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"3 0 obj\n\
              << /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100]\n\
                 /Contents 4 0 R\n\
                 /Resources << /XObject << /Im0 5 0 R >> >>\n\
              >>\nendobj\n\n",
        );

        let content = b"q 100 0 0 100 0 0 cm /Im0 Do Q";
        offsets.push(pdf.len());
        let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
        pdf.extend_from_slice(hdr.as_bytes());
        pdf.extend_from_slice(content);
        pdf.extend_from_slice(b"\nendstream\nendobj\n\n");

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"5 0 obj\n\
              << /Type /XObject /Subtype /Image /Width 4 /Height 0\n\
                 /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 0 >>\n\
              stream\n\nendstream\nendobj\n\n",
        );

        let xref_offset = pdf.len();
        let n_obj = offsets.len() + 1;
        let mut xref = format!("xref\n0 {}\n", n_obj);
        xref.push_str("0000000000 65535 f \n");
        for off in &offsets {
            xref.push_str(&format!("{:010} 00000 n \n", off));
        }
        pdf.extend_from_slice(xref.as_bytes());
        let trailer = format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            n_obj, xref_offset
        );
        pdf.extend_from_slice(trailer.as_bytes());
        pdf
    }

    /// `IntSize::from_wh` rejects a zero dimension, so a degenerate image must
    /// take the same quiet-skip path as any other pixmap that cannot be built.
    #[test]
    fn zero_dimension_image_blit_is_skipped() {
        use crate::document::PdfDocument;

        let pdf = build_zero_height_image_pdf();
        let doc = PdfDocument::from_bytes(pdf).expect("parse degenerate-image PDF");

        let opts = RenderOptions {
            format: ImageFormat::RawRgba8,
            ..RenderOptions::with_dpi(72)
        };
        let mut renderer = PageRenderer::new(opts);
        let img = renderer.render_page(&doc, 0).expect("render page");

        assert_eq!(img.data.len(), (img.width * img.height * 4) as usize);
    }
    /// A Type 0 sampled tint transform's input arity comes straight from the
    /// document's `/Domain` array, and the evaluator interpolates across
    /// `2^arity` grid corners. An all-ones `/Size` collapses the declared
    /// sample count to 1, so a one-byte stream satisfies every length guard at
    /// any arity whatsoever — which leaves [`MAX_SAMPLED_FUNCTION_DIMS`] as the
    /// only thing bounding the corner loop.
    fn all_ones_type0_function(dims: usize) -> (Object, std::collections::HashMap<String, Object>) {
        let mut dict = std::collections::HashMap::new();
        dict.insert(
            "Domain".to_string(),
            Object::Array(
                (0..dims)
                    .flat_map(|_| [Object::Integer(0), Object::Integer(1)])
                    .collect(),
            ),
        );
        dict.insert(
            "Range".to_string(),
            Object::Array(vec![Object::Integer(0), Object::Integer(1)]),
        );
        dict.insert("Size".to_string(), Object::Array(vec![Object::Integer(1); dims]));
        dict.insert("BitsPerSample".to_string(), Object::Integer(8));
        let obj = Object::Stream {
            dict: dict.clone(),
            data: bytes::Bytes::from_static(&[0u8]),
        };
        (obj, dict)
    }

    /// The cap is a real ceiling, not a rejection of everything: a tint
    /// transform sitting exactly on it must still evaluate. Without this the
    /// dimension guard could be tightened to zero and every negative test
    /// would keep passing.
    #[test]
    fn sampled_tint_transform_at_the_dimension_cap_still_evaluates() {
        let (obj, dict) = all_ones_type0_function(MAX_SAMPLED_FUNCTION_DIMS);
        let bc = vec![0.5_f32; MAX_SAMPLED_FUNCTION_DIMS];

        assert_eq!(
            evaluate_type0_multi(&obj, &dict, &bc),
            Some(vec![0.0]),
            "a {MAX_SAMPLED_FUNCTION_DIMS}-dimension tint transform is legal and must still evaluate"
        );
    }

    /// One dimension past the cap must be rejected on arity alone. Against
    /// uncapped code this returns `Some` — every other guard is satisfied by
    /// the all-ones `/Size` — so the corner count is document-controlled.
    #[test]
    fn sampled_tint_transform_past_the_dimension_cap_is_rejected() {
        let dims = MAX_SAMPLED_FUNCTION_DIMS + 1;
        let (obj, dict) = all_ones_type0_function(dims);
        let bc = vec![0.5_f32; dims];

        assert_eq!(
            evaluate_type0_multi(&obj, &dict, &bc),
            None,
            "a {dims}-dimension /Domain must be rejected before the 2^{dims} corner loop"
        );
    }
}
