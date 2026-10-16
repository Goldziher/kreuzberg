use crate::{
    base_net::BaseNet,
    constants::{IMAGENET_MEAN_VALUES, IMAGENET_NORM_VALUES},
    inference::{self, ModelBackend},
    ocr_error::OcrError,
    ocr_result::{self, TextBox},
    ocr_utils::OcrUtils,
    scale_param::ScaleParam,
};
use geo_clipper::{Clipper, EndType, JoinType};
use geo_types::{Coord, LineString, Polygon};
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

const DB_DILATION_KERNEL_SIZE: u32 = 2;
const BINARY_FOREGROUND: u8 = u8::MAX;

/// How many shape-pinned detection plans stay resident at once.
///
/// Each plan owns a full copy of the optimized network, so the cache is deliberately small.
/// A document's pages nearly all resize to the same extent, and both orientations of one page
/// size plus a couple of odd pages still fit; overflow evicts the least recently used plan,
/// costing a rebuild only if that extent comes back.
const DETECTION_PLAN_CACHE_CAPACITY: usize = 4;

/// `(height, width)` of a DBNet input — the key a shape-pinned plan is built for.
type InputShape = (usize, usize);

/// Detection acceptance thresholds, bundled so [`DbNet::get_text_boxes_core`] stays under
/// the workspace parameter-count limit. ~keep
#[derive(Debug, Clone, Copy)]
struct DetectionThresholds {
    box_score_thresh: f32,
    box_thresh: f32,
    un_clip_ratio: f32,
}

/// How a loaded DBNet is executed.
enum Detector {
    /// One plan that accepts any input shape, reused for every page (`ort`).
    ShapeAgnostic(Arc<dyn ModelBackend>),
    /// Plans built per concrete input shape, on demand (`tract`).
    ShapePinned(ShapePinnedPlans),
}

/// Lazily built, shape-keyed DBNet plans for a backend that needs a concrete input shape.
///
/// The model bytes are retained so a plan can be built for whatever extent a page resizes to.
/// Every plan is built for the page's **exact** resized extent — never for a larger canvas the
/// page is padded into, which is what an earlier revision of this file did.
///
/// Padding is not a valid substitute, and not because of border effects. Both PaddleOCR
/// detection backbones are PP-LCNets carrying `GlobalAveragePool` squeeze-and-excitation blocks
/// (10 in PP-OCRv5 `det/mobile`, 8 in PP-OCRv6 `det/tiny`) that reduce over the *whole* spatial
/// extent, so enlarging the input rescales every channel gate and perturbs the probability map
/// across the entire page. Measured on a 791x1024 scan resized to 480x640: padding it into a
/// 640x640 canvas moved the map by up to 0.77 (mean 2.6e-3), spread evenly over the content
/// rather than concentrated at the padding seam, which flipped 827 of 307200 pixels across
/// DBNet's binarization threshold and merged two text lines into one region. Running the same
/// page at 480x640 on both engines instead leaves a max difference of 5.0e-5 (mean 2.4e-7) and
/// identical boxes. No pad value, margin, or edge-replication scheme fixes this — any padding at all
/// changes the global average.
struct ShapePinnedPlans {
    backend: inference::Backend,
    model_bytes: Vec<u8>,
    num_thread: usize,
    /// Most-recently-used first, capped at [`DETECTION_PLAN_CACHE_CAPACITY`].
    plans: Mutex<Vec<(InputShape, Arc<dyn ModelBackend>)>>,
}

impl ShapePinnedPlans {
    /// Retain the bytes for later, failing now if they are not a model this backend can read.
    fn new(backend: inference::Backend, model_bytes: Vec<u8>, num_thread: usize) -> Result<Self, OcrError> {
        inference::validate_model(backend, &model_bytes)?;
        Ok(Self {
            backend,
            model_bytes,
            num_thread,
            plans: Mutex::new(Vec::new()),
        })
    }

