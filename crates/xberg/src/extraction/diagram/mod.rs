//! Deterministic diagram recovery from vector sources.
//!
//! A vector diagram already contains its own graph. Boxes are closed outlines,
//! connectors are open strokes, and labels are text drawn on top. Nothing has to
//! be inferred from pixels, so the recovered graph is exact rather than
//! probabilistic, and it carries the styling that a detection model cannot
//! recover at all.
//!
//! Each front end turns one source format into the geometry-carrying
//! intermediates below; [`assemble`] turns those into a [`DiagramGraph`]. The
//! split is what lets [`svg`] and [`pdf`] share every matching rule while
//! agreeing on nothing but the intermediates, and what would let a third
//! format (DrawingML, EMF) join them without touching the matching at all.

// The flat ODF front end names its nodes and edges outright and uses none of the
// geometry below, which only the SVG and PDF front ends match on. ~keep
#![cfg_attr(not(any(feature = "svg", feature = "pdf")), allow(dead_code))]

#[cfg(feature = "xml")]
pub(crate) mod odf;
#[cfg(feature = "pdf")]
pub(crate) mod pdf;
mod polyline;
#[cfg(all(feature = "svg", feature = "xml"))]
pub(crate) mod svg;

use crate::types::diagram::{DiagramEdge, DiagramGraph, DiagramNode, DiagramShape};

/// Axis-aligned rectangle in canvas coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    fn area(&self) -> f32 {
        self.width() * self.height()
    }

    fn centre_x(&self) -> f32 {
        (self.x0 + self.x1) / 2.0
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }

    /// Whether this rectangle fully contains `other`.
    fn encloses(&self, other: &Rect) -> bool {
        self.x0 <= other.x0 && self.y0 <= other.y0 && self.x1 >= other.x1 && self.y1 >= other.y1
    }

    /// Area shared with `other`.
    fn overlap_area(&self, other: &Rect) -> f32 {
        let width = (self.x1.min(other.x1) - self.x0.max(other.x0)).max(0.0);
        let height = (self.y1.min(other.y1) - self.y0.max(other.y0)).max(0.0);
        width * height
    }

    /// How far inside the rectangle a point sits, measured to the nearest
    /// edge. Zero on the border and outside.
    fn depth_of(&self, x: f32, y: f32) -> f32 {
        if !self.contains(x, y) {
            return 0.0;
        }
        (x - self.x0).min(self.x1 - x).min(y - self.y0).min(self.y1 - y)
    }

    /// Distance from a point to the rectangle, zero when inside.
    fn distance_to(&self, x: f32, y: f32) -> f32 {
        let dx = (self.x0 - x).max(0.0).max(x - self.x1);
        let dy = (self.y0 - y).max(0.0).max(y - self.y1);
        (dx * dx + dy * dy).sqrt()
    }
}

/// A closed outline, i.e. a node candidate.
#[derive(Debug, Clone)]
pub(crate) struct Outline {
    pub bbox: Rect,
    pub shape: DiagramShape,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f32>,
    pub dashed: bool,
}

/// An open stroke, i.e. an edge candidate.
///
/// The endpoints decide which shapes it joins. The midpoint is carried
/// separately because it is where a renderer puts the edge label, and on a
/// curved connector that is nowhere near the average of the two ends.
#[derive(Debug, Clone)]
pub(crate) struct Connector {
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub midpoint: (f32, f32),
    pub stroke: Option<String>,
    pub dashed: bool,
}

/// A run of text with the anchor point it is drawn from.
#[derive(Debug, Clone)]
pub(crate) struct Label {
    pub x: f32,
    pub y: f32,
    pub text: String,
}

/// Upper bound on outlines considered. Matching is quadratic in this, and a
/// diagram with more nodes than this is not a diagram.
const MAX_OUTLINES: usize = 2_000;

/// Upper bound on connectors considered, for the same reason.
const MAX_CONNECTORS: usize = 5_000;

/// Upper bound on labels considered. Owner resolution is `labels x outlines`,
/// and a source can hand this a text run for every glyph on the page rather
/// than one per caption; a diagram does not caption more shapes than this.
const MAX_LABELS: usize = 20_000;

/// A shape covering at least this fraction of the canvas is a background
/// panel, not a node. Charts and dashboards routinely draw one.
const BACKGROUND_AREA_RATIO: f32 = 0.9;

