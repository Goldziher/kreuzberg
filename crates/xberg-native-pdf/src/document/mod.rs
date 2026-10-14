//! PDF document model.

use crate::encryption::EncryptionHandler;
use crate::error::{Error, Result};
use crate::layout::TextSpan;
use crate::object::{Object, ObjectRef};
use crate::parser::parse_object;
use crate::parser_config::ParserOptions;
use crate::xref::{CrossRefTable, find_xref_offset, parse_xref};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub(crate) use crate::cache::MutexExt;

/// Reading order mode for span extraction.
///
/// Controls how text spans are sorted after extraction from a PDF page.
/// The default `TopToBottom` mode uses simple geometric sorting, while
/// `ColumnAware` uses the XY-Cut algorithm to detect columns and read
/// each column top-to-bottom before moving to the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadingOrder {
    /// Simple top-to-bottom, left-to-right ordering.
    ///
    /// Sorts spans by Y-coordinate descending (top of page first),
    /// then by X-coordinate ascending (left to right).
    #[default]
    TopToBottom,
    /// Column-aware ordering using the XY-Cut algorithm.
    ///
    /// Detects columns via projection-profile analysis and reads each
    /// column fully (top-to-bottom) before moving to the next column.
    /// Best for newspapers, academic papers, and multi-column layouts.
    ColumnAware,
    /// Logical-structure ordering from the document's `/StructTreeRoot`.
    ///
    /// For a Tagged PDF, ISO 32000-1:2008 §14.8.2.3 makes a pre-order traversal
    /// of the structure hierarchy AUTHORITATIVE for reading order - it is the
    /// producer's declared sequence, independent of glyph geometry, so it reads
    /// tables and complex layouts correctly where a geometric XY-cut guesses.
    ///
    /// Spans are ordered by their marked-content id (`/MCID`) following that
    /// traversal; any span without a matching MCID is appended in geometric
    /// (`ColumnAware`) order. When the structure tree is absent or not
    /// trustworthy for ordering (untagged, or `/Suspects true`), this falls back
    /// to `ColumnAware` entirely, so it is always safe to request.
    Structure,
}

/// Distinguishes the three cases `get_page_rotation` folds into `0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageRotation {
    /// No `/Rotate` entry. ISO 32000-1 §7.7.3.3 default of 0 applies.
    Absent,
    /// A spec-valid multiple of 90, normalised to 0, 90, 180 or 270.
    Valid(i32),
    /// A `/Rotate` entry that is present but not a spec-valid multiple of 90.
    Malformed,
}

/// In-memory reader used by `open()` and `from_bytes()`. Wrapping in an enum
/// is kept (rather than using `BufReader<Cursor<Vec<u8>>>` directly) so a
/// future file-backed variant can be re-introduced without touching call
/// sites.
enum PdfReader {
    Memory(BufReader<Cursor<Vec<u8>>>),
}

impl Read for PdfReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            PdfReader::Memory(r) => r.read(buf),
        }
    }
}

impl Seek for PdfReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match self {
            PdfReader::Memory(r) => r.seek(pos),
        }
    }
}

impl BufRead for PdfReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        match self {
            PdfReader::Memory(r) => r.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match self {
            PdfReader::Memory(r) => r.consume(amt),
        }
    }
}

/// Maximum recursion depth for object resolution
const MAX_RECURSION_DEPTH: u32 = 100;

/// Maximum depth of a `/Kids` chain any page-tree walker will descend.
///
/// Every page-tree walk in this crate is recursive, so depth is stack frames, and
/// an uncapped chain overflows the stack — which aborts the process rather than
/// raising a catchable panic (GH#1755). A conforming page tree is balanced, so at
/// the minimum useful fan-out of 2 a depth of 128 already addresses 2^128 pages:
/// more than `/Count`'s 32-bit range can even express, let alone any real
/// document. Workspace feature unification makes the debug-build
/// `get_page_from_tree_inner` frame large enough that a 256-node chain exhausts
/// Rust's 2 MiB test-thread stack, so the cap is 128 to retain headroom in every
/// supported feature combination. Raising it would mean rewriting these walkers
/// iteratively with an explicit stack; the obstacle is
/// `get_page_from_tree_inner`, which merges inherited attributes along the path
/// as it descends. ~keep
const MAX_PAGE_TREE_DEPTH: u32 = 128;

/// Page information for rendering.
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Media box defining the page boundaries
    pub media_box: crate::geometry::Rect,
    /// Crop box if specified (for visible area)
    pub crop_box: Option<crate::geometry::Rect>,
    /// Page rotation in degrees (0, 90, 180, 270)
    pub rotation: i32,
}

/// Default maximum size in bytes for the object cache (64 MB).
///
/// This is a soft guardrail, not a hard ceiling. Real memory usage can be
/// 1.5–2× the cap because `estimate_size` does not account for HashMap bucket
/// overhead, Arc headers, or allocator padding.
const DEFAULT_OBJECT_CACHE_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Default maximum number of entries for the XObject span/image caches.
const DEFAULT_XOBJECT_CACHE_MAX_ENTRIES: usize = 1024;

/// Maximum accounted bytes retained as parsed object-stream maps.
const DEFAULT_OBJECT_STREAM_CACHE_MAX_BYTES: usize = DEFAULT_OBJECT_CACHE_MAX_BYTES / 4;
const DEFAULT_OBJECT_STREAM_RECOVERY_MARKERS: usize = 4096;

/// Heuristic multiplier for the forward-gap guard in the main
/// assembly loop's compound newline predicate
/// (`y_diff > 2.0 && gap > K * max(fs)`). Visual gap-sweep over
/// synthetic two-column examples at fs=10 and fs=14 placed the
/// plausible operating band at roughly 0.7-1.5; 1.25 is a
/// conservative interim pick. Not corpus-calibrated; a page-level
/// layout signal would be a stronger long-term replacement for
/// this pairwise heuristic.
const FORWARD_GAP_K: f32 = 1.25;

/// Maximum allowed inter-span X gap inside a candidate same-line reorder run.
/// If the candidate's tentative X-order contains a larger gap, the run is
/// probably a disjoint footer/header/field layout rather than a local
/// mixed-baseline repair.
const SAME_LINE_REORDER_MAX_GAP_FACTOR: f32 = 3.0;

pub(crate) use crate::cache::BoundedEntryCache;

/// Size-bounded object cache with FIFO eviction.
///
/// Wraps a `HashMap<ObjectRef, Object>` with byte-size tracking. When an
/// insertion would push total estimated size past `max_bytes`, the oldest
/// entries are evicted first (FIFO order via a `VecDeque` of keys).
///
/// FIFO is chosen over LRU because the access pattern is predominantly
/// insert-once-read-once — higher-level caches (font caches, xobject stream
/// cache) serve repeated lookups, so recency is not a useful signal here.
struct BoundedObjectCache {
    map: HashMap<ObjectRef, Object>,
    insertion_order: std::collections::VecDeque<ObjectRef>,
    current_bytes: usize,
    max_bytes: usize,
}

/// Byte-accounted FIFO cache for decoded object-stream maps.
struct BoundedObjectStreamCache {
    map: HashMap<ObjectRef, Arc<HashMap<u32, Object>>>,
    insertion_order: std::collections::VecDeque<ObjectRef>,
    current_bytes: usize,
    max_bytes: usize,
}

struct BoundedRecoveryTelemetry {
    seen: HashSet<u32>,
    max_entries: usize,
    saturated: bool,
}

impl BoundedRecoveryTelemetry {
    fn new(max_entries: usize) -> Self {
        Self {
            seen: HashSet::new(),
            max_entries,
            saturated: false,
        }
    }

    fn should_emit(&mut self, stream_object_id: u32) -> bool {
        if self.saturated || self.seen.contains(&stream_object_id) {
            return false;
        }
        if self.seen.len() >= self.max_entries {
            self.seen.clear();
            self.saturated = true;
            return false;
        }
        self.seen.insert(stream_object_id)
    }
}

impl BoundedObjectStreamCache {
    fn checked_capacity_bytes(capacity: usize, element_size: usize) -> Option<usize> {
        capacity.checked_mul(element_size)
    }

    fn new(max_bytes: usize) -> Self {
        Self {
            map: HashMap::new(),
            insertion_order: std::collections::VecDeque::new(),
            current_bytes: 0,
            max_bytes,
        }
    }

    fn get(&self, key: &ObjectRef) -> Option<&Arc<HashMap<u32, Object>>> {
        self.map.get(key)
    }

    fn insert(&mut self, key: ObjectRef, value: Arc<HashMap<u32, Object>>) -> bool {
        let Some(entry_size) = Self::estimate_size(&value) else {
            return false;
        };
        if entry_size > self.max_bytes {
            return false;
        }
        self.remove(&key);
        while self.current_bytes.saturating_add(entry_size) > self.max_bytes {
            let Some(old_key) = self.insertion_order.pop_front() else {
                return false;
            };
            self.remove(&old_key);
        }
        self.current_bytes = self.current_bytes.saturating_add(entry_size);
        self.map.insert(key, value);
        self.insertion_order.push_back(key);
        true
    }