    /// The plan for `shape`, built and cached on first use.
    fn plan_for(&self, shape: InputShape) -> Result<Arc<dyn ModelBackend>, OcrError> {
        let mut plans = self
            .plans
            .lock()
            .map_err(|_| OcrError::inference("the DBNet plan cache lock is poisoned"))?;

        if let Some(index) = plans.iter().position(|(cached, _)| *cached == shape) {
            let entry = plans.remove(index);
            let plan = Arc::clone(&entry.1);
            plans.insert(0, entry);
            return Ok(plan);
        }

        // Built while holding the lock on purpose: concurrent pages of one document ask for the
        // same extent, so serializing here builds that plan once instead of once per thread.
        let plan: Arc<dyn ModelBackend> = Arc::from(inference::load_backend(
            self.backend,
            &self.model_bytes,
            self.num_thread,
            Some(&[1, 3, shape.0, shape.1]),
        )?);
        plans.insert(0, (shape, Arc::clone(&plan)));
        plans.truncate(DETECTION_PLAN_CACHE_CAPACITY);
        Ok(plan)
    }

    /// Number of plans currently resident, or `None` while another thread holds the cache.
    fn resident_plan_count(&self) -> Option<usize> {
        self.plans.try_lock().ok().map(|plans| plans.len())
    }
}

pub struct DbNet {
    detector: Option<Detector>,
}

impl std::fmt::Debug for DbNet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (backend, resident_plans) = match &self.detector {
            None => (None, None),
            Some(Detector::ShapeAgnostic(backend)) => (Some(backend.name()), None),
            Some(Detector::ShapePinned(plans)) => (Some(plans.backend.name()), plans.resident_plan_count()),
        };
        f.debug_struct("DbNet")
            .field("initialized", &self.detector.is_some())
            .field("backend", &backend)
            .field("shape_pinned", &matches!(self.detector, Some(Detector::ShapePinned(_))))
            .field("resident_plans", &resident_plans)
            .finish()
    }
}

impl BaseNet for DbNet {
    fn new() -> Self {
        Self { detector: None }
    }

    fn set_backend(&mut self, backend: Option<Box<dyn ModelBackend>>) {
        self.detector = backend.map(|backend| Detector::ShapeAgnostic(Arc::from(backend)));
    }

    /// Overridden because DBNet is the one net whose plan may have to be shape-pinned.
    ///
    /// The default implementation builds a single shape-agnostic plan, which `tract` cannot do
    /// for DBNet at all: its FPN skip connections fail to unify under a symbolic H/W. Such a
    /// backend instead keeps the bytes here and builds a plan per page extent on first use, so
    /// it runs the identical tensor `ort` runs (see [`ShapePinnedPlans`]).
    fn init_model_from_memory_on(
        &mut self,
        backend: inference::Backend,
        model_bytes: &[u8],
        num_thread: usize,
    ) -> Result<(), OcrError> {
        if backend.requires_concrete_input_shape() {
            self.detector = Some(Detector::ShapePinned(ShapePinnedPlans::new(
                backend,
                model_bytes.to_vec(),
                num_thread,
            )?));
            return Ok(());
        }
        let backend = inference::load_backend(backend, model_bytes, num_thread, None)?;
        self.set_backend(Some(backend));
        Ok(())
    }
}

impl DbNet {
    /// The plan that runs a `width x height` detection input.
    ///
    /// A shape-agnostic backend hands back its single plan; a shape-pinned one builds (or
    /// reuses) the plan for exactly this extent, which is what makes the two paths run the
    /// same tensor and produce the same probability map.
    fn plan_for(&self, height: u32, width: u32) -> Result<Arc<dyn ModelBackend>, OcrError> {
        match &self.detector {
            None => Err(OcrError::SessionNotInitialized),
            Some(Detector::ShapeAgnostic(backend)) => Ok(Arc::clone(backend)),
            Some(Detector::ShapePinned(plans)) => plans.plan_for((height as usize, width as usize)),
        }
    }

