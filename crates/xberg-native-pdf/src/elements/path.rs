//! Path/vector graphics content element types.
//!
//! This module provides the `PathContent` type for representing
//! vector graphics in PDFs.

use crate::extractors::text::ArtifactType;
use crate::geometry::Rect;
use crate::layout::Color;

/// Vector path content that can be extracted from or written to a PDF.
///
/// This represents vector graphics such as lines, curves, and shapes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PathContent {
    /// Bounding box of the path
    pub bbox: Rect,
    /// Path operations
    pub operations: Vec<PathOperation>,
    /// Stroke color (None for no stroke)
    pub stroke_color: Option<Color>,
    /// Fill color (None for no fill)
    pub fill_color: Option<Color>,
    /// Stroke width in points
    pub stroke_width: f32,
    /// Line cap style
    pub line_cap: LineCap,
    /// Line join style
    pub line_join: LineJoin,
    /// Optional dash pattern: `Some((dashes, phase))` emits a
    /// `[dashes...] phase d` operator before stroking. `dashes` is on/
    /// off lengths in points (e.g. `[3.0, 2.0]` = dash 3 pt, gap 2 pt,
    /// repeating); `phase` is the starting offset. `None` leaves the
    /// line solid.
    #[serde(default)]
    pub dash_pattern: Option<(Vec<f32>, f32)>,
    /// Optional 2D affine transform in PDF row order `[a b c d e f]`.
    /// When set, the path is wrapped in `q ... cm ... Q` on emission
    /// so graphics-state stays scoped. Populated by
    /// `FluentPageBuilder::{rotated, scaled, translated, with_transform}`
    /// closures — v0.3.39 (text-only) extended here to cover paths.
    /// #393 Bundle A-2 follow-up.
    #[serde(default)]
    pub matrix: Option<[f32; 6]>,
    /// Reading order index
    pub reading_order: Option<usize>,
    /// When set, this path is wrapped in `/Artifact <</Type /T>>  BDC … EMC`
    /// markers so accessibility tools can skip it. Useful for decorative
    /// separator lines (footnote rules, header/footer rules).
    #[serde(skip)]
    pub artifact_type: Option<ArtifactType>,
    /// Optional Content Group (PDF "layer") name resolved from the
    /// surrounding `BDC /OC … EMC` markers in the content stream. Set
    /// during path extraction by `PathExtractor` when an OCG is active;
    /// `None` for paths emitted outside any `/OC`-tagged marked-content
    /// region or in PDFs that do not declare optional content.
    ///
    /// The string is the human-readable `/Name` entry of the referenced
    /// `OptionalContentGroup`, e.g. `"A-GRID"`, `"S-COLS"`, `"A-WALL-DIM"`
    /// for PDFs exported from Revit/AutoCAD with layer metadata intact.
    /// Reference: ISO 32000-1:2008 §8.11 (Optional Content) + §14.6
    /// (Marked Content).
    #[serde(default)]
    pub layer: Option<String>,
}

impl PathContent {
    /// Create a new empty path content element.
    pub fn new(bbox: Rect) -> Self {
        Self {
            bbox,
            operations: Vec::new(),
            stroke_color: Some(Color::black()),
            fill_color: None,
            stroke_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            dash_pattern: None,
            matrix: None,
            reading_order: None,
            artifact_type: None,
            layer: None,
        }
    }

    /// Create a path from operations.
    pub fn from_operations(operations: Vec<PathOperation>) -> Self {
        let bbox = path_operations_bbox(&operations);
        Self {
            bbox,
            operations,
            stroke_color: Some(Color::black()),
            fill_color: None,
            stroke_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            dash_pattern: None,
            matrix: None,
            reading_order: None,
            artifact_type: None,
            layer: None,
        }
    }

    /// Set stroke color.
    pub fn with_stroke(mut self, color: Color) -> Self {
        self.stroke_color = Some(color);
        self
    }

    /// Set fill color.
    pub fn with_fill(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }

