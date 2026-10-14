//! Opening and decryption; page and document classification lives in `classify.rs`.
//!
//! Split out of the parent's single 18,544-line `impl PdfDocument`, which made
//! `document.rs` 1.2 MiB and tripped the 500 KiB file-safety limit. A child module's
//! `impl` is the same inherent impl and sees the parent's private items unchanged. ~keep

use super::*;

impl PdfDocument {
    /// Open a PDF document from in-memory bytes.
    ///
    /// This is the primary constructor for cases where
    /// the PDF data is already fully loaded in memory. This parses the PDF by
    /// wrapping the bytes in a memory reader and delegating to internal parsers.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF data is invalid, unsupported, or cannot be parsed.
    #[tracing::instrument(name = "pdf.from_bytes", skip_all, fields(bytes = data.len()))]
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        match Self::open_from_bytes_inner(data) {
            Ok(document) => Ok(document),
            Err(error) => {
                trace_open_error(&error);
                Err(error)
            }
        }
    }

    /// Deprecated alias for `from_bytes`.
    #[deprecated(since = "0.3.15", note = "Use `from_bytes` instead")]
    pub fn open_from_bytes(data: Vec<u8>) -> Result<Self> {
        Self::from_bytes(data)
    }

    /// Open a PDF document from a file path.
    ///
    /// Reads the entire file into memory, then parses the PDF structure.
    /// This is the standard constructor for desktop/server environments.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be opened or read
    /// - The PDF header is invalid
    /// - The cross-reference table is corrupted
    /// - The trailer dictionary is invalid
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xberg_native_pdf::document::PdfDocument;
    ///
    /// let doc = PdfDocument::open("sample.pdf")?;
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    #[tracing::instrument(name = "pdf.open", skip_all, fields(path = %path.as_ref().display()))]
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        // Read once and route through `from_bytes` so the in-memory
        // `source_bytes` field is populated. Path-loaded documents that
        // skip this lose access to APIs that re-read the bytes
        // (notably `compliance::convert_to_pdf_a`, which constructs a
        // `DocumentEditor` from `source_bytes` — an empty Vec breaks
        // it with `"Invalid PDF header: ... File is empty"`).
        //
        // The doc comment on this function already promised "Reads the
        // entire file into memory"; this is making it true. ~keep
        let data = match std::fs::read(path.as_ref()) {
            Ok(data) => data,
            Err(error) => {
                let error = Error::Io(error);
                trace_open_error(&error);
                return Err(error);
            }
        };
        Self::from_bytes(data)
    }

    /// Parse `data` into a document. The reader built here lives only for the
    /// header, xref and trailer parse; the finished document keeps the bytes and
    /// reads them by offset instead. ~keep
    fn open_from_bytes_inner(data: Vec<u8>) -> Result<Self> {
        let mut reader = PdfReader::Memory(BufReader::new(Cursor::new(data)));
        // Parse header with lenient mode by default (handle PDFs with binary prefixes) ~keep
        let (major, minor, header_offset) = parse_header(&mut reader, true)?;
        let version = (major, minor);

        let (mut xref, trailer, mut xref_reconstructed, mut synthetic_objects) =
            Self::load_initial_xref_and_trailer(&mut reader)?;

        if header_offset > 0 {
            Self::adjust_xref_for_header_offset(&mut reader, &mut xref, &trailer, header_offset);
        }

        // Validate the /Root catalog is actually loadable. If not, the xref data is
        // corrupt despite parsing successfully — fall back to reconstruction. ~keep
        let (xref, trailer) = if validate_root_loadable(&mut reader, &xref, &trailer) {
            (xref, trailer)
        } else {
            tracing::warn!(target: LOG_TARGET, "Root object not loadable after xref parse, falling back to xref reconstruction");
            match Self::try_reconstruct_xref(&mut reader) {
                Ok((x, t, syn)) => {
                    xref_reconstructed = true;
                    synthetic_objects = syn;
                    (x, t)
                }
                Err(_) => (xref, trailer),
            }
        };

        let prepopulated_scan = Self::build_prepopulated_scan(&xref, xref_reconstructed);

        // The reader ends here: the document keeps the bytes and reads them by
        // offset, so nothing downstream shares a cursor. ~keep
        let PdfReader::Memory(buffered) = reader;
        let source_bytes = buffered.into_inner().into_inner();

        // Note: Encryption initialization was originally lazy, but decode_stream_with_encryption
        // only has &self access which prevents initialization.
        // We now initialize eagerly to ensure the handler is ready when needed. ~keep
        let document = Self::build_document(
            (source_bytes, version, xref, trailer),
            (header_offset, xref_reconstructed, prepopulated_scan),
        );

        // Seed any SYNTHETIC recovery objects (a Catalog / page-tree root rebuilt
        // for a truncated file) into the object cache. They have no byte offset,
        // so `load_object` - which checks the cache before the xref - is the only
        // way to reach them. Done before encryption init so the /Root resolves. ~keep
        if !synthetic_objects.is_empty() {
            let mut cache = document.object_cache.lock_or_recover();
            for (obj_ref, obj) in synthetic_objects {
                cache.insert(obj_ref, obj);
            }
        }

        document.recover_object_streams_if_reconstructed(); // GH#1774 ~keep

        if let Err(error) = document.ensure_encryption_initialized() {
            trace_recoverable_pdf_error("initialize_encryption", &error);
            // We continue anyway, as it might just be an unsupported security handler
            // and maybe we can still read parts of the file (or fail later) ~keep
        }

        Ok(document)
    }

    /// A reconstruction scan already located every uncompressed "N G obj" in
    /// the file, so a later full-file rescan (on the first object miss) would
    /// find nothing new. Pre-seed the scan-offset cache so that miss is O(1);
    /// only when reconstructed, since a normal (parsed) xref may be
    /// legitimately partial and the full scan is the intended recovery path. ~keep
    fn build_prepopulated_scan(xref: &CrossRefTable, xref_reconstructed: bool) -> Option<HashMap<u32, u64>> {
        if !xref_reconstructed {
            return None;
        }
        Some(
            xref.all_object_numbers()
                .filter_map(|id| {
                    xref.get(id).and_then(|e| {
                        (e.in_use && e.entry_type == crate::xref::XRefEntryType::Uncompressed).then_some((id, e.offset))
                    })
                })
                .collect(),
        )
    }

    fn load_initial_xref_and_trailer<R: Read + Seek>(
        reader: &mut R,
    ) -> Result<(CrossRefTable, Object, bool, Vec<(ObjectRef, Object)>)> {
        let mut xref_reconstructed = false;
        let mut synthetic_objects: Vec<(ObjectRef, Object)> = Vec::new();

        let (xref, trailer) = match Self::try_open_regular(reader) {
            Ok((xref, trailer)) => {
                // Success with regular parsing
                // However, if the xref is suspiciously small (< 5 entries), it's likely corrupted
                // Try reconstruction to get a complete table ~keep
                if xref.is_empty() {
                    tracing::warn!(target: LOG_TARGET, "Regular xref parsing succeeded but table is empty, attempting reconstruction");
                    xref_reconstructed = true;
                    let (x, t, syn) = Self::try_reconstruct_xref(reader)?;
                    synthetic_objects = syn;
                    (x, t)
                } else {
                    // A valid xref can have any number of entries (§7.5.4).
                    // Small xrefs (e.g. portfolio PDFs with 3-4 objects) are perfectly
                    // normal — don't trigger expensive full-file reconstruction for them. ~keep
                    (xref, trailer)
                }
            }
            Err(e) => {
                trace_xref_parse_failure(&e);
                match Self::try_reconstruct_xref(reader) {
                    Ok((reconstructed_xref, reconstructed_trailer, syn)) => {
                        tracing::info!(target: LOG_TARGET, "Successfully reconstructed xref table");
                        xref_reconstructed = true;
                        synthetic_objects = syn;
                        (reconstructed_xref, reconstructed_trailer)
                    }
                    Err(recon_err) => {
                        tracing::warn!(
                            target: crate::LOG_TARGET_ROOT,
                            operation = "reconstruct_xref",
                            primary_error_code = e.telemetry_code(),
                            primary_error_offset = ?e.telemetry_offset(),
                            reconstruction_error_code = recon_err.telemetry_code(),
                            reconstruction_error_offset = ?recon_err.telemetry_offset(),
                            "xref reconstruction failed; propagating original parse error"
                        );
                        return Err(e);
                    }
                }
            }
        };

        Ok((xref, trailer, xref_reconstructed, synthetic_objects))
    }

    /// If PDF header is not at byte 0 (garbage-prepended), xref offsets may need adjustment.
    /// The xref offsets are relative to the original PDF start, but file positions are
    /// shifted by header_offset bytes. ~keep
    fn adjust_xref_for_header_offset<R: Read + Seek>(
        reader: &mut R,
        xref: &mut CrossRefTable,
        trailer: &Object,
        header_offset: u64,
    ) {
        // Probe an object to decide whether xref offsets are off by
        // header_offset. Prefer /Root (common case), but the probe MUST
        // be seek-validatable: `validate_object_at_offset` returns true
        // for *compressed* entries without seeking, so a /Root that
        // lives in an object stream would falsely report "no shift
        // needed" and leave every uncompressed offset wrong. Use /Root
        // only when its entry is in-use + uncompressed; otherwise (no
        // /Root — or a compressed /Root) fall back to the
        // first in-use uncompressed object. ~keep
        let probe = get_root_ref_from_trailer(trailer)
            .filter(|r| {
                xref.get(r.id)
                    .is_some_and(|e| e.in_use && e.entry_type == crate::xref::XRefEntryType::Uncompressed)
            })
            .or_else(|| first_in_use_uncompressed(xref));
        if let Some(probe_ref) = probe
            && !validate_object_at_offset(reader, xref, probe_ref)
        {
            tracing::warn!(target: LOG_TARGET,
                "Probe object {} not loadable at xref offset, adjusting all offsets by header_offset={}",
                probe_ref.id,
                header_offset
            );
            xref.shift_offsets(header_offset);
        }
    }

    fn build_document(
        parsed: (Vec<u8>, (u8, u8), CrossRefTable, Object),
        recovery: (u64, bool, Option<HashMap<u32, u64>>),
    ) -> Self {
        let (source_bytes, version, xref, trailer) = parsed;
        let (header_offset, xref_reconstructed, prepopulated_scan) = recovery;
        Self {
            source_bytes,
            version,
            xref,
            trailer,
            object_cache: Mutex::new(BoundedObjectCache::new(DEFAULT_OBJECT_CACHE_MAX_BYTES)),
            object_stream_cache: Mutex::new(BoundedObjectStreamCache::new(DEFAULT_OBJECT_STREAM_CACHE_MAX_BYTES)),
            object_stream_telemetry_seen: Mutex::new(BoundedRecoveryTelemetry::new(
                DEFAULT_OBJECT_STREAM_RECOVERY_MARKERS,
            )),
            encryption_handler: Mutex::new(None),
            encrypt_dict_ref: Mutex::new(None),
            options: ParserOptions::default(),
            header_offset,
            font_cache: Mutex::new(BoundedEntryCache::new(512)),
            font_set_cache: Mutex::new(BoundedEntryCache::new(256)),
            font_fingerprint_cache: Mutex::new(BoundedEntryCache::new(256)),
            font_name_set_cache: Mutex::new(BoundedEntryCache::new(256)),
            font_set_donated_cache: Mutex::new(BoundedEntryCache::new(256)),
            font_fingerprint_donated_cache: Mutex::new(BoundedEntryCache::new(256)),
            font_name_set_donated_cache: Mutex::new(BoundedEntryCache::new(256)),
            #[cfg(test)]
            donation_call_count: AtomicUsize::new(0),
            #[cfg(test)]
            donation_apply_count: AtomicUsize::new(0),
            font_identity_cache: Mutex::new(BoundedEntryCache::new(512)),
            font_id_hash_cache: Mutex::new(HashMap::new()),
            font_reference_hash_cache: Mutex::new(BoundedEntryCache::new(FONT_IDENTITY_MAX_RESOLVED_REFERENCES)),
            font_identity_hashed_bytes: AtomicUsize::new(0),
            font_identity_shared_cache_enabled: AtomicBool::new(true),
            structure_tree_cache: Mutex::new(None),
            structure_content_cache: Mutex::new(None),
            actualtext_index_cache: Mutex::new(None),
            mc_actualtext_mcids: Mutex::new(HashMap::new()),
            table_elements_cache: Mutex::new(None),
            page_cache: Mutex::new(HashMap::new()),
            page_cache_populated: AtomicBool::new(false),
            scanned_object_offsets: Mutex::new(prepopulated_scan),
            objstm_recovery_done: Mutex::new(false),
            xref_reconstructed,
            image_xobject_cache: Mutex::new(HashSet::new()),
            xobject_text_free_cache: Mutex::new(HashSet::new()),
            xobject_stream_cache: Mutex::new(HashMap::new()),
            xobject_stream_cache_bytes: AtomicUsize::new(0),
            xobject_spans_cache: Mutex::new(BoundedEntryCache::new(DEFAULT_XOBJECT_CACHE_MAX_ENTRIES)),
            form_xobject_images_cache: Mutex::new(BoundedEntryCache::new(DEFAULT_XOBJECT_CACHE_MAX_ENTRIES)),
            erase_regions: Mutex::new(HashMap::new()),
            page_content_cache: Mutex::new(BoundedEntryCache::new(64)),
            page_spans_cache: Mutex::new(BoundedEntryCache::new(8)),
            search_index: Mutex::new(HashMap::new()),
            page_chars_cache: Mutex::new(BoundedEntryCache::new(8)),
            running_artifact_signatures: Mutex::new(None),
            article_threads_cache: Mutex::new(None),
            output_intent_cmyk_profile_cache: Mutex::new(None),
            accumulated_warnings: Mutex::new(Vec::new()),
            warning_sink: crate::extractors::warnings::WarningSink::new(),
            recovery: std::sync::Arc::default(),
        }
    }

    /// Try to open the PDF using regular xref parsing.
    fn try_open_regular<R: Read + Seek>(reader: &mut R) -> Result<(CrossRefTable, Object)> {
        let xref_offset = find_xref_offset(reader)?;

        let xref = parse_xref(reader, xref_offset)?;

        let trailer = if let Some(trailer_dict) = xref.trailer() {
            Object::Dictionary(trailer_dict.clone())
        } else {
            reader.seek(SeekFrom::Start(xref_offset))?;
            parse_trailer(reader)?
        };

        Ok((xref, trailer))
    }

    /// Try to reconstruct the xref table by scanning the file. The third tuple
    /// element is any SYNTHETIC objects (a rebuilt Catalog / page-tree root for a
    /// truncated file) the caller must seed into the object cache - empty in the
    /// ordinary case.
    fn try_reconstruct_xref<R: Read + Seek>(
        reader: &mut R,
    ) -> Result<(CrossRefTable, Object, Vec<(ObjectRef, Object)>)> {
        crate::xref_reconstruction::reconstruct_xref(reader)
    }

    /// Initialize encryption handler lazily if PDF is encrypted.
    ///
    /// PDF Spec: Section 7.6.1 - Encryption dictionary in trailer
    ///
    /// This checks for the /Encrypt entry in the trailer, loads it if it's a
    /// reference, and creates an encryption handler. It automatically attempts
    /// to authenticate with an empty password (common for PDFs with default encryption).
    ///
    /// This is called lazily the first time we need to decrypt something, after
    /// the document is fully constructed and can load objects.
    pub(super) fn ensure_encryption_initialized(&self) -> Result<()> {
        if self.encryption_handler.lock_or_recover().is_some() {
            return Ok(());
        }
        let Some(trailer_dict) = self.trailer.as_dict() else {
            return Ok(());
        };
        let encrypt_ref = match trailer_dict.get("Encrypt") {
            Some(obj) => obj.clone(),
            None => {
                tracing::debug!(target: LOG_TARGET, "PDF is not encrypted (no /Encrypt entry)");
                return Ok(());
            }
        };
        let file_id = match trailer_dict.get("ID") {
            Some(Object::Array(arr)) => match arr.first() {
                Some(first_id) => match first_id.as_string() {
                    Some(id_bytes) => id_bytes.to_vec(),
                    None => {
                        tracing::warn!(target: LOG_TARGET, "Invalid /ID array entry (not a string), using empty file ID");
                        vec![]
                    }
                },
                None => {
                    tracing::warn!(target: LOG_TARGET, "Empty /ID array, using empty file ID");
                    vec![]
                }
            },
            _ => {
                tracing::warn!(target: LOG_TARGET, "Missing or invalid /ID entry in trailer, using empty file ID");
                vec![]
            }
        };
        let encrypt_obj = match encrypt_ref {
            Object::Dictionary(_) => encrypt_ref,
            Object::Reference(obj_ref) => {
                tracing::debug!(target: LOG_TARGET,
                    object_id = obj_ref.id,
                    generation = obj_ref.generation,
                    "loading /Encrypt object reference"
                );
                *self.encrypt_dict_ref.lock_or_recover() = Some(obj_ref);
                self.load_object(obj_ref)?
            }
            _ => {
                return Err(Error::InvalidPdf(format!(
                    "Invalid /Encrypt entry type: {}",
                    encrypt_ref.type_name()
                )));
            }
        };
        let encrypt_obj = match encrypt_obj.as_dict() {
            Some(dict) => Object::Dictionary(resolve_encrypt_dictionary_references(dict, |reference| {
                self.load_object(reference)
            })),
            None => encrypt_obj,
        };
        let mut handler = EncryptionHandler::new(&encrypt_obj, file_id)?;

        match handler.authenticate(b"") {
            Ok(true) => {
                tracing::info!(target: LOG_TARGET, "Successfully authenticated with empty password");
            }
            Ok(false) => {
                tracing::warn!(target: LOG_TARGET, "PDF is encrypted and requires a password");
                self.push_warning(
                    "PDF is encrypted and requires a password; call authenticate() before extracting text".to_string(),
                );
            }
            Err(error) => {
                trace_fatal_pdf_error("authenticate_empty_password", &error);
                return Err(error);
            }
        }
        *self.encryption_handler.lock_or_recover() = Some(handler);
        Ok(())
    }

    /// Decode stream data with encryption support.
    ///
    /// This is a helper method that decodes stream data using the PDF's encryption handler
    /// if the document is encrypted. It automatically handles object-specific key derivation.
    ///
    /// # Arguments
    ///
    /// * `stream_obj` - The stream object to decode
    /// * `obj_ref` - The object reference (for encryption key derivation)
    ///
    /// # Returns
    ///
    /// The decoded (and decrypted if needed) stream data.
    ///
    /// # PDF Spec Reference
    ///
    /// ISO 32000-1:2008, Section 7.6.2 - Streams must be decrypted BEFORE applying filters.
    pub(crate) fn decode_stream_with_encryption(&self, stream_obj: &Object, obj_ref: ObjectRef) -> Result<Vec<u8>> {
        self.decode_stream_with_encryption_and_expected_size(stream_obj, obj_ref, None)
    }

    pub(crate) fn decode_image_stream_with_encryption(
        &self,
        stream_obj: &Object,
        obj_ref: ObjectRef,
        expected_filter_output_size: usize,
    ) -> Result<Vec<u8>> {
        self.decode_stream_with_encryption_and_expected_size(stream_obj, obj_ref, Some(expected_filter_output_size))
    }

    fn decode_stream_with_encryption_and_expected_size(
        &self,
        stream_obj: &Object,
        obj_ref: ObjectRef,
        expected_filter_output_size: Option<usize>,
    ) -> Result<Vec<u8>> {
        if matches!(stream_obj, Object::Null) {
            return Ok(Vec::new());
        }

        // Per ISO 32000-2:2020 Section 7.6.3, object streams (/Type /ObjStm)
        // and cross-reference streams (/Type /XRef) shall NOT be encrypted.
        // Skip decryption for these stream types to avoid AES block-size errors
        // on data that was never encrypted in the first place. ~keep
        let is_unencrypted_stream_type = if let Object::Stream { dict, .. } = stream_obj {
            dict.get("Type")
                .and_then(|t| t.as_name())
                .map(|name| name == "ObjStm" || name == "XRef")
                .unwrap_or(false)
        } else {
            false
        };

        let handler_ref = self.encryption_handler.lock_or_recover();
        if let Some(handler) = handler_ref.as_ref() {
            if is_unencrypted_stream_type {
                drop(handler_ref);
                return match expected_filter_output_size {
                    Some(expected_size) => stream_obj.decode_image_stream_data(expected_size),
                    None => stream_obj.decode_stream_data(),
                };
            }
            let decrypt_fn = |data: &[u8]| -> Result<Vec<u8>> {
                handler.decrypt_stream(data, obj_ref.id, obj_ref.generation as u32)
            };
            stream_obj.decode_stream_data_with_decryption_and_expected_size(
                Some(&decrypt_fn),
                obj_ref.id,
                obj_ref.generation as u32,
                expected_filter_output_size,
            )
        } else {
            drop(handler_ref);
            match expected_filter_output_size {
                Some(expected_size) => stream_obj.decode_image_stream_data(expected_size),
                None => stream_obj.decode_stream_data(),
            }
        }
    }

    /// Open with custom extraction profile.
    ///
    /// Currently, the profile is not used at the document level but is reserved
    /// for future integration with document-type-specific extraction settings.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_with_config(path: impl AsRef<Path>, _config: impl std::any::Any) -> Result<Self> {
        Self::open(path)
    }

    /// Authenticate with a password to decrypt encrypted PDFs.
    ///
    /// If the PDF is encrypted, `open()` automatically tries an empty password.
    /// Call this method to authenticate with a non-empty password.
    ///
    /// # Arguments
    ///
    /// * `password` - The password as bytes
    ///
    /// # Returns
    ///
    /// `Ok(true)` if authentication succeeded, `Ok(false)` if the password was wrong,
    /// or `Ok(true)` if the PDF is not encrypted (no authentication needed).
    pub fn authenticate(&self, password: &[u8]) -> Result<bool> {
        self.ensure_encryption_initialized()?;
        // Capture current authentication state *before* calling the
        // handler so we can detect the transition from "not authenticated"
        // to "authenticated" and invalidate the object cache accordingly.
        // Any objects loaded and cached before successful authentication
        // still hold ciphertext strings (see `load_uncompressed_object_impl`
        // at the `handler.is_authenticated()` guard), so a cache hit after
        // authentication would return those stale values forever. ~keep
        let was_authenticated = self
            .encryption_handler
            .lock_or_recover()
            .as_ref()
            .map(|h| h.is_authenticated())
            .unwrap_or(true);

        let result = match self.encryption_handler.lock_or_recover().as_mut() {
            Some(handler) => handler.authenticate(password),
            None => return Ok(true),
        };

        if let Ok(true) = result
            && !was_authenticated
        {
            // Transitioned from "encrypted, not authenticated" to
            // "authenticated". Drop every cached object so subsequent
            // `load_object` calls re-parse through the path that now
            // runs `decrypt_strings_in_object` on the uncompressed
            // string values. The `/Encrypt` dictionary is not in this
            // cache path (it is resolved independently), so clearing
            // is always safe. ~keep
            self.object_cache.lock_or_recover().clear();
            tracing::debug!(target: LOG_TARGET,
                "authenticate(): object cache cleared after successful authentication \
                     to force re-decryption of any pre-auth cached objects"
            );
        }

        result
    }

    /// Check if the PDF is encrypted.
    ///
    /// Returns `true` if the PDF has an `/Encrypt` entry in its trailer,
    /// regardless of whether it has been authenticated.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xberg_native_pdf::document::PdfDocument;
    /// # let mut doc = PdfDocument::open("sample.pdf")?;
    /// if doc.is_encrypted() {
    ///     println!("PDF is encrypted");
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn is_encrypted(&self) -> bool {
        if self.encryption_handler.lock_or_recover().is_some() {
            return true;
        }
        self.trailer.as_dict().and_then(|d| d.get("Encrypt")).is_some()
    }

    /// Whether content extraction is permitted right now — `true` if the
    /// PDF is unencrypted, or encrypted and successfully authenticated.
    ///
    /// Cheap, side-effect-free preflight for the auto-extraction
    /// classifier: lets it emit
    /// [`ReasonCode::EncryptedNoExtractPermission`](crate::extractors::auto::ReasonCode)
    /// gracefully instead of attempting extraction and erroring.
    #[must_use]
    pub fn is_authenticated(&self) -> bool {
        // Fail closed: if encryption init errors (malformed / unsupported
        // `/Encrypt`), the document IS encrypted but we cannot have
        // authenticated it — a security preflight must report `false`
        // here, not `true`. Only when init succeeds
        // (incl. the trivial unencrypted case) do we trust the guard. ~keep
        if self.ensure_encryption_initialized().is_err() {
            return false;
        }
        !self.is_encrypted_and_unauthenticated()
    }

    /// Document Info dictionary `/Producer` (decoded, trimmed), if present
    /// and non-empty. A weak document-level prior for the scanner-vs-
    /// authoring heuristic (case P) — never decisive.
    #[must_use]
    pub fn document_producer(&self) -> Option<String> {
        self.document_info_string("Producer")
    }

    /// Document Info dictionary `/Creator` (decoded, trimmed), if present
    /// and non-empty. See [`document_producer`](Self::document_producer).
    #[must_use]
    pub fn document_creator(&self) -> Option<String> {
        self.document_info_string("Creator")
    }

    fn document_info_string(&self, key: &str) -> Option<String> {
        let info_raw = self.trailer.as_dict()?.get("Info")?;
        let info = self.resolve_obj_ref(info_raw);
        let val_raw = info.as_dict()?.get(key)?.clone();
        let val = self.resolve_obj_ref(&val_raw);
        let s = Self::decode_pdf_text_string(val.as_string()?);
        let trimmed = s.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    }

    /// Check if the PDF is encrypted but has NOT been successfully authenticated.
    ///
    /// This returns `true` when the document requires a password that has not
    /// yet been provided. Extraction methods use this to return a clear error
    /// instead of silently producing empty output.
    fn is_encrypted_and_unauthenticated(&self) -> bool {
        if let Some(handler) = self.encryption_handler.lock_or_recover().as_ref() {
            !handler.is_authenticated()
        } else {
            // Handler not yet initialized — check if /Encrypt exists
            // If it does, we don't know auth state yet, so return false
            // (ensure_encryption_initialized will handle it) ~keep
            false
        }
    }

    /// Guard that returns `Err(Error::EncryptedPdf)` if the PDF is encrypted
    /// and not authenticated. Call this at the top of extraction methods.
    pub(super) fn require_authenticated(&self) -> Result<()> {
        self.ensure_encryption_initialized()?;
        if self.is_encrypted_and_unauthenticated() {
            return Err(Error::EncryptedPdf);
        }
        Ok(())
    }

    /// True once the empty user password has been tried and the document is
    /// still locked. Text extraction degrades to empty output in this case
    /// (matching pdftotext/PyMuPDF) rather than erroring; `page_count` and
    /// write paths keep using [`Self::require_authenticated`].
    pub(super) fn is_encrypted_unreadable(&self) -> bool {
        let _ = self.ensure_encryption_initialized();
        self.is_encrypted_and_unauthenticated()
    }
}