    pub fn get_text_boxes(
        &self,
        img_src: &image::RgbImage,
        scale: &ScaleParam,
        box_score_thresh: f32,
        box_thresh: f32,
        un_clip_ratio: f32,
    ) -> Result<Vec<TextBox>, OcrError> {
        let src_resize = image::imageops::resize(
            img_src,
            scale.dst_width,
            scale.dst_height,
            image::imageops::FilterType::Triangle,
        );

        let input_tensor =
            OcrUtils::substract_mean_normalize(&src_resize, &IMAGENET_MEAN_VALUES, &IMAGENET_NORM_VALUES);

        let plan = self.plan_for(src_resize.height(), src_resize.width())?;
        let (pred_shape, pred_data) = inference::run_flat(plan.as_ref(), input_tensor.into_dyn())?;
        Self::check_prediction_extent(&pred_shape, pred_data.len(), src_resize.width(), src_resize.height())?;

        let text_boxes = Self::get_text_boxes_core(
            &pred_data,
            src_resize.height(),
            src_resize.width(),
            &ScaleParam::new(
                scale.src_width,
                scale.src_height,
                scale.dst_width,
                scale.dst_height,
                scale.scale_width,
                scale.scale_height,
            ),
            DetectionThresholds {
                box_score_thresh,
                box_thresh,
                un_clip_ratio,
            },
        )?;

        Ok(text_boxes)
    }

    /// Reject a probability map whose spatial extent is not the resized page's.
    ///
    /// DBNet is fully convolutional and returns its map at the input extent, on every engine,
    /// so this always holds -- but it is checked rather than assumed. Downstream the map is
    /// wrapped in an `ImageBuffer` sized from the *page*, and the `scale_width`/`scale_height`
    /// back-mapping in [`DbNet::get_text_boxes_core`] assumes the two agree; a silent mismatch
    /// would put boxes in the wrong place instead of failing.
    fn check_prediction_extent(
        pred_shape: &[usize],
        value_count: usize,
        width: u32,
        height: u32,
    ) -> Result<(), OcrError> {
        let [.., map_height, map_width] = pred_shape else {
            return Err(OcrError::inference(format!(
                "DBNet output shape {pred_shape:?} has no spatial dimensions"
            )));
        };
        let (width, height) = (width as usize, height as usize);
        if *map_width != width || *map_height != height || value_count < width * height {
            return Err(OcrError::inference(format!(
                "DBNet returned a {map_width}x{map_height} map ({value_count} values) for a {width}x{height} page"
            )));
        }
        Ok(())
    }

