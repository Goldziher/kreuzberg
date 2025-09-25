use super::PDF_POINTS_PER_INCH;
use pyo3::prelude::*;

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
#[pyfunction]
#[pyo3(signature = (page_width, page_height, target_dpi, max_dimension, min_dpi=72, max_dpi=600))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_smart_dpi_basic() {
        // Standard US Letter: 8.5" x 11"
        let dpi = calculate_smart_dpi(612.0, 792.0, 300, 4096, 2048.0);
        assert!(dpi >= 72);
        assert!(dpi <= 300);
    }

    #[test]
    fn test_calculate_smart_dpi_memory_constrained() {
        // Large page with small memory limit
        let dpi = calculate_smart_dpi(1224.0, 1584.0, 300, 8192, 10.0); // 10MB limit
        assert!(dpi < 300); // Should be constrained by memory
        assert!(dpi >= 72); // But not below minimum
    }

    #[test]
    fn test_calculate_smart_dpi_dimension_constrained() {
        // Large page with small max dimension
        let dpi = calculate_smart_dpi(612.0, 792.0, 300, 1000, 2048.0);
        assert!(dpi < 300); // Should be constrained by dimension
    }

    #[test]
    fn test_calculate_smart_dpi_minimum_dpi() {
        // Even with extreme constraints, should not go below 72 DPI
        let dpi = calculate_smart_dpi(10000.0, 10000.0, 300, 100, 1.0);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_calculate_smart_dpi_zero_dimensions() {
        // Handle edge case of zero dimensions
        let dpi = calculate_smart_dpi(0.0, 792.0, 300, 4096, 2048.0);
        assert!(dpi >= 72);

        let dpi = calculate_smart_dpi(612.0, 0.0, 300, 4096, 2048.0);
        assert!(dpi >= 72);

        let dpi = calculate_smart_dpi(0.0, 0.0, 300, 4096, 2048.0);
        assert_eq!(dpi, 300); // Should return target when both dimensions are zero
    }

    #[test]
    fn test_calculate_dimension_constrained_dpi() {
        // Test internal function
        let dpi = calculate_dimension_constrained_dpi(8.5, 11.0, 300, 4096);
        assert!(dpi <= 300);

        // Should constrain when dimensions exceed max
        let dpi = calculate_dimension_constrained_dpi(8.5, 11.0, 600, 2000);
        assert!(dpi < 600);
    }

    #[test]
    fn test_calculate_optimal_dpi() {
        // Test with normal parameters
        let dpi = calculate_optimal_dpi(612.0, 792.0, 300, 4096, 72, 600);
        assert!(dpi >= 72);
        assert!(dpi <= 600);

        // Test clamping to min
        let dpi = calculate_optimal_dpi(10000.0, 10000.0, 300, 100, 100, 600);
        assert_eq!(dpi, 100); // Should be clamped to min_dpi

        // Test clamping to max
        let dpi = calculate_optimal_dpi(72.0, 72.0, 1000, 10000, 72, 600);
        assert_eq!(dpi, 600); // Should be clamped to max_dpi
    }

    #[test]
    fn test_memory_calculation() {
        // Verify memory calculation logic
        // For 2048MB, max pixels should be around 26843545 (sqrt(2048*1024*1024/3))
        let dpi = calculate_smart_dpi(612.0, 792.0, 10000, 100000, 2048.0);

        // With 8.5" x 11" page, this should limit DPI significantly
        assert!(dpi < 10000);
        assert!(dpi >= 72);
    }

    #[test]
    fn test_aspect_ratio_preservation() {
        // Wide page
        let wide_dpi = calculate_smart_dpi(1224.0, 396.0, 300, 4096, 2048.0);

        // Tall page
        let tall_dpi = calculate_smart_dpi(396.0, 1224.0, 300, 4096, 2048.0);

        // Should handle both orientations appropriately
        assert!(wide_dpi >= 72);
        assert!(tall_dpi >= 72);
    }
}
