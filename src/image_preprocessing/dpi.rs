use super::PDF_POINTS_PER_INCH;

/// Calculate smart DPI that respects memory constraints
#[allow(clippy::cast_possible_truncation)]
pub fn calculate_smart_dpi(
    page_width: f64,
    page_height: f64,
    target_dpi: i32,
    max_dimension: i32,
    max_memory_mb: f64,
) -> i32 {
    let width_inches = page_width / PDF_POINTS_PER_INCH;
    let height_inches = page_height / PDF_POINTS_PER_INCH;

    let max_pixels = ((max_memory_mb * 1024.0 * 1024.0 / 3.0).sqrt()).round() as i32;

    let max_dpi_for_memory_width = if width_inches > 0.0 {
        (f64::from(max_pixels) / width_inches).round() as i32
    } else {
        target_dpi
    };

    let max_dpi_for_memory_height = if height_inches > 0.0 {
        (f64::from(max_pixels) / height_inches).round() as i32
    } else {
        target_dpi
    };

    let memory_constrained_dpi = max_dpi_for_memory_width.min(max_dpi_for_memory_height);

    let dimension_constrained_dpi =
        calculate_dimension_constrained_dpi(width_inches, height_inches, target_dpi, max_dimension);

    let final_dpi = target_dpi.min(memory_constrained_dpi).min(dimension_constrained_dpi);

    final_dpi.max(72)
}

#[allow(clippy::cast_possible_truncation)]
fn calculate_dimension_constrained_dpi(
    width_inches: f64,
    height_inches: f64,
    target_dpi: i32,
    max_dimension: i32,
) -> i32 {
    let target_width_pixels = (width_inches * f64::from(target_dpi)).round() as i32;
    let target_height_pixels = (height_inches * f64::from(target_dpi)).round() as i32;
    let max_pixel_dimension = target_width_pixels.max(target_height_pixels);

    if max_pixel_dimension > max_dimension {
        let max_dpi_for_width = if width_inches > 0.0 {
            (f64::from(max_dimension) / width_inches).round() as i32
        } else {
            target_dpi
        };

        let max_dpi_for_height = if height_inches > 0.0 {
            (f64::from(max_dimension) / height_inches).round() as i32
        } else {
            target_dpi
        };

        max_dpi_for_width.min(max_dpi_for_height)
    } else {
        target_dpi
    }
}

/// Calculate optimal DPI based on page dimensions and constraints
#[allow(dead_code)]
pub fn calculate_optimal_dpi(
    page_width: f64,
    page_height: f64,
    target_dpi: i32,
    max_dimension: i32,
    min_dpi: i32,
    max_dpi: i32,
) -> i32 {
    let smart_dpi = calculate_smart_dpi(page_width, page_height, target_dpi, max_dimension, 2048.0);

    min_dpi.max(smart_dpi.min(max_dpi))
}
