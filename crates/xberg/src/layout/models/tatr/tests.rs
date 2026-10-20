use super::*;

#[test]
fn test_compute_detr_resize_landscape() {
    let (w, h) = compute_detr_resize(1600, 1200);
    assert_eq!((w, h), (1000, 750));
}

#[test]
fn test_compute_detr_resize_portrait() {
    let (w, h) = compute_detr_resize(600, 1000);
    assert_eq!((w, h), (600, 1000));
}

#[test]
fn test_compute_detr_resize_very_elongated() {
    let (w, h) = compute_detr_resize(100, 3000);
    assert_eq!((w, h), (33, 1000));
}

#[test]
fn test_compute_detr_resize_square() {
    let (w, h) = compute_detr_resize(800, 800);
    assert_eq!(w, 800);
    assert_eq!(h, 800);
}

#[test]
fn test_compute_detr_resize_truncates_like_hugging_face() {
    assert_eq!(compute_detr_resize(102, 101), (807, 800));
    assert_eq!(compute_detr_resize(6, 17), (353, 1000));
}

#[test]
fn test_compute_detr_resize_small() {
    let (w, h) = compute_detr_resize(200, 300);
    assert_eq!((w, h), (666, 1000));
}

#[test]
fn test_cxcywh_to_xyxy_center() {
    let bbox = cxcywh_to_xyxy(0.5, 0.5, 0.5, 0.5, 100.0, 100.0);
    assert!((bbox[0] - 25.0).abs() < 1e-5, "x1={}", bbox[0]);
    assert!((bbox[1] - 25.0).abs() < 1e-5, "y1={}", bbox[1]);
    assert!((bbox[2] - 75.0).abs() < 1e-5, "x2={}", bbox[2]);
    assert!((bbox[3] - 75.0).abs() < 1e-5, "y2={}", bbox[3]);
}

#[test]
fn test_cxcywh_to_xyxy_top_left() {
    let bbox = cxcywh_to_xyxy(0.5, 0.5, 1.0, 1.0, 200.0, 100.0);
    assert!((bbox[0] - 0.0).abs() < 1e-5);
    assert!((bbox[1] - 0.0).abs() < 1e-5);
    assert!((bbox[2] - 200.0).abs() < 1e-5);
    assert!((bbox[3] - 100.0).abs() < 1e-5);
}

#[test]
fn test_cxcywh_to_xyxy_clamps_negative() {
    let bbox = cxcywh_to_xyxy(0.0, 0.0, 0.5, 0.5, 100.0, 100.0);
    assert_eq!(bbox[0], 0.0, "x1 should be clamped to 0");
    assert_eq!(bbox[1], 0.0, "y1 should be clamped to 0");
}

#[test]
fn test_softmax_argmax_clear_winner() {
    let logits = [0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0];
    let (idx, prob) = softmax_argmax(&logits);
    assert_eq!(idx, 2);
    assert!(prob > 0.99, "confidence should be ~1.0, got {prob}");
}

#[test]
fn test_softmax_argmax_uniform() {
    let logits = [1.0; 7];
    let (_, prob) = softmax_argmax(&logits);
    assert!(
        (prob - 1.0 / 7.0).abs() < 1e-5,
        "uniform logits should give ~1/7 confidence, got {prob}"
    );
}

#[test]
fn test_softmax_argmax_negative() {
    let logits = [-10.0, -5.0, -1.0, -20.0, -30.0, -2.0, -100.0];
    let (idx, _) = softmax_argmax(&logits);
    assert_eq!(idx, 2, "should pick the least negative");
}

#[test]
fn test_iob_full_containment() {
    let a = [10.0, 10.0, 20.0, 20.0];
    let b = [0.0, 0.0, 100.0, 100.0];
    let result = iob(a, b);
    assert!((result - 1.0).abs() < 1e-5, "fully contained → IoB=1.0, got {result}");
}

#[test]
fn test_iob_no_overlap() {
    let a = [0.0, 0.0, 10.0, 10.0];
    let b = [20.0, 20.0, 30.0, 30.0];
    let result = iob(a, b);
    assert_eq!(result, 0.0);
}

