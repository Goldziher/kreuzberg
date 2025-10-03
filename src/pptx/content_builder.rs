//! Efficient content builder for markdown generation

/// Efficient string builder for slide content generation
pub struct ContentBuilder {
    content: String,
}

impl ContentBuilder {
    pub fn new() -> Self {
        Self {
            content: String::with_capacity(8192),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            content: String::with_capacity(capacity),
        }
    }

    /// Add slide header with number
    pub fn add_slide_header(&mut self, slide_number: u32) {
        self.content.reserve(50);
        self.content.push_str("\n\n<!-- Slide number: ");
        self.content.push_str(&slide_number.to_string());
        self.content.push_str(" -->\n");
    }

    /// Add text content
    pub fn add_text(&mut self, text: &str) {
        if !text.trim().is_empty() {
            self.content.push_str(text);
        }
    }

    /// Add title (markdown header)
    pub fn add_title(&mut self, title: &str) {
        if !title.trim().is_empty() {
            self.content.push_str("# ");
            self.content.push_str(title.trim());
            self.content.push('\n');
        }
    }

    /// Add HTML table
    pub fn add_table(&mut self, rows: &[Vec<String>]) {
        if rows.is_empty() {
            return;
        }

        self.content.push_str("\n<table>");
        for (i, row) in rows.iter().enumerate() {
            self.content.push_str("<tr>");
            let tag = if i == 0 { "th" } else { "td" };

            for cell in row {
                self.content.push('<');
                self.content.push_str(tag);
                self.content.push('>');
                self.content.push_str(&html_escape(cell));
                self.content.push_str("</");
                self.content.push_str(tag);
                self.content.push('>');
            }
            self.content.push_str("</tr>");
        }
        self.content.push_str("</table>\n");
    }

    /// Add list items
    pub fn add_list_item(&mut self, level: u32, is_ordered: bool, text: &str) {
        let indent_count = level.saturating_sub(1) as usize;
        for _ in 0..indent_count {
            self.content.push_str("  ");
        }

        let marker = if is_ordered { "1." } else { "-" };
        self.content.push_str(marker);
        self.content.push(' ');
        self.content.push_str(text.trim());
        self.content.push('\n');
    }

    /// Add image reference
    pub fn add_image(&mut self, image_id: &str, slide_number: u32) {
        let filename = format!("slide_{}_image_{}.jpg", slide_number, image_id);
        self.content.push_str("![");
        self.content.push_str(image_id);
        self.content.push_str("](");
        self.content.push_str(&filename);
        self.content.push_str(")\n");
    }

    /// Add notes section
    pub fn add_notes(&mut self, notes: &str) {
        if !notes.trim().is_empty() {
            self.content.push_str("\n\n### Notes:\n");
            self.content.push_str(notes);
            self.content.push('\n');
        }
    }

    /// Build final content string
    pub fn build(self) -> String {
        self.content.trim().to_string()
    }
}

impl Default for ContentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// HTML escape utility function
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