/// Shapes below this fraction of the canvas's larger dimension on either side
/// are decoration (arrowheads, bullets, tick marks) rather than nodes.
const MIN_NODE_SIDE_RATIO: f32 = 0.02;

/// How far a connector endpoint may sit from a shape and still be taken to
/// touch it, as a fraction of the canvas's larger dimension. Connectors are
/// usually drawn onto the boundary exactly; this absorbs the gap left by
/// arrowhead markers and by hand-placed endpoints.
const SNAP_RATIO: f32 = 0.02;

/// Absolute floor and ceiling applied to both ratios above, so a very small or
/// very large canvas does not produce a degenerate threshold.
const RATIO_FLOOR: f32 = 4.0;
const MIN_NODE_SIDE_CEILING: f32 = 20.0;
const SNAP_CEILING: f32 = 40.0;

/// An outline enclosed by another and covering at least this fraction of its
/// area is the inner ring of a double border, not a node of its own.
const CONCENTRIC_AREA_RATIO: f32 = 0.5;

/// Shapes an outline must enclose before it is read as a container rather than
/// a node. Graphviz clusters, BPMN pools, PlantUML swimlanes and grouping
/// panels all draw a box around their members, and the box is not a node.
///
/// Two, because one enclosed shape is a double border, which
/// [`collapse_concentric`] has already dealt with by this point.
const CONTAINER_MIN_MEMBERS: usize = 2;

/// Text runs an outline must hold before its layout is worth reading as a
/// grid. Two is a wrapped caption; a table has a header row and a body.
const GRID_MIN_CELLS: usize = 3;

/// How far apart two text anchors must be, as a fraction of the shape they sit
/// in, to count as different rows or columns.
const GRID_CLUSTER_RATIO: f32 = 0.1;

/// How much of the smaller of two shapes must be shared before they count as
/// overlapping rather than merely adjacent. Boxes drawn edge to edge, and the
/// rounding slack around them, must not trip it.
const OVERLAP_RATIO: f32 = 0.1;

/// An unlabelled shape below this fraction of the median labelled node's area
/// is decoration rather than a node.
const UNLABELLED_DECORATION_RATIO: f32 = 0.25;

/// An arrowhead covers at most this fraction of the largest shape's area.
/// Graphviz draws a 7x10 head against nodes an order of magnitude larger.
const ARROWHEAD_AREA_RATIO: f32 = 0.2;

/// An arrowhead is also small in absolute terms: neither side exceeds this
/// fraction of the canvas. The area ratio alone is not enough, because a
/// diagram with one large panel makes every real node look small beside it.
const ARROWHEAD_MAX_SIDE_RATIO: f32 = 0.05;

/// How far from a connector's midpoint, in units of the snap tolerance, text
/// may sit and still be that connector's label. Renderers offset edge labels
/// clear of the line, so this has to reach further than shape snapping does.
const EDGE_LABEL_REACH: f32 = 4.0;

/// How far below a shape a caption may sit, in units of the snap tolerance,
/// and still be that shape's name. Shorter than the edge-label reach, because
/// a caption is set tight under the thing it names while an edge label is
/// pushed clear of the line it belongs to.
const CAPTION_REACH: f32 = 3.0;

/// Absolute ceiling on that reach. The gap under a caption is set in text, so
/// it scales with the type size and not with the drawing: on a poster-sized
/// canvas three snap tolerances is over an inch, and everything within an inch
/// below a shape would become its name.
const CAPTION_CEILING: f32 = 48.0;

/// Minimum number of parallel, same-style strokes before a group is read as
/// chart gridlines rather than a coincidence. A chain of linked shapes
/// routinely has two edges running in the same direction; three sharing a
/// colour, a dash style, a spacing *and* a span besides is what a plotting
/// library draws for its scale, and nothing else a diagram draws does all
/// four at once.
const GRIDLINE_MIN_COUNT: usize = 3;

/// How far a stroke's own two ends may differ, on the axis it is meant to run
/// square to, and still count as axis-aligned. A gridline is drawn dead
/// straight; this only has to absorb floating point noise carried through a
/// transform, not a genuinely diagonal or curved connector.
const GRIDLINE_AXIS_EPSILON: f32 = 0.75;

/// How far the gap between two adjacent members of a candidate gridline
/// family may deviate from the family's own average gap, as a fraction of
/// that average, and still count as "regular". A plotting library spaces
/// gridlines by a fixed data interval, so real ones are near-exact; this only
/// has to absorb rounding in the source file.
const GRIDLINE_SPACING_TOLERANCE_RATIO: f32 = 0.15;