    /// Set stroke width.
    pub fn with_stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Set reading order.
    pub fn with_reading_order(mut self, order: usize) -> Self {
        self.reading_order = Some(order);
        self
    }

    /// Set the Optional Content Group (PDF "layer") name. Used by
    /// `PathExtractor` while walking the content stream to attach the
    /// active OCG name to each extracted path.
    pub fn with_layer(mut self, layer: impl Into<String>) -> Self {
        self.layer = Some(layer.into());
        self
    }

    /// Add a path operation.
    pub fn push(&mut self, op: PathOperation) {
        self.operations.push(op);
    }

    /// Check if this path has a stroke.
    pub fn has_stroke(&self) -> bool {
        self.stroke_color.is_some() && self.stroke_width > 0.0
    }

    /// Check if this path has a fill.
    pub fn has_fill(&self) -> bool {
        self.fill_color.is_some()
    }

    /// Check if this path represents a single straight line (v0.3.14).
    ///
    /// A path is a straight line if it has exactly 2 operations:
    /// MoveTo followed by LineTo.
    pub fn is_straight_line(&self) -> bool {
        (self.operations.len() == 2
            && matches!(self.operations[0], PathOperation::MoveTo(_, _))
            && matches!(self.operations[1], PathOperation::LineTo(_, _)))
            || (self.operations.len() == 3
                && matches!(self.operations[0], PathOperation::MoveTo(_, _))
                && matches!(self.operations[1], PathOperation::LineTo(_, _))
                && matches!(self.operations[2], PathOperation::ClosePath))
    }

    /// Rendered extents of this path: the geometric [`bbox`](Self::bbox)
    /// inflated by the stroke (ISO 32000-1:2008 §8.4.3.2 — half the line
    /// width straddles each side of the path).
    ///
    /// Print-era producers draw a table's vertical rule as a ~1 pt
    /// horizontal segment stroked as wide as the table is tall, so the
    /// geometric bbox (1×0 pt) bears no resemblance to the bar the reader
    /// sees. Classifiers that reason about what is *rendered*
    /// (line orientation, table primitives, region hit-testing) should use
    /// this; consumers that need the raw path geometry (corner tests, SVG
    /// path data, editing) keep `bbox`.
    ///
    /// For a single straight segment the inflation is exact: half the
    /// stroke width perpendicular to the segment, plus the cap extension
    /// along it (`Butt` caps add nothing axially; `Round`/`Square` add half
    /// the width per end, §8.4.3.3). For any other shape the bbox is
    /// inflated by half the width on all four sides — a conservative
    /// superset that is exact for axis-aligned stroked outlines.
    ///
    /// Unstroked paths return `bbox` unchanged.
    pub fn rendered_bbox(&self) -> Rect {
        if !self.has_stroke() {
            return self.bbox;
        }
        let half = self.stroke_width * 0.5;

        if let [PathOperation::MoveTo(x0, y0), PathOperation::LineTo(x1, y1), ..] = self.operations.as_slice()
            && self.is_straight_line()
        {
            let (dx, dy) = (x1 - x0, y1 - y0);
            let len = dx.hypot(dy);
            // A zero-length butt-capped segment paints nothing
            // (§8.4.3.3), so its rendered extent is its degenerate
            // geometry; Round/Square caps paint a dot and fall through
            // to the all-sides outset below. ~keep
            if len <= f32::EPSILON && self.line_cap == LineCap::Butt {
                return self.bbox;
            }
            if len > f32::EPSILON {
                // Perpendicular unit vector scaled to the half-width,
                // plus the axial cap extension. ~keep
                let (px, py) = ((dy / len * half).abs(), (dx / len * half).abs());
                let (cx, cy) = if self.line_cap == LineCap::Butt {
                    (0.0, 0.0)
                } else {
                    ((dx / len * half).abs(), (dy / len * half).abs())
                };
                return Rect::new(
                    self.bbox.x - px - cx,
                    self.bbox.y - py - cy,
                    self.bbox.width + 2.0 * (px + cx),
                    self.bbox.height + 2.0 * (py + cy),
                );
            }
        }

        Rect::new(
            self.bbox.x - half,
            self.bbox.y - half,
            self.bbox.width + 2.0 * half,
            self.bbox.height + 2.0 * half,
        )
    }