#[test]
fn test_iob_partial_overlap() {
    let a = [0.0, 0.0, 10.0, 10.0];
    let b = [5.0, 0.0, 15.0, 10.0];
    let result = iob(a, b);
    assert!((result - 0.5).abs() < 1e-5, "expected 0.5, got {result}");
}

#[test]
fn test_iob_zero_area() {
    let a = [5.0, 5.0, 5.0, 5.0];
    let b = [0.0, 0.0, 10.0, 10.0];
    let result = iob(a, b);
    assert_eq!(result, 0.0, "zero-area box should return 0.0");
}

#[test]
fn test_nms_suppresses_overlapping() {
    let detections = vec![
        TatrDetection {
            bbox: [0.0, 0.0, 100.0, 20.0],
            confidence: 0.9,
            class_name: TatrClass::Row,
        },
        TatrDetection {
            bbox: [0.0, 2.0, 100.0, 22.0],
            confidence: 0.7,
            class_name: TatrClass::Row,
        },
    ];
    let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();
    let kept = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_ROWS);
    assert_eq!(kept.len(), 1, "overlapping detection should be suppressed");
    assert_eq!(kept[0], [0.0, 0.0, 100.0, 20.0]);
}

#[test]
fn test_nms_keeps_non_overlapping() {
    let detections = vec![
        TatrDetection {
            bbox: [0.0, 0.0, 100.0, 20.0],
            confidence: 0.9,
            class_name: TatrClass::Row,
        },
        TatrDetection {
            bbox: [0.0, 50.0, 100.0, 70.0],
            confidence: 0.8,
            class_name: TatrClass::Row,
        },
    ];
    let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();
    let kept = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_ROWS);
    assert_eq!(kept.len(), 2, "non-overlapping detections should both be kept");
}

#[test]
fn test_nms_keeps_adjacent_rows_with_minor_overlap() {
    let detections = vec![
        TatrDetection {
            bbox: [0.0, 0.0, 100.0, 20.0],
            confidence: 0.9,
            class_name: TatrClass::Row,
        },
        TatrDetection {
            bbox: [0.0, 18.0, 100.0, 38.0],
            confidence: 0.8,
            class_name: TatrClass::Row,
        },
    ];
    let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();
    let kept = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_ROWS);
    assert_eq!(kept.len(), 2, "adjacent rows with minor overlap should both be kept");
}

#[test]
fn test_build_cell_grid_2x2() {
    let result = TatrResult {
        rows: vec![
            TatrDetection {
                bbox: [0.0, 0.0, 100.0, 20.0],
                confidence: 0.9,
                class_name: TatrClass::Row,
            },
            TatrDetection {
                bbox: [0.0, 20.0, 100.0, 40.0],
                confidence: 0.85,
                class_name: TatrClass::Row,
            },
        ],
        columns: vec![
            TatrDetection {
                bbox: [0.0, 0.0, 50.0, 40.0],
                confidence: 0.9,
                class_name: TatrClass::Column,
            },
            TatrDetection {
                bbox: [50.0, 0.0, 100.0, 40.0],
                confidence: 0.85,
                class_name: TatrClass::Column,
            },
        ],
        headers: Vec::new(),
        spanning: Vec::new(),
        table_bbox: None,
    };

    let grid = build_cell_grid(&result, None);
    assert_eq!(grid.len(), 2, "should have 2 rows");
    assert_eq!(grid[0].len(), 2, "should have 2 columns per row");

    let tl = &grid[0][0];
    assert!((tl.x1 - 0.0).abs() < 1e-5);
    assert!((tl.y1 - 0.0).abs() < 1e-5);
    assert!((tl.x2 - 50.0).abs() < 1e-5);
    assert!((tl.y2 - 20.0).abs() < 1e-5);

    let br = &grid[1][1];
    assert!((br.x1 - 50.0).abs() < 1e-5);
    assert!((br.y1 - 20.0).abs() < 1e-5);
    assert!((br.x2 - 100.0).abs() < 1e-5);
    assert!((br.y2 - 40.0).abs() < 1e-5);
}

