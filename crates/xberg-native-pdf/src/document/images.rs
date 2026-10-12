//! Image and embedded-file extraction, including form-XObject traversal.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Check for circular references in the object graph.
    ///
    /// This is a diagnostic method that performs a depth-first search
    /// through the object graph to detect cycles.
    ///
    /// # Returns
    ///
    /// A vector of tuples representing edges that create cycles.
    /// Each tuple is (from_object, to_object) where to_object is
    /// already in the path when encountered again.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// let cycles = doc.check_for_circular_references();
    /// if !cycles.is_empty() {
    ///     println!("Found {} circular references", cycles.len());
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn check_for_circular_references(&self) -> Vec<(ObjectRef, ObjectRef)> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        let obj_nums: Vec<u32> = self.xref.entries.keys().copied().collect();
        for obj_num in obj_nums {
            let obj_ref = ObjectRef::new(obj_num, 0);
            if !visited.contains(&obj_ref) {
                self.dfs_check_cycles(obj_ref, &mut visited, &mut path, &mut cycles);
            }
        }

        cycles
    }

    /// Depth-first search helper for cycle detection.
    fn dfs_check_cycles(
        &self,
        obj_ref: ObjectRef,
        visited: &mut HashSet<ObjectRef>,
        path: &mut Vec<ObjectRef>,
        cycles: &mut Vec<(ObjectRef, ObjectRef)>,
    ) {
        if path.contains(&obj_ref) {
            if let Some(&prev) = path.last() {
                cycles.push((prev, obj_ref));
            }
            return;
        }

        if visited.contains(&obj_ref) {
            return;
        }

        visited.insert(obj_ref);
        path.push(obj_ref);

        if let Ok(obj) = self.load_object(obj_ref) {
            for ref_found in Self::find_references(&obj) {
                self.dfs_check_cycles(ref_found, visited, path, cycles);
            }
        }

        path.pop();
    }

    /// Find all object references within an object.
    pub(super) fn find_references(obj: &Object) -> Vec<ObjectRef> {
        let mut refs = Vec::new();

        match obj {
            Object::Reference(r) => refs.push(*r),
            Object::Array(arr) => {
                for item in arr {
                    refs.extend(Self::find_references(item));
                }
            }
            Object::Dictionary(dict) => {
                for value in dict.values() {
                    refs.extend(Self::find_references(value));
                }
            }
            Object::Stream { dict, .. } => {
                for value in dict.values() {
                    refs.extend(Self::find_references(value));
                }
            }
            _ => {}
        }

        refs
    }

    /// Extract images from a page.
    ///
    /// Extracts all images from the specified page by processing the content stream.
    /// This includes:
    /// - Images referenced via `Do` operators (XObject calls)
    /// - Images in nested Form XObjects (with recursion)
    /// - Inline images (BI...ID...EI sequences)
    ///
    /// This method processes PDF content streams instead of only iterating the XObject
    /// dictionary. This ensures that images referenced via the `Do` operator in the content
    /// stream are properly extracted, including those in nested Form XObjects. ColorSpace
    /// indirect references are also resolved.
    ///
    /// Returns a vector of PdfImage objects representing the extracted images.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    ///
    /// A vector of PdfImage objects, one for each image found on the page.
    ///
    /// # Errors
    ///
    /// Returns an error if the page cannot be accessed or if image extraction fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// let images = doc.extract_images(0)?;
    /// println!("Found {} images on page 1", images.len());
    /// for (i, image) in images.iter().enumerate() {
    ///     image.save_as_png(&format!("image_{}.png", i))?;
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn extract_images(&self, page_index: usize) -> Result<Vec<crate::extractors::PdfImage>> {
        self.require_authenticated()?;
        self.extract_images_filtered(page_index, &ImageExtractFilter::default())
    }

    /// Extract embedded files / attachments (WS1.8a, ISO 32000-1 §7.11.4).
    ///
    /// Walks the catalog's `/Names /EmbeddedFiles` name tree and returns
    /// `(filename, decoded bytes)` for each `/Filespec`'s `/EF /F` (or `/UF`)
    /// embedded-file stream. Complements the existing embedded-file writer.
    /// Returns an empty vector when the document has no attachments.
    pub fn extract_embedded_files(&self) -> Result<Vec<(String, Vec<u8>)>> {
        self.require_authenticated()?;
        let catalog = self.catalog()?;
        let Some(cat_dict) = catalog.as_dict() else {
            return Ok(Vec::new());
        };
        let names = cat_dict.get("Names").and_then(|n| self.resolve_object(n).ok());
        let Some(ef_root) = names
            .as_ref()
            .and_then(|n| n.as_dict())
            .and_then(|d| d.get("EmbeddedFiles"))
            .and_then(|e| self.resolve_object(e).ok())
        else {
            return Ok(Vec::new());
        };

        let mut filespecs: Vec<Object> = Vec::new();
        self.collect_embedded_filespecs(&ef_root, &mut filespecs, 0);

        let mut out = Vec::new();
        for fs in &filespecs {
            let Some(fs_dict) = fs.as_dict() else {
                continue;
            };
            let filename = fs_dict
                .get("UF")
                .or_else(|| fs_dict.get("F"))
                .and_then(|n| self.resolve_object(n).ok())
                .and_then(|n| n.as_string().map(|s| String::from_utf8_lossy(s).into_owned()))
                .unwrap_or_else(|| "attachment".to_string());
            let Some(ef) = fs_dict.get("EF").and_then(|e| self.resolve_object(e).ok()) else {
                continue;
            };
            let Some(ef_dict) = ef.as_dict() else {
                continue;
            };
            let Some(stream_ref) = ef_dict
                .get("F")
                .or_else(|| ef_dict.get("UF"))
                .and_then(|r| r.as_reference())
            else {
                continue;
            };
            if let Ok(stream_obj) = self.load_object(stream_ref)
                && let Ok(bytes) = self.decode_stream_with_encryption(&stream_obj, stream_ref)
            {
                out.push((filename, bytes));
            }
        }
        Ok(out)
    }

    /// Recursively collect `/Filespec` objects from an `/EmbeddedFiles`
    /// name-tree node: leaf `/Names [key filespec …]` pairs plus `/Kids`.
    fn collect_embedded_filespecs(&self, node: &Object, out: &mut Vec<Object>, depth: u8) {
        if depth > 32 {
            return;
        }
        let Ok(node) = self.resolve_object(node) else {
            return;
        };
        let Some(dict) = node.as_dict() else {
            return;
        };
        if let Some(names) = dict.get("Names").and_then(|n| self.resolve_object(n).ok())
            && let Some(arr) = names.as_array()
        {
            // Flat [key1 filespec1 key2 filespec2 …]: the odd indices. ~keep
            let mut i = 1;
            while i < arr.len() {
                if let Ok(fs) = self.resolve_object(&arr[i]) {
                    out.push(fs);
                }
                i += 2;
            }
        }
        if let Some(kids) = dict.get("Kids").and_then(|k| self.resolve_object(k).ok())
            && let Some(arr) = kids.as_array()
        {
            for kid in arr {
                self.collect_embedded_filespecs(kid, out, depth + 1);
            }
        }
    }

    /// Build the resource-name → colour-space-object map from a resolved
    /// `/Resources` dictionary's `/ColorSpace` subdictionary (§8.6.3 / §7.8.3),
    /// resolving one indirect-ref hop per entry so the stored value is a colour
    /// space name or array. Empty when there is no `/ColorSpace` subdictionary;
    /// the standard device names parse directly and need no entry. Consumed by
    /// the image-handle builders so `decode()` / the handle's `color_space` can
    /// resolve names like `/CS0` (§8.6.6, §8.9.7).
    fn build_color_space_map(&self, resources: Option<&Object>) -> std::collections::HashMap<String, Object> {
        let mut map = std::collections::HashMap::new();
        let Some(res) = resources else {
            return map;
        };
        let res = if let Some(r) = res.as_reference() {
            match self.load_object(r) {
                Ok(o) => o,
                Err(_) => return map,
            }
        } else {
            res.clone()
        };
        let Some(res_dict) = res.as_dict() else {
            return map;
        };
        let Some(cs_entry) = res_dict.get("ColorSpace") else {
            return map;
        };
        let cs_obj = if let Some(r) = cs_entry.as_reference() {
            match self.load_object(r) {
                Ok(o) => o,
                Err(_) => return map,
            }
        } else {
            cs_entry.clone()
        };
        let Some(cs_dict) = cs_obj.as_dict() else {
            return map;
        };
        for (name, value) in cs_dict.iter() {
            let resolved = if let Some(r) = value.as_reference() {
                self.load_object(r).unwrap_or_else(|_| value.clone())
            } else {
                value.clone()
            };
            map.insert(name.clone(), resolved);
        }
        map
    }

    /// Enumerate images on a page without decompressing any stream (Phase 1).
    ///
    /// Walks the page content stream once and reads image metadata (dimensions,
    /// colour space, filter chain, compressed size) directly from each Image
    /// XObject dictionary. No pixel data is decoded. Returns a handle per image
    /// in content-stream paint order.
    ///
    /// Call [`crate::PdfImageHandle::decode`] on individual handles to materialise only
    /// the images you need, or [`crate::PdfImageHandle::raw_compressed_bytes`] to forward
    /// compressed data (e.g. JPEG bytes) without recompression.
    ///
    /// Form XObjects (subtype `/Form`) are recursed into, matching the behaviour
    /// of [`PdfDocument::extract_images`]. Cycle detection (depth limit 100) and
    /// the document's Form stream cache are used. Images inside nested or shared
    /// Forms receive the correct final CTM-composed `bbox` / `rotation_degrees`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use xberg_native_pdf::PdfDocument;
    /// # let bytes = std::fs::read("page.pdf").unwrap();
    /// let doc = PdfDocument::from_bytes(bytes).unwrap();
    ///
    /// // Decode only images larger than a thumbnail threshold
    /// let images: Vec<_> = doc.page_image_handles(0)?
    ///     .into_iter()
    ///     .filter(|h| h.width >= 200 && h.height >= 200)
    ///     .map(|h| h.decode())
    ///     .collect::<Result<_, _>>()?;
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn page_image_handles(&self, page_index: usize) -> Result<Vec<crate::extractors::images::PdfImageHandle<'_>>> {
        use crate::content::Operator;
        use crate::content::parse_content_stream_images_only;
        use crate::extractors::images::image_handle_from_inline;

        self.require_authenticated()?;

        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;

        let content_data = self.get_page_content_data(page_index)?;

        let resources = match page_dict.get("Resources") {
            Some(res) => {
                if let Some(ref_obj) = res.as_reference() {
                    Some(self.load_object(ref_obj)?)
                } else {
                    Some(res.clone())
                }
            }
            None => None,
        };

        let operators = match parse_content_stream_images_only(&content_data) {
            Ok(ops) => ops,
            Err(_) => return Ok(Vec::new()),
        };

        // Resource-name colour-space map for this page scope (§8.6.6 / §8.9.7). ~keep
        let cs_map = self.build_color_space_map(resources.as_ref());

        let xobject_dict = if let Some(ref res) = resources {
            if let Some(res_dict) = res.as_dict() {
                if let Some(xobj_entry) = res_dict.get("XObject") {
                    let resolved = if let Some(ref_obj) = xobj_entry.as_reference() {
                        self.load_object(ref_obj)?
                    } else {
                        xobj_entry.clone()
                    };
                    resolved.as_dict().cloned()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let mut handles = Vec::new();
        let mut ctm_stack = vec![crate::content::Matrix::identity()];
        let mut paint_order: usize = 0;
        let mut xobject_stack: Vec<crate::object::ObjectRef> = Vec::new();

        for op in operators {
            match op {
                Operator::SaveState => {
                    if let Some(current) = ctm_stack.last() {
                        ctm_stack.push(*current);
                    }
                }
                Operator::RestoreState => {
                    if ctm_stack.len() > 1 {
                        ctm_stack.pop();
                    }
                }
                Operator::Cm { a, b, c, d, e, f } => {
                    if let Some(current) = ctm_stack.last_mut() {
                        let m = crate::content::Matrix { a, b, c, d, e, f };
                        *current = m.multiply(current);
                    }
                }
                Operator::Do { name } => {
                    if let Some(ref xobj_dict_map) = xobject_dict {
                        let ctm = ctm_stack
                            .last()
                            .copied()
                            .unwrap_or_else(crate::content::Matrix::identity);
                        if let Ok(mut more) = self.collect_handles_from_do(
                            &name,
                            xobj_dict_map,
                            resources.as_ref(),
                            ctm,
                            &mut paint_order,
                            &mut xobject_stack,
                        ) {
                            handles.append(&mut more);
                        }
                    }
                }
                Operator::InlineImage { dict, data } => {
                    let ctm = ctm_stack
                        .last()
                        .copied()
                        .unwrap_or_else(crate::content::Matrix::identity);
                    if let Some(handle) = image_handle_from_inline(self, &dict, data, ctm, paint_order, &cs_map) {
                        handles.push(handle);
                        paint_order += 1;
                    }
                }
                _ => {}
            }
        }

        Ok(handles)
    }

    /// Collect zero or more image handles for a `Do` operator.
    ///
    /// If the target is an Image XObject, returns a vec containing one handle
    /// (paint_order is advanced). If it is a Form XObject, recurses and returns
    /// all image handles found inside (including nested Forms), with correct
    /// paint_order and CTM composition for every handle.
    fn collect_handles_from_do<'s>(
        &'s self,
        name: &str,
        xobject_dict: &std::collections::HashMap<String, Object>,
        resources: Option<&Object>,
        ctm: crate::content::Matrix,
        paint_order: &mut usize,
        xobject_stack: &mut Vec<crate::object::ObjectRef>,
    ) -> Result<Vec<crate::extractors::images::PdfImageHandle<'s>>> {
        use crate::extractors::images::image_handle_from_xobject;

        let xobject_ref_obj = match xobject_dict.get(name) {
            Some(o) => o,
            None => return Ok(Vec::new()),
        };

        let xobject_ref_opt = xobject_ref_obj.as_reference();
        let xobject = if let Some(ref_obj) = xobject_ref_opt {
            self.load_object(ref_obj)?
        } else {
            xobject_ref_obj.clone()
        };
        let xobj_dict = match xobject.as_dict() {
            Some(d) => d,
            None => return Ok(Vec::new()),
        };

        let subtype = xobj_dict.get("Subtype").and_then(|s| s.as_name()).unwrap_or("");

        match subtype {
            "Image" => {
                if let Some(ref_obj) = xobject_ref_opt {
                    let cs_map = self.build_color_space_map(resources);
                    match image_handle_from_xobject(self, ref_obj, xobj_dict, ctm, *paint_order, &cs_map) {
                        Some(h) => {
                            *paint_order += 1;
                            let mut handles = vec![h];
                            handles.extend(self.collect_referenced_mask_handles(xobj_dict, ctm, paint_order, &cs_map));
                            Ok(handles)
                        }
                        _ => Ok(Vec::new()),
                    }
                } else {
                    Ok(Vec::new())
                }
            }
            "Form" => {
                if let (Some(ref_obj), Some(parent_res)) = (xobject_ref_opt, resources) {
                    self.collect_image_handles_from_form_xobject(
                        ref_obj,
                        &xobject,
                        parent_res,
                        ctm,
                        paint_order,
                        xobject_stack,
                    )
                } else {
                    Ok(Vec::new())
                }
            }
            _ => Ok(Vec::new()),
        }
    }

    fn collect_referenced_mask_handles<'s>(
        &'s self,
        image_dict: &std::collections::HashMap<String, Object>,
        ctm: crate::content::Matrix,
        paint_order: &mut usize,
        color_spaces: &std::collections::HashMap<String, Object>,
    ) -> Vec<crate::extractors::images::PdfImageHandle<'s>> {
        use crate::extractors::images::image_handle_from_xobject;

        let mut handles = Vec::new();
        for key in ["Mask", "SMask"] {
            let Some(mask_ref) = image_dict.get(key).and_then(Object::as_reference) else {
                continue;
            };
            let Ok(mask) = self.load_object(mask_ref) else {
                continue;
            };
            let Some(mask_dict) = mask.as_dict() else {
                continue;
            };
            if mask_dict.get("Subtype").and_then(Object::as_name) != Some("Image") {
                continue;
            }
            if let Some(handle) = image_handle_from_xobject(self, mask_ref, mask_dict, ctm, *paint_order, color_spaces)
            {
                handles.push(handle);
                *paint_order += 1;
            }
        }
        handles
    }

    /// Recursively collect image handles from a Form XObject.
    ///
    /// This is the handles-side equivalent of `extract_images_from_form_xobject`.
    /// It uses the same cycle detection (ObjectRef stack + depth 100), the same
    /// Form Resources fallback rules, the same Form /Matrix handling, and reuses
    /// the document's xobject_stream_cache (50 MiB bound) for decompressed Form
    /// content.
    ///
    /// Unlike the materialised path, we do not cache "raw" handles — we compose
    /// the full CTM (`parent_ctm * form_matrix`) at entry and let every inner
    /// handle (and nested Form) naturally receive the final geometry. This is
    /// simpler for the two-phase API and produces correct `bbox`/`rotation_degrees`
    /// / `ctm` fields on the returned handles.
    fn collect_image_handles_from_form_xobject<'s>(
        &'s self,
        xobject_ref: crate::object::ObjectRef,
        xobject: &Object,
        parent_resources: &Object,
        parent_ctm: crate::content::Matrix,
        paint_order: &mut usize,
        xobject_stack: &mut Vec<crate::object::ObjectRef>,
    ) -> Result<Vec<crate::extractors::images::PdfImageHandle<'s>>> {
        use crate::content::Operator;
        use crate::content::parse_content_stream_images_only;
        use crate::extractors::images::image_handle_from_inline;

        // Cycle detection — identical policy to the materialised extraction path. ~keep
        if xobject_stack.contains(&xobject_ref) || xobject_stack.len() >= 100 {
            return Ok(Vec::new());
        }

        xobject_stack.push(xobject_ref);

        let xobj_dict = match xobject.as_dict() {
            Some(d) => d,
            None => {
                xobject_stack.pop();
                return Ok(Vec::new());
            }
        };

        let form_resources = if let Some(form_res) = xobj_dict.get("Resources") {
            if let Some(ref_obj) = form_res.as_reference() {
                self.load_object(ref_obj)?
            } else {
                form_res.clone()
            }
        } else {
            parent_resources.clone()
        };

        let form_xobject_dict = if let Some(res_dict) = form_resources.as_dict() {
            if let Some(xobj_entry) = res_dict.get("XObject") {
                let resolved = if let Some(ref_obj) = xobj_entry.as_reference() {
                    self.load_object(ref_obj)?
                } else {
                    xobj_entry.clone()
                };
                resolved.as_dict().cloned()
            } else {
                None
            }
        } else {
            None
        };

        let form_matrix = if let Some(matrix_obj) = xobj_dict.get("Matrix") {
            self.parse_matrix_from_object(matrix_obj)
                .unwrap_or_else(crate::content::Matrix::identity)
        } else {
            crate::content::Matrix::identity()
        };

        let cached_stream = self.xobject_stream_cache.lock_or_recover().get(&xobject_ref).cloned();
        let stream_data = if let Some(cached) = cached_stream {
            cached.as_ref().clone()
        } else {
            match self.decode_stream_with_encryption(xobject, xobject_ref) {
                Ok(data) => {
                    const MAX_STREAM_CACHE_BYTES: usize = 50 * 1024 * 1024;
                    let current_bytes = self.xobject_stream_cache_bytes.load(Ordering::Relaxed);
                    if current_bytes + data.len() <= MAX_STREAM_CACHE_BYTES {
                        self.xobject_stream_cache_bytes
                            .store(current_bytes + data.len(), Ordering::Relaxed);
                        self.xobject_stream_cache
                            .lock_or_recover()
                            .insert(xobject_ref, std::sync::Arc::new(data.clone()));
                    }
                    data
                }
                Err(error) => {
                    tracing::warn!(target: LOG_TARGET,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "failed to decode Form XObject stream; skipping"
                    );
                    xobject_stack.pop();
                    return Ok(Vec::new());
                }
            }
        };

        let operators = match parse_content_stream_images_only(&stream_data) {
            Ok(ops) => ops,
            Err(_) => {
                xobject_stack.pop();
                return Ok(Vec::new());
            }
        };

        // Critical CTM composition (ISO 32000-1 §8.10.1): the form /Matrix is applied
        // before the CTM in effect at `Do`. `a.multiply(b)` applies `a` first, so this is
        // `form_matrix.multiply(parent_ctm)`; the reverse scales the page translation
        // by the form matrix (GH#2103).
        // Start the form's internal graphics state with that composition.
        // Every image (and nested Form) discovered inside will then have its
        // handle's bbox/rotation/ctm computed with the *final* transform that
        // will be active when the image is painted on the page. ~keep
        let start_ctm = form_matrix.multiply(&parent_ctm);
        let mut ctm_stack = vec![start_ctm];
        let mut handles = Vec::new();

        for op in operators {
            match op {
                Operator::SaveState => {
                    if let Some(current) = ctm_stack.last() {
                        ctm_stack.push(*current);
                    }
                }
                Operator::RestoreState => {
                    if ctm_stack.len() > 1 {
                        ctm_stack.pop();
                    }
                }
                Operator::Cm { a, b, c, d, e, f } => {
                    if let Some(current) = ctm_stack.last_mut() {
                        let m = crate::content::Matrix { a, b, c, d, e, f };
                        *current = m.multiply(current);
                    }
                }

                Operator::Do { name } => {
                    if let Some(ref xobj_d) = form_xobject_dict {
                        let current_ctm = ctm_stack
                            .last()
                            .copied()
                            .unwrap_or_else(crate::content::Matrix::identity);
                        if let Ok(mut more) = self.collect_handles_from_do(
                            &name,
                            xobj_d,
                            Some(&form_resources),
                            current_ctm,
                            paint_order,
                            xobject_stack,
                        ) {
                            handles.append(&mut more);
                        }
                    }
                }

                Operator::InlineImage { dict, data } => {
                    let current_ctm = ctm_stack
                        .last()
                        .copied()
                        .unwrap_or_else(crate::content::Matrix::identity);
                    let cs_map = self.build_color_space_map(Some(&form_resources));
                    if let Some(h) = image_handle_from_inline(self, &dict, data, current_ctm, *paint_order, &cs_map) {
                        handles.push(h);
                        *paint_order += 1;
                    }
                }

                _ => {}
            }
        }

        xobject_stack.pop();
        Ok(handles)
    }

    /// Extract images with pre-decompression filtering.
    ///
    /// Applies dimension and pixel-count checks using XObject dictionary metadata
    /// BEFORE expensive stream decompression. This avoids decompressing oversized
    /// images (e.g., 36MP presentation slides) or tiny glyph fragments that will
    /// be discarded downstream.
    fn extract_images_filtered(
        &self,
        page_index: usize,
        filter: &ImageExtractFilter,
    ) -> Result<Vec<crate::extractors::PdfImage>> {
        use crate::content::Operator;
        use crate::content::parse_content_stream_images_only;

        let page = self.get_page(page_index)?;
        let page_dict = page.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Page is not a dictionary".to_string(),
        })?;

        let content_data = self.get_page_content_data(page_index)?;

        let resources = match page_dict.get("Resources") {
            Some(res) => {
                if let Some(ref_obj) = res.as_reference() {
                    Some(self.load_object(ref_obj)?)
                } else {
                    Some(res.clone())
                }
            }
            None => None,
        };

        // Parse content stream with image-only fast path (skips BT/ET text blocks) ~keep
        let operators = match parse_content_stream_images_only(&content_data) {
            Ok(ops) => ops,
            Err(_) => {
                return Ok(Vec::new());
            }
        };

        let mut images = Vec::new();
        let mut ctm_stack = vec![crate::content::Matrix::identity()];
        // Shared cycle detection stack for Form XObject recursion.
        // This must persist across all Do operator calls to detect circular references
        // (e.g., Form X0 references X1 which references X0). ~keep
        let mut xobject_stack = Vec::new();

        // Pre-resolve XObject dictionary once (avoids re-resolving per Do operator) ~keep
        let xobject_dict = if let Some(ref res) = resources {
            if let Some(res_dict) = res.as_dict() {
                if let Some(xobj_entry) = res_dict.get("XObject") {
                    let resolved = if let Some(ref_obj) = xobj_entry.as_reference() {
                        self.load_object(ref_obj)?
                    } else {
                        xobj_entry.clone()
                    };
                    resolved.as_dict().cloned()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        for op in operators {
            match op {
                Operator::SaveState => {
                    if let Some(current_ctm) = ctm_stack.last() {
                        ctm_stack.push(*current_ctm);
                    }
                }
                Operator::RestoreState => {
                    if ctm_stack.len() > 1 {
                        ctm_stack.pop();
                    }
                }
                Operator::Cm { a, b, c, d, e, f } => {
                    if let Some(current_ctm) = ctm_stack.last_mut() {
                        let matrix = crate::content::Matrix { a, b, c, d, e, f };
                        // PDF spec ISO 32000-1:2008 §8.3.4: cm concatenates as M_cm × CTM ~keep
                        *current_ctm = matrix.multiply(current_ctm);
                    }
                }

                Operator::Do { name } => {
                    if let Some(ref xobj_dict) = xobject_dict {
                        let current_ctm = ctm_stack
                            .last()
                            .copied()
                            .unwrap_or_else(crate::content::Matrix::identity);
                        if let Ok(mut xobj_images) = self.extract_images_from_xobject_do(
                            &name,
                            xobj_dict,
                            resources.as_ref(),
                            current_ctm,
                            &mut xobject_stack,
                            filter,
                        ) {
                            images.append(&mut xobj_images);
                        }
                    }
                }

                Operator::InlineImage { dict, data } => {
                    let current_ctm = ctm_stack
                        .last()
                        .copied()
                        .unwrap_or_else(crate::content::Matrix::identity);
                    if let Ok(image) = self.extract_image_from_inline(&dict, &data, current_ctm) {
                        images.push(image);
                    }
                }

                _ => {}
            }
        }

        Ok(images)
    }

    /// Extract images referenced by a Do operator in the content stream.
    ///
    /// Accepts a pre-resolved XObject dictionary to avoid redundant lookups
    /// when called repeatedly (e.g., 194 Do operators on a single page).
    fn extract_images_from_xobject_do(
        &self,
        name: &str,
        xobject_dict: &std::collections::HashMap<String, Object>,
        resources: Option<&Object>,
        ctm: crate::content::Matrix,
        xobject_stack: &mut Vec<ObjectRef>,
        filter: &ImageExtractFilter,
    ) -> Result<Vec<crate::extractors::PdfImage>> {
        use crate::extractors::extract_image_from_xobject;

        let mut images = Vec::new();

        let xobject_ref_obj = match xobject_dict.get(name) {
            Some(obj) => obj,
            None => return Ok(images),
        };

        let xobject_ref_opt = xobject_ref_obj.as_reference();
        let xobject = if let Some(ref_obj) = xobject_ref_opt {
            self.load_object(ref_obj)?
        } else {
            xobject_ref_obj.clone()
        };
        let xobject_dict = xobject.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "XObject is not a dictionary".to_string(),
        })?;

        let subtype = xobject_dict.get("Subtype").and_then(|s| s.as_name()).unwrap_or("");

        match subtype {
            "Image" => {
                // Pre-decompression filtering using dictionary metadata.
                // These checks use Width/Height/ColorSpace from the XObject dictionary
                // which are available WITHOUT decompressing the image stream data.
                // /Width and /Height may themselves be indirect references
                // (ISO 32000-1 §7.3.10); resolve them the same way
                // `extract_image_from_xobject` does, so an indirect width
                // doesn't fall through to the `unwrap_or(0)` default and get
                // silently filtered out by the `min_width`/`min_height` gate
                // below. ~keep
                let resolve_int = |o: &Object| -> Option<i64> {
                    match o.as_reference() {
                        Some(r) => self.load_object(r).ok().and_then(|v| v.as_integer()),
                        None => o.as_integer(),
                    }
                };
                let w = xobject_dict.get("Width").and_then(resolve_int).unwrap_or(0);
                let h = xobject_dict.get("Height").and_then(resolve_int).unwrap_or(0);
                if w < filter.min_width || h < filter.min_height {
                    return Ok(images);
                }
                if (w as u64) * (h as u64) > filter.max_pixels {
                    return Ok(images);
                }
                // Skip small Indexed colorspace images (Type3 font glyph fragments) ~keep
                if filter.skip_indexed_small > 0
                    && (w < filter.skip_indexed_small || h < filter.skip_indexed_small)
                    && let Some(cs_obj) = xobject_dict.get("ColorSpace")
                {
                    let is_indexed = match cs_obj {
                        Object::Name(n) => n == "Indexed",
                        Object::Array(arr) if !arr.is_empty() => arr[0].as_name() == Some("Indexed"),
                        _ => false,
                    };
                    if is_indexed {
                        return Ok(images);
                    }
                }

                // Only clone+modify when ColorSpace needs resolving from indirect ref ~keep
                let needs_cs_resolve = matches!(
                    &xobject,
                    Object::Stream { dict, .. } if matches!(dict.get("ColorSpace"), Some(Object::Reference(_)))
                );

                let resolved_xobject;
                let xobject_for_extract = if needs_cs_resolve {
                    if let Object::Stream { dict, data } = &xobject {
                        let mut new_dict = dict.clone();
                        if let Some(Object::Reference(cs_ref)) = dict.get("ColorSpace")
                            && let Ok(resolved_cs) = self.load_object(*cs_ref)
                        {
                            new_dict.insert("ColorSpace".to_string(), resolved_cs);
                        }
                        resolved_xobject = Object::Stream {
                            dict: new_dict,
                            data: data.clone(),
                        };
                        &resolved_xobject
                    } else {
                        &xobject
                    }
                } else {
                    &xobject
                };

                if let Ok(mut image) =
                    extract_image_from_xobject(Some(self), xobject_for_extract, xobject_ref_opt, None)
                {
                    // In PDF, images are mapped from unit square (0,0 to 1,1) to the CTM. ~keep
                    let unit_rect = crate::geometry::Rect::new(0.0, 0.0, 1.0, 1.0);
                    let bbox = self.transform_bbox_with_ctm(&unit_rect, ctm);
                    image.set_bbox(bbox);

                    image.set_matrix([ctm.a, ctm.b, ctm.c, ctm.d, ctm.e, ctm.f]);
                    image.set_rotation_degrees(Self::matrix_to_rotation(ctm));

                    images.push(image);
                }
            }
            "Form" => {
                if let (Some(ref_obj), Some(parent_res)) = (xobject_ref_opt, resources)
                    && let Ok(mut form_images) =
                        self.extract_images_from_form_xobject(ref_obj, &xobject, parent_res, ctm, xobject_stack, filter)
                {
                    images.append(&mut form_images);
                }
            }
            _ => {}
        }

        Ok(images)
    }

    /// Recursively extract images from a Form XObject.
    ///
    /// Uses a document-level cache: images are extracted once using only the Form's
    /// own Matrix, then cached. On subsequent references, cached images are cloned
    /// and the caller's CTM is applied to transform bboxes.
    fn extract_images_from_form_xobject(
        &self,
        xobject_ref: ObjectRef,
        xobject: &Object,
        parent_resources: &Object,
        parent_ctm: crate::content::Matrix,
        xobject_stack: &mut Vec<ObjectRef>,
        filter: &ImageExtractFilter,
    ) -> Result<Vec<crate::extractors::PdfImage>> {
        use crate::content::Operator;
        use crate::content::parse_content_stream_images_only;

        if xobject_stack.contains(&xobject_ref) || xobject_stack.len() >= 100 {
            return Ok(Vec::new());
        }

        // Check image result cache — images stored with Form's own Matrix only.
        // Scope the borrow to ensure it's dropped before potential recursion. ~keep
        {
            if let Some(cached_images) = self.form_xobject_images_cache.lock_or_recover().get(&xobject_ref) {
                let images = cached_images
                    .iter()
                    .map(|img| {
                        let mut cloned = img.clone();
                        if let Some(rect) = cloned.bbox() {
                            cloned.set_bbox(self.transform_bbox_with_ctm(rect, parent_ctm));
                        }
                        cloned
                    })
                    .collect();
                return Ok(images);
            }
        }

        xobject_stack.push(xobject_ref);

        let xobj_dict = xobject.as_dict().ok_or_else(|| Error::ParseError {
            offset: 0,
            reason: "Form XObject is not a dictionary".to_string(),
        })?;

        let form_resources = if let Some(form_res) = xobj_dict.get("Resources") {
            if let Some(ref_obj) = form_res.as_reference() {
                self.load_object(ref_obj)?
            } else {
                form_res.clone()
            }
        } else {
            parent_resources.clone()
        };

        let form_xobject_dict = if let Some(res_dict) = form_resources.as_dict() {
            if let Some(xobj_entry) = res_dict.get("XObject") {
                let resolved = if let Some(ref_obj) = xobj_entry.as_reference() {
                    self.load_object(ref_obj)?
                } else {
                    xobj_entry.clone()
                };
                resolved.as_dict().cloned()
            } else {
                None
            }
        } else {
            None
        };

        let form_matrix = if let Some(matrix_obj) = xobj_dict.get("Matrix") {
            self.parse_matrix_from_object(matrix_obj)
                .unwrap_or_else(crate::content::Matrix::identity)
        } else {
            crate::content::Matrix::identity()
        };

        let cached_stream = self.xobject_stream_cache.lock_or_recover().get(&xobject_ref).cloned();
        let stream_data = if let Some(cached) = cached_stream {
            cached.as_ref().clone()
        } else {
            match self.decode_stream_with_encryption(xobject, xobject_ref) {
                Ok(data) => {
                    const MAX_STREAM_CACHE_BYTES: usize = 50 * 1024 * 1024;
                    let current_bytes = self.xobject_stream_cache_bytes.load(Ordering::Relaxed);
                    if current_bytes + data.len() <= MAX_STREAM_CACHE_BYTES {
                        self.xobject_stream_cache_bytes
                            .store(current_bytes + data.len(), Ordering::Relaxed);
                        self.xobject_stream_cache
                            .lock_or_recover()
                            .insert(xobject_ref, std::sync::Arc::new(data.clone()));
                    }
                    data
                }
                Err(error) => {
                    tracing::warn!(target: LOG_TARGET,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "failed to decode Form XObject stream; skipping"
                    );
                    xobject_stack.pop();
                    return Ok(Vec::new());
                }
            }
        };

        // Parse operators using fast image-only path (skips text operators) ~keep
        let operators = match parse_content_stream_images_only(&stream_data) {
            Ok(ops) => ops,
            Err(_) => {
                xobject_stack.pop();
                return Ok(Vec::new());
            }
        };

        // Extract using only the Form's own Matrix (no parent_ctm yet).
        // This allows caching the results and applying different parent CTMs later. ~keep
        let mut raw_images = Vec::new();
        let mut ctm_stack = vec![form_matrix];

        for op in operators {
            match op {
                Operator::SaveState => {
                    if let Some(current_ctm) = ctm_stack.last() {
                        ctm_stack.push(*current_ctm);
                    }
                }
                Operator::RestoreState => {
                    if ctm_stack.len() > 1 {
                        ctm_stack.pop();
                    }
                }
                Operator::Cm { a, b, c, d, e, f } => {
                    if let Some(current_ctm) = ctm_stack.last_mut() {
                        let matrix = crate::content::Matrix { a, b, c, d, e, f };
                        // PDF spec ISO 32000-1:2008 §8.3.4: cm concatenates as M_cm × CTM ~keep
                        *current_ctm = matrix.multiply(current_ctm);
                    }
                }

                Operator::Do { name } => {
                    if let Some(ref xobj_d) = form_xobject_dict {
                        let current_ctm = ctm_stack
                            .last()
                            .copied()
                            .unwrap_or_else(crate::content::Matrix::identity);
                        // For nested Do operators, pass identity as parent_ctm since
                        // we're building raw (un-transformed) images for caching ~keep
                        if let Ok(mut xobj_images) = self.extract_images_from_xobject_do(
                            &name,
                            xobj_d,
                            Some(&form_resources),
                            current_ctm,
                            xobject_stack,
                            filter,
                        ) {
                            raw_images.append(&mut xobj_images);
                        }
                    }
                }

                Operator::InlineImage { dict, data } => {
                    let current_ctm = ctm_stack
                        .last()
                        .copied()
                        .unwrap_or_else(crate::content::Matrix::identity);
                    if let Ok(image) = self.extract_image_from_inline(&dict, &data, current_ctm) {
                        raw_images.push(image);
                    }
                }

                _ => {}
            }
        }

        xobject_stack.pop();

        // Cache the raw images (with Form's own Matrix applied, but no parent CTM) ~keep
        self.form_xobject_images_cache
            .lock_or_recover()
            .insert(xobject_ref, raw_images.clone());

        let images = raw_images
            .into_iter()
            .map(|mut img| {
                if let Some(rect) = img.bbox() {
                    img.set_bbox(self.transform_bbox_with_ctm(rect, parent_ctm));
                }
                img
            })
            .collect();

        Ok(images)
    }

    /// Extract an inline image from the content stream.
    fn extract_image_from_inline(
        &self,
        dict: &std::collections::HashMap<String, Object>,
        data: &[u8],
        ctm: crate::content::Matrix,
    ) -> Result<crate::extractors::PdfImage> {
        use crate::extractors::expand_inline_image_dict;

        let expanded_dict = expand_inline_image_dict(dict.clone());

        let stream_obj = Object::Stream {
            dict: expanded_dict,
            data: bytes::Bytes::copy_from_slice(data),
        };

        let mut image = crate::extractors::extract_image_from_xobject(Some(self), &stream_obj, None, None)?;

        // In PDF, images are mapped from unit square (0,0 to 1,1) to the CTM. ~keep
        let unit_rect = crate::geometry::Rect::new(0.0, 0.0, 1.0, 1.0);
        let bbox = self.transform_bbox_with_ctm(&unit_rect, ctm);
        image.set_bbox(bbox);

        image.set_matrix([ctm.a, ctm.b, ctm.c, ctm.d, ctm.e, ctm.f]);
        image.set_rotation_degrees(Self::matrix_to_rotation(ctm));

        Ok(image)
    }

    /// Helper to derive rotation angle from transformation matrix.
    fn matrix_to_rotation(m: crate::content::Matrix) -> i32 {
        let angle_rad = m.b.atan2(m.a);
        let angle_deg = (angle_rad.to_degrees().round() as i32) % 360;
        if angle_deg < 0 { angle_deg + 360 } else { angle_deg }
    }

    /// Transform a bounding box using CTM.
    ///
    /// Transforms all four corners and computes the axis-aligned bounding box,
    /// which correctly handles rotation, shear, and negative scaling.
    pub(super) fn transform_bbox_with_ctm(
        &self,
        rect: &crate::geometry::Rect,
        ctm: crate::content::Matrix,
    ) -> crate::geometry::Rect {
        let x0 = rect.x;
        let y0 = rect.y;
        let x1 = rect.x + rect.width;
        let y1 = rect.y + rect.height;

        let tx0 = ctm.a * x0 + ctm.c * y0 + ctm.e;
        let ty0 = ctm.b * x0 + ctm.d * y0 + ctm.f;

        let tx1 = ctm.a * x1 + ctm.c * y0 + ctm.e;
        let ty1 = ctm.b * x1 + ctm.d * y0 + ctm.f;

        let tx2 = ctm.a * x0 + ctm.c * y1 + ctm.e;
        let ty2 = ctm.b * x0 + ctm.d * y1 + ctm.f;

        let tx3 = ctm.a * x1 + ctm.c * y1 + ctm.e;
        let ty3 = ctm.b * x1 + ctm.d * y1 + ctm.f;

        let min_x = tx0.min(tx1).min(tx2).min(tx3);
        let max_x = tx0.max(tx1).max(tx2).max(tx3);
        let min_y = ty0.min(ty1).min(ty2).min(ty3);
        let max_y = ty0.max(ty1).max(ty2).max(ty3);

        crate::geometry::Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    /// Parse a Matrix object from PDF.
    pub(super) fn parse_matrix_from_object(&self, obj: &Object) -> Option<crate::content::Matrix> {
        if let Some(array) = obj.as_array()
            && array.len() >= 6
        {
            let mut values = [0.0f32; 6];
            for (i, val) in array.iter().take(6).enumerate() {
                let num = if let Some(f) = val.as_real() {
                    f as f32
                } else {
                    let i_val = val.as_integer()?;
                    i_val as f32
                };
                values[i] = num;
            }

            return Some(crate::content::Matrix {
                a: values[0],
                b: values[1],
                c: values[2],
                d: values[3],
                e: values[4],
                f: values[5],
            });
        }
        None
    }

    /// Extract images from a page and save them to files.
    ///
    /// Each image is saved as a separate file in `output_dir` with the given
    /// `prefix` and an incrementing index starting from `start_index`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn extract_images_to_files(
        &self,
        page_index: usize,
        output_dir: impl AsRef<Path>,
        prefix: Option<&str>,
        start_index: Option<usize>,
    ) -> Result<Vec<ExtractedImageRef>> {
        use std::fs;

        let images = self.extract_images(page_index)?;

        let output_dir = output_dir.as_ref();
        if !output_dir.exists() {
            fs::create_dir_all(output_dir).map_err(Error::Io)?;
        }

        let prefix = prefix.unwrap_or("img");
        let mut index = start_index.unwrap_or(1);
        let mut result = Vec::new();

        for image in images {
            let (format, extension) = match image.data() {
                crate::extractors::ImageData::Jpeg(_) => (ImageFormat::Jpeg, "jpg"),
                _ => (ImageFormat::Png, "png"),
            };

            let filename = format!("{}_{:03}.{}", prefix, index, extension);
            let filepath = output_dir.join(&filename);

            match format {
                ImageFormat::Jpeg => image.save_as_jpeg(&filepath)?,
                ImageFormat::Png => image.save_as_png(&filepath)?,
            }

            result.push(ExtractedImageRef {
                filename,
                format,
                width: image.width(),
                height: image.height(),
                bbox: image.bbox().cloned(),
                rotation: image.rotation_degrees(),
                matrix: image.matrix(),
            });

            index += 1;
        }

        Ok(result)
    }

    /// Public wrapper for `get_page` (normally private).
    /// Exposed for profiling examples that need to time page tree lookup separately.
    pub fn get_page_for_debug(&self, page_index: usize) -> Result<Object> {
        self.get_page(page_index)
    }

    /// Public wrapper for `may_contain_text` (normally pub(crate)).
    /// Returns true if the content stream might contain text operators (BT or Do).
    pub fn may_contain_text_public(data: &[u8]) -> bool {
        Self::may_contain_text(data)
    }
}
