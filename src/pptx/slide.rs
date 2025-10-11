use crate::pptx::config::ParserConfig;
use crate::pptx::content_builder::ContentBuilder;
use crate::pptx::parser::rels::parse_slide_rels;
use crate::pptx::parser::xml::parse_slide_xml;
use crate::pptx::types::{Result, Slide, SlideElement};
use std::collections::HashMap;

impl Slide {
    pub fn from_xml(slide_number: u32, xml_data: &[u8], rels_data: Option<&[u8]>) -> Result<Self> {
        let elements = parse_slide_xml(xml_data)?;

        let images = if let Some(rels) = rels_data {
            parse_slide_rels(rels)?
        } else {
            Vec::new()
        };

        Ok(Self {
            slide_number,
            elements,
            images,
            image_data: HashMap::new(),
        })
    }

    pub fn to_markdown(&self, config: &ParserConfig) -> String {
        let mut builder = ContentBuilder::new();

        if config.include_slide_comment {
            builder.add_slide_header(self.slide_number);
        }

        for element in &self.elements {
            match element {
                SlideElement::Text(text, _) => {
                    let text_content: String = text.runs.iter().map(|run| run.render_as_md()).collect();

                    let normalized = text_content.replace('\n', " ");
                    let is_title = normalized.len() < 100 && !normalized.trim().is_empty();

                    if is_title {
                        builder.add_title(normalized.trim());
                    } else {
                        builder.add_text(&text_content);
                    }
                }
                SlideElement::Table(table, _) => {
                    let table_rows: Vec<Vec<String>> = table
                        .rows
                        .iter()
                        .map(|row| {
                            row.cells
                                .iter()
                                .map(|cell| cell.runs.iter().map(|run| run.extract()).collect::<String>())
                                .collect()
                        })
                        .collect();
                    builder.add_table(&table_rows);
                }
                SlideElement::List(list, _) => {
                    for item in &list.items {
                        let item_text: String = item.runs.iter().map(|run| run.extract()).collect();
                        builder.add_list_item(item.level, item.is_ordered, &item_text);
                    }
                }
                SlideElement::Image(img_ref, _) => {
                    builder.add_image(&img_ref.id, self.slide_number);
                }
            }
        }

        builder.build()
    }

    pub fn image_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| matches!(e, SlideElement::Image(_, _)))
            .count()
    }

    pub fn table_count(&self) -> usize {
        self.elements
            .iter()
            .filter(|e| matches!(e, SlideElement::Table(_, _)))
            .count()
    }

    pub fn set_image_data(&mut self, image_data: HashMap<String, Vec<u8>>) {
        self.image_data = image_data;
    }
}