#[test]
fn test_build_cell_grid_empty() {
    let result = TatrResult {
        rows: Vec::new(),
        columns: Vec::new(),
        headers: Vec::new(),
        spanning: Vec::new(),
        table_bbox: None,
    };
    let grid = build_cell_grid(&result, None);
    assert!(grid.is_empty());
}

#[test]
fn test_build_cell_grid_with_table_bbox() {
    let result = TatrResult {
        rows: vec![TatrDetection {
            bbox: [10.0, 5.0, 90.0, 25.0],
            confidence: 0.9,
            class_name: TatrClass::Row,
        }],
        columns: vec![TatrDetection {
            bbox: [0.0, 0.0, 50.0, 30.0],
            confidence: 0.9,
            class_name: TatrClass::Column,
        }],
        headers: Vec::new(),
        spanning: Vec::new(),
        table_bbox: None,
    };

    let grid = build_cell_grid(&result, Some([0.0, 0.0, 100.0, 30.0]));
    assert_eq!(grid.len(), 1);
    assert_eq!(grid[0].len(), 1);
    let cell = &grid[0][0];
    assert!((cell.x1 - 0.0).abs() < 1e-5, "x1={}", cell.x1);
    assert!((cell.x2 - 50.0).abs() < 1e-5, "x2={}", cell.x2);
}

#[test]
fn test_tatr_class_from_index() {
    assert_eq!(TatrClass::from_index(0), Some(TatrClass::Table));
    assert_eq!(TatrClass::from_index(1), Some(TatrClass::Column));
    assert_eq!(TatrClass::from_index(2), Some(TatrClass::Row));
    assert_eq!(TatrClass::from_index(3), Some(TatrClass::ColumnHeader));
    assert_eq!(TatrClass::from_index(4), Some(TatrClass::ProjectedRowHeader));
    assert_eq!(TatrClass::from_index(5), Some(TatrClass::SpanningCell));
    assert_eq!(TatrClass::from_index(6), None);
    assert_eq!(TatrClass::from_index(7), None);
}

#[test]
fn test_build_cell_grid_rows_sorted_spatially() {
    let result = TatrResult {
        rows: vec![
            TatrDetection {
                bbox: [0.0, 30.0, 100.0, 50.0],
                confidence: 0.95,
                class_name: TatrClass::Row,
            },
            TatrDetection {
                bbox: [0.0, 0.0, 100.0, 20.0],
                confidence: 0.80,
                class_name: TatrClass::Row,
            },
        ],
        columns: vec![TatrDetection {
            bbox: [0.0, 0.0, 100.0, 50.0],
            confidence: 0.9,
            class_name: TatrClass::Column,
        }],
        headers: Vec::new(),
        spanning: Vec::new(),
        table_bbox: None,
    };

    let grid = build_cell_grid(&result, None);
    assert_eq!(grid.len(), 2, "should have 2 rows");
    assert!(
        grid[0][0].y1 < grid[1][0].y1,
        "grid rows should be sorted top-to-bottom: row0.y1={} row1.y1={}",
        grid[0][0].y1,
        grid[1][0].y1,
    );
}

#[test]
fn test_build_cell_grid_columns_sorted_spatially() {
    let result = TatrResult {
        rows: vec![TatrDetection {
            bbox: [0.0, 0.0, 100.0, 20.0],
            confidence: 0.9,
            class_name: TatrClass::Row,
        }],
        columns: vec![
            TatrDetection {
                bbox: [60.0, 0.0, 100.0, 20.0],
                confidence: 0.95,
                class_name: TatrClass::Column,
            },
            TatrDetection {
                bbox: [0.0, 0.0, 50.0, 20.0],
                confidence: 0.80,
                class_name: TatrClass::Column,
            },
        ],
        headers: Vec::new(),
        spanning: Vec::new(),
        table_bbox: None,
    };

    let grid = build_cell_grid(&result, None);
    assert_eq!(grid[0].len(), 2, "should have 2 columns");
    assert!(
        grid[0][0].x1 < grid[0][1].x1,
        "grid columns should be sorted left-to-right: col0.x1={} col1.x1={}",
        grid[0][0].x1,
        grid[0][1].x1,
    );
}