/// Minimum number of shapes an axis-aligned stroke must lie flush against,
/// each for more than a single point of contact, before it is read as a
/// chart's own axis or frame line rather than a connector. See
/// [`find_frame_lines`]. Shares [`GRIDLINE_MIN_COUNT`]'s value and rationale:
/// three is the smallest count a coincidence cannot plausibly explain, and
/// nothing a layout engine draws lines a connector up against the shared
/// border of that many shapes it does not join.
const FRAME_MIN_FLUSH_EDGES: usize = GRIDLINE_MIN_COUNT;

/// Build a graph from recovered geometry, or `None` when the geometry is not a
/// graph.
///
/// A vector drawing is not necessarily a diagram. Bar charts, logos and
/// illustrations all yield closed outlines, and reporting those as an
/// edgeless node list would be noise dressed up as structure. Recovery
/// therefore requires at least one connector that resolves to a pair of
/// distinct shapes.
///
/// The result is deterministic. Nodes are ordered top to bottom then left to
/// right, edges by their endpoints, and duplicates of both are collapsed.
/// Select node-candidate outlines from raw vector geometry, and split off the shapes
/// too small to be a node as decoration.
///
/// Filtering decoration away before looking for arrowheads would leave only the
/// arrowheads big enough to have been nodes, which on a diagram measured in points is
/// none of them, so both collections are returned rather than only the kept outlines.
///
/// The kept outlines come back in deterministic order: top to bottom then left to
/// right, with the remaining bounds breaking ties so two shapes sharing a corner still
/// sort the same way every time. A shape drawn twice (a fill path under a stroke path,
/// a shadow copy) is one node, and so is a shape drawn with a double border --
/// Graphviz's `doublecircle` and the double-ruled boxes BPMN and ER diagrams use are two
/// concentric outlines around one node, and reporting the ring and the disc separately
/// both invents a node and gives the connectors two things to land on.
fn select_candidate_outlines(
    outlines: Vec<Outline>,
    canvas_area: f32,
    min_side: f32,
) -> Option<(Vec<Outline>, Vec<Outline>)> {
    let (mut kept, decoration): (Vec<Outline>, Vec<Outline>) = outlines
        .into_iter()
        .take(MAX_OUTLINES)
        .filter(|o| o.bbox.area() < canvas_area * BACKGROUND_AREA_RATIO)
        .partition(|o| o.bbox.width() >= min_side && o.bbox.height() >= min_side);

    kept.sort_by(|a, b| {
        a.bbox
            .y0
            .total_cmp(&b.bbox.y0)
            .then(a.bbox.x0.total_cmp(&b.bbox.x0))
            .then(a.bbox.y1.total_cmp(&b.bbox.y1))
            .then(a.bbox.x1.total_cmp(&b.bbox.x1))
    });
    kept.dedup_by(|a, b| a.bbox == b.bbox);
    collapse_concentric(&mut kept);

    if kept.is_empty() {
        return None;
    }
    Some((kept, decoration))
}

/// Scale-dependent thresholds derived once from the canvas size and threaded through
/// the rest of [`assemble`], or `None` when the canvas has no valid size to derive them
/// from.
struct Scale {
    area: f32,
    max: f32,
    min_node_side: f32,
    snap: f32,
    edge_label_reach: f32,
}

fn compute_scale(canvas: (f32, f32)) -> Option<Scale> {
    let (canvas_w, canvas_h) = canvas;
    if !canvas_w.is_finite() || !canvas_h.is_finite() || canvas_w <= 0.0 || canvas_h <= 0.0 {
        return None;
    }
    let max = canvas_w.max(canvas_h);
    let snap = (max * SNAP_RATIO).clamp(RATIO_FLOOR, SNAP_CEILING);
    Some(Scale {
        area: canvas_w * canvas_h,
        max,
        min_node_side: (max * MIN_NODE_SIDE_RATIO).clamp(RATIO_FLOOR, MIN_NODE_SIDE_CEILING),
        snap,
        edge_label_reach: snap * EDGE_LABEL_REACH,
    })
}

