//! Lazy slide iterator for streaming processing

use crate::pptx::config::ParserConfig;
use crate::pptx::container::PptxContainer;
use crate::pptx::streaming::cache::ResourceCache;
use crate::pptx::types::{Result, Slide};
use std::collections::HashMap;

/// Iterator for processing slides one at a time
pub struct SlideIterator {
    container: PptxContainer,
    cache: ResourceCache,
    config: ParserConfig,
    current_index: usize,
    slide_paths: Vec<String>,
}

impl SlideIterator {
    /// Create a new slide iterator
    pub fn new(container: PptxContainer, config: ParserConfig) -> Self {
        let slide_paths = container.slide_paths().to_vec();
        let cache = ResourceCache::new(config.max_cache_size_mb, config.max_cached_images);

        Self {
            container,
            cache,
            config,
            current_index: 0,
            slide_paths,
        }
    }

    /// Get total slide count
    pub fn slide_count(&self) -> usize {
        self.slide_paths.len()
    }

    /// Load next slide, returning None when done
    pub fn next_slide(&mut self) -> Result<Option<Slide>> {
        if self.current_index >= self.slide_paths.len() {
            return Ok(None);
        }

        let slide_path = self.slide_paths[self.current_index].clone();
        let slide_number = (self.current_index + 1) as u32;

        let slide_xml = self.container.read_file(&slide_path)?;

        let rels_path = self.container.get_slide_rels_path(&slide_path);
        let rels_data = self.container.read_file(&rels_path).ok();

        let mut slide = Slide::from_xml(slide_number, &slide_xml, rels_data.as_deref())?;

        if self.config.extract_images {
            let image_data = self.load_slide_images(&slide, &slide_path)?;
            slide.set_image_data(image_data);
        }

        self.current_index += 1;
        Ok(Some(slide))
    }

    fn load_slide_images(&mut self, slide: &Slide, slide_path: &str) -> Result<HashMap<String, Vec<u8>>> {
        let mut image_data = HashMap::new();

        for img_ref in &slide.images {
            let image_path = PptxContainer::get_full_image_path(slide_path, &img_ref.target);

            if let Some(cached_data) = self.cache.get(&image_path) {
                image_data.insert(img_ref.id.clone(), cached_data.clone());
                continue;
            }

            match self.container.read_file(&image_path) {
                Ok(data) => {
                    self.cache.insert(image_path.clone(), data.clone());
                    image_data.insert(img_ref.id.clone(), data);
                }
                Err(_) => {
                    eprintln!("Warning: Image not found: {}", image_path);
                }
            }
        }

        Ok(image_data)
    }
}

/// Iterator trait implementation for convenient use in for loops
impl Iterator for SlideIterator {
    type Item = Result<Slide>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_slide() {
            Ok(Some(slide)) => Some(Ok(slide)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.slide_paths.len() - self.current_index;
        (remaining, Some(remaining))
    }
}