#[test]
fn test_preprocess_detr_output_shape() {
    let img = RgbImage::new(640, 480);
    let (tensor, rw, rh) = preprocess_detr(&img);
    let shape = tensor.shape();
    assert_eq!(shape[0], 1, "batch dim");
    assert_eq!(shape[1], 3, "channel dim");
    assert_eq!(shape[2], rh as usize, "height dim");
    assert_eq!(shape[3], rw as usize, "width dim");
    assert_eq!(rh, 750);
    assert_eq!(rw, 1000);
}

#[test]
fn test_nms_col_threshold_preserves_narrow_adjacent_columns() {
    let col_width = 20.0;
    let overlap = 7.0;
    let detections = vec![
        TatrDetection {
            bbox: [0.0, 0.0, col_width, 100.0],
            confidence: 0.9,
            class_name: TatrClass::Column,
        },
        TatrDetection {
            bbox: [col_width - overlap, 0.0, 2.0 * col_width - overlap, 100.0],
            confidence: 0.85,
            class_name: TatrClass::Column,
        },
    ];
    let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();

    let kept_row = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_ROWS);
    assert_eq!(kept_row.len(), 2, "row threshold should keep both");

    let kept_col = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_COLS);
    assert_eq!(
        kept_col.len(),
        1,
        "column threshold should suppress heavily overlapping column"
    );
}

#[test]
fn test_nms_col_threshold_keeps_well_separated_columns() {
    let detections = vec![
        TatrDetection {
            bbox: [0.0, 0.0, 20.0, 100.0],
            confidence: 0.9,
            class_name: TatrClass::Column,
        },
        TatrDetection {
            bbox: [17.0, 0.0, 37.0, 100.0],
            confidence: 0.85,
            class_name: TatrClass::Column,
        },
    ];
    let bboxes: Vec<[f32; 4]> = detections.iter().map(|d| d.bbox).collect();

    let kept = nms_by_iob(&detections, &bboxes, NMS_IOB_THRESHOLD_COLS);
    assert_eq!(kept.len(), 2, "well-separated columns should both be kept");
}

#[test]
fn test_min_col_width_filter_removes_noise_columns() {
    let result = TatrResult {
        rows: vec![TatrDetection {
            bbox: [0.0, 0.0, 100.0, 20.0],
            confidence: 0.9,
            class_name: TatrClass::Row,
        }],
        columns: vec![
            TatrDetection {
                bbox: [0.0, 0.0, 50.0, 20.0],
                confidence: 0.9,
                class_name: TatrClass::Column,
            },
            TatrDetection {
                bbox: [60.0, 0.0, 60.5, 20.0],
                confidence: 0.5,
                class_name: TatrClass::Column,
            },
            TatrDetection {
                bbox: [70.0, 0.0, 100.0, 20.0],
                confidence: 0.85,
                class_name: TatrClass::Column,
            },
        ],
        headers: Vec::new(),
        spanning: Vec::new(),
        table_bbox: None,
    };

    let grid = build_cell_grid(&result, Some([0.0, 0.0, 100.0, 20.0]));
    assert_eq!(
        grid[0].len(),
        2,
        "noise column should be filtered, leaving 2 real columns"
    );
}

#[test]
fn test_build_cell_grid_uses_per_class_nms() {
    let result = TatrResult {
        rows: vec![
            TatrDetection {
                bbox: [0.0, 0.0, 100.0, 25.0],
                confidence: 0.9,
                class_name: TatrClass::Row,
            },
            TatrDetection {
                bbox: [0.0, 15.0, 100.0, 40.0],
                confidence: 0.85,
                class_name: TatrClass::Row,
            },
        ],
        columns: vec![TatrDetection {
            bbox: [0.0, 0.0, 50.0, 40.0],
            confidence: 0.9,
            class_name: TatrClass::Column,
        }],
        headers: Vec::new(),
        spanning: Vec::new(),
        table_bbox: None,
    };

    let grid = build_cell_grid(&result, None);
    assert_eq!(
        grid.len(),
        2,
        "rows with 0.4 IoB should both survive row NMS (threshold 0.5)"
    );
}