    /// Check if this path is a horizontal line within a tolerance (v0.3.16).
    ///
    /// `tolerance` bounds the *geometric* minor extent as before (a nearly
    /// axis-aligned segment or sliver rectangle). Orientation is then
    /// confirmed on the [`rendered_bbox`](Self::rendered_bbox): a 1 pt
    /// segment stroked 430 pt wide renders as a vertical bar and must not
    /// classify as a horizontal line just because its raw geometry is a
    /// horizontal speck.
    pub fn is_horizontal_line(&self, tolerance: f32) -> bool {
        if !self.is_straight_line() && !self.is_rectangle() {
            return false;
        }
        let rendered = self.rendered_bbox();
        self.bbox.height.abs() < tolerance && rendered.width.abs() >= rendered.height.abs()
    }

    /// Check if this path is a vertical line within a tolerance (v0.3.16).
    ///
    /// Same contract as [`is_horizontal_line`](Self::is_horizontal_line):
    /// geometric thinness within `tolerance`, orientation confirmed on the
    /// rendered extents.
    pub fn is_vertical_line(&self, tolerance: f32) -> bool {
        if !self.is_straight_line() && !self.is_rectangle() {
            return false;
        }
        let rendered = self.rendered_bbox();
        self.bbox.width.abs() < tolerance && rendered.height.abs() >= rendered.width.abs()
    }

    /// Check if this path's bounding box is nearly touching another (v0.3.16).
    pub fn is_nearly_touching(&self, other: &Rect, tolerance: f32) -> bool {
        let expanded = Rect::new(
            self.bbox.x - tolerance,
            self.bbox.y - tolerance,
            self.bbox.width + 2.0 * tolerance,
            self.bbox.height + 2.0 * tolerance,
        );
        expanded.intersects(other)
    }

    /// Check if this path represents a single rectangle (v0.3.14).
    ///
    /// A path is a rectangle if it has exactly 1 operation: Rectangle,
    /// or if it has 5 operations: MoveTo, 3x LineTo, ClosePath that form a rectangle.
    pub fn is_rectangle(&self) -> bool {
        if self.operations.len() == 1 && matches!(self.operations[0], PathOperation::Rectangle(_, _, _, _)) {
            return true;
        }

        // Case 2: MoveTo + 3x LineTo + (Optional ClosePath)
        // Must be axis-aligned. We check that consecutive points share X or Y. ~keep
        if ((self.operations.len() == 5 && matches!(self.operations[4], PathOperation::ClosePath))
            || (self.operations.len() == 4))
            && let (
                PathOperation::MoveTo(x0, y0),
                PathOperation::LineTo(x1, y1),
                PathOperation::LineTo(x2, y2),
                PathOperation::LineTo(x3, y3),
            ) = (
                &self.operations[0],
                &self.operations[1],
                &self.operations[2],
                &self.operations[3],
            )
        {
            let tol = 0.1;
            let side1 = ((x0 - x1).abs() < tol) || ((y0 - y1).abs() < tol);
            let side2 = ((x1 - x2).abs() < tol) || ((y1 - y2).abs() < tol);
            let side3 = ((x2 - x3).abs() < tol) || ((y2 - y3).abs() < tol);

            return side1 && side2 && side3;
        }

        false
    }

