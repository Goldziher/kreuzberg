use super::*;

#[test]
fn test_path_content_creation() {
    let path = PathContent::new(Rect::new(0.0, 0.0, 100.0, 100.0))
        .with_stroke(Color::black())
        .with_stroke_width(2.0);

    assert!(path.has_stroke());
    assert!(!path.has_fill());
    assert_eq!(path.stroke_width, 2.0);
}

#[test]
fn test_path_from_operations() {
    let ops = vec![
        PathOperation::MoveTo(10.0, 10.0),
        PathOperation::LineTo(50.0, 10.0),
        PathOperation::LineTo(50.0, 50.0),
        PathOperation::LineTo(10.0, 50.0),
        PathOperation::ClosePath,
    ];

    let path = PathContent::from_operations(ops);

    assert_eq!(path.bbox.x, 10.0);
    assert_eq!(path.bbox.y, 10.0);
    assert_eq!(path.bbox.width, 40.0);
    assert_eq!(path.bbox.height, 40.0);
}

#[test]
fn test_path_with_fill() {
    let path = PathContent::new(Rect::new(0.0, 0.0, 100.0, 100.0)).with_fill(Color::new(1.0, 0.0, 0.0));

    assert!(path.has_fill());
    assert!(path.has_stroke());
}

#[test]
fn test_compute_bbox_from_rectangle() {
    let ops = vec![PathOperation::Rectangle(20.0, 30.0, 100.0, 50.0)];
    let path = PathContent::from_operations(ops);

    assert_eq!(path.bbox.x, 20.0);
    assert_eq!(path.bbox.y, 30.0);
    assert_eq!(path.bbox.width, 100.0);
    assert_eq!(path.bbox.height, 50.0);
}

#[test]
fn is_page_frame_rectangle_excludes_a4_and_letter_page_backgrounds() {
    // GH#1656: a full-page background rect must be recognized as page
    // furniture against both an A4 and a US-Letter MediaBox. ~keep
    let a4_media_box = (0.0, 0.0, 595.28, 842.0);
    let a4_frame = PathContent::rect(0.0, 0.0, 595.28, 842.0);
    assert!(a4_frame.is_page_frame_rectangle(a4_media_box));

    let letter_media_box = (0.0, 0.0, 612.0, 792.0);
    let letter_frame = PathContent::rect(0.0, 0.0, 612.0, 792.0);
    assert!(letter_frame.is_page_frame_rectangle(letter_media_box));
}

#[test]
fn is_page_frame_rectangle_admits_a_genuine_table_cell_sized_box() {
    // A table cell box is nowhere near 90% of the page in either
    // dimension and must not be treated as furniture. ~keep
    let media_box = (0.0, 0.0, 595.28, 842.0);
    let cell = PathContent::rect(45.36, 472.70, 138.24, 34.02);
    assert!(!cell.is_page_frame_rectangle(media_box));
}

#[test]
fn is_page_frame_rectangle_rejects_degenerate_media_box() {
    // A missing/zero/non-finite MediaBox must not be treated as
    // license to filter — leave the primitive alone instead. ~keep
    let page_sized = PathContent::rect(0.0, 0.0, 595.28, 842.0);
    assert!(!page_sized.is_page_frame_rectangle((0.0, 0.0, 0.0, 0.0)));
    assert!(!page_sized.is_page_frame_rectangle((0.0, 0.0, f32::NAN, 842.0)));
    assert!(!page_sized.is_page_frame_rectangle((0.0, 0.0, f32::INFINITY, 842.0)));
}

#[test]
fn is_page_frame_rectangle_ignores_non_rectangle_paths() {
    // A full-page-width line is not a rectangle at all and must never
    // be classified as page-frame furniture, regardless of extent. ~keep
    let media_box = (0.0, 0.0, 595.28, 842.0);
    let full_width_line = PathContent::line(0.0, 48.30, 595.28, 48.30);
    assert!(!full_width_line.is_page_frame_rectangle(media_box));
}