fn header_detection(bbox: [f32; 4]) -> TatrDetection {
    TatrDetection {
        bbox,
        confidence: 0.9,
        class_name: TatrClass::ColumnHeader,
    }
}

fn spanning_detection(bbox: [f32; 4]) -> TatrDetection {
    TatrDetection {
        bbox,
        confidence: 0.9,
        class_name: TatrClass::SpanningCell,
    }
}

#[test]
fn test_compute_header_row_count_single_header_row() {
    let rows = [[0.0, 0.0, 100.0, 20.0], [0.0, 20.0, 100.0, 40.0]];
    let headers = [header_detection([0.0, 0.0, 100.0, 20.0])];
    assert_eq!(compute_header_row_count(&headers, &rows), 1);
}

#[test]
fn test_compute_header_row_count_multi_row_header() {
    let rows = [
        [0.0, 0.0, 100.0, 20.0],
        [0.0, 20.0, 100.0, 40.0],
        [0.0, 40.0, 100.0, 60.0],
    ];
    let headers = [header_detection([0.0, 0.0, 100.0, 40.0])];
    assert_eq!(
        compute_header_row_count(&headers, &rows),
        2,
        "a header detection spanning two row bands must mark both as header rows"
    );
}

#[test]
fn test_compute_header_row_count_no_headers_returns_zero() {
    let rows = [[0.0, 0.0, 100.0, 20.0]];
    assert_eq!(
        compute_header_row_count(&[], &rows),
        0,
        "absence of TATR header detections must not be conflated with an explicit single header row"
    );
}

#[test]
fn test_compute_spans_merges_two_columns() {
    let rows = [[0.0, 0.0, 100.0, 20.0]];
    let cols = [[0.0, 0.0, 50.0, 20.0], [50.0, 0.0, 100.0, 20.0]];
    let spanning = [spanning_detection([0.0, 0.0, 100.0, 20.0])];
    let spans = compute_spans(&spanning, &rows, &cols);
    assert_eq!(spans, vec![(0, 1, 0, 2)]);
}

#[test]
fn test_compute_spans_skips_single_cell_overlap() {
    let rows = [[0.0, 0.0, 100.0, 20.0]];
    let cols = [[0.0, 0.0, 50.0, 20.0], [50.0, 0.0, 100.0, 20.0]];
    // Covers only the first column: not an actual merge.
    let spanning = [spanning_detection([0.0, 0.0, 50.0, 20.0])];
    let spans = compute_spans(&spanning, &rows, &cols);
    assert!(spans.is_empty(), "a detection covering a single cell is not a span");
}

#[test]
fn test_build_cell_grid_with_structure_reports_header_and_span() {
    let result = TatrResult {
        rows: vec![
            TatrDetection {
                bbox: [0.0, 0.0, 100.0, 20.0],
                confidence: 0.9,
                class_name: TatrClass::Row,
            },
            TatrDetection {
                bbox: [0.0, 20.0, 100.0, 40.0],
                confidence: 0.9,
                class_name: TatrClass::Row,
            },
        ],
        columns: vec![
            TatrDetection {
                bbox: [0.0, 0.0, 50.0, 40.0],
                confidence: 0.9,
                class_name: TatrClass::Column,
            },
            TatrDetection {
                bbox: [50.0, 0.0, 100.0, 40.0],
                confidence: 0.9,
                class_name: TatrClass::Column,
            },
        ],
        headers: vec![header_detection([0.0, 0.0, 100.0, 20.0])],
        spanning: vec![spanning_detection([0.0, 0.0, 100.0, 20.0])],
        table_bbox: Some([0.0, 0.0, 100.0, 40.0]),
    };

    let (grid, structure) = build_cell_grid_with_structure(&result, result.table_bbox);
    assert_eq!(grid.len(), 2, "grid shape must be unaffected by structure metadata");
    assert_eq!(grid[0].len(), 2);
    assert_eq!(structure.header_row_count, 1);
    assert_eq!(structure.spans, vec![(0, 1, 0, 2)]);
}