/// Cap and order the raw labels, then assign each one to the innermost outline whose
/// bounds contain its anchor point -- a label inside a box that is itself inside a
/// panel names the box.
///
/// Capped for the same reason as outlines and connectors: a source can hand this a text
/// run per glyph rather than per caption, and owner resolution below is `labels x
/// outlines`.
fn assign_label_owners(labels: Vec<Label>, kept: &[Outline]) -> (Vec<Label>, Vec<Option<usize>>) {
    let mut labels: Vec<Label> = labels.into_iter().take(MAX_LABELS).collect();
    labels.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));

    let owners: Vec<Option<usize>> = labels
        .iter()
        .map(|label| {
            kept.iter()
                .enumerate()
                .filter(|(_, o)| o.bbox.contains(label.x, label.y))
                .min_by(|(_, a), (_, b)| a.bbox.area().total_cmp(&b.bbox.area()))
                .map(|(i, _)| i)
        })
        .collect();

    (labels, owners)
}

/// Drop chart chrome (gridlines, axis/frame lines) from the connector candidates and
/// cap their count, before arrowhead detection and every distance test downstream can
/// mistake either for a real connector.
///
/// A plotting library's gridlines are a family of parallel, same-style strokes at a
/// regular interval, each crossing the same span of the plot; its axis and outer frame
/// are drawn as a single stroke lying flush with the shared edge of the shapes it
/// scales, with no peer of its own style to be caught by the family test.
fn filter_diagram_connectors(connectors: Vec<Connector>, kept: &[Outline], snap: f32) -> Vec<Connector> {
    let connectors: Vec<Connector> = connectors.into_iter().take(MAX_CONNECTORS).collect();
    let is_gridline = find_gridlines(&connectors, snap);
    let is_frame = find_frame_lines(kept, &connectors, snap);
    connectors
        .into_iter()
        .zip(is_gridline.into_iter().zip(is_frame))
        .filter_map(|(connector, (gridline, frame_line))| (!gridline && !frame_line).then_some(connector))
        .collect()
}

/// Every arrowhead a connector may have to reach across: outlines large enough to have
/// been mistaken for nodes, plus decoration-sized shapes sitting on a connector
/// endpoint. Only the geometry is needed downstream, so both sources collapse to one
/// list of boxes.
fn compute_arrowhead_reach(
    arrowheads: &[bool],
    kept: &[Outline],
    decoration: &[Outline],
    connectors: &[Connector],
    snap: f32,
) -> Vec<Rect> {
    arrowheads
        .iter()
        .zip(kept)
        .filter(|(is_arrowhead, _)| **is_arrowhead)
        .map(|(_, outline)| outline.bbox)
        .chain(
            decoration
                .iter()
                .filter(|o| {
                    connectors.iter().any(|c| {
                        o.bbox.distance_to(c.start.0, c.start.1) <= snap || o.bbox.distance_to(c.end.0, c.end.1) <= snap
                    })
                })
                .map(|o| o.bbox),
        )
        .collect()
}

/// Group labels by the outline that owns them, producing one list of labels per entry
/// in `kept`.
///
/// Used twice with different `owners` snapshots: once before caption adoption to judge
/// interior layout only (a grid is judged on the text drawn inside the shape, never on
/// a caption adopted from outside it -- three lines of one wrapped caption are rows, and
/// their anchors differ enough in x to read as columns too, which would condemn the
/// shape as a table), and once after, for the final per-node caption set.
fn labels_owned_per_shape<'a>(kept: &[Outline], labels: &'a [Label], owners: &[Option<usize>]) -> Vec<Vec<&'a Label>> {
    let mut grouped: Vec<Vec<&Label>> = vec![Vec::new(); kept.len()];
    for (label, owner) in labels.iter().zip(owners) {
        if let Some(index) = owner {
            grouped[*index].push(label);
        }
    }
    grouped
}

/// Which connector endpoints land on each shape, with the arrowhead reach the edge
/// builder uses. Computed once: the container test asks it of every enclosed shape, and
/// recomputing it per enclosure-member pair would make classification quadratic in
/// outlines and linear in connectors at once.
fn compute_landings(kept: &[Outline], connectors: &[Connector], reach: &[Rect], snap: f32) -> Vec<Vec<(f32, f32)>> {
    let endpoints: Vec<((f32, f32), f32)> = connectors
        .iter()
        .flat_map(|c| {
            [
                (c.start, snap + touching_arrowhead(reach, c.start, snap).unwrap_or(0.0)),
                (c.end, snap + touching_arrowhead(reach, c.end, snap).unwrap_or(0.0)),
            ]
        })
        .collect();
    kept.iter()
        .map(|outline| {
            endpoints
                .iter()
                .filter(|(point, tolerance)| outline.bbox.distance_to(point.0, point.1) <= *tolerance)
                .map(|(point, _)| *point)
                .collect()
        })
        .collect()
}

