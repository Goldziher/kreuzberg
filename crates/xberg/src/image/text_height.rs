//! Estimate how tall the text in a raster is, in pixels.
//!
//! Used when a raster carries no resolution metadata: the resample factor then follows from the
//! size of the glyphs instead of from an assumed DPI (see `preprocessing::unknown_raster_dpi`).

/// Largest raster, in pixels, that is scanned. A larger one has no estimate.
const MAX_SCANNED_PIXELS: usize = 16_000_000;
/// Fewest glyph-sized components needed before the estimate is trusted.
const MIN_COMPONENTS: usize = 5;
/// Components shorter than this are specks, not glyphs.
const MIN_COMPONENT_HEIGHT: usize = 2;
/// A component taller than this fraction of the raster is a rule, a border or a picture.
const MAX_COMPONENT_HEIGHT_FRACTION: f64 = 0.25;
/// A component wider than this fraction of the raster is a rule, a border or a picture.
const MAX_COMPONENT_WIDTH_FRACTION: f64 = 0.5;
/// Largest share of the raster the ink may cover for its components to be read as text. Clean
/// text and pages cover 1-11% (176 synthetic images and 48 rendered pages); the receipts and photos
/// measured cover 30-45%, and their blobs are not letters (a receipt read as 34px text, when the
/// 3x upscale it already got is what recovers its header). ~keep
const MAX_INK_FRACTION: f64 = 0.20;
/// Percentile of component heights taken as the height of a tall glyph.
const TALL_GLYPH_PERCENTILE: f64 = 0.9;
/// Components under this fraction of a tall glyph are dots, commas and accents, not letter bodies.
const LETTER_BODY_MIN_FRACTION: f64 = 0.5;

/// Median height of the letter-sized connected components of dark-on-light (or light-on-dark)
/// text, or `None` when the raster is too large, uniform, or holds too few glyph-like components.
///
/// The cap height of the text, in practice: a printed line's tall letters dominate the
/// components that survive the dot-and-comma cut. `rgb_data` is tightly packed RGB8.
pub(crate) fn median_glyph_height(rgb_data: &[u8], width: usize, height: usize) -> Option<f64> {
    let pixel_count = width.checked_mul(height)?;
    if pixel_count == 0 || pixel_count > MAX_SCANNED_PIXELS || rgb_data.len() != pixel_count.checked_mul(3)? {
        return None;
    }

    let luminance: Vec<u8> = rgb_data
        .as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| ((299 * u32::from(pixel[0]) + 587 * u32::from(pixel[1]) + 114 * u32::from(pixel[2])) / 1000) as u8)
        .collect();
    let threshold = otsu_threshold(&luminance)?;
    let mut foreground: Vec<bool> = luminance.iter().map(|&value| value <= threshold).collect();
    let foreground_pixels = foreground.iter().filter(|&&is_foreground| is_foreground).count();
    if foreground_pixels == 0 || foreground_pixels == pixel_count {
        return None;
    }
    if foreground_pixels * 2 > pixel_count {
        foreground
            .iter_mut()
            .for_each(|is_foreground| *is_foreground = !*is_foreground);
    }
    let ink_pixels = foreground_pixels.min(pixel_count - foreground_pixels);
    if ink_pixels as f64 > MAX_INK_FRACTION * pixel_count as f64 {
        return None;
    }

    let mut heights = component_heights(&mut foreground, width, height);
    if heights.len() < MIN_COMPONENTS {
        return None;
    }
    heights.sort_unstable();
    let tall_glyph = percentile(&heights, TALL_GLYPH_PERCENTILE);
    let letter_bodies: Vec<usize> = heights
        .into_iter()
        .filter(|&component_height| component_height as f64 >= LETTER_BODY_MIN_FRACTION * tall_glyph)
        .collect();
    Some(percentile(&letter_bodies, 0.5))
}

/// Linear-interpolated percentile `fraction` (0..=1) of a sorted, non-empty slice.
fn percentile(sorted: &[usize], fraction: f64) -> f64 {
    let position = fraction * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let weight = position - lower as f64;
    sorted[lower] as f64 * (1.0 - weight) + sorted[upper] as f64 * weight
}

/// Otsu's threshold: the grey level that best separates the histogram into two classes.
fn otsu_threshold(luminance: &[u8]) -> Option<u8> {
    let mut histogram = [0u64; 256];
    for &value in luminance {
        histogram[usize::from(value)] += 1;
    }
    let total: u64 = histogram.iter().sum();
    let weighted_total: u64 = histogram
        .iter()
        .enumerate()
        .map(|(level, &count)| level as u64 * count)
        .sum();

    let mut below_count = 0u64;
    let mut below_weight = 0u64;
    let mut best: Option<(f64, u8)> = None;
    for (level, &count) in histogram.iter().enumerate() {
        below_count += count;
        if below_count == 0 {
            continue;
        }
        let above_count = total - below_count;
        if above_count == 0 {
            break;
        }
        below_weight += level as u64 * count;
        let below_mean = below_weight as f64 / below_count as f64;
        let above_mean = (weighted_total - below_weight) as f64 / above_count as f64;
        let variance = below_count as f64 * above_count as f64 * (below_mean - above_mean).powi(2);
        if best.is_none_or(|(best_variance, _)| variance > best_variance) {
            best = Some((variance, level as u8));
        }
    }
    best.map(|(_, level)| level)
}