#[test]
fn footer_rule_full_page_width_stays_a_table_primitive() {
    // Proves the page-frame filter does not over-filter: a thin
    // horizontal rule spanning the full page width is real ruling, not
    // furniture, because only ONE dimension (width) reaches page scale
    // — is_page_frame_rectangle requires both. GH#1656. ~keep
    let footer_rule = PathContent::line(0.0, 48.30, 595.28, 48.30);
    assert!(footer_rule.is_table_primitive());
    assert!(!footer_rule.is_page_frame_rectangle((0.0, 0.0, 595.28, 842.0)));
}

#[test]
fn trailing_move_to_does_not_extend_the_bounding_box_to_the_page() {
    // GH#1759 carrier: `0 42.63 595.28 -28.35 re  0 842 m  f*`. The
    // lone trailing `m` starts a subpath no segment follows, so it
    // paints nothing and must not stretch the box to the page. ~keep
    let path = PathContent::from_operations(vec![
        PathOperation::Rectangle(0.0, 14.28, 595.28, 28.35),
        PathOperation::MoveTo(0.0, 842.0),
    ]);

    assert!(
        (path.bbox.y - 14.28).abs() < 1e-3,
        "expected the footer band's own y (14.28), got {}",
        path.bbox.y
    );
    assert!(
        (path.bbox.height - 28.35).abs() < 1e-3,
        "expected the footer band's own height (28.35), got {}",
        path.bbox.height
    );
    assert!((path.bbox.x - 0.0).abs() < 1e-3, "got x {}", path.bbox.x);
    assert!((path.bbox.width - 595.28).abs() < 1e-3, "got width {}", path.bbox.width);
}

#[test]
fn negative_extent_rectangle_is_normalised_when_mixed_with_a_segment() {
    // `PathExtractor::rectangle` stores the CTM-transformed delta, so a
    // flipping CTM or a negative-extent `re` yields a negative stored
    // width/height. Mixed with another operation the box is built from
    // min/max across operations, so the rectangle's own corners must be
    // ordered first. GH#1759. ~keep
    let path = PathContent::from_operations(vec![
        PathOperation::Rectangle(100.0, 200.0, -40.0, -30.0),
        PathOperation::LineTo(80.0, 180.0),
    ]);

    assert_eq!(path.bbox.x, 60.0, "expected the normalised left edge");
    assert_eq!(path.bbox.y, 170.0, "expected the normalised bottom edge");
    assert_eq!(path.bbox.width, 40.0, "expected the normalised width");
    assert_eq!(path.bbox.height, 30.0, "expected the normalised height");
}

#[test]
fn move_to_followed_by_a_segment_still_contributes_both_points() {
    // Control for GH#1759: a `MoveTo` that begins a real subpath is
    // painted and must keep contributing its point. ~keep
    let lined = PathContent::from_operations(vec![
        PathOperation::MoveTo(10.0, 20.0),
        PathOperation::LineTo(110.0, 120.0),
    ]);
    assert_eq!(lined.bbox.x, 10.0);
    assert_eq!(lined.bbox.y, 20.0);
    assert_eq!(lined.bbox.width, 100.0);
    assert_eq!(lined.bbox.height, 100.0);

    let curved = PathContent::from_operations(vec![
        PathOperation::MoveTo(10.0, 20.0),
        PathOperation::CurveTo(30.0, 40.0, 50.0, 60.0, 70.0, 80.0),
    ]);
    assert_eq!(curved.bbox.x, 10.0);
    assert_eq!(curved.bbox.y, 20.0);

    let closed = PathContent::from_operations(vec![
        PathOperation::MoveTo(10.0, 20.0),
        PathOperation::LineTo(110.0, 20.0),
        PathOperation::MoveTo(10.0, 200.0),
        PathOperation::ClosePath,
    ]);
    assert_eq!(closed.bbox.y, 20.0);
    assert_eq!(closed.bbox.height, 180.0);
}

/// Ground-truth cubic Bézier evaluation, used to validate flattening.
fn cubic_at(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), t: f32) -> (f32, f32) {
    let mt = 1.0 - t;
    let x = mt * mt * mt * p0.0 + 3.0 * mt * mt * t * p1.0 + 3.0 * mt * t * t * p2.0 + t * t * t * p3.0;
    let y = mt * mt * mt * p0.1 + 3.0 * mt * mt * t * p1.1 + 3.0 * mt * t * t * p2.1 + t * t * t * p3.1;
    (x, y)
}