    /// Check if this path is "box-like" or "line-like" based on its dimensions (v0.3.16).
    /// This is a fuzzy heuristic for table detection.
    ///
    /// Line length is judged on the [`rendered_bbox`](Self::rendered_bbox)
    /// while thinness stays geometric, so a table rule encoded as a short
    /// segment with a table-height stroke width — a 1×0 pt geometric speck
    /// rendering as a 1×430 pt bar — is recognized as the ruling it draws
    ///, and an ordinary rule with a thick stroke keeps
    /// classifying as a line exactly as before.
    ///
    /// Thresholds (inherited from the v0.3.16 heuristic): a ruling must
    /// run more than 5 pt — half an em at 10 pt body text — so tick marks
    /// and list bullets don't qualify; it must be under 2 pt across, twice
    /// the heaviest common ruling weight (typical PDF rules are 0.5–1 pt)
    /// while safely below any filled cell; and a box must stay under
    /// 1000 pt per side so absurdly large shapes (chart backgrounds, full
    /// artboards) don't seed a cluster on their own.
    ///
    /// GH#1656: `1000 pt` does **not** exclude an ordinary page — A4
    /// (595×842) and US-Letter (612×792) both pass this bound, so a
    /// full-page background rectangle (common Office-to-PDF output) is
    /// accepted here as a table primitive and can cluster with every
    /// other primitive on the page. This function has no access to the
    /// page's MediaBox (`PathContent` carries no page dimensions), so it
    /// cannot make that call itself. Page-relative filtering of such
    /// furniture happens one level up, in `PdfDocument::extract_tables_with_config`
    /// and `PdfDocument::extract_page_tables` (`document/tables.rs`), which use
    /// [`PathContent::is_page_frame_rectangle`](Self::is_page_frame_rectangle)
    /// against the real MediaBox before primitives reach the clusterer in
    /// `spatial_table_detector.rs`. ~keep
    pub fn is_table_primitive(&self) -> bool {
        let rendered = self.rendered_bbox();

        // Very thin horizontal or vertical line: extends > 5.0 along its
        // rendered axis, geometrically thinner than 2.0 across it, and
        // rendered at least 2:1 elongated — a zero-length segment with a
        // fat round/square cap (dot leaders, bullets) renders as a square
        // blob, not a ruling, and must not seed line clusters. ~keep
        if (rendered.width.abs() > 5.0
            && self.bbox.height.abs() < 2.0
            && rendered.width.abs() >= 2.0 * rendered.height.abs())
            || (rendered.height.abs() > 5.0
                && self.bbox.width.abs() < 2.0
                && rendered.height.abs() >= 2.0 * rendered.width.abs())
        {
            return true;
        }

        let w = self.bbox.width.abs();
        let h = self.bbox.height.abs();
        if w > 5.0 && h > 5.0 && w < 1000.0 && h < 1000.0 {
            return true;
        }

        false
    }

    /// True when this path is a rectangle covering at least this fraction of
    /// the MediaBox in *both* dimensions. 0.9 leaves generous margin for a
    /// page background that stops just short of the trim box while still
    /// rejecting anything smaller than page furniture — a genuine table
    /// spanning most of a page's text column is still well under 90% of the
    /// full MediaBox once margins are accounted for. GH#1656. ~keep
    pub const PAGE_FRAME_COVERAGE_THRESHOLD: f32 = 0.9;