fn query_outputs(classes: &[(usize, f32)], boxes: &[[f32; 4]]) -> Vec<TatrOutput> {
    let logits = classes
        .iter()
        .flat_map(|&(class, score)| {
            let mut values = [0.0; NUM_CLASSES];
            values[class] = score;
            values
        })
        .collect();
    vec![
        (vec![1, classes.len(), NUM_CLASSES], logits),
        (vec![1, boxes.len(), 4], boxes.iter().flatten().copied().collect()),
    ]
}

#[test]
fn should_filter_background_and_low_confidence_spans_without_losing_rows() {
    let outputs = query_outputs(&[(2, 20.0), (6, 20.0), (5, 1.0), (0, 20.0)], &[[0.5, 0.5, 0.5, 0.5]; 4]);
    let result = decode_tatr_outputs(outputs, (200.0, 100.0), (1000, 500)).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].bbox, [50.0, 25.0, 150.0, 75.0]);
    assert_eq!(result.rows[0].class_name, TatrClass::Row);
    assert_eq!(result.spanning.len(), 0);
    assert_eq!(result.columns.len(), 0);
    assert_eq!(result.headers.len(), 0);
    assert_eq!(result.table_bbox, Some([50.0, 25.0, 150.0, 75.0]));
}

#[test]
fn should_sort_grid_detections_and_keep_the_first_equally_confident_table() {
    let classes = [(2, 20.0), (2, 20.0), (1, 20.0), (1, 20.0), (0, 20.0), (0, 20.0)];
    let boxes = [
        [0.5, 0.75, 1.0, 0.25],
        [0.5, 0.25, 1.0, 0.25],
        [0.75, 0.5, 0.25, 1.0],
        [0.25, 0.5, 0.25, 1.0],
        [0.5, 0.5, 1.0, 1.0],
        [0.5, 0.5, 0.5, 0.5],
    ];
    let result = decode_tatr_outputs(query_outputs(&classes, &boxes), (200.0, 100.0), (1000, 500)).unwrap();
    assert_eq!(result.rows.iter().map(|d| d.bbox[3]).collect::<Vec<_>>(), [37.5, 87.5]);
    assert_eq!(
        result.columns.iter().map(|d| d.bbox[2]).collect::<Vec<_>>(),
        [75.0, 175.0]
    );
    assert_eq!(result.table_bbox, Some([0.0, 0.0, 200.0, 100.0]));
}

#[test]
fn should_accept_boxes_before_logits_and_clip_to_the_original_image() {
    let mut outputs = query_outputs(&[(2, 20.0)], &[[0.5, 0.5, 2.0, 2.0]]);
    outputs.swap(0, 1);
    let result = decode_tatr_outputs(outputs, (200.0, 100.0), (1000, 500)).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].bbox, [0.0, 0.0, 200.0, 100.0]);
}

#[test]
fn should_preserve_missing_output_error_and_empty_shape_results() {
    let error = decode_tatr_outputs(Vec::new(), (200.0, 100.0), (1000, 500)).unwrap_err();
    assert!(
        matches!(error, LayoutError::InvalidOutput(ref message) if message == "TATR expected 2 float outputs, got 0")
    );
    for (logits_shape, boxes_shape) in [(vec![1, 0, 7], vec![1, 0, 4]), (vec![1, 1, 7], vec![1, 1, 3])] {
        let outputs = vec![(logits_shape, Vec::new()), (boxes_shape, Vec::new())];
        let result = decode_tatr_outputs(outputs, (200.0, 100.0), (1000, 500)).unwrap();
        assert_eq!(
            result.rows.len() + result.columns.len() + result.headers.len() + result.spanning.len(),
            0
        );
        assert_eq!(result.table_bbox, None);
    }
}

#[test]
fn should_not_read_box_data_for_background_queries() {
    let mut outputs = query_outputs(&[(6, 20.0)], &[[0.0; 4]]);
    outputs[1].1.clear();
    let result = decode_tatr_outputs(outputs, (200.0, 100.0), (1000, 500)).unwrap();
    assert_eq!(
        result.rows.len() + result.columns.len() + result.headers.len() + result.spanning.len(),
        0
    );
    assert_eq!(result.table_bbox, None);
}