/// A run of anonymous shapes lying on top of each other is one drawing rather than
/// several nodes: the wedges of a pie all meet at its centre, and the parts of an icon
/// glyph all sit inside its footprint, where a layout engine keeps real nodes apart.
///
/// Enclosure is excluded, because it is the one way two shapes can overlap completely
/// and still be two things: a container holds its members, and a member sitting wholly
/// inside its cluster overlaps it by the member's whole area. Counting that would
/// condemn every unlabelled node in a cluster, which is the opposite of what grouping
/// means. Partial overlap, which is what a pie or a glyph draws, still condemns.
///
/// Named nodes are nodes whatever their geometry does, so this test (like
/// [`compute_decoration_cutoff`]) only ever condemns a shape nobody labelled.
fn compute_overlapping(kept: &[Outline]) -> Vec<bool> {
    kept.iter()
        .enumerate()
        .map(|(i, outline)| {
            kept.iter().enumerate().any(|(j, other)| {
                i != j
                    && !other.bbox.encloses(&outline.bbox)
                    && !outline.bbox.encloses(&other.bbox)
                    && outline.bbox.overlap_area(&other.bbox)
                        > outline.bbox.area().min(other.bbox.area()) * OVERLAP_RATIO
            })
        })
        .collect()
}

/// Size cutoff below which an anonymous shape is decoration (a PlantUML terminal dot, a
/// leader bullet) rather than a node: an anonymous shape much smaller than the shapes
/// that are labelled. Size carries this rather than the missing label alone, because a
/// shape the same size as its labelled neighbours is a node whether or not anyone named
/// it.
fn compute_decoration_cutoff(
    kept: &[Outline],
    arrowheads: &[bool],
    grids: &[bool],
    owned: &[Vec<&Label>],
) -> Option<f32> {
    median_area(
        kept.iter()
            .enumerate()
            .filter(|(i, _)| !arrowheads[*i] && !grids[*i] && !owned[*i].is_empty())
            .map(|(_, o)| o.bbox.area()),
    )
    .map(|median| median * UNLABELLED_DECORATION_RATIO)
}

/// A renderer that writes a label onto an edge puts an opaque box behind the text so
/// the line does not run through it. The box is the text's own background rather than a
/// shape in the drawing, and it only reaches here from a PDF writer: mermaid states the
/// same background as a CSS fill on an HTML label, which the SVG carries as no path at
/// all and a PDF writer turns into a real filled rectangle.
///
/// Two things together tell it from a node, and one alone would not. It is far smaller
/// than the shapes that are nodes, which is the same measure decoration is judged by.
/// And every word written in it sits on a connector, which is where an edge label goes
/// and where a node's caption does not.
fn compute_label_backgrounds(
    kept: &[Outline],
    owned: &[Vec<&Label>],
    decoration_cutoff: Option<f32>,
    connectors: &[Connector],
    edge_label_reach: f32,
) -> Vec<bool> {
    kept.iter()
        .enumerate()
        .map(|(i, outline)| {
            !owned[i].is_empty()
                && decoration_cutoff.is_some_and(|cutoff| outline.bbox.area() < cutoff)
                && owned[i].iter().all(|label| {
                    connectors
                        .iter()
                        .any(|c| (label.x - c.midpoint.0).hypot(label.y - c.midpoint.1) <= edge_label_reach)
                })
        })
        .collect()
}

/// Per-shape classification signals [`select_node_indices`] combines to decide which
/// candidate outlines are real nodes. Bundled into one struct purely to keep that
/// function's parameter list manageable -- each field is otherwise independent.
struct NodeSignals<'a> {
    arrowheads: &'a [bool],
    grids: &'a [bool],
    is_container: &'a [bool],
    owned: &'a [Vec<&'a Label>],
    overlapping: &'a [bool],
    decoration_cutoff: Option<f32>,
    label_backgrounds: &'a [bool],
}