    fn remove(&mut self, key: &ObjectRef) {
        if let Some(old_value) = self.map.remove(key) {
            let old_size = Self::estimate_size(&old_value).unwrap_or(self.max_bytes);
            self.current_bytes = self.current_bytes.saturating_sub(old_size);
        }
        if let Some(position) = self.insertion_order.iter().position(|candidate| candidate == key) {
            self.insertion_order.remove(position);
        }
    }

    fn estimate_size(objects: &HashMap<u32, Object>) -> Option<usize> {
        let headers =
            std::mem::size_of::<HashMap<u32, Object>>().checked_add(std::mem::size_of::<Arc<HashMap<u32, Object>>>())?;
        let buckets = Self::checked_capacity_bytes(
            objects.capacity(),
            std::mem::size_of::<u32>() + std::mem::size_of::<Object>() + std::mem::size_of::<usize>(),
        )?;
        let mut total = headers.checked_add(buckets)?;
        let mut stack: Vec<&Object> = objects.values().collect();
        while let Some(object) = stack.pop() {
            total = total.checked_add(Self::estimate_dynamic_size(object, &mut stack)?)?;
        }
        Some(total)
    }

    fn estimate_dynamic_size<'a>(object: &'a Object, stack: &mut Vec<&'a Object>) -> Option<usize> {
        match object {
            Object::String(value) => Some(value.capacity()),
            Object::Name(value) => Some(value.capacity()),
            Object::Array(values) => {
                stack.extend(values);
                Self::checked_capacity_bytes(values.capacity(), std::mem::size_of::<Object>())
            }
            Object::Dictionary(values) => Self::estimate_dictionary(values, stack),
            Object::Stream { dict, data } => Self::estimate_dictionary(dict, stack)?.checked_add(data.len()),
            _ => Some(0),
        }
    }

    fn estimate_dictionary<'a>(values: &'a HashMap<String, Object>, stack: &mut Vec<&'a Object>) -> Option<usize> {
        let buckets = Self::checked_capacity_bytes(
            values.capacity(),
            std::mem::size_of::<String>() + std::mem::size_of::<Object>() + std::mem::size_of::<usize>(),
        )?;
        let mut total = std::mem::size_of::<HashMap<String, Object>>().checked_add(buckets)?;
        for (key, value) in values {
            total = total.checked_add(key.capacity())?;
            stack.push(value);
        }
        Some(total)
    }
}

impl BoundedObjectCache {
    fn new(max_bytes: usize) -> Self {
        Self {
            map: HashMap::new(),
            insertion_order: std::collections::VecDeque::new(),
            current_bytes: 0,
            max_bytes,
        }
    }

    fn get(&self, key: &ObjectRef) -> Option<&Object> {
        self.map.get(key)
    }

    fn insert(&mut self, key: ObjectRef, value: Object) {
        let entry_size = Self::estimate_size(&value);

        if entry_size > self.max_bytes {
            return;
        }

        if let Some(old_val) = self.map.get(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(Self::estimate_size(old_val));
        }

        // Evict oldest entries until under budget. If the front of the
        // queue is the key we're about to (re)insert, skip past it so a
        // larger replacement doesn't leave the cache over budget — keep
        // evicting other entries instead. ~keep
        let mut skipped_self = false;
        while self.current_bytes + entry_size > self.max_bytes {
            match self.insertion_order.pop_front() {
                Some(old_key) => {
                    if old_key == key {
                        if skipped_self {
                            self.insertion_order.push_front(old_key);
                            break;
                        }
                        self.insertion_order.push_back(old_key);
                        skipped_self = true;
                        continue;
                    }
                    if let Some(old_val) = self.map.remove(&old_key) {
                        self.current_bytes = self.current_bytes.saturating_sub(Self::estimate_size(&old_val));
                    }
                }
                None => break,
            }
        }

        if self.map.insert(key, value).is_none() {
            self.insertion_order.push_back(key);
        }
        self.current_bytes += entry_size;
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn keys(&self) -> impl Iterator<Item = &ObjectRef> {
        self.map.keys()
    }

    fn clear(&mut self) {
        self.map.clear();
        self.insertion_order.clear();
        self.current_bytes = 0;
    }

    fn estimate_size(obj: &Object) -> usize {
        Self::estimate_size_depth(obj, 8)
    }

    /// Rough estimate of an Object's heap size in bytes.
    /// Recurses into nested containers up to `depth` levels to avoid
    /// both underestimation and stack overflow on adversarial input.
    fn estimate_size_depth(obj: &Object, depth: u8) -> usize {
        if depth == 0 {
            return 64;
        }
        match obj {
            Object::Stream { dict, data } => {
                let dict_size: usize = dict
                    .iter()
                    .map(|(k, v)| k.len() + 32 + Self::estimate_size_depth(v, depth - 1))
                    .sum();
                data.len() + dict_size + 64
            }
            Object::Dictionary(d) => {
                let inner: usize = d
                    .iter()
                    .map(|(k, v)| k.len() + 32 + Self::estimate_size_depth(v, depth - 1))
                    .sum();
                inner + 64
            }
            Object::Array(a) => {
                let inner: usize = a.iter().map(|v| Self::estimate_size_depth(v, depth - 1)).sum();
                inner + 64
            }
            Object::String(s) => s.len() + 32,
            Object::Name(s) => s.len() + 32,
            _ => 32,
        }
    }
}

// Per-thread resolving stack and recursion depth for load_object.
// Thread-local storage avoids document-global lock contention and prevents
// false "circular reference" errors when two threads resolve the same object
// concurrently (Race C). ~keep
thread_local! {
    // HashSet::new is not const on the supported Rust toolchain. ~keep
    #[allow(clippy::missing_const_for_thread_local)]
    static RESOLVING_STACK: RefCell<HashSet<ObjectRef>> = RefCell::new(HashSet::new());
    static RECURSION_DEPTH: RefCell<u32> = const { RefCell::new(0) };
}

const FONT_IDENTITY_MAX_REFERENCE_DEPTH: usize = 32;
const FONT_IDENTITY_MAX_RESOLVED_REFERENCES: usize = 4096;
const FONT_IDENTITY_MAX_HASHED_BYTES: usize = 32 * 1024 * 1024;
const FONT_HASH_BYTE_LIMIT_MARKER: u8 = 247;
const FONT_HASH_REFERENCE_LIMIT_MARKER: u8 = 248;
const FONT_HASH_RESOLVED_REFERENCE_MARKER: u8 = 249;
const FONT_HASH_DEPTH_LIMIT_MARKER: u8 = 250;
const FONT_HASH_UNRESOLVED_REFERENCE_MARKER: u8 = 251;
const FONT_HASH_REFERENCE_CYCLE_MARKER: u8 = 252;
const FONT_IDENTITY_SEMANTIC_FIELDS: &[&str] = &[
    "Encoding",
    "ToUnicode",
    "FontDescriptor",
    "Widths",
    "DescendantFonts",
    "FontMatrix",
];
const FONT_IDENTITY_CONSUMED_DICTIONARY_FIELDS: &[&str] = &[
    "Ascent",
    "BaseEncoding",
    "BitsPerComponent",
    "BlackIs1",
    "CIDSystemInfo",
    "CIDToGIDMap",
    "CMapName",
    "ColorTransform",
    "Colors",
    "Columns",
    "DW",
    "DW2",
    "DamagedRowsBeforeError",
    "DecodeParms",
    "Descent",
    "Differences",
    "EarlyChange",
    "EncodedByteAlign",
    "EndOfBlock",
    "EndOfLine",
    "Filter",
    "Flags",
    "FontDescriptor",
    "FontFile",
    "FontFile2",
    "FontFile3",
    "FontWeight",
    "JBIG2Globals",
    "K",
    "Name",
    "Ordering",
    "Predictor",
    "Registry",
    "StemV",
    "Subtype",
    "Supplement",
    "W",
    "W2",
];

#[derive(Clone, Copy)]
struct FontIdentityHash {
    value: u64,
    cacheable: bool,
    subtree_depth: usize,
}

#[derive(Default)]
struct FontHashTraversal {
    resolving: HashSet<ObjectRef>,
    resolved_hashes: HashMap<ObjectRef, FontIdentityHash>,
    resolved_references: usize,
    depth_frames: Vec<FontHashDepthFrame>,
}

struct FontHashDepthFrame {
    entry_depth: usize,
    max_depth: usize,
}

impl FontHashTraversal {
    fn begin_reference(&mut self, entry_depth: usize) {
        self.depth_frames.push(FontHashDepthFrame {
            entry_depth,
            max_depth: entry_depth,
        });
    }

    fn observe_depth(&mut self, depth: usize) {
        for frame in &mut self.depth_frames {
            frame.max_depth = frame.max_depth.max(depth);
        }
    }