/// Shortest distance from a point to a polyline (min over its segments),
/// reusing the production segment-distance helper.
fn dist_to_polyline(p: (f32, f32), poly: &[(f32, f32)]) -> f32 {
    poly.windows(2)
        .map(|s| dist_point_to_segment(p, s[0], s[1]))
        .fold(f32::MAX, f32::min)
}

#[test]
fn test_to_points_straight_line_passthrough() {
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(10.0, 10.0),
        PathOperation::LineTo(100.0, 40.0),
    ]);
    let pts = path.to_points(0.5);
    assert_eq!(pts, vec![vec![(10.0, 10.0), (100.0, 40.0)]]);
}

#[test]
fn test_to_points_curve_endpoints_preserved() {
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(0.0, 0.0),
        PathOperation::CurveTo(0.0, 100.0, 100.0, 100.0, 100.0, 0.0),
    ]);
    let sub = &path.to_points(0.1)[0];
    assert_eq!(sub.first().copied(), Some((0.0, 0.0)), "must start at P0");
    let last = sub.last().copied().unwrap();
    assert!(
        (last.0 - 100.0).abs() < 1e-3 && (last.1 - 0.0).abs() < 1e-3,
        "must end at P3, got {last:?}"
    );
}

#[test]
fn test_to_points_curve_within_tolerance() {
    let p0 = (0.0, 0.0);
    let p1 = (0.0, 100.0);
    let p2 = (100.0, 100.0);
    let p3 = (100.0, 0.0);
    let tol = 0.5;
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(p0.0, p0.1),
        PathOperation::CurveTo(p1.0, p1.1, p2.0, p2.1, p3.0, p3.1),
    ]);
    let poly = &path.to_points(tol)[0];
    // Every point on the true curve must lie within `tol` of the polyline. ~keep
    for i in 0..=200 {
        let t = i as f32 / 200.0;
        let truth = cubic_at(p0, p1, p2, p3, t);
        let d = dist_to_polyline(truth, poly);
        assert!(d <= tol + 1e-3, "curve point at t={t} is {d} from polyline (tol={tol})");
    }
}

#[test]
fn test_to_points_tolerance_monotonic() {
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(0.0, 0.0),
        PathOperation::CurveTo(0.0, 100.0, 100.0, 100.0, 100.0, 0.0),
    ]);
    let coarse = path.to_points(10.0)[0].len();
    let fine = path.to_points(0.05)[0].len();
    assert!(
        fine >= coarse,
        "finer tolerance must not reduce point count ({fine} < {coarse})"
    );
    assert!(fine > 2, "fine flattening must densify the curve, got {fine}");
}

#[test]
fn test_to_points_rectangle_is_closed_subpath() {
    let path = PathContent::from_operations(vec![PathOperation::Rectangle(10.0, 20.0, 30.0, 40.0)]);
    let pts = path.to_points(1.0);
    assert_eq!(
        pts,
        vec![vec![
            (10.0, 20.0),
            (40.0, 20.0),
            (40.0, 60.0),
            (10.0, 60.0),
            (10.0, 20.0)
        ]]
    );
}

#[test]
fn test_to_points_segment_after_rectangle_continues_from_current_point() {
    // Per PDF §8.5.2 the current point after `re` is the rectangle's
    // lower-left; a following segment continues from there as a new subpath. ~keep
    let path = PathContent::from_operations(vec![
        PathOperation::Rectangle(0.0, 0.0, 10.0, 10.0),
        PathOperation::LineTo(20.0, 20.0),
    ]);
    let pts = path.to_points(1.0);
    assert_eq!(pts.len(), 2);
    assert_eq!(
        pts[0],
        vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0), (0.0, 0.0)]
    );
    assert_eq!(pts[1], vec![(0.0, 0.0), (20.0, 20.0)]);
}

#[test]
fn test_to_points_closepath_appends_start() {
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(0.0, 0.0),
        PathOperation::LineTo(10.0, 0.0),
        PathOperation::LineTo(10.0, 10.0),
        PathOperation::ClosePath,
    ]);
    assert_eq!(
        path.to_points(1.0),
        vec![vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 0.0)]]
    );
}