    fn get_text_boxes_core(
        pred_data: &[f32],
        rows: u32,
        cols: u32,
        s: &ScaleParam,
        thresholds: DetectionThresholds,
    ) -> Result<Vec<TextBox>, OcrError> {
        let max_side_thresh = 3.0;
        let DetectionThresholds {
            box_score_thresh,
            box_thresh,
            un_clip_ratio,
        } = thresholds;

        let threshold_img = Self::binarize_predictions(pred_data, rows, cols, box_thresh)?;
        let dilated_img = Self::dilate_db_mask(&threshold_img);

        let pred_img: image::ImageBuffer<image::Luma<f32>, Vec<f32>> =
            image::ImageBuffer::from_vec(cols, rows, pred_data.to_vec()).ok_or_else(|| {
                OcrError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "Failed to create image buffer from predictions: {} x {} dimensions may be invalid",
                        cols, rows
                    ),
                ))
            })?;

        let img_contours: Vec<imageproc::contours::Contour<i32>> = imageproc::contours::find_contours(&dilated_img);

        let mut rs_boxes = Vec::with_capacity(img_contours.len());

        for contour in img_contours {
            if contour.points.len() <= 2 {
                continue;
            }

            let mut max_side = 0.0;
            let min_box = Self::get_mini_box(&contour.points, &mut max_side)?;
            if max_side < max_side_thresh {
                continue;
            }

            let score = Self::get_score(&contour, &pred_img)?;
            if score < box_score_thresh {
                continue;
            }

            let clip_box = Self::unclip(&min_box, un_clip_ratio)?;
            if clip_box.is_empty() {
                continue;
            }

            let mut clip_contour = Vec::new();
            for point in &clip_box {
                clip_contour.push(*point);
            }

            let mut max_side_clip = 0.0;
            let clip_min_box = Self::get_mini_box(&clip_contour, &mut max_side_clip)?;
            if max_side_clip < max_side_thresh + 2.0 {
                continue;
            }

            let text_box = TextBox {
                score,
                points: Self::scale_points_to_source(&clip_min_box, s),
            };

            rs_boxes.push(text_box);
        }

        Ok(rs_boxes)
    }

    /// Map DBNet's detection-space points (in the resized page's pixel grid) back to the
    /// original page pixel grid, clamped to its bounds. ~keep
    fn scale_points_to_source(points: &[imageproc::point::Point<f32>], s: &ScaleParam) -> Vec<ocr_result::Point> {
        points
            .iter()
            .map(|item| {
                let x = (item.x / s.scale_width) as u32;
                let y = (item.y / s.scale_height) as u32;
                ocr_result::Point {
                    x: x.min(s.src_width),
                    y: y.min(s.src_height),
                }
            })
            .collect()
    }

    fn binarize_predictions(
        predictions: &[f32],
        rows: u32,
        cols: u32,
        threshold: f32,
    ) -> Result<image::GrayImage, OcrError> {
        let binary_data = predictions
            .iter()
            .map(|&prediction| if prediction > threshold { BINARY_FOREGROUND } else { 0 })
            .collect();

        image::GrayImage::from_vec(cols, rows, binary_data).ok_or_else(|| {
            OcrError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Failed to create DB mask: {} x {} dimensions do not match predictions",
                    cols, rows
                ),
            ))
        })
    }

    fn dilate_db_mask(mask: &image::GrayImage) -> image::GrayImage {
        let mut dilated = image::GrayImage::new(mask.width(), mask.height());
        for (x, y, pixel) in mask.enumerate_pixels() {
            if pixel.0[0] == 0 {
                continue;
            }

            for offset_y in 0..DB_DILATION_KERNEL_SIZE {
                for offset_x in 0..DB_DILATION_KERNEL_SIZE {
                    if let Some(destination) = dilated.get_pixel_mut_checked(x + offset_x, y + offset_y) {
                        *destination = image::Luma([BINARY_FOREGROUND]);
                    }
                }
            }
        }
        dilated
    }

    fn get_mini_box(
        contour_points: &[imageproc::point::Point<i32>],
        min_edge_size: &mut f32,
    ) -> Result<Vec<imageproc::point::Point<f32>>, OcrError> {
        let rect = imageproc::geometry::min_area_rect(contour_points);

        let mut rect_points: Vec<imageproc::point::Point<f32>> = rect
            .iter()
            .map(|p| imageproc::point::Point::new(p.x as f32, p.y as f32))
            .collect();

        let dx_w = rect_points[0].x - rect_points[1].x;
        let dy_w = rect_points[0].y - rect_points[1].y;
        let width = (dx_w * dx_w + dy_w * dy_w).sqrt();
        let dx_h = rect_points[1].x - rect_points[2].x;
        let dy_h = rect_points[1].y - rect_points[2].y;
        let height = (dx_h * dx_h + dy_h * dy_h).sqrt();

        *min_edge_size = width.min(height);

        rect_points.sort_by(|a, b| {
            if a.x > b.x {
                return Ordering::Greater;
            }
            if a.x == b.x {
                return Ordering::Equal;
            }
            Ordering::Less
        });

        let mut box_points = Vec::new();
        let (index_1, index_4) = if rect_points[1].y > rect_points[0].y {
            (0, 1)
        } else {
            (1, 0)
        };

        let (index_2, index_3) = if rect_points[3].y > rect_points[2].y {
            (2, 3)
        } else {
            (3, 2)
        };

        box_points.push(rect_points[index_1]);
        box_points.push(rect_points[index_2]);
        box_points.push(rect_points[index_3]);
        box_points.push(rect_points[index_4]);

        Ok(box_points)
    }

    fn get_score(
        contour: &imageproc::contours::Contour<i32>,
        f_map_mat: &image::ImageBuffer<image::Luma<f32>, Vec<f32>>,
    ) -> Result<f32, OcrError> {
        let mut xmin = i32::MAX;
        let mut xmax = i32::MIN;
        let mut ymin = i32::MAX;
        let mut ymax = i32::MIN;

        for point in contour.points.iter() {
            let x = point.x;
            let y = point.y;

            if x < xmin {
                xmin = x;
            }
            if x > xmax {
                xmax = x;
            }
            if y < ymin {
                ymin = y;
            }
            if y > ymax {
                ymax = y;
            }
        }

        let width = f_map_mat.width() as i32;
        let height = f_map_mat.height() as i32;

        xmin = xmin.max(0).min(width - 1);
        xmax = xmax.max(0).min(width - 1);
        ymin = ymin.max(0).min(height - 1);
        ymax = ymax.max(0).min(height - 1);

        let roi_width = xmax - xmin + 1;
        let roi_height = ymax - ymin + 1;

        if roi_width <= 0 || roi_height <= 0 {
            return Ok(0.0);
        }

        let mut mask = image::GrayImage::new(roi_width as u32, roi_height as u32);

        let mut pts = Vec::<imageproc::point::Point<i32>>::new();
        for point in contour.points.iter() {
            pts.push(imageproc::point::Point::new(point.x - xmin, point.y - ymin));
        }

        imageproc::drawing::draw_polygon_mut(&mut mask, pts.as_slice(), image::Luma([255]));

        let cropped_img =
            image::imageops::crop_imm(f_map_mat, xmin as u32, ymin as u32, roi_width as u32, roi_height as u32)
                .to_image();

        let mean = OcrUtils::calculate_mean_with_mask(&cropped_img, &mask);

        Ok(mean)
    }

    fn unclip(
        box_points: &[imageproc::point::Point<f32>],
        unclip_ratio: f32,
    ) -> Result<Vec<imageproc::point::Point<i32>>, OcrError> {
        let dx_w = box_points[0].x - box_points[1].x;
        let dy_w = box_points[0].y - box_points[1].y;
        let clip_rect_width = (dx_w * dx_w + dy_w * dy_w).sqrt();
        let dx_h = box_points[1].x - box_points[2].x;
        let dy_h = box_points[1].y - box_points[2].y;
        let clip_rect_height = (dx_h * dx_h + dy_h * dy_h).sqrt();

        if clip_rect_height < 1.001 && clip_rect_width < 1.001 {
            return Ok(Vec::new());
        }

        let mut the_cliper_pts = Vec::new();
        for pt in box_points {
            let a1 = Coord {
                x: pt.x as f64,
                y: pt.y as f64,
            };
            the_cliper_pts.push(a1);
        }

        let area = Self::signed_polygon_area(box_points).abs();
        let length = Self::length_of_points(box_points);
        let distance = area * unclip_ratio / length as f32;

        let co = Polygon::new(LineString::new(the_cliper_pts), vec![]);
        let solution = co
            .offset(distance as f64, JoinType::Round(2.0), EndType::ClosedPolygon, 1.0)
            .0;

        if solution.is_empty() {
            return Ok(Vec::new());
        }

        let first_polygon = solution.first().ok_or_else(|| {
            OcrError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Polygon solution list was empty after offset operation",
            ))
        })?;

        let ret_pts: Vec<_> = first_polygon
            .exterior()
            .points()
            .map(|ip| imageproc::point::Point::new(ip.x() as i32, ip.y() as i32))
            .collect();

        Ok(ret_pts)
    }

    fn signed_polygon_area(points: &[imageproc::point::Point<f32>]) -> f32 {
        let num_points = points.len();
        let mut pts = Vec::with_capacity(num_points + 1);
        pts.extend_from_slice(points);
        pts.push(points[0]);

        let mut area = 0.0;
        for i in 0..num_points {
            area += (pts[i + 1].x - pts[i].x) * (pts[i + 1].y + pts[i].y) / 2.0;
        }

        area
    }

    fn length_of_points(box_points: &[imageproc::point::Point<f32>]) -> f64 {
        if box_points.is_empty() {
            return 0.0;
        }

        let mut length = 0.0;
        let mut x0 = box_points[0].x as f64;
        let mut y0 = box_points[0].y as f64;

        for pt in &box_points[1..] {
            let x1 = pt.x as f64;
            let y1 = pt.y as f64;
            let dx = x1 - x0;
            let dy = y1 - y0;
            length += (dx * dx + dy * dy).sqrt();
            x0 = x1;
            y0 = y1;
        }

        let dx = box_points[0].x as f64 - x0;
        let dy = box_points[0].y as f64 - y0;
        length += (dx * dx + dy * dy).sqrt();

        length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shape-pinned backend defers building its plan until a page extent is known, so the
    /// model bytes are the only thing it can reject at load time — and it must, or a bad model
    /// would surface mid-extraction instead of where every other model failure is reported.
    #[cfg(feature = "tract")]
    #[test]
    fn should_reject_a_malformed_model_at_load_time_on_a_shape_pinned_backend() {
        let mut detector = DbNet::new();

        let error = detector
            .init_model_from_memory_on(inference::Backend::Tract, b"not an ONNX model", 1)
            .expect_err("garbage bytes are not a loadable model");

        assert!(matches!(error, OcrError::Inference { .. }), "error was: {error}");
        assert!(
            format!("{detector:?}").contains("initialized: false"),
            "a failed load must leave the detector uninitialized: {detector:?}"
        );
    }

    #[test]
    fn should_accept_a_prediction_map_at_the_page_extent() {
        let predictions: Vec<f32> = (0..6).map(|value| value as f32).collect();

        DbNet::check_prediction_extent(&[1, 1, 2, 3], predictions.len(), 3, 2).expect("shapes match");
    }

    #[test]
    fn should_reject_a_prediction_map_that_is_not_the_page_extent() {
        // What a plan pinned to the wrong shape would return: a canvas-sized map for a
        // smaller page. Cropping it back would silently mis-place every box.
        let error =
            DbNet::check_prediction_extent(&[1, 1, 4, 4], 16, 3, 2).expect_err("a 4x4 map is not a 3x2 page's map");

        assert!(matches!(error, OcrError::Inference { .. }));
        assert!(error.to_string().contains("4x4 map"), "error was: {error}");
        assert!(error.to_string().contains("3x2 page"), "error was: {error}");
    }

    #[test]
    fn should_reject_a_prediction_map_smaller_than_the_resized_page() {
        let error =
            DbNet::check_prediction_extent(&[1, 1, 2, 2], 4, 3, 2).expect_err("a 2x2 map cannot cover a 3x2 page");

        assert!(matches!(error, OcrError::Inference { .. }));
        assert!(error.to_string().contains("2x2"), "error was: {error}");
    }

    #[test]
    fn should_reject_a_prediction_shape_without_spatial_dimensions() {
        let error = DbNet::check_prediction_extent(&[1], 1, 1, 1).expect_err("rank 1 has no spatial dimensions");

        assert!(matches!(error, OcrError::Inference { .. }));
    }

    #[test]
    fn should_use_configured_db_threshold_when_binarizing_predictions() {
        let predictions = [0.29, 0.30, 0.31, 0.80];

        let mask = DbNet::binarize_predictions(&predictions, 1, 4, 0.30).unwrap();

        assert_eq!(mask.as_raw(), &[0, 0, BINARY_FOREGROUND, BINARY_FOREGROUND]);
    }

    #[test]
    fn should_apply_rapidocr_two_by_two_dilation_to_db_mask() {
        let mut mask = image::GrayImage::new(4, 4);
        mask.put_pixel(1, 1, image::Luma([BINARY_FOREGROUND]));

        let dilated = DbNet::dilate_db_mask(&mask);

        assert_eq!(
            dilated.as_raw(),
            &[
                0,
                0,
                0,
                0,
                0,
                BINARY_FOREGROUND,
                BINARY_FOREGROUND,
                0,
                0,
                BINARY_FOREGROUND,
                BINARY_FOREGROUND,
                0,
                0,
                0,
                0,
                0,
            ]
        );
    }
}