    fn end_reference(&mut self) -> usize {
        let Some(frame) = self.depth_frames.pop() else {
            return FONT_IDENTITY_MAX_REFERENCE_DEPTH;
        };
        frame.max_depth.saturating_sub(frame.entry_depth)
    }
}

/// PDF document.
///
/// This structure represents an open PDF document, providing access to:
/// - Document metadata (version, catalog, trailer)
/// - Page information (count, page tree)
/// - Object loading and dereferencing
///
/// # Example
///
/// ```no_run
/// use xberg_native_pdf::document::PdfDocument;
///
/// let mut doc = PdfDocument::open("sample.pdf")?;
/// println!("PDF version: {}.{}", doc.version().0, doc.version().1);
/// println!("Page count: {}", doc.page_count()?);
/// # Ok::<(), xberg_native_pdf::error::Error>(())
/// ```
///
/// # Memory management
///
/// The document maintains several internal caches for performance. The main
/// object cache is bounded at 64 MB (see `DEFAULT_OBJECT_CACHE_MAX_BYTES`)
/// uses FIFO eviction to prevent unbounded heap growth when processing
/// many pages sequentially.
///
/// # Thread Safety
///
/// All interior-mutable fields use `Mutex` / `AtomicUsize`, making
/// `PdfDocument` both `Send` and `Sync`.
pub struct PdfDocument {
    /// Raw bytes of the document. Every read of the file body addresses this
    /// by absolute offset (see `bytes_at`), so the document carries no shared
    /// read cursor and a page read never depends on what another page read
    /// first. ~keep
    pub source_bytes: Vec<u8>,
    /// PDF version (major, minor)
    version: (u8, u8),
    /// Cross-reference table mapping object IDs to byte offsets
    xref: CrossRefTable,
    /// Trailer dictionary
    trailer: Object,
    /// Cache for loaded objects to avoid re-parsing.
    /// Bounded at [`DEFAULT_OBJECT_CACHE_MAX_BYTES`] with FIFO eviction to
    /// prevent unbounded heap growth during multi-page extraction.
    object_cache: Mutex<BoundedObjectCache>,
    /// Parsed object streams keyed by the stream reference and bounded by accounted bytes.
    object_stream_cache: Mutex<BoundedObjectStreamCache>,
    /// Bounded markers for streams whose aggregated recovery telemetry fired.
    /// Saturation fails closed by suppressing further recovery events. ~keep
    object_stream_telemetry_seen: Mutex<BoundedRecoveryTelemetry>,
    /// Encryption handler (if PDF is encrypted).
    /// Wrapped in RefCell for interior mutability (lazy initialization from &self).
    encryption_handler: Mutex<Option<EncryptionHandler>>,
    /// ObjectRef of the /Encrypt dictionary, cached so its strings are
    /// skipped during per-object string decryption. The entries in the
    /// encryption dict (/O, /U, /OE, /UE, /Perms, …) are key material used
    /// to derive the encryption key, not ciphertext, and must never be
    /// passed through `decrypt_string`.
    encrypt_dict_ref: Mutex<Option<ObjectRef>>,
    /// Parser configuration options for error handling and recovery
    #[allow(dead_code)]
    options: ParserOptions,
    /// Byte offset where PDF header was found (may not be 0 for malformed PDFs)
    #[allow(dead_code)]
    header_offset: u64,
    /// Font cache keyed by indirect ObjectRef to avoid re-parsing fonts across pages.
    /// Arc-wrapped to eliminate deep cloning when populating per-page TextExtractor.
    /// Bounded at 512 entries — TeX PDFs can create unique font objects per page.
    font_cache: Mutex<BoundedEntryCache<ObjectRef, Arc<crate::fonts::FontInfo>>>,
    /// Cached font sets keyed by /Font dictionary ObjectRef.
    /// Pages sharing the same /Font dict skip the entire load_fonts() loop.
    /// Bounded at 256 entries.
    font_set_cache: Mutex<BoundedEntryCache<ObjectRef, Vec<(String, Arc<crate::fonts::FontInfo>)>>>,
    /// Fingerprint-based font set cache for direct /Font dictionaries.
    /// Keyed by sorted font ObjectRefs hash, catches pages with different
    /// /Resources but same font references. Bounded at 256 entries.
    font_fingerprint_cache: Mutex<BoundedEntryCache<u64, Vec<(String, Arc<crate::fonts::FontInfo>)>>>,
    /// Name-based font set cache keyed by hash of sorted font names.
    /// Catches pages with different font ObjectRefs but the same font name→base font
    /// mapping (common in PDFs that create new font objects per page).
    /// Stores the resolved font set (Arc-wrapped to avoid cloning) plus a combined
    /// identity hash over ALL fonts for verification before reuse. Bounded at 256 entries.
    font_name_set_cache: Mutex<BoundedEntryCache<u64, (Arc<Vec<(String, Arc<crate::fonts::FontInfo>)>>, u64)>>,
    /// Donated companions to `font_set_cache`/`font_fingerprint_cache`/`font_name_set_cache`,
    /// keyed identically. Holds the SAME dictionary's font set after TrueType cmap donation
    /// has run only among fonts that dictionary itself resolves — never against fonts a
    /// sibling dictionary (a different page, or a Form XObject) happens to have contributed
    /// to the extractor first. That keeps a cache hit's donated value a function of this
    /// dictionary's own resources alone, so it cannot vary with which page or XObject was
    /// read before it. Donation that depends on a donor OUTSIDE this dictionary is not
    /// cached here; `TextExtractor::share_truetype_cmaps` still runs on every `load_fonts`
    /// call to cover that case, and is cheap when these fonts already carry a cmap. ~keep
    font_set_donated_cache: Mutex<BoundedEntryCache<ObjectRef, Vec<(String, Arc<crate::fonts::FontInfo>)>>>,
    /// Donated companion to `font_fingerprint_cache`. See `font_set_donated_cache`.
    font_fingerprint_donated_cache: Mutex<BoundedEntryCache<u64, Vec<(String, Arc<crate::fonts::FontInfo>)>>>,
    /// Donated companion to `font_name_set_cache`. See `font_set_donated_cache`.
    font_name_set_donated_cache: Mutex<BoundedEntryCache<u64, Arc<Vec<(String, Arc<crate::fonts::FontInfo>)>>>>,
    /// Counts calls to `donate_truetype_cmaps_within_set`, i.e. actual donation
    /// work rather than a cache hit. Test-only: proves donation runs once per
    /// font set across repeated page reads instead of once per page. ~keep
    #[cfg(test)]
    donation_call_count: AtomicUsize,
    /// Sum of `TextExtractor::share_truetype_cmaps`'s return value across every
    /// `load_fonts` call on this document, i.e. the number of fonts actually
    /// mutated by cmap donation. Test-only: proves a cache hit that already
    /// carries a donated cmap does not redo that mutation. ~keep
    #[cfg(test)]
    donation_apply_count: AtomicUsize,
    /// Per-font identity cache keyed by the resolved semantic content consumed by
    /// `FontInfo::from_dict`. Skips expensive parsing when a structurally identical
    /// font was already parsed.
    /// Bounded at 512 entries.
    font_identity_cache: Mutex<BoundedEntryCache<u64, Arc<crate::fonts::FontInfo>>>,
    /// Per-object resolved font identity, memoized. An object's content is fixed
    /// within a document, so the Layer-4 cache guard need not traverse each font's
    /// indirect semantic objects on every page. `None` means the identity was not
    /// safe for cross-object or cross-document reuse.
    font_id_hash_cache: Mutex<HashMap<ObjectRef, Option<u64>>>,
    /// Resolved identities that were proven safe for reuse by another font
    /// root in this document. Bounded by the traversal reference cap. ~keep
    font_reference_hash_cache: Mutex<BoundedEntryCache<ObjectRef, FontIdentityHash>>,
    /// Total stream bytes hashed while establishing semantic font identities.
    font_identity_hashed_bytes: AtomicUsize,
    /// Fail-closed gate for cross-object and cross-document font identity reuse.
    font_identity_shared_cache_enabled: AtomicBool,
    /// Cached structure tree (None = not yet checked, Some(None) = untagged, Some(Some) = tagged).
    /// Uses Arc to avoid expensive deep clones on every page extraction.
    /// Mutex provides interior mutability for `&self` read-path methods.
    structure_tree_cache: Mutex<Option<Option<Arc<crate::structure::StructTreeRoot>>>>,
    /// Cached per-page structure tree traversal results.
    /// Built once from the structure tree, then O(1) lookup per page.
    /// Mutex provides interior mutability for `&self` read-path methods.
    structure_content_cache: Mutex<Option<HashMap<u32, Vec<crate::structure::OrderedContent>>>>,
    /// Cached resolved structure-tree `/ActualText` scopes.
    ///
    /// `None` = not yet built, `Some(None)` = built and the document has
    /// no resolvable ActualText (untagged, or every bearing element
    /// dropped during finalisation), `Some(Some(idx))` = built.
    ///
    /// Mirrors `structure_tree_cache` so every extraction surface
    /// applies tree-scope ActualText consistently without re-walking the
    /// structure tree. Decoupled from `/MarkInfo /Suspects`: producer-
    /// supplied ActualText is trusted regardless of Suspects (it is
    /// content replacement, not reading order — see
    /// `actualtext_index`).
    actualtext_index_cache: Mutex<Option<Option<Arc<crate::structure::ActualTextIndex>>>>,
    /// Per-page set of MCIDs whose marked-content sequence carried an
    /// inline `/ActualText` property (ISO 32000-1:2008 §14.6).
    ///
    /// Populated by `extract_spans_impl` from the text extractor's
    /// per-call detection: the per-page entry is REPLACED on each
    /// extraction so MC-scope precedence reflects the latest run, not
    /// stale data from an earlier filter set.
    ///
    /// The struct-tree-scope ActualText applier consults this set to
    /// enforce the precedence rule: the MC-scope (inline) replacement
    /// is the innermost and most specific declaration for the MCID
    /// it covers, so a struct-tree-scope `/ActualText` on an ancestor
    /// element must NOT override it.
    pub(crate) mc_actualtext_mcids: Mutex<HashMap<usize, HashSet<u32>>>,
    /// `Table` structure elements bucketed by page, built once via
    /// `find_table_elements_all_pages` (one tree walk) so the converter table
    /// path does an O(1) lookup instead of walking the tree per page.
    /// `None` = not yet built.
    table_elements_cache: Mutex<Option<HashMap<u32, Vec<crate::structure::StructElem>>>>,
    /// Page object cache keyed by page index to avoid re-traversing the page tree.
    /// The page tree structure is static (§7.7.3.2), so pages can be safely cached.
    /// Mutex provides interior mutability for `&self` read-path methods.
    page_cache: Mutex<HashMap<usize, Object>>,
    /// Whether the bulk page tree walk has been attempted (successful or not).
    /// Prevents re-walking the tree on every cache miss for malformed PDFs.
    page_cache_populated: AtomicBool,
    /// Cached object offsets from full file scan (built on first xref miss).
    /// Maps object number to byte offset in file.
    scanned_object_offsets: Mutex<Option<HashMap<u32, u64>>>,
    /// Whether the one-time object-stream recovery sweep has been attempted.
    /// See `recover_from_object_streams`. Separate from the scanned offsets
    /// cache because the sweep is only triggered on free-entry misses that
    /// also failed the file-body scan — the common path never needs it.
    objstm_recovery_done: Mutex<bool>,
    /// Whether this document's xref table came from full-file reconstruction
    /// (GH#1774) rather than a parsed xref table/stream. Reconstruction finds
    /// objects only via a literal "N G obj" header scan, so it is structurally
    /// blind to objects that live solely inside a compressed `/ObjStm`
    /// container — unlike a *parsed* xref, which can carry a type-2
    /// (compressed) entry pointing straight at the container. Set once at
    /// construction and never mutated, so a plain `bool` (no `Mutex`)
    /// suffices. Used to (a) proactively sweep object streams once so
    /// ObjStm-packed objects resolve normally, and (b) gate the
    /// `XrefRecovery` warning so it fires only when a reference is missing
    /// *because* reconstruction couldn't see it — never for a healthy,
    /// normally-parsed document, and never for a reference that is legitimately
    /// free per §7.3.10. ~keep
    xref_reconstructed: bool,
    /// Cache of XObject refs known to NOT be Form XObjects (i.e., Image or unknown).
    /// Used by text extraction to skip expensive full-object loads for images.
    image_xobject_cache: Mutex<HashSet<ObjectRef>>,
    /// Document-level cache of Form XObject refs whose streams contain NO text
    /// operators (BT) and no nested Do invocations. Persists across pages so that
    /// shared graphics-only XObjects (watermarks, logos, chart elements) are
    /// decompressed and scanned at most once across the entire document.
    pub(crate) xobject_text_free_cache: Mutex<HashSet<ObjectRef>>,
    /// Cache of decompressed Form XObject streams. Bounded at 50MB total.
    /// Avoids repeated FlateDecode decompression of shared Form XObjects.
    pub(crate) xobject_stream_cache: Mutex<HashMap<ObjectRef, std::sync::Arc<Vec<u8>>>>,
    pub(crate) xobject_stream_cache_bytes: AtomicUsize,
    /// Cache of extracted TextSpan results from self-contained Form XObjects
    /// (those with own /Resources/Font). None = processed but no spans.
    /// Key is `(ObjectRef, [i64; 6])` where the array encodes the caller's CTM
    /// as millipoint-rounded integers, allowing the same Form XObject to cache
    /// distinct results for each unique CTM it is painted with.
    /// Bounded at [`DEFAULT_XOBJECT_CACHE_MAX_ENTRIES`] entries with FIFO eviction.
    pub(crate) xobject_spans_cache:
        Mutex<BoundedEntryCache<(ObjectRef, [i64; 6]), Option<Vec<crate::layout::TextSpan>>>>,
    /// Cache of extracted images from Form XObjects (keyed by ObjectRef).
    /// Images are stored without CTM applied — caller applies its own CTM.
    /// Bounded at [`DEFAULT_XOBJECT_CACHE_MAX_ENTRIES`] entries with FIFO eviction.
    pub(crate) form_xobject_images_cache: Mutex<BoundedEntryCache<ObjectRef, Vec<crate::extractors::PdfImage>>>,
    /// Regions marked for erasure per page. Mutex for `&self` write-path methods.
    pub(crate) erase_regions: Mutex<HashMap<usize, Vec<crate::geometry::Rect>>>,
    /// LRU cache of decompressed page content streams, keyed by page index.
    page_content_cache: Mutex<BoundedEntryCache<usize, std::sync::Arc<Vec<u8>>>>,
    /// LRU cache of postprocessed [`TextSpan`]s per page. `to_markdown`/`to_html`
    /// reach `extract_spans` twice per page — once directly, once via
    /// `extract_page_tables` → `extract_words` → `page_reading_order`; this serves
    /// the second from cache. Cleared by redaction (`erase_region` /
    /// `clear_erase_regions`), the only span-affecting mutation.
    page_spans_cache: Mutex<BoundedEntryCache<usize, std::sync::Arc<Vec<crate::layout::TextSpan>>>>,
    /// Per-page lightweight search index (page text + span bounding boxes,
    /// no fonts/glyph widths) built lazily by `search()`/`search_page()` and
    /// reused across repeated calls on the same document. Unlike
    /// `page_spans_cache` this is never size-bounded/evicted — search never
    /// reads font/glyph data, so retaining every page's projection is far
    /// cheaper than retaining every page's full `TextSpan`s would be.
    /// Cleared alongside `page_spans_cache` wherever spans are invalidated.
    search_index: Mutex<HashMap<usize, std::sync::Arc<crate::search::SearchPageIndex>>>,
    /// Per-page character cache for the unfiltered (`extract_chars`) result.
    /// `postprocess_spans` needs the same char sequence the public API returns,
    /// so without this every span extraction re-parses the content stream a
    /// second time purely to stamp per-glyph x-origins.
    page_chars_cache: Mutex<BoundedEntryCache<usize, std::sync::Arc<Vec<crate::layout::TextChar>>>>,
    /// Cached signatures of running headers/footers detected via cross-page
    /// repetition. A span whose normalized text matches a signature
    /// sits near the top/bottom of the page is treated as an artifact.
    /// Populated lazily on first access; `Some(set)` with an empty set
    /// means detection ran and found nothing (vs `None` = not yet run).
    /// Signatures of running headers/footers plus the first page index where
    /// each signature was observed. Used to mark repeat occurrences as
    /// pagination artifacts while keeping the first appearance intact — the
    /// first appearance is often the document's cover-page title that just
    /// happens to echo into the header band on every page (pdfa_010
    /// would otherwise drop "University of Oklahoma 2009").
    running_artifact_signatures: Mutex<Option<std::sync::Arc<std::collections::HashMap<String, usize>>>>,
    /// Document-wide article threads (`/Threads`), parsed once. Reading-order
    /// resolution consults them per page, and parsing walks the whole page
    /// tree — so without this the cost per page scaled with the document.
    article_threads_cache: Mutex<Option<std::sync::Arc<Vec<crate::structure::ArticleThread>>>>,
    /// Memoised result of [`PdfDocument::output_intent_cmyk_profile`].
    ///
    /// The accessor walks `/OutputIntents` and decodes + parses the ICC
    /// stream every call. The hot transparency / overprint paths invoke
    /// it once per paint and the parse is non-trivial (qcms / lcms2
    /// header validation + LUT decode on a profile blob that can be
    /// hundreds of KB), so the result is cached for the document
    /// lifetime here. `Some(None)` means "checked once, no usable CMYK
    /// OutputIntent" — distinct from `None` (not yet checked).
    output_intent_cmyk_profile_cache: Mutex<Option<Option<std::sync::Arc<crate::color::IccProfile>>>>,
    /// Accumulated extraction warnings for programmatic inspection.
    /// Populated when silent fallbacks occur (font not found, CMap absent, etc.).
    /// Retrieve with [`PdfDocument::warnings`]; drain with [`PdfDocument::take_warnings`].
    accumulated_warnings: Mutex<Vec<String>>,
    /// structured warnings accumulator. Each
    /// internal warning site that previously only called `tracing::warn!`
    /// can additionally push a typed [`crate::extractors::warnings::Warning`]
    /// here, letting callers retrieve diagnostics as structured data
    /// (via [`PdfDocument::structured_warnings`]) instead of parsing
    /// stderr text. The existing String-list `accumulated_warnings`
    /// stays for back-compat.
    warning_sink: crate::extractors::warnings::WarningSink,
    /// Counts of input this document recovered from — a missing embedded font,
    /// an object outside the xref table, an unreadable CFF version. Reported
    /// once, as a single DEBUG event, when the document is dropped; the
    /// individual occurrences are TRACE. See `extractors::recovery_tally` for
    /// why these are counted rather than warned about (GH#1547). ~keep
    recovery: std::sync::Arc<crate::extractors::recovery_tally::RecoveryCounts>,
}

/// Tracing target for every event raised under `document`, pinned to the parent
/// module path.
///
/// Targets are public API in this workspace and semver-relevant, so splitting
/// `document.rs` into files must not rename them. Without this, each child module
/// inherits its own `module_path!()` and 110 call sites silently move from
/// `xberg_native_pdf::document` to `xberg_native_pdf::document::<child>`, breaking
/// every consumer `EnvFilter` that names the old target. Only one test caught it. ~keep
pub(super) const LOG_TARGET: &str = module_path!();

impl PdfDocument {
    /// The document's recovery counters, so extraction paths outside this module
    /// can install a [`RecoveryScope`](crate::extractors::recovery_tally) and let
    /// the free functions in `fonts::` charge their recoveries here. ~keep
    pub(crate) fn recovery_counts(&self) -> &std::sync::Arc<crate::extractors::recovery_tally::RecoveryCounts> {
        &self.recovery
    }
}

impl Drop for PdfDocument {
    /// Emits the one-per-document recovery summary (GH#1547).
    ///
    /// Drop rather than an explicit call at the end of extraction: a document is
    /// opened once and then extracted from, rendered, and queried through many
    /// separate public entry points, so drop is the only point that sees the
    /// whole document's totals exactly once. Silent when nothing was recovered.
    /// ~keep
    fn drop(&mut self) {
        crate::extractors::recovery_tally::report(&self.recovery.snapshot());
    }
}

const _: () = {
    fn _assert_send_sync<T: Send + Sync>() {}
    fn _check() {
        _assert_send_sync::<PdfDocument>();
    }
};

impl std::fmt::Debug for PdfDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfDocument")
            .field("version", &self.version)
            .field("xref_entries", &self.xref.len())
            .field("cached_objects", &self.object_cache.lock_or_recover().len())
            .finish_non_exhaustive()
    }
}

