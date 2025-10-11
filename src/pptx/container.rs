use crate::pptx::parser::rels::parse_presentation_rels;
use crate::pptx::types::{PptxError, Result};
use crate::pptx::utils::{get_full_image_path, get_slide_rels_path};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

pub struct PptxContainer {
    archive: ZipArchive<File>,
    slide_paths: Vec<String>,
}

impl PptxContainer {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mut archive = ZipArchive::new(file)?;

        let slide_paths = Self::find_slide_paths(&mut archive)?;

        Ok(Self { archive, slide_paths })
    }

    pub fn slide_paths(&self) -> &[String] {
        &self.slide_paths
    }

    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>> {
        match self.archive.by_name(path) {
            Ok(mut file) => {
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                Ok(contents)
            }
            Err(zip::result::ZipError::FileNotFound) => Err(PptxError::ParseError("File not found in archive")),
            Err(e) => Err(PptxError::Zip(e)),
        }
    }

    pub fn get_slide_rels_path(&self, slide_path: &str) -> String {
        get_slide_rels_path(slide_path)
    }

    pub fn get_full_image_path(slide_path: &str, image_target: &str) -> String {
        get_full_image_path(slide_path, image_target)
    }

    fn find_slide_paths(archive: &mut ZipArchive<File>) -> Result<Vec<String>> {
        if let Ok(rels_data) = Self::read_file_from_archive(archive, "ppt/_rels/presentation.xml.rels")
            && let Ok(paths) = parse_presentation_rels(&rels_data)
        {
            return Ok(paths);
        }

        let mut slide_paths = Vec::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name();
                if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                    slide_paths.push(name.to_string());
                }
            }
        }

        slide_paths.sort();
        Ok(slide_paths)
    }

    fn read_file_from_archive(archive: &mut ZipArchive<File>, path: &str) -> Result<Vec<u8>> {
        let mut file = archive.by_name(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        Ok(contents)
    }
}