/// Arrowheads are geometry belonging to the connector that ends in them, not shapes in
/// their own right, so they never become nodes -- nor do grids, containers, anonymous
/// decoration, or edge-label backgrounds. `None` when nothing survived selection.
fn select_node_indices(kept: &[Outline], signals: &NodeSignals) -> Option<Vec<usize>> {
    let node_indices: Vec<usize> = (0..kept.len())
        .filter(|i| {
            let anonymous_decoration = signals.owned[*i].is_empty()
                && (signals.overlapping[*i]
                    || signals
                        .decoration_cutoff
                        .is_some_and(|cutoff| kept[*i].bbox.area() < cutoff));
            !signals.arrowheads[*i]
                && !signals.grids[*i]
                && !signals.is_container[*i]
                && !anonymous_decoration
                && !signals.label_backgrounds[*i]
        })
        .collect();
    (!node_indices.is_empty()).then_some(node_indices)
}

/// Map `kept` outline indices to their post-selection node index, or `usize::MAX` for
/// an outline that was not selected as a node.
fn map_outlines_to_nodes(kept: &[Outline], node_indices: &[usize]) -> Vec<usize> {
    let mut to_node = vec![usize::MAX; kept.len()];
    for (new, old) in node_indices.iter().enumerate() {
        to_node[*old] = new;
    }
    to_node
}

/// Build the output `DiagramNode` list from the outlines that survived node selection.
/// Captions are filled in afterwards by [`distribute_free_labels`].
fn build_diagram_nodes(node_outlines: &[&Outline]) -> Vec<DiagramNode> {
    node_outlines
        .iter()
        .enumerate()
        .map(|(i, o)| DiagramNode {
            id: format!("n{i}"),
            label: String::new(),
            shape: o.shape,
            fill: o.fill.clone(),
            stroke: o.stroke.clone(),
            stroke_width: o.stroke_width,
            dashed: o.dashed,
        })
        .collect()
}

/// Fill in each node's caption from the labels it owns, and return the labels that
/// belong to no node (free labels, matched to the nearest edge by
/// [`build_diagram_edges`]).
fn distribute_free_labels<'a>(
    labels: &'a [Label],
    owners: &[Option<usize>],
    to_node: &[usize],
    nodes: &mut [DiagramNode],
) -> Vec<&'a Label> {
    let mut free_labels: Vec<&Label> = Vec::new();
    for (label, owner) in labels.iter().zip(owners) {
        match owner.map(|old| to_node[old]).filter(|new| *new != usize::MAX) {
            Some(i) => {
                let node_label = &mut nodes[i].label;
                if !node_label.is_empty() {
                    node_label.push('\n');
                }
                node_label.push_str(&label.text);
            }
            None => free_labels.push(label),
        }
    }
    free_labels
}

/// Build the output edge list from filtered connectors, matching each endpoint to a
/// node outline (across any arrowhead reach) and resolving direction from whichever end
/// carries the arrowhead. `None` when no connector resolved to an edge.
///
/// Shapes no connector reached are kept regardless: an org chart that draws leaf
/// departments without lines down to them still has those departments in it, and
/// dropping them would silently lose content the source states.
fn build_diagram_edges(
    connectors: Vec<Connector>,
    node_outlines: &[&Outline],
    reach: &[Rect],
    free_labels: &[&Label],
    snap: f32,
    edge_label_reach: f32,
) -> Option<Vec<DiagramEdge>> {
    let mut edges: Vec<DiagramEdge> = Vec::new();
    for connector in connectors {
        // An arrowhead sits between the connector's endpoint and the shape it
        // points at, so the endpoint has to reach across it. Widening the
        // tolerance by the arrowhead's own size does that without inventing a
        // coordinate the source never drew.
        let head_at_start = touching_arrowhead(reach, connector.start, snap);
        let head_at_end = touching_arrowhead(reach, connector.end, snap);
        let (Some(from), Some(to)) = (
            snap_to_outline(node_outlines, connector.start, snap + head_at_start.unwrap_or(0.0)),
            snap_to_outline(node_outlines, connector.end, snap + head_at_end.unwrap_or(0.0)),
        ) else {
            continue;
        };
        // A connector returning to the shape it left is a self-loop, but only
        // if it actually went somewhere: a real one arcs clear of the node so
        // it can be seen, while a stroke lying entirely within a shape is
        // interior detail, a divider or a glyph, and joins nothing. ~keep
        if from == to
            && node_outlines[from]
                .bbox
                .contains(connector.midpoint.0, connector.midpoint.1)
        {
            continue;
        }

        // The arrowhead is the direction. Absent one, the path's own point
        // order stands in, which is what the source author drew first.
        let (from, to, bidirectional) = match (head_at_start.is_some(), head_at_end.is_some()) {
            (true, false) => (to, from, false),
            (true, true) => (from, to, true),
            _ => (from, to, false),
        };

        edges.push(DiagramEdge {
            from,
            to,
            bidirectional,
            label: nearest_free_label(free_labels, connector.midpoint, edge_label_reach),
            stroke: connector.stroke,
            dashed: connector.dashed,
        });
    }

    if edges.is_empty() {
        return None;
    }
    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    edges.dedup_by(|a, b| a.from == b.from && a.to == b.to && a.label == b.label);
    Some(edges)
}