/// Pre-decompression filter for image extraction.
///
/// Dimensions are checked against XObject dictionary metadata (Width, Height,
/// ColorSpace) BEFORE the stream is decompressed, avoiding expensive decoding
/// of images that will be discarded downstream.
struct ImageExtractFilter {
    /// Minimum width in pixels (images narrower are skipped).
    min_width: i64,
    /// Minimum height in pixels (images shorter are skipped).
    min_height: i64,
    /// Maximum total pixels (images exceeding this are skipped).
    max_pixels: u64,
    /// Skip Indexed-colorspace images below this dimension.
    /// 0 means disabled.
    skip_indexed_small: i64,
}

impl Default for ImageExtractFilter {
    fn default() -> Self {
        Self {
            min_width: 8,
            min_height: 8,
            max_pixels: u64::MAX,
            skip_indexed_small: 0,
        }
    }
}

/// Area of a page for targeted header/footer operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageArea {
    /// Top region (Header)
    Header,
    /// Bottom region (Footer)
    Footer,
}

/// Scan raw file bytes for candidate ObjStm positions.
///
/// Each hit is `(object_number, byte_offset_of_N_G_obj_header)`. We look
/// for the shape `N G obj ... /Type /ObjStm` within a small window after
/// each object header so that the caller can then `load_uncompressed_object`
/// at exactly that offset without parsing the whole file body.
///
/// The scan is intentionally tolerant: it doesn't require `/Type`
/// `/ObjStm` to be separated by whitespace (many producers write
/// `/Type/ObjStm`), doesn't anchor on any particular position within the
/// header, and doesn't rely on xref entries being correct — which is the
/// whole point of the recovery path it serves.
fn find_objstm_candidates(content: &[u8]) -> Vec<(u32, u64)> {
    const DICT_PEEK_BYTES: usize = 2048;
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < content.len() {
        let valid_start =
            pos == 0 || content[pos - 1] == b'\n' || content[pos - 1] == b'\r' || content[pos - 1] == b' ';
        if !valid_start || !content[pos].is_ascii_digit() {
            pos += 1;
            continue;
        }
        let header_start = pos;

        let num_start = pos;
        while pos < content.len() && content[pos].is_ascii_digit() {
            pos += 1;
        }
        if pos >= content.len() || content[pos] != b' ' {
            pos = header_start + 1;
            continue;
        }
        let obj_num: u32 = match std::str::from_utf8(&content[num_start..pos])
            .ok()
            .and_then(|s| s.parse().ok())
        {
            Some(n) => n,
            None => {
                pos = header_start + 1;
                continue;
            }
        };
        pos += 1;

        let gen_start = pos;
        while pos < content.len() && content[pos].is_ascii_digit() {
            pos += 1;
        }
        if pos >= content.len() || content[pos] != b' ' {
            pos = header_start + 1;
            continue;
        }
        if std::str::from_utf8(&content[gen_start..pos])
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .is_none()
        {
            pos = header_start + 1;
            continue;
        }
        pos += 1;

        if pos + 3 > content.len() || &content[pos..pos + 3] != b"obj" {
            pos = header_start + 1;
            continue;
        }

        // Peek up to DICT_PEEK_BYTES ahead for `/Type` followed (after
        // optional whitespace) by `/ObjStm`. We don't decompress — the
        // ObjStm dict header is always uncompressed plaintext even when
        // the stream body is Flate-encoded. ~keep
        let window_end = (pos + DICT_PEEK_BYTES).min(content.len());
        let window = &content[pos..window_end];
        if contains_objstm_marker(window) {
            out.push((obj_num, header_start as u64));
        }

        pos = header_start + 1;
    }
    out
}