#[test]
fn test_to_points_closepath_terminates_subpath() {
    // Per §8.5.2 Table 59, `h` terminates the subpath; a following segment
    // begins a NEW subpath (seeded from the subpath start = current point),
    // rather than extending the closed loop. ~keep
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(0.0, 0.0),
        PathOperation::LineTo(10.0, 0.0),
        PathOperation::ClosePath,
        PathOperation::LineTo(5.0, 5.0),
    ]);
    let pts = path.to_points(1.0);
    assert_eq!(pts.len(), 2, "h must terminate the subpath");
    assert_eq!(pts[0], vec![(0.0, 0.0), (10.0, 0.0), (0.0, 0.0)]);
    assert_eq!(pts[1], vec![(0.0, 0.0), (5.0, 5.0)]);
}

#[test]
fn test_to_points_consecutive_moveto_leaves_no_vestige() {
    // Per §8.5.2 Table 59, a consecutive `m` overrides the previous one with
    // no vestige; the orphaned start point must not appear as a subpath. ~keep
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(1.0, 1.0),
        PathOperation::MoveTo(2.0, 2.0),
        PathOperation::LineTo(3.0, 3.0),
    ]);
    assert_eq!(path.to_points(1.0), vec![vec![(2.0, 2.0), (3.0, 3.0)]]);
}

#[test]
fn test_to_points_lone_moveto_is_dropped() {
    // A subpath that adds no segment paints nothing and yields no polyline. ~keep
    let path = PathContent::from_operations(vec![PathOperation::MoveTo(5.0, 5.0)]);
    assert!(path.to_points(1.0).is_empty());
}

#[test]
fn test_to_points_multiple_subpaths() {
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(0.0, 0.0),
        PathOperation::LineTo(1.0, 0.0),
        PathOperation::MoveTo(5.0, 5.0),
        PathOperation::LineTo(6.0, 5.0),
    ]);
    assert_eq!(
        path.to_points(1.0),
        vec![vec![(0.0, 0.0), (1.0, 0.0)], vec![(5.0, 5.0), (6.0, 5.0)]]
    );
}

#[test]
fn test_to_points_empty() {
    let path = PathContent::from_operations(vec![]);
    assert!(path.to_points(1.0).is_empty());
}

#[test]
fn test_to_points_nonpositive_tolerance_terminates() {
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(0.0, 0.0),
        PathOperation::CurveTo(0.0, 100.0, 100.0, 100.0, 100.0, 0.0),
    ]);
    for tol in [0.0, -1.0, f32::NAN] {
        let pts = path.to_points(tol);
        assert_eq!(pts.len(), 1);
        let n = pts[0].len();
        assert!(
            (2..100_000).contains(&n),
            "tol={tol} produced {n} points (expected bounded)"
        );
        assert!(pts[0].iter().all(|(x, y)| x.is_finite() && y.is_finite()));
    }
}

#[test]
fn test_to_points_curve_starts_at_current_point() {
    // The curve's implicit P0 is the current point (here the LineTo endpoint),
    // not the path origin. Guards against using the wrong start point. ~keep
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(0.0, 0.0),
        PathOperation::LineTo(50.0, 0.0),
        PathOperation::CurveTo(60.0, 20.0, 70.0, 20.0, 80.0, 0.0),
    ]);
    let sub = &path.to_points(0.5)[0];
    assert_eq!(sub[0], (0.0, 0.0));
    assert_eq!(sub[1], (50.0, 0.0));
    // First flattened curve vertex departs from the LineTo endpoint (50,0),
    // so it must sit to its right (x > 50), never back near the origin. ~keep
    assert!(
        sub[2].0 > 50.0,
        "curve did not start at current point; sub[2]={:?}",
        sub[2]
    );
    assert!((sub.last().unwrap().0 - 80.0).abs() < 1e-3);
}

#[test]
fn test_to_points_degenerate_curve() {
    // All control points coincide: a zero-length curve must not subdivide forever. ~keep
    let path = PathContent::from_operations(vec![
        PathOperation::MoveTo(5.0, 5.0),
        PathOperation::CurveTo(5.0, 5.0, 5.0, 5.0, 5.0, 5.0),
    ]);
    let sub = &path.to_points(0.1)[0];
    assert!(sub.len() < 10, "degenerate curve over-subdivided: {} points", sub.len());
    assert!(sub.iter().all(|&p| p == (5.0, 5.0)));
}