/// Heights of the 8-connected foreground components that can be glyphs. Consumes `foreground`:
/// every visited pixel is cleared.
fn component_heights(foreground: &mut [bool], width: usize, height: usize) -> Vec<usize> {
    let max_height = MAX_COMPONENT_HEIGHT_FRACTION * height as f64;
    let max_width = MAX_COMPONENT_WIDTH_FRACTION * width as f64;
    let mut heights = Vec::new();
    let mut stack = Vec::new();

    for start in 0..foreground.len() {
        if !foreground[start] {
            continue;
        }
        foreground[start] = false;
        stack.push(start);
        let (mut min_x, mut max_x) = (start % width, start % width);
        let (mut min_y, mut max_y) = (start / width, start / width);

        while let Some(index) = stack.pop() {
            let (x, y) = (index % width, index / width);
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            claim_neighbours(foreground, &mut stack, (x, y), (width, height));
        }

        let component_height = max_y - min_y + 1;
        let component_width = max_x - min_x + 1;
        if component_height >= MIN_COMPONENT_HEIGHT
            && component_height as f64 <= max_height
            && component_width as f64 <= max_width
        {
            heights.push(component_height);
        }
    }
    heights
}

/// Clear and queue the foreground pixels among the eight neighbours of `(x, y)`.
fn claim_neighbours(
    foreground: &mut [bool],
    stack: &mut Vec<usize>,
    (x, y): (usize, usize),
    (width, height): (usize, usize),
) {
    for next_y in y.saturating_sub(1)..=(y + 1).min(height - 1) {
        for next_x in x.saturating_sub(1)..=(x + 1).min(width - 1) {
            let neighbour = next_y * width + next_x;
            if foreground[neighbour] {
                foreground[neighbour] = false;
                stack.push(neighbour);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Black rectangles standing in for letters: `count` of them, each `glyph_height` tall,
    /// `glyph_width` wide and spaced `pitch` apart, on a white raster.
    fn raster_of_glyphs(
        width: usize,
        height: usize,
        glyph_height: usize,
        glyph_width: usize,
        pitch: usize,
        count: usize,
    ) -> Vec<u8> {
        let mut rgb = vec![255u8; width * height * 3];
        for glyph in 0..count {
            let left = 10 + glyph * pitch;
            for y in 10..10 + glyph_height {
                for x in left..left + glyph_width {
                    let index = (y * width + x) * 3;
                    rgb[index..index + 3].fill(0);
                }
            }
        }
        rgb
    }

    #[test]
    fn should_report_the_height_of_the_glyph_sized_components() {
        let rgb = raster_of_glyphs(400, 200, 26, 14, 20, 15);
        assert_eq!(median_glyph_height(&rgb, 400, 200), Some(26.0));
    }

    #[test]
    fn should_report_the_same_height_for_light_text_on_a_dark_ground() {
        let mut rgb = raster_of_glyphs(400, 100, 12, 8, 14, 20);
        rgb.iter_mut().for_each(|channel| *channel = 255 - *channel);
        assert_eq!(median_glyph_height(&rgb, 400, 100), Some(12.0));
    }

    #[test]
    fn should_ignore_dots_beside_the_letters() {
        let mut rgb = raster_of_glyphs(400, 100, 20, 10, 16, 20);
        for dot in 0..30 {
            let index = ((60 + dot % 3) * 400 + 12 + dot * 12) * 3;
            rgb[index..index + 3].fill(0);
        }
        assert_eq!(median_glyph_height(&rgb, 400, 100), Some(20.0));
    }

    #[test]
    fn should_give_no_estimate_for_a_raster_without_text() {
        assert_eq!(median_glyph_height(&vec![255u8; 100 * 100 * 3], 100, 100), None);
        let few = raster_of_glyphs(400, 100, 20, 10, 40, 3);
        assert_eq!(median_glyph_height(&few, 400, 100), None);
    }

    #[test]
    fn should_give_no_estimate_when_the_ink_covers_a_large_share_of_the_raster() {
        let mut rgb = raster_of_glyphs(400, 200, 26, 14, 20, 15);
        for pixel in rgb.as_chunks_mut::<3>().0.iter_mut().skip(30_000).step_by(2) {
            pixel.fill(0);
        }
        assert_eq!(median_glyph_height(&rgb, 400, 200), None);
    }

    #[test]
    fn should_give_no_estimate_for_a_buffer_that_does_not_match_the_size() {
        assert_eq!(median_glyph_height(&[0u8; 10], 100, 100), None);
        assert_eq!(median_glyph_height(&[], 0, 0), None);
    }
}