    /// True when this path is a rectangle spanning at least
    /// [`PAGE_FRAME_COVERAGE_THRESHOLD`](Self::PAGE_FRAME_COVERAGE_THRESHOLD)
    /// of `media_box` in both width and height — page furniture (a
    /// full-bleed background fill, a decorative frame) rather than a table
    /// primitive. GH#1656: `is_table_primitive`'s `< 1000 pt` bound admits
    /// ordinary page-sized rectangles (A4, Letter), which then seed a
    /// whole-page false-positive cluster; this predicate is how callers with
    /// access to the page's MediaBox (`PathContent` itself carries none) can
    /// exclude that furniture before it reaches the clusterer.
    ///
    /// `media_box` is `(llx, lly, urx, ury)` as returned by
    /// `PdfDocument::get_page_media_box`. Requiring *both* dimensions to
    /// clear the threshold is deliberate: a full-page-width footer rule
    /// (wide, but a fraction of a point tall) must not be classified as page
    /// furniture and dropped — it is a legitimate table/ruling primitive.
    /// A degenerate `media_box` (zero or negative area, non-finite) can't
    /// establish "most of the page", so this returns `false` — callers
    /// should treat a failed MediaBox lookup as "leave every primitive
    /// alone", not as license to filter on garbage. ~keep
    pub fn is_page_frame_rectangle(&self, media_box: (f32, f32, f32, f32)) -> bool {
        if !self.is_rectangle() {
            return false;
        }

        let (llx, lly, urx, ury) = media_box;
        let page_width = (urx - llx).abs();
        let page_height = (ury - lly).abs();
        if !page_width.is_finite() || !page_height.is_finite() || page_width <= 0.0 || page_height <= 0.0 {
            return false;
        }

        let w = self.bbox.width.abs();
        let h = self.bbox.height.abs();
        if !w.is_finite() || !h.is_finite() {
            return false;
        }

        w >= page_width * Self::PAGE_FRAME_COVERAGE_THRESHOLD && h >= page_height * Self::PAGE_FRAME_COVERAGE_THRESHOLD
    }

    /// Create a line path from (x1, y1) to (x2, y2).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let line = PathContent::line(10.0, 10.0, 100.0, 100.0);
    /// ```
    pub fn line(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        let ops = vec![PathOperation::MoveTo(x1, y1), PathOperation::LineTo(x2, y2)];
        Self::from_operations(ops)
    }

    /// Create a rectangle path.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rect = PathContent::rect(10.0, 10.0, 100.0, 50.0);
    /// ```
    pub fn rect(x: f32, y: f32, width: f32, height: f32) -> Self {
        let ops = vec![PathOperation::Rectangle(x, y, width, height)];
        Self::from_operations(ops)
    }

    /// Create an approximate circle path using Bezier curves.
    ///
    /// Uses 4 cubic Bezier curves to approximate a circle.
    /// The approximation uses the constant k = 0.5522847498 for control points.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let circle = PathContent::circle(100.0, 100.0, 50.0);
    /// ```
    pub fn circle(cx: f32, cy: f32, radius: f32) -> Self {
        // Magic constant for approximating a quarter circle with a cubic Bezier
        // k = 4 * (sqrt(2) - 1) / 3 ≈ 0.5522847498 ~keep
        const K: f32 = 0.552_284_8;
        let k = radius * K;

        let ops = vec![
            PathOperation::MoveTo(cx, cy + radius),
            PathOperation::CurveTo(cx + k, cy + radius, cx + radius, cy + k, cx + radius, cy),
            PathOperation::CurveTo(cx + radius, cy - k, cx + k, cy - radius, cx, cy - radius),
            PathOperation::CurveTo(cx - k, cy - radius, cx - radius, cy - k, cx - radius, cy),
            PathOperation::CurveTo(cx - radius, cy + k, cx - k, cy + radius, cx, cy + radius),
            PathOperation::ClosePath,
        ];
        Self::from_operations(ops)
    }

    /// Create a rounded rectangle path.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate of the bottom-left corner
    /// * `y` - Y coordinate of the bottom-left corner
    /// * `width` - Width of the rectangle
    /// * `height` - Height of the rectangle
    /// * `radius` - Corner radius (clamped to min(width, height) / 2)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let rounded = PathContent::rounded_rect(10.0, 10.0, 100.0, 50.0, 5.0);
    /// ```
    pub fn rounded_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> Self {
        let max_radius = width.min(height) / 2.0;
        let r = radius.min(max_radius).max(0.0);

        if r <= 0.0 {
            return Self::rect(x, y, width, height);
        }

        // Magic constant for approximating a quarter circle with a cubic Bezier ~keep
        const K: f32 = 0.552_284_8;
        let k = r * K;

        let x_right = x + width;
        let y_top = y + height;

        let ops = vec![
            PathOperation::MoveTo(x + r, y),
            PathOperation::LineTo(x_right - r, y),
            PathOperation::CurveTo(x_right - r + k, y, x_right, y + r - k, x_right, y + r),
            PathOperation::LineTo(x_right, y_top - r),
            PathOperation::CurveTo(x_right, y_top - r + k, x_right - r + k, y_top, x_right - r, y_top),
            PathOperation::LineTo(x + r, y_top),
            PathOperation::CurveTo(x + r - k, y_top, x, y_top - r + k, x, y_top - r),
            PathOperation::LineTo(x, y + r),
            PathOperation::CurveTo(x, y + r - k, x + r - k, y, x + r, y),
            PathOperation::ClosePath,
        ];
        Self::from_operations(ops)
    }

