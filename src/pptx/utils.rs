//! Utility functions for PPTX processing

/// Extract slide name from slide path (e.g. "ppt/slides/slide1.xml" -> "slide1.xml")
pub fn extract_slide_name(slide_path: &str) -> &str {
    slide_path.split('/').next_back().unwrap_or("slide1.xml")
}

/// Extract base name from slide name (e.g. "slide1.xml" -> "slide1")
pub fn extract_base_name(slide_name: &str) -> &str {
    slide_name.strip_suffix(".xml").unwrap_or(slide_name)
}

/// Get slide relationships path for a slide
pub fn get_slide_rels_path(slide_path: &str) -> String {
    let slide_name = extract_slide_name(slide_path);
    let base_name = extract_base_name(slide_name);
    format!("ppt/slides/_rels/{}.xml.rels", base_name)
}

/// Get notes path for a slide
pub fn get_slide_notes_path(slide_path: &str) -> String {
    let slide_name = extract_slide_name(slide_path);
    let base_name = extract_base_name(slide_name);
    let slide_num = base_name.strip_prefix("slide").unwrap_or("1");
    format!("ppt/notesSlides/notesSlide{}.xml", slide_num)
}

/// Get full image path within the archive
pub fn get_full_image_path(slide_path: &str, image_target: &str) -> String {
    // Handle absolute paths (e.g., /ppt/media/image.jpg)
    if let Some(stripped) = image_target.strip_prefix("/") {
        stripped.to_string()
    }
    // Handle relative parent paths (e.g., ../media/image.jpg)
    else if let Some(stripped) = image_target.strip_prefix("../") {
        format!("ppt/{}", stripped)
    }
    // Handle relative paths from slide directory
    else {
        let slide_dir = slide_path.rsplit_once('/').map(|x| x.0).unwrap_or("ppt/slides");
        format!("{}/{}", slide_dir, image_target)
    }
}