/// Build a graph from recovered geometry, or `None` when the geometry is not a
/// graph.
///
/// A vector drawing is not necessarily a diagram. Bar charts, logos and
/// illustrations all yield closed outlines, and reporting those as an
/// edgeless node list would be noise dressed up as structure. Recovery
/// therefore requires at least one connector that resolves to a pair of
/// distinct shapes.
///
/// The result is deterministic. Nodes are ordered top to bottom then left to
/// right, edges by their endpoints, and duplicates of both are collapsed.
pub(crate) fn assemble(
    name: Option<String>,
    canvas: (f32, f32),
    outlines: Vec<Outline>,
    connectors: Vec<Connector>,
    labels: Vec<Label>,
) -> Option<DiagramGraph> {
    let scale = compute_scale(canvas)?;
    let (kept, decoration) = select_candidate_outlines(outlines, scale.area, scale.min_node_side)?;
    let (labels, mut owners) = assign_label_owners(labels, &kept);

    let connectors = filter_diagram_connectors(connectors, &kept, scale.snap);
    let arrowheads = find_arrowheads(&kept, &connectors, &owners, scale.snap, scale.max);
    let reach = compute_arrowhead_reach(&arrowheads, &kept, &decoration, &connectors, scale.snap);

    let contained = labels_owned_per_shape(&kept, &labels, &owners);
    let grids: Vec<bool> = kept
        .iter()
        .zip(&contained)
        .map(|(outline, texts)| holds_a_grid(&outline.bbox, texts))
        .collect();

    let landings = compute_landings(&kept, &connectors, &reach, scale.snap);

    // An icon node has no outline to put its caption inside, so the caption
    // sits underneath the glyph. Adopting it here, after arrowheads are known,
    // keeps a head from acquiring a name it never had.
    adopt_captions(&kept, &labels, &arrowheads, &connectors, &mut owners, scale.snap);

    // The labels each candidate owns, which is what separates a node from the
    // two things that look most like one: a table and a piece of decoration.
    let owned = labels_owned_per_shape(&kept, &labels, &owners);

    // Containers can only be told apart from real nodes once labels and
    // arrowheads are both known: a container is an enclosure nobody labelled,
    // and its enclosed members exclude arrowheads, which sit inside a node's
    // own border without making it one.
    let is_container = find_containers(&kept, &arrowheads, &owned, &landings, scale.snap);

    let overlapping = compute_overlapping(&kept);
    let decoration_cutoff = compute_decoration_cutoff(&kept, &arrowheads, &grids, &owned);
    let label_backgrounds =
        compute_label_backgrounds(&kept, &owned, decoration_cutoff, &connectors, scale.edge_label_reach);

    let node_indices = select_node_indices(
        &kept,
        &NodeSignals {
            arrowheads: &arrowheads,
            grids: &grids,
            is_container: &is_container,
            owned: &owned,
            overlapping: &overlapping,
            decoration_cutoff,
            label_backgrounds: &label_backgrounds,
        },
    )?;
    let to_node = map_outlines_to_nodes(&kept, &node_indices);
    let node_outlines: Vec<&Outline> = node_indices.iter().map(|i| &kept[*i]).collect();

    let mut nodes = build_diagram_nodes(&node_outlines);
    let free_labels = distribute_free_labels(&labels, &owners, &to_node, &mut nodes);
    let edges = build_diagram_edges(
        connectors,
        &node_outlines,
        &reach,
        &free_labels,
        scale.snap,
        scale.edge_label_reach,
    )?;

    Some(DiagramGraph { name, nodes, edges })
}

mod matching;
use matching::*;

#[cfg(test)]
mod tests;