    /// Flatten this path into polylines — one `Vec<(x, y)>` per subpath.
    ///
    /// Straight segments (`MoveTo`/`LineTo`) pass through unchanged; cubic
    /// Béziers (`CurveTo`) are adaptively subdivided so the resulting polyline
    /// stays within `tolerance` of the true curve. A `Rectangle` becomes its
    /// own closed 5-point subpath, and `ClosePath` appends the subpath's start
    /// point. A new subpath begins at every `MoveTo` (and every `Rectangle`).
    ///
    /// `tolerance` is expressed in the same coordinate space as the path's own
    /// points, and the returned points are in that space verbatim. For paths
    /// from [`crate::document::PdfDocument::extract_paths`] that is the page's
    /// default user space (PDF points, y-up), with any content-stream `cm` and
    /// Form-XObject `/Matrix` transforms already folded in — so a path drawn
    /// under a scaling transform is returned (and flattened) at its final
    /// on-page size, and no tolerance rescaling is needed. It is *not* device or
    /// pixel space: page `/Rotate` and rendering DPI are not applied. Smaller
    /// values yield more, finer points. A non-positive or non-finite tolerance
    /// is floored to a small epsilon so subdivision always terminates.
    ///
    /// This is intended for consumers that need sampled coordinates rather than
    /// drawing operators — e.g. digitising chart/ECG/CAD traces from vector PDFs.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::elements::{PathContent, PathOperation};
    ///
    /// let path = PathContent::from_operations(vec![
    ///     PathOperation::MoveTo(0.0, 0.0),
    ///     PathOperation::CurveTo(0.0, 10.0, 10.0, 10.0, 10.0, 0.0),
    /// ]);
    /// let subpaths = path.to_points(0.25);
    /// assert_eq!(subpaths.len(), 1);
    /// assert_eq!(subpaths[0][0], (0.0, 0.0)); // starts at P0
    /// ```
    pub fn to_points(&self, tolerance: f32) -> Vec<Vec<(f32, f32)>> {
        // A non-finite or non-positive tolerance would make the flatness test
        // never pass, recursing to the depth cap on every curve. Floor it. ~keep
        let tol = if tolerance.is_finite() && tolerance > 0.0 {
            tolerance
        } else {
            FLATTEN_TOLERANCE_FLOOR
        };

        let mut subpaths: Vec<Vec<(f32, f32)>> = Vec::new();
        let mut current: Vec<(f32, f32)> = Vec::new();
        let mut pos = (0.0_f32, 0.0_f32);
        let mut start = (0.0_f32, 0.0_f32);

        for op in &self.operations {
            match *op {
                PathOperation::MoveTo(x, y) => {
                    // `m` begins a new subpath; a consecutive `m` overrides the
                    // previous one with "no vestige" (§8.5.2, Table 59), which
                    // flush_subpath honours by dropping the lone start point. ~keep
                    flush_subpath(&mut current, &mut subpaths);
                    pos = (x, y);
                    start = pos;
                    current.push(pos);
                }
                PathOperation::LineTo(x, y) => {
                    // A segment with no open subpath (e.g. right after `re`/`h`,
                    // or a malformed leading `l`) starts at the current point. ~keep
                    if current.is_empty() {
                        start = pos;
                        current.push(pos);
                    }
                    pos = (x, y);
                    current.push(pos);
                }
                PathOperation::CurveTo(c1x, c1y, c2x, c2y, ex, ey) => {
                    if current.is_empty() {
                        start = pos;
                        current.push(pos);
                    }
                    flatten_cubic(pos, (c1x, c1y), (c2x, c2y), (ex, ey), tol, 0, &mut current);
                    pos = (ex, ey);
                }
                PathOperation::Rectangle(x, y, w, h) => {
                    flush_subpath(&mut current, &mut subpaths);
                    // `re` is a complete closed subpath equivalent to
                    // `x y m / (x+w) y l / (x+w) (y+h) l / x (y+h) l / h`
                    // (§8.5.2, Table 59). The current point afterwards is (x, y). ~keep
                    subpaths.push(vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)]);
                    pos = (x, y);
                    start = pos;
                }
                PathOperation::ClosePath => {
                    // `h` appends a segment back to the subpath start and
                    // terminates the subpath; a following segment begins a new
                    // one (§8.5.2, Table 59). On an already-closed/empty subpath
                    // it does nothing. ~keep
                    if !current.is_empty() {
                        current.push(start);
                        flush_subpath(&mut current, &mut subpaths);
                        pos = start;
                    }
                }
            }
        }
        flush_subpath(&mut current, &mut subpaths);
        subpaths
    }
}