fn contains_objstm_marker(window: &[u8]) -> bool {
    let mut i = 0;
    while i + 5 <= window.len() {
        if &window[i..i + 5] == b"/Type" {
            let mut j = i + 5;
            while j < window.len()
                && (window[j] == b' ' || window[j] == b'\t' || window[j] == b'\r' || window[j] == b'\n')
            {
                j += 1;
            }
            if j + 7 <= window.len() && &window[j..j + 7] == b"/ObjStm" {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Append ink names declared by `Separation` and `DeviceN` colour spaces
/// in `cs_dict` to `out`, including one that is the base of an `Indexed`
/// space or the underlying space of a `Pattern`. Reserved colorants `/All` and `/None` (§8.6.6.4)
/// are skipped. Caller is responsible for deduping across multiple calls.
///
/// When `doc` is `Some`, indirect references inside each colour-space array
/// (e.g. a DeviceN whose names list is `4 0 R` rather than inline) are
/// resolved. Tools that hand-build inline arrays and don't need indirection
/// resolution can pass `None`.
///
/// Used by both [`PdfDocument::get_page_inks`] and
/// [`PdfDocument::get_page_inks_deep`] so the per-colorant rules live in
/// exactly one place.
fn extract_inks_from_color_space_dict(
    cs_dict: &std::collections::HashMap<String, Object>,
    doc: Option<&PdfDocument>,
    out: &mut Vec<String>,
) {
    let mut visited: std::collections::HashSet<ObjectRef> = std::collections::HashSet::new();
    for cs_def in cs_dict.values() {
        collect_inks_from_color_space(cs_def, doc, out, &mut visited, 0);
    }
}

/// Inner walker — surfaces inks from a single colour-space definition.
/// Factored out of [`extract_inks_from_color_space_dict`] so the
/// Pattern and Indexed arm can recurse into its underlying colour space
/// without requiring a synthetic single-entry dict.
///
/// **Cycle handling:** the Pattern and Indexed arm recurses into the
/// underlying colour space (§8.7.3.1, §8.6.6.3). A self-referential array such as
/// `5 0 obj [/Pattern 5 0 R]` would otherwise blow the stack, so
/// indirect references are de-duplicated via `visited` (keyed on
/// `ObjectRef`) and total depth is capped at `MAX_RECURSION_DEPTH`
/// — the same backstop used by [`PdfDocument::walk_form_xobject_tree_for_inks`].
fn collect_inks_from_color_space(
    cs_def: &Object,
    doc: Option<&PdfDocument>,
    out: &mut Vec<String>,
    visited: &mut std::collections::HashSet<ObjectRef>,
    depth: u32,
) {
    if depth >= MAX_RECURSION_DEPTH {
        return;
    }
    let deref = |obj: &Object| -> Object {
        match (obj.as_reference(), doc) {
            (Some(r), Some(d)) => d.load_object(r).unwrap_or_else(|_| obj.clone()),
            _ => obj.clone(),
        }
    };

    let arr = match cs_def.as_array() {
        Some(a) => a,
        None => return,
    };
    if arr.len() < 2 {
        return;
    }
    let cs_type = match arr.first().and_then(Object::as_name) {
        Some(n) => n,
        None => return,
    };
    match cs_type {
        "Pattern" | "Indexed" => {
            // ISO 32000-1 §8.7.3.1: a Pattern colour space's
            // optional second array element is the underlying
            // colour space (uncoloured Tiling carries the
            // underlying space's tints). §8.6.6.3: an Indexed
            // space's second element is its base, and every
            // palette entry paints in that base. Recurse so a
            // /Separation or /DeviceN underneath either one
            // surfaces its spot colorants for plate allocation.
            //
            // Guard against self-referential cycles (e.g.
            // `5 0 obj [/Pattern 5 0 R]`): an indirect underlying
            // ref is recorded in `visited`; a repeat hit terminates
            // the recursion silently. ~keep
            if let Some(r) = arr[1].as_reference()
                && !visited.insert(r)
            {
                return;
            }
            let underlying = deref(&arr[1]);
            collect_inks_from_color_space(&underlying, doc, out, visited, depth + 1);
        }
        "Separation" => {
            // §8.6.6.2: [/Separation /InkName /AlternateCS /TintTransform].
            // The name slot is usually inline but resolve indirects for safety. ~keep
            let name_obj = deref(&arr[1]);
            if let Some(ink) = name_obj.as_name()
                && ink != "All"
                && ink != "None"
            {
                out.push(ink.to_string());
            }
        }
        "DeviceN" => {
            // §8.6.6.5: [/DeviceN <names-array> /AlternateCS /TintTransform <attrs>].
            // The names array is commonly emitted as an indirect reference
            // when the same colorant set is shared across multiple DeviceN
            // spaces; resolve before unpacking the names. ~keep
            let names_obj = match arr.get(1) {
                Some(o) => deref(o),
                None => return,
            };
            // ISO 32000-1 §8.6.6.5 / Table 73: the optional 5th array
            // element is the attributes dictionary. When its `/Process`
            // sub-dictionary declares a `/Components` array, those names
            // are PROCESS colorants (riding the page's process plates),
            // not spot inks. The same rule applies whether the attrs
            // dict's `/Subtype` is `/DeviceN` (the default, PDF 1.6) or
            // `/NChannel` (PDF 1.7 stricter subtype) — §8.6.6.5 names the
            // /Process key on both subtypes. Build the process-name set
            // here so the colorants loop can filter against it. ~keep
            let process_names: std::collections::HashSet<String> = arr
                .get(4)
                .map(&deref)
                .as_ref()
                .and_then(Object::as_dict)
                .and_then(|attrs| attrs.get("Process"))
                .map(&deref)
                .as_ref()
                .and_then(Object::as_dict)
                .and_then(|proc_dict| proc_dict.get("Components"))
                .map(&deref)
                .as_ref()
                .and_then(Object::as_array)
                .map(|comps| comps.iter().filter_map(|o| o.as_name().map(str::to_string)).collect())
                .unwrap_or_default();
            if let Some(inks) = names_obj.as_array() {
                for ink_obj in inks {
                    if let Some(ink) = ink_obj.as_name()
                        && ink != "All"
                        && ink != "None"
                        && !process_names.contains(ink)
                    {
                        out.push(ink.to_string());
                    }
                }
            }
        }
        _ => {}
    }
}

/// Per-page MCID action computed from the
/// [`crate::structure::ActualTextIndex`].
///
/// Drives every consumer of struct-tree-scope `/ActualText`
/// (`extract_text`'s structure-order assembler, the raw-span applier,
/// and the ordered-span applier). The map is computed once per page
/// from the cached `ActualTextIndex` plus the visibility / MC-scope
/// filters; consumers then dispatch per MCID without re-walking the
/// structure tree.
#[derive(Debug, Clone)]
pub(crate) enum ActualTextAction {
    /// Replace this MCID's span text with the supplied string AND drop
    /// subsequent spans / MCIDs in the same consecutive-replacement
    /// run. Assigned to exactly one MCID per emitting run: the first
    /// visible MCID that is not exempted by MC-scope-wins.
    EmitAndSuppress(std::sync::Arc<str>),
    /// Suppress the raw glyphs for this MCID without emitting anything.
    /// Used for run continuations after the run's emission MCID, for
    /// suppress-only entries (non-first-page coverage of a multi-page
    /// ActualText scope), and for MCIDs in a fully-hidden run.
    Suppress,
}

fn trace_open_error(error: &Error) {
    if let Some(error_offset) = error.telemetry_offset() {
        tracing::error!(error_code = error.telemetry_code(), error_offset, "PDF open failed");
    } else {
        tracing::error!(error_code = error.telemetry_code(), "PDF open failed");
    }
}

fn trace_xref_parse_failure(error: &Error) {
    if let Some(error_offset) = error.telemetry_offset() {
        tracing::warn!(
            error_code = error.telemetry_code(),
            error_offset,
            "regular xref parsing failed; attempting reconstruction"
        );
    } else {
        tracing::warn!(
            error_code = error.telemetry_code(),
            "regular xref parsing failed; attempting reconstruction"
        );
    }
}

fn trace_recoverable_pdf_error(operation: &'static str, error: &Error) {
    crate::error::trace_recovery(operation, error);
}

fn trace_fatal_pdf_error(operation: &'static str, error: &Error) {
    crate::error::trace_failure(operation, error);
}

fn resolve_encrypt_dictionary_references(
    dictionary: &HashMap<String, Object>,
    mut load: impl FnMut(ObjectRef) -> Result<Object>,
) -> HashMap<String, Object> {
    let mut resolved_dictionary = dictionary.clone();
    let mut unresolved_reference_count = 0usize;
    for value in resolved_dictionary.values_mut() {
        if let Object::Reference(object_reference) = value {
            match load(*object_reference) {
                Ok(resolved) => *value = resolved,
                Err(_) => unresolved_reference_count += 1,
            }
        }
    }
    if unresolved_reference_count > 0 {
        crate::error::trace_recovery_count(
            "resolve_encrypt_reference",
            "unresolved_reference",
            unresolved_reference_count,
        );
    }
    resolved_dictionary
}

impl PdfDocument {
    /// Collapse a gated RTL visual line into one VISUAL-order span: explode every
    /// span into per-glyph `(x, char)` (reusing the `to_chars` advance arithmetic),
    /// drop producer shatter spaces, sort base letters by ascending x (visual
    /// left-to-right), bind each combining mark to its nearest base, and re-insert
    /// a single space at genuine inter-word x-gaps. The downstream
    /// [`push_span_text_bidi`] then reverses this to correct logical order with
    /// marks kept attached (`reverse_rtl_keeping_marks`).
    /// Private-use sentinel that [`merge_rtl_line_to_visual_span`] emits in place
    /// of a SPACE at an AUTHORITATIVE producer-segmented Arabic word boundary, so
    /// the downstream [`strip_interior_arabic_spaces`] (which strips only U+0020)
    /// leaves it intact instead of mistaking a genuine word break for a
    /// cursive-shatter artefact. Every output site restores it to a SPACE right
    /// after the strip ([`push_span_text_bidi`] for plain text,
    /// [`apply_rtl_logical_order_to_ordered_spans`] for md/html). U+F8FF is in the
    /// Unicode private-use area and never appears in real producer text reaching
    /// the pure-RTL merge path.
    const RTL_WORD_BOUNDARY: char = '\u{F8FF}';

    /// Return all extraction warnings accumulated since this document was opened.
    ///
    /// Warnings are recorded when silent fallbacks occur during text extraction
    /// (e.g., missing ToUnicode CMap, font not found, malformed structure tree).
    /// They do NOT consume the warning list — use [`Self::take_warnings`] to drain it.
    ///
    /// This API makes previously invisible extraction degradations programmatically
    /// observable without requiring callers to hook into the `log` crate.
    pub fn warnings(&self) -> Vec<String> {
        self.accumulated_warnings.lock_or_recover().clone()
    }

    /// Drain and return all accumulated extraction warnings, clearing the list.
    ///
    /// After this call, [`Self::warnings`] returns an empty `Vec` until new warnings
    /// are generated. Useful for incremental processing pipelines that want to
    /// inspect warnings on a per-page or per-operation basis.
    pub fn take_warnings(&self) -> Vec<String> {
        std::mem::take(&mut *self.accumulated_warnings.lock_or_recover())
    }

    /// Record an extraction warning. Called internally when a silent fallback occurs.
    pub(crate) fn push_warning(&self, msg: impl Into<String>) {
        self.accumulated_warnings.lock_or_recover().push(msg.into());
    }

    /// Return the document's accumulated structured warnings as a
    /// snapshot. Each entry carries the warning's
    /// [`WarningCategory`](crate::extractors::warnings::WarningCategory),
    /// page (if applicable), human-readable message, and PDF spec
    /// section reference (when applicable).
    ///
    /// Unlike [`Self::warnings`] which returns plain strings, this
    /// accessor returns structured records callers can filter, route
    /// to observability dashboards, or assert on in tests without
    /// parsing message text. Pairs with the `pyo3_log` per-target
    /// default-level downgrade to give Python users a clean stderr
    /// experience plus an opt-in structured surface.
    ///
    /// Returns the warnings in insertion order. The vector is
    /// non-destructive: subsequent calls return the same entries
    /// plus any new ones pushed since the last call. Use
    /// [`Self::take_structured_warnings`] to drain.
    ///
    /// Merges the process-wide `GLOBAL_WARNING_SINK` (where
    /// free-function log sites like `SPEC VIOLATION`,
    /// operator-cap-exceeded, and Type0/Type3 font fallbacks push
    /// their structured records) into the per-document sink on each
    /// call. The drain attribution follows the "first caller wins"
    /// rule documented at the global sink — process-wide scope means
    /// the first document to call `structured_warnings` collects
    /// the global tail that accumulated since the last drain.
    ///
    /// Renamed from `flatten_warnings` in to avoid colliding
    /// with the pre-existing `DocumentEditor::flatten_warnings`
    /// (which returns the form-flattening side-effect log, a
    /// `&[String]` — different feature). Both the Rust and Python
    /// (`PyDocument`) surfaces now agree on `structured_warnings`.
    pub fn structured_warnings(&self) -> Vec<crate::extractors::warnings::Warning> {
        let global = crate::extractors::warnings::drain_global_warnings();
        if !global.is_empty() {
            self.warning_sink.extend(global);
        }
        self.warning_sink.snapshot()
    }

    /// Drain and return all accumulated structured warnings.
    /// Companion to [`Self::structured_warnings`].
    pub fn take_structured_warnings(&self) -> Vec<crate::extractors::warnings::Warning> {
        self.warning_sink.take()
    }

    /// Record a structured warning. Hook called from migrated
    /// `tracing::warn!` sites that also want to surface the warning as
    /// structured data.
    ///
    /// Exposed as `pub` so external diagnostic sources (custom
    /// extractors, FFI hooks) can also push warnings into the same
    /// sink that [`Self::structured_warnings`] surfaces.
    pub fn push_structured_warning(&self, warning: crate::extractors::warnings::Warning) {
        self.warning_sink.push(warning);
    }

    // ========================================================================
    // Debug/profiling helpers — thin pub wrappers over internal methods.
    // Used by examples/debug_katalog.rs to break extract_spans into phases.
    // ======================================================================== ~keep
}

/// Reference to an extracted image file.
///
/// Contains metadata about an image that has been extracted and saved to a file.
/// Used for HTML export to embed images with correct dimensions and format.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedImageRef {
    /// Filename of the saved image (e.g., "img_001.png")
    pub filename: String,
    /// Image format
    pub format: ImageFormat,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Bounding box in PDF user space
    pub bbox: Option<crate::geometry::Rect>,
    /// Rotation in degrees
    pub rotation: i32,
    /// Transformation matrix
    pub matrix: [f32; 6],
}

/// Image format for extracted images.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// PNG format (lossless)
    Png,
    /// JPEG format (lossy, preserves DCT-encoded images)
    Jpeg,
}

/// Extract the /Root reference from a trailer dictionary.
fn get_root_ref_from_trailer(trailer: &Object) -> Option<ObjectRef> {
    trailer.as_dict()?.get("Root")?.as_reference()
}

/// First in-use *uncompressed* object in the xref, used as a /Root-independent
/// probe for the garbage-prefix offset-shift decision. Compressed
/// entries can't be seek-validated, so they're skipped.
fn first_in_use_uncompressed(xref: &crate::xref::CrossRefTable) -> Option<ObjectRef> {
    xref.all_object_numbers()
        .filter_map(|n| xref.get(n).map(|e| (n, e)))
        .find(|(_, e)| e.in_use && e.entry_type == crate::xref::XRefEntryType::Uncompressed)
        .map(|(n, e)| ObjectRef::new(n, e.generation))
}

/// Heuristic: does this candidate table actually look like wrapped prose
/// clustered into x-columns rather than a real grid?
///
/// Cell contents in real data tables are atomic units (numbers, codes,
/// names, short labels): they almost always start with an uppercase
/// letter, a digit, or a symbol (currency, +/-, punctuation marker)
/// rarely end with a mid-sentence comma or semicolon. Prose-as-table
/// cells, by contrast, are fragments of running sentences — they
/// frequently start with a lowercase stopword ("and", "the", "to") because
/// the column boundary fell mid-clause, and frequently end with `,` or
/// `;` for the same reason.
///
/// We reject the candidate when either signal exceeds its threshold:
///   • > 12 % of cells end in `,` or `;` (mid-sentence tails), or
///   • > 25 % of cells start with a lowercase ASCII letter
///     (continuation fragments).
///
/// Thresholds chosen to clear the false positives flagged in the 88-PDF
/// regression (`searchable.pdf`, the WFMYY press-release, several arxiv
/// preprints) without disturbing legitimate data tables — sailing scores,
/// IRS forms, and the CJK traffic-volume grid all stay well below both
/// bars.
fn looks_like_prose_table(table: &crate::structure::Table) -> bool {
    let mut total = 0usize;
    let mut sentence_tails = 0usize;
    let mut lower_starts = 0usize;
    let mut leader_dots = 0usize;
    for row in &table.rows {
        for cell in &row.cells {
            let trimmed = cell.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            total += 1;
            if let Some(last) = trimmed.chars().last()
                && matches!(last, ',' | ';')
            {
                sentence_tails += 1;
            }
            if let Some(first) = trimmed.chars().next()
                && first.is_ascii_lowercase()
            {
                lower_starts += 1;
            }
            // Table-of-contents leader runs (". . . . . . ." between an
            // entry's title and its page number) cluster into their own
            // x-columns and create phantom 10–12-column "tables" out of
            // an ordinary three-column TOC. A cell whose content is
            // exclusively dots and spaces is the leader, not data. ~keep
            if trimmed.chars().all(|c| c == '.' || c == ' ') {
                leader_dots += 1;
            }
        }
    }
    if total < 10 {
        return false;
    }
    let tail_ratio = sentence_tails as f32 / total as f32;
    let lower_ratio = lower_starts as f32 / total as f32;
    let leader_ratio = leader_dots as f32 / total as f32;
    tail_ratio > 0.12 || lower_ratio > 0.25 || leader_ratio > 0.10
}

/// Check whether the object at the xref offset for `obj_ref` looks like a valid header.
fn validate_object_at_offset<R: Read + Seek>(
    reader: &mut R,
    xref: &crate::xref::CrossRefTable,
    obj_ref: ObjectRef,
) -> bool {
    let entry = match xref.get(obj_ref.id) {
        Some(e) => e,
        None => return false,
    };
    // Compressed objects live inside object streams — their "offset" is the
    // stream object number, not a byte position. We cannot validate them by
    // seeking, but their presence in a correctly parsed xref stream is
    // sufficient proof that the xref is valid. ~keep
    if entry.entry_type == crate::xref::XRefEntryType::Compressed {
        return true;
    }
    if reader.seek(SeekFrom::Start(entry.offset)).is_err() {
        return false;
    }
    let mut buf = [0u8; 32];
    let n = reader.read(&mut buf).unwrap_or(0);
    if n == 0 {
        return false;
    }
    let s = String::from_utf8_lossy(&buf[..n]);
    // A valid object header starts with "N G obj" ~keep
    let mut parts = s.split_whitespace();
    let first_is_num = parts.next().is_some_and(|t| t.parse::<u32>().is_ok());
    let second_is_num = parts.next().is_some_and(|t| t.parse::<u16>().is_ok());
    let third_is_obj = parts.next().is_some_and(|t| t == "obj" || t.starts_with("obj"));
    first_is_num && second_is_num && third_is_obj
}

/// Validate that the /Root catalog object is loadable from the xref.
fn validate_root_loadable<R: Read + Seek>(reader: &mut R, xref: &crate::xref::CrossRefTable, trailer: &Object) -> bool {
    let root_ref = match get_root_ref_from_trailer(trailer) {
        Some(r) => r,
        None => return false,
    };
    validate_object_at_offset(reader, xref, root_ref)
}

/// Check if a byte slice contains the standalone "obj" keyword (not "endobj").
///
/// This is used during multi-line object header parsing to detect when we've
/// accumulated enough lines to have a complete header. A naive `contains("obj")`
/// would match "endobj" and cause the loop to exit prematurely.
///
/// Takes `&[u8]` rather than `&str`: the caller's header text comes from
/// `String::from_utf8_lossy` over attacker-controlled bytes at an
/// attacker-controlled xref offset, so it can legitimately decode to a
/// `String` containing multi-byte UTF-8 characters (e.g. a 4-byte emoji)
/// immediately followed by the ASCII bytes `obj`. Slicing that `&str` at a
/// fixed byte offset (as the previous `&s[i - 3..i]` did) panics with "byte
/// index N is not a char boundary" whenever the offset lands inside such a
/// character — the same defect shape fixed for `fonts::cmap` in
/// `29fdd59d69`. Operating on the raw bytes instead has no char-boundary
/// concept to violate: any `i` satisfying the existing bounds checks below
/// is always a valid slice index. ~keep
fn has_standalone_obj_keyword(s: &[u8]) -> bool {
    for (i, window) in s.windows(3).enumerate() {
        if window != b"obj" {
            continue;
        }
        if i >= 3 && &s[i - 3..i] == b"end" {
            continue;
        }
        // Must be at a word boundary: preceded by whitespace, digit, or start of string ~keep
        if i == 0 || s[i - 1].is_ascii_whitespace() || s[i - 1].is_ascii_digit() {
            return true;
        }
    }
    false
}

/// Parse PDF header (%PDF-x.y) from a reader.
///
/// # Arguments
///
/// * `reader` - A readable and seekable source (e.g., File, Cursor)
/// * `lenient` - If false, fail if header not at byte 0; if true, search first 8192 bytes
///
/// # Returns
///
/// Returns `Ok((major, minor, offset))` with the PDF version and byte offset where header was found.
/// In strict mode, offset will be 0 if successful. In lenient mode, offset may be > 0 for PDFs
/// with leading binary data (compliant with ISO 32000-1:2008, page 41).
///
/// # Examples
///
/// ```rust
/// use std::io::Cursor;
/// # use xberg_native_pdf::document::parse_header;
///
/// let data = b"%PDF-1.7\n";
/// let mut cursor = Cursor::new(data);
/// let (major, minor, offset) = parse_header(&mut cursor, false).unwrap();
/// assert_eq!((major, minor, offset), (1, 7, 0));
/// ```
pub fn parse_header<R: Read + Seek>(reader: &mut R, lenient: bool) -> Result<(u8, u8, u64)> {
    let start_pos = reader.stream_position().unwrap_or(0);

    // Read first 8 bytes for fast path (header at byte 0) ~keep
    let mut header = [0u8; 8];
    let strict_read_ok = match reader.read_exact(&mut header) {
        Ok(_) => {
            if &header[0..5] == b"%PDF-" {
                return parse_version_from_header(&header, lenient).map(|(major, minor)| (major, minor, 0));
            }
            true
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                if !lenient {
                    return Err(Error::InvalidHeader(
                        "File too short for PDF header (expected at least 8 bytes)".to_string(),
                    ));
                }
                false
            } else {
                return Err(Error::InvalidHeader(format!("Failed to read file: {}", e)));
            }
        }
    };

    if !lenient && strict_read_ok {
        return Err(Error::InvalidHeader(format!(
            "Expected '%PDF-' at byte 0, found '{}'",
            String::from_utf8_lossy(&header[0..5])
        )));
    }

    reader.seek(SeekFrom::Start(start_pos))?;

    let mut buffer = vec![0u8; 8192];
    let bytes_read = match reader.read(&mut buffer) {
        Ok(0) => return Err(Error::InvalidHeader("File is empty (0 bytes read)".to_string())),
        Ok(n) => n,
        Err(e) => {
            return Err(Error::InvalidHeader(format!(
                "I/O error while searching for PDF header: {}",
                e
            )));
        }
    };

    buffer.truncate(bytes_read);

    match find_substring(&buffer, b"%PDF-") {
        Some(offset) => {
            if offset + 8 > buffer.len() {
                return Err(Error::InvalidHeader(
                    "PDF header found but insufficient bytes for version".to_string(),
                ));
            }

            let header_bytes = &buffer[offset..offset + 8];
            let mut header_arr = [0u8; 8];
            header_arr.copy_from_slice(header_bytes);

            let (major, minor) = parse_version_from_header(&header_arr, true)?;

            // Standardize reader position to just after the header
            // (consistent with strict mode behavior at line 4378) ~keep
            let header_start = start_pos + offset as u64;
            let after_header = header_start + 8;
            reader.seek(SeekFrom::Start(after_header))?;

            Ok((major, minor, header_start))
        }
        None => {
            if lenient {
                // Some PDFs lack a %PDF- header entirely (e.g., start with a binary
                // comment like %\xe2\xe3\xcf\xd3). Default to version 1.4. ~keep
                tracing::warn!(
                    operation = "parse_pdf_header",
                    reason = "missing_header",
                    "PDF header recovery defaulted to version 1.4"
                );
                reader.seek(SeekFrom::Start(0))?;
                Ok((1, 4, 0))
            } else {
                Err(Error::InvalidHeader(
                    "No PDF header found in first 8192 bytes of file".to_string(),
                ))
            }
        }
    }
}

/// Parse version information from a header buffer.
/// Assumes buffer starts with "%PDF-" and has at least 8 bytes.
///
/// When `lenient` is true, malformed version strings (e.g., `%PDF-1.\n`, `%PDF-a.4`)
/// default to version (1, 4) instead of returning an error.
fn parse_version_from_header(header: &[u8; 8], lenient: bool) -> Result<(u8, u8)> {
    if &header[0..5] != b"%PDF-" {
        return Err(Error::InvalidHeader(format!(
            "Expected '%PDF-', found '{}'",
            String::from_utf8_lossy(&header[0..5])
        )));
    }

    // Parse version (e.g., "1.7")
    // Format: %PDF-M.m where M is major version (1 digit), m is minor version (1 digit) ~keep
    if header[6] != b'.' {
        if lenient {
            tracing::warn!(
                operation = "parse_pdf_version",
                reason = "invalid_version_separator",
                "Malformed PDF version; defaulting to 1.4"
            );
            return Ok((1, 4));
        }
        return Err(Error::InvalidHeader(format!(
            "Invalid version format: expected '.', found '{}'",
            header[6] as char
        )));
    }

    let major = header[5];
    let minor = header[7];

    if !major.is_ascii_digit() || !minor.is_ascii_digit() {
        if lenient {
            tracing::warn!(
                operation = "parse_pdf_version",
                reason = "non_numeric_version",
                "Malformed PDF version; defaulting to 1.4"
            );
            return Ok((1, 4));
        }
        return Err(Error::InvalidHeader(format!(
            "Invalid version: {}.{} (not digits)",
            major as char, minor as char
        )));
    }

    let major = major - b'0';
    let minor = minor - b'0';

    if major > 2 || (major == 0 && minor == 0) {
        if lenient {
            tracing::warn!(
                operation = "parse_pdf_version",
                reason = "unsupported_version",
                version_major = major,
                version_minor = minor,
                "Unsupported PDF version; defaulting to 1.4"
            );
            return Ok((1, 4));
        }
        return Err(Error::UnsupportedVersion(format!("{}.{}", major, minor)));
    }

    Ok((major, minor))
}

/// Parse the trailer dictionary from a reader.
///
/// The trailer comes immediately after the xref table and before "startxref".
/// It starts with the keyword "trailer" followed by a dictionary.
///
/// # Example Format
///
/// ```text
/// trailer
/// << /Size 6 /Root 1 0 R /Info 5 0 R >>
/// startxref
/// 1234
/// %%EOF
/// ```
///
/// # Arguments
///
/// * `reader` - A readable source positioned after the xref table
///
/// # Returns
///
/// Returns the trailer dictionary as an `Object`.
///
/// # Errors
///
/// Returns an error if:
/// - The "trailer" keyword is not found
/// - The dictionary following "trailer" cannot be parsed
/// - The reader encounters an I/O error
pub fn parse_trailer<R: Read>(reader: &mut R) -> Result<Object> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    let content = String::from_utf8_lossy(&buffer);
    let trailer_pos = content
        .find("trailer")
        .ok_or_else(|| Error::InvalidPdf("Trailer keyword not found after xref table".to_string()))?;

    let dict_start = trailer_pos + 7;
    if dict_start >= buffer.len() {
        return Err(Error::UnexpectedEof);
    }

    let (_, trailer_dict) = parse_object(&buffer[dict_start..]).map_err(|e| Error::ParseError {
        offset: dict_start,
        reason: format!("Failed to parse trailer dictionary: {:?}", e),
    })?;

    if trailer_dict.as_dict().is_none() {
        return Err(Error::InvalidPdf("Trailer is not a dictionary".to_string()));
    }

    Ok(trailer_dict)
}

/// Find the first occurrence of a substring in a byte slice.
///
/// Returns the index of the first occurrence, or None if not found.
fn find_substring(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    haystack.windows(needle.len()).position(|window| window == needle)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod ink_dict_extractor_tests;

#[cfg(test)]
mod open_signals_tests;

mod annotations;
mod catalog;
mod classify;
mod columns;
mod extract_api;
mod fonts;
mod images;
mod key_value_rows;
mod objects;
mod open;
mod pages;
mod paths;
mod reading_order;
mod rect_api;
mod redaction;
mod span_postprocess;
mod spans_text;
mod tables;
pub use key_value_rows::reorder_ltr_key_value_rows;
mod text_assembly;
