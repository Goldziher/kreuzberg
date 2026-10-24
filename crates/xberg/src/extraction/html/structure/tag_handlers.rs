//! `<tag>` open/close dispatch for the HTML structure walker, split out of
//! `structure.rs` to keep that file under the line-count limit.

use super::*;

impl HtmlWalker<'_, '_> {
    pub(super) fn handle_open_tag(&mut self, tag: &str, attrs_str: &str, is_self_closing: bool) {
        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => self.handle_open_heading(attrs_str),
            "p" => self.handle_open_p(attrs_str),
            "br" => self.handle_open_br(),
            "strong" | "b" => self.push_inline(InlineKind::Bold),
            "em" | "i" => self.push_inline(InlineKind::Italic),
            "code" => self.handle_open_code(attrs_str),
            "u" | "ins" => self.push_inline(InlineKind::Underline),
            "s" | "del" | "strike" => self.push_inline(InlineKind::Strikethrough),
            "sub" => self.push_inline(InlineKind::Subscript),
            "sup" => self.push_inline(InlineKind::Superscript),
            "mark" => self.push_inline(InlineKind::Highlight),
            "a" => self.handle_open_a(attrs_str),
            "pre" => self.handle_open_pre(),
            "blockquote" => self.handle_open_blockquote(attrs_str),
            "ul" => self.handle_open_ul(),
            "ol" => self.handle_open_ol(attrs_str),
            "li" => self.handle_open_li(),
            "table" => self.handle_open_table(),
            "tr" | "thead" | "tbody" | "tfoot" => self.handle_open_row_group(tag),
            "th" | "td" if self.nested_table_depth > 0 => self.handle_open_nested_cell(),
            "th" | "td" => self.handle_open_cell(tag, attrs_str),
            "img" => self.handle_open_img(attrs_str),
            "figure" => self.handle_open_figure(),
            "figcaption" => self.handle_open_figcaption(),
            "dl" => self.handle_open_dl(),
            "dt" => self.handle_open_dt(),
            "dd" => self.handle_open_dd(),
            "head" => self.handle_open_head(),
            "meta" if self.in_head => self.handle_open_meta(attrs_str),
            "script" | "style" => self.handle_open_raw_block(tag),
            "video" | "audio" => self.handle_open_media(tag),
            "math" => self.handle_open_math(attrs_str, is_self_closing),
            "hr" => self.flush_paragraph(),
            "div" | "section" | "article" | "main" | "aside" | "header" | "footer" | "nav" | "details" | "summary" => {
                self.flush_paragraph();
            }
            // The head's `<title>` names the document, in a browser tab, and is not part of its
            // text. The `<title>` of an inline SVG is its accessible name and stays (#140). ~keep
            "title" if self.in_head => self.handle_open_media(tag),
            "span" | "html" | "body" | "title" | "link" => {}
            _ => {}
        }
    }

    fn handle_open_heading(&mut self, attrs_str: &str) {
        self.flush_paragraph();
        self.text_buf.clear();
        self.annotations.clear();
        self.pending_classes = extract_attr(attrs_str, "class").map(|s| s.to_string());
    }

    fn handle_open_p(&mut self, attrs_str: &str) {
        self.flush_paragraph();
        self.pending_classes = extract_attr(attrs_str, "class").map(|s| s.to_string());
    }

    fn handle_open_br(&mut self) {
        if self.in_pre || self.pre_block.is_some() {
            if let Some(ref mut pre) = self.pre_block {
                pre.text.push('\n');
            }
        } else if self.in_list_item {
            self.list_item_text.push('\n');
        } else {
            self.text_buf.push('\x01');
        }
    }

    fn handle_open_code(&mut self, attrs_str: &str) {
        if self.in_pre {
            let lang = extract_attr(attrs_str, "class").and_then(|c| extract_language_from_class(c));
            self.pre_block = Some(PreBlock {
                language: lang.map(|s| s.to_string()),
                text: String::new(),
            });
        } else {
            self.push_inline(InlineKind::Code);
        }
    }

    fn handle_open_a(&mut self, attrs_str: &str) {
        let href = extract_attr(attrs_str, "href").unwrap_or("").to_string();
        let title = extract_attr(attrs_str, "title").map(|s| s.to_string());
        self.push_inline(InlineKind::Link { href, title });
    }

    fn handle_open_pre(&mut self) {
        self.flush_paragraph();
        self.in_pre = true;
        self.pre_block = Some(PreBlock {
            language: None,
            text: String::new(),
        });
    }

    fn handle_open_blockquote(&mut self, attrs_str: &str) {
        self.flush_paragraph();
        let idx = self.builder.push_quote(None);
        if let Some(cite) = extract_attr(attrs_str, "cite") {
            let mut attrs = AHashMap::new();
            attrs.insert("cite".to_string(), cite.to_string());
            self.builder.set_attributes(idx, attrs);
        }
    }

    fn handle_open_ul(&mut self) {
        // Flush any pending parent `<li>` text against the still-current (outer)
        // list before descending, so it doesn't get misattributed to the list
        // we're about to push (see task #719).
        //
        // The item is flushed *before* the paragraph: while an `<li>` is open
        // `handle_text` buffers into `list_item_text`, so the item is the live
        // context and owns the pending annotations, which `flush_paragraph` would
        // otherwise discard on its way past an empty paragraph buffer (task #727).
        self.flush_list_item();
        self.flush_paragraph();
        let idx = self.push_list_node(false);
        self.list_stack.push(ListContext {
            node_idx: idx,
            item_open: false,
            last_item_idx: None,
        });
    }

    fn handle_open_ol(&mut self, attrs_str: &str) {
        self.flush_list_item();
        self.flush_paragraph();
        let idx = self.push_list_node(true);
        if let Some(start_val) = extract_attr(attrs_str, "start") {
            let mut attrs = AHashMap::new();
            attrs.insert("start".to_string(), start_val.to_string());
            self.builder.set_attributes(idx, attrs);
        }
        self.list_stack.push(ListContext {
            node_idx: idx,
            item_open: false,
            last_item_idx: None,
        });
    }

    fn handle_open_li(&mut self) {
        self.flush_list_item();
        self.in_list_item = true;
        self.list_item_text.clear();
        if let Some(ctx) = self.list_stack.last_mut() {
            ctx.item_open = true;
            ctx.last_item_idx = None;
        }
    }

    fn handle_open_table(&mut self) {
        if let Some(ref mut table) = self.table {
            self.nested_table_depth += 1;
            table.push_text(" ");
        } else {
            self.flush_paragraph();
            self.table = Some(TableAccumulator::new());
        }
    }

    fn handle_open_row_group(&mut self, tag: &str) {
        if tag == "tr"
            && self.nested_table_depth == 0
            && let Some(ref mut table) = self.table
        {
            table.open_row();
        }
    }

    fn handle_open_nested_cell(&mut self) {
        if let Some(ref mut table) = self.table {
            table.push_text(" ");
        }
    }

    fn handle_open_cell(&mut self, tag: &str, attrs_str: &str) {
        if let Some(ref mut table) = self.table {
            // Clamped at parse time so an out-of-range attribute (a hostile
            // `colspan="4294967295"`, say) never enters `CellMeta`/`GridCell` at
            // all, on top of the same clamp `grid_flatten::resolve_span_grid`
            // applies when it consumes these values — belt and suspenders, since
            // that helper also has to trust spans from an external crate it can't
            // control (see `extraction::grid_flatten` module docs). The bounds
            // themselves are the HTML Living Standard's own caps on these
            // attributes, not values we invented.
            let col_span = extract_attr(attrs_str, "colspan")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1)
                .clamp(1, crate::extraction::grid_flatten::MAX_COL_SPAN);
            let row_span = extract_attr(attrs_str, "rowspan")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1)
                .clamp(1, crate::extraction::grid_flatten::MAX_ROW_SPAN);
            table.open_cell(col_span, row_span, tag == "th");
        }
    }

    fn handle_open_img(&mut self, attrs_str: &str) {
        let alt = extract_attr(attrs_str, "alt");
        let src = extract_attr(attrs_str, "src").map(|s| s.to_string());
        let width = extract_attr(attrs_str, "width").map(|s| s.to_string());
        let height = extract_attr(attrs_str, "height").map(|s| s.to_string());

        if let Some(ref mut fig) = self.figure {
            fig.img_alt = alt.map(|s| s.to_string());
            fig.img_src = src;
            fig.img_width = width;
            fig.img_height = height;
        } else {
            self.flush_paragraph();
            let idx = self.builder.push_image_with_src(alt, src.as_deref(), None, None, None);
            if width.is_some() || height.is_some() {
                let mut attrs = AHashMap::new();
                if let Some(w) = width {
                    attrs.insert("width".to_string(), w);
                }
                if let Some(h) = height {
                    attrs.insert("height".to_string(), h);
                }
                self.builder.set_attributes(idx, attrs);
            }
        }
    }

    fn handle_open_figure(&mut self) {
        self.flush_paragraph();
        self.figure = Some(FigureContext {
            img_alt: None,
            img_src: None,
            img_width: None,
            img_height: None,
            caption: None,
            in_caption: false,
        });
    }

    fn handle_open_figcaption(&mut self) {
        if let Some(ref mut fig) = self.figure {
            fig.in_caption = true;
            fig.caption = Some(String::new());
        }
    }

    fn handle_open_dl(&mut self) {
        self.flush_paragraph();
        let idx = self.builder.push_definition_list(None);
        self.def_list = Some(DefListContext {
            list_idx: idx,
            current_term: None,
        });
    }

    fn handle_open_dt(&mut self) {
        self.flush_definition_item();
        self.in_dt = true;
        self.dt_text.clear();
    }

    fn handle_open_dd(&mut self) {
        self.in_dt = false;
        if let Some(ref mut dl) = self.def_list {
            let term = normalize_whitespace(&self.dt_text);
            if !term.is_empty() {
                dl.current_term = Some(term);
            }
        }
        self.dt_text.clear();
        self.in_dd = true;
        self.dd_text.clear();
    }

    fn handle_open_head(&mut self) {
        self.in_head = true;
        self.meta_entries.clear();
    }

    fn handle_open_meta(&mut self, attrs_str: &str) {
        let name = extract_attr(attrs_str, "name");
        let content_val = extract_attr(attrs_str, "content");
        if let (Some(n), Some(c)) = (name, content_val) {
            self.meta_entries.push((n.to_string(), c.to_string()));
        }
    }

    /// Start and end, from the walker's position, of the closing tag of a `<script>`, `<style>`,
    /// `<title>`, `<video>`, `<audio>` or `<math>`. Every unclosed one used to search the rest of
    /// the document for its closing tag again, so a run of them took quadratic time. A closing tag
    /// that is missing once is now not searched for again.
    fn find_close_tag(&mut self, tag: &str) -> Option<(usize, usize)> {
        if self.missing_close_tags.iter().any(|missing| missing == tag) {
            return None;
        }
        let found = find_closing_tag(&self.src[self.pos..], tag);
        if found.is_none() {
            self.missing_close_tags.push(tag.to_string());
        }
        found
    }

    fn handle_open_raw_block(&mut self, tag: &str) {
        if let Some((close_start, close_end)) = self.find_close_tag(tag) {
            let block_content = &self.src[self.pos..self.pos + close_start];
            self.pos += close_end;
            if !block_content.trim().is_empty() {
                self.builder.push_raw_block(tag, block_content.trim(), None);
            }
        }
    }

    fn handle_open_media(&mut self, tag: &str) {
        if let Some((_, close_end)) = self.find_close_tag(tag) {
            self.pos += close_end;
        }
    }

    fn handle_open_math(&mut self, attrs_str: &str, is_self_closing: bool) {
        self.flush_paragraph();
        if is_self_closing {
            return;
        }
        let Some((close_start, close_end)) = self.find_close_tag("math") else {
            return;
        };
        let inner = &self.src[self.pos..self.pos + close_start];
        let raw_xml = if attrs_str.is_empty() {
            format!("<math>{inner}</math>")
        } else {
            format!("<math {attrs_str}>{inner}</math>")
        };
        self.pos += close_end;
        if let Some(latex) = convert_math_subtree_to_latex(&raw_xml) {
            self.builder.push_formula(&latex, None);
        }
    }

    pub(super) fn handle_close_tag(&mut self, tag: &str, _tag_start: usize) {
        match tag {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => self.handle_close_heading(tag),
            "p" => {
                self.flush_paragraph();
            }
            "strong" | "b" => self.pop_inline(InlineKind::Bold),
            "em" | "i" => self.pop_inline(InlineKind::Italic),
            "code" => {
                if self.in_pre {
                } else {
                    self.pop_inline(InlineKind::Code);
                }
            }
            "u" | "ins" => self.pop_inline(InlineKind::Underline),
            "s" | "del" | "strike" => self.pop_inline(InlineKind::Strikethrough),
            "sub" => self.pop_inline(InlineKind::Subscript),
            "sup" => self.pop_inline(InlineKind::Superscript),
            "mark" => self.pop_inline(InlineKind::Highlight),
            "a" => {
                self.pop_inline_link();
            }
            "pre" => self.handle_close_pre(),
            "blockquote" => {
                self.flush_paragraph();
                self.builder.exit_container();
            }
            "ul" | "ol" => self.handle_close_list(),
            "li" => {
                self.flush_list_item();
                if let Some(ctx) = self.list_stack.last_mut() {
                    ctx.item_open = false;
                }
            }
            "table" if self.nested_table_depth > 0 => {
                self.nested_table_depth -= 1;
            }
            "table" => self.handle_close_table(),
            "tr" | "th" | "td" if self.nested_table_depth > 0 => {}
            "tr" => {
                if let Some(ref mut table) = self.table {
                    table.close_cell();
                    table.close_row();
                }
            }
            "th" | "td" => {
                if let Some(ref mut table) = self.table {
                    table.close_cell();
                }
            }
            "dl" => {
                self.flush_definition_item();
                self.def_list = None;
            }
            "dt" => {
                self.in_dt = false;
            }
            "dd" => {
                // `flush_definition_item` gates its `dd` branch on `self.in_dd` still being
                // `true` (it's what tells it there's a pending definition to push) — it must
                // run before that flag is cleared, or the definition item is silently
                // dropped (issue #127: this was the actual reason `<dl>/<dt>/<dd>` content
                // never reached `DefinitionItem` nodes at all).
                self.flush_definition_item();
                self.in_dd = false;
            }
            "figure" => self.handle_close_figure(),
            "figcaption" => {
                if let Some(ref mut fig) = self.figure {
                    fig.in_caption = false;
                }
            }
            "head" => self.handle_close_head(),
            "div" | "section" | "article" | "main" | "aside" | "header" | "footer" | "nav" | "details" | "summary" => {
                self.flush_paragraph();
            }
            _ => {}
        }
    }

    fn handle_close_heading(&mut self, tag: &str) {
        let level: u8 = tag[1..].parse().unwrap_or(1);
        let text = normalize_whitespace(&self.text_buf).trim().to_string();
        if !text.is_empty() {
            let idx = self.builder.push_heading(level, &text, None, None);
            if let Some(classes) = self.pending_classes.take() {
                let mut attrs = AHashMap::new();
                attrs.insert("class".to_string(), classes);
                self.builder.set_attributes(idx, attrs);
            }
        }
        self.text_buf.clear();
        self.annotations.clear();
        self.inline_stack.clear();
    }

    fn handle_close_pre(&mut self) {
        if let Some(pre) = self.pre_block.take() {
            let text = pre.text.trim_end_matches('\n').to_string();
            if !text.is_empty() {
                self.builder.push_code(&text, pre.language.as_deref(), None);
            }
        }
        self.in_pre = false;
    }

    fn handle_close_list(&mut self) {
        self.flush_list_item();
        self.list_stack.pop();
        // Content can resume in the enclosing `<li>` after a sublist closes
        // (`<li>before<ul>…</ul>after</li>`). Without restoring the flag that text
        // falls through to the paragraph buffer and is emitted as a bare paragraph
        // instead of staying list content (see task #721).
        self.in_list_item = self.list_stack.last().is_some_and(|ctx| ctx.item_open);
    }

    fn handle_close_table(&mut self) {
        if let Some(mut table) = self.table.take() {
            table.close_cell();
            table.close_row();
            if !table.rows.is_empty() {
                self.emit_table_with_spans(&table.rows);
            }
        }
    }

    fn handle_close_figure(&mut self) {
        let Some(fig) = self.figure.take() else {
            return;
        };
        // `alt` and `figcaption` are different content: alt is the image's text
        // alternative, the caption is prose about it. Preferring the caption with
        // `.or(alt)` dropped the alt text outright whenever a figure had both, so
        // it never reached any rendered output. Keep both when they differ. ~keep
        let caption = fig.caption.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let alt = fig.img_alt.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let combined;
        let desc = match (alt, caption) {
            (Some(alt_text), Some(caption_text)) if alt_text != caption_text => {
                combined = format!("{alt_text}. {caption_text}");
                Some(combined.as_str())
            }
            (Some(text), None) => Some(text),
            (_, Some(text)) => Some(text),
            (None, None) => None,
        };
        let idx = self
            .builder
            .push_image_with_src(desc, fig.img_src.as_deref(), None, None, None);
        let has_dims = fig.img_width.is_some() || fig.img_height.is_some();
        if has_dims {
            let mut attrs = AHashMap::new();
            if let Some(w) = fig.img_width {
                attrs.insert("width".to_string(), w);
            }
            if let Some(h) = fig.img_height {
                attrs.insert("height".to_string(), h);
            }
            self.builder.set_attributes(idx, attrs);
        }
    }

    fn handle_close_head(&mut self) {
        self.in_head = false;
        if !self.meta_entries.is_empty() {
            let entries = std::mem::take(&mut self.meta_entries);
            self.builder.push_metadata_block(entries, None);
        }
    }
}

/// Start and end of the first closing tag for `tag` in `html`. As in HTML, the name matches in any
/// ASCII case and may be followed by whitespace or `/` before the `>` (`</TITLE >`), so a later
/// exact `</title>`, such as an SVG's, is not taken for it. Each `</` is looked at once and the
/// whitespace after a name is not rescanned, so the search stays linear.
fn find_closing_tag(html: &str, tag: &str) -> Option<(usize, usize)> {
    let bytes = html.as_bytes();
    let mut from = 0;
    while let Some(offset) = html[from..].find("</") {
        let start = from + offset;
        let name_end = start + 2 + tag.len();
        if bytes
            .get(start + 2..name_end)
            .is_some_and(|name| name.eq_ignore_ascii_case(tag.as_bytes()))
        {
            let mut end = name_end;
            while bytes
                .get(end)
                .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'/')
            {
                end += 1;
            }
            if bytes.get(end) == Some(&b'>') {
                return Some((start, end + 1));
            }
            from = end;
        } else {
            from = start + 2;
        }
    }
    None
}