/// True when `operations[index]` is a `MoveTo` that begins a subpath no
/// segment follows. Such a subpath paints nothing (§8.5.2: path construction
/// operators place no marks; a lone point has "no vestige"), so it must not
/// extend the bounding box. GH#1759: a footer band followed by a stray
/// `0 842 m` otherwise reports a page-sized box, which unions every rule on
/// the page into one cluster. ~keep
fn is_degenerate_move_to(operations: &[PathOperation], index: usize) -> bool {
    !matches!(
        operations.get(index + 1),
        Some(PathOperation::LineTo(..) | PathOperation::CurveTo(..) | PathOperation::ClosePath)
    )
}

/// Compute the bounding box of a sequence of path operations.
///
/// Shared by [`PathContent::from_operations`] and
/// `PathExtractor::finalize_path`, which must agree exactly: the two were
/// independent, byte-identical copies, so GH#1759 had to be fixed twice.
pub(crate) fn path_operations_bbox(operations: &[PathOperation]) -> Rect {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for (index, op) in operations.iter().enumerate() {
        match op {
            PathOperation::MoveTo(..) if is_degenerate_move_to(operations, index) => {}
            PathOperation::MoveTo(x, y) | PathOperation::LineTo(x, y) => {
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
            PathOperation::CurveTo(x1, y1, x2, y2, x3, y3) => {
                for (x, y) in [(*x1, *y1), (*x2, *y2), (*x3, *y3)] {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
            PathOperation::Rectangle(x, y, w, h) => {
                // `PathExtractor::rectangle` stores the CTM-transformed
                // delta, so a flipping CTM (negative `a`/`d`) or a
                // negative-extent `re` yields a negative `w`/`h`. Order the
                // corners here — `Rect::new` normalises a whole rect, but is
                // handed pre-reduced min/extent values below. ~keep
                min_x = min_x.min(x.min(*x + *w));
                min_y = min_y.min(y.min(*y + *h));
                max_x = max_x.max(x.max(*x + *w));
                max_y = max_y.max(y.max(*y + *h));
            }
            PathOperation::ClosePath => {}
        }
    }

    if min_x == f32::MAX {
        Rect::new(0.0, 0.0, 0.0, 0.0)
    } else {
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// Minimum flattening tolerance (path units) used by [`PathContent::to_points`]
/// when given a non-positive or non-finite value, so subdivision terminates.
const FLATTEN_TOLERANCE_FLOOR: f32 = 1e-3;

/// Hard recursion-depth backstop for adaptive cubic subdivision. The flatness
/// test halts long before this in practice; it only guards pathological input.
const FLATTEN_MAX_DEPTH: u8 = 24;

/// Move a finished subpath from `current` into `out`, discarding it if it has
/// fewer than two points. A lone point is a subpath that adds no segment and
/// paints nothing (§8.5.2: construction operators place no marks), e.g. a
/// trailing `m` or the first of two consecutive `m` operators ("no vestige").
fn flush_subpath(current: &mut Vec<(f32, f32)>, out: &mut Vec<Vec<(f32, f32)>>) {
    if current.len() >= 2 {
        out.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

/// Adaptively subdivide a cubic Bézier (`p0`→`p3`, controls `p1`,`p2`),
/// appending flattened vertices to `out`. The start point `p0` is assumed to
/// already be the last element of `out`, so only intermediate vertices and the
/// endpoint `p3` are pushed.
fn flatten_cubic(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    tol: f32,
    depth: u8,
    out: &mut Vec<(f32, f32)>,
) {
    // The curve lies inside the convex hull of its control points; since p0 and
    // p3 are on the chord, the curve's deviation from the chord is bounded by
    // the larger control-point offset. When that is within `tol`, a single
    // straight segment is a good-enough approximation. ~keep
    if depth >= FLATTEN_MAX_DEPTH || cubic_is_flat(p0, p1, p2, p3, tol) {
        out.push(p3);
        return;
    }
    // de Casteljau split at t = 0.5; the split point lies on the true curve. ~keep
    let p01 = midpoint(p0, p1);
    let p12 = midpoint(p1, p2);
    let p23 = midpoint(p2, p3);
    let p012 = midpoint(p01, p12);
    let p123 = midpoint(p12, p23);
    let mid = midpoint(p012, p123);
    flatten_cubic(p0, p01, p012, mid, tol, depth + 1, out);
    flatten_cubic(mid, p123, p23, p3, tol, depth + 1, out);
}

fn midpoint(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5)
}

/// True when both control points lie within `tol` of the chord `p0`→`p3`.
fn cubic_is_flat(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), tol: f32) -> bool {
    dist_point_to_segment(p1, p0, p3).max(dist_point_to_segment(p2, p0, p3)) <= tol
}

/// Shortest distance from point `p` to the line segment `a`→`b`.
fn dist_point_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        // Degenerate chord (a == b): distance to the single point. ~keep
        return ((p.0 - a.0).powi(2) + (p.1 - a.1).powi(2)).sqrt();
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt()
}

impl Default for PathContent {
    fn default() -> Self {
        Self {
            bbox: Rect::new(0.0, 0.0, 0.0, 0.0),
            operations: Vec::new(),
            stroke_color: Some(Color::black()),
            fill_color: None,
            stroke_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            dash_pattern: None,
            matrix: None,
            reading_order: None,
            artifact_type: None,
            layer: None,
        }
    }
}

/// A single path operation.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum PathOperation {
    /// Move to a point (m operator)
    MoveTo(f32, f32),
    /// Line to a point (l operator)
    LineTo(f32, f32),
    /// Bezier curve to a point (c operator)
    /// (control1_x, control1_y, control2_x, control2_y, end_x, end_y)
    CurveTo(f32, f32, f32, f32, f32, f32),
    /// Rectangle (re operator)
    /// (x, y, width, height)
    Rectangle(f32, f32, f32, f32),
    /// Close the current path (h operator)
    ClosePath,
}

/// Line cap style for strokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub enum LineCap {
    /// Butt cap - line ends exactly at endpoint
    #[default]
    Butt,
    /// Round cap - semicircle at endpoint
    Round,
    /// Square cap - half square at endpoint
    Square,
}

/// Line join style for strokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub enum LineJoin {
    /// Miter join - sharp corner
    #[default]
    Miter,
    /// Round join - circular arc
    Round,
    /// Bevel join - diagonal corner
    Bevel,
}

#[cfg(test)]
mod tests;
