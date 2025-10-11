pub fn extract_slide_name(slide_path: &str) -> &str {
    slide_path.split('/').next_back().unwrap_or("slide1.xml")
}

pub fn extract_base_name(slide_name: &str) -> &str {
    slide_name.strip_suffix(".xml").unwrap_or(slide_name)
}

pub fn get_slide_rels_path(slide_path: &str) -> String {
    let slide_name = extract_slide_name(slide_path);
    let base_name = extract_base_name(slide_name);
    format!("ppt/slides/_rels/{}.xml.rels", base_name)
}

pub fn get_slide_notes_path(slide_path: &str) -> String {
    let slide_name = extract_slide_name(slide_path);
    let base_name = extract_base_name(slide_name);
    let slide_num = base_name.strip_prefix("slide").unwrap_or("1");
    format!("ppt/notesSlides/notesSlide{}.xml", slide_num)
}

pub fn get_full_image_path(slide_path: &str, image_target: &str) -> String {
    if let Some(stripped) = image_target.strip_prefix("/") {
        stripped.to_string()
    } else if let Some(stripped) = image_target.strip_prefix("../") {
        format!("ppt/{}", stripped)
    } else {
        let slide_dir = slide_path.rsplit_once('/').map(|x| x.0).unwrap_or("ppt/slides");
        format!("{}/{}", slide_dir, image_target)
    }
}
