use crate::api::TessDeleteText;
use crate::enums::TessPageIteratorLevel;
use crate::error::{Result, TesseractError};
use std::ffi::CStr;
use std::os::raw::{c_char, c_float, c_int, c_void};
use std::sync::{Arc, Mutex};

/// Font attributes detected by Tesseract for a word.
#[derive(Debug, Clone)]
pub struct FontAttributes {
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underlined: bool,
    pub is_monospace: bool,
    pub is_serif: bool,
    pub is_smallcaps: bool,
    pub pointsize: i32,
    pub font_id: i32,
}

/// Complete word data extracted in a single mutex lock.
#[derive(Debug, Clone, Default)]
pub struct WordData {
    pub text: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub confidence: f32,
    pub font_attrs: Option<FontAttributes>,
    /// Language that recognized this specific word (e.g. `"eng"`, `"deu"`), from
    /// `TessResultIteratorWordRecognitionLanguage`. `None` when Tesseract could
    /// not report a language for this word.
    pub language: Option<String>,
}

/// A recognised word with the text and box of each of its symbols, from
/// [`ResultIterator::extract_word_symbols`].
#[derive(Debug, Clone)]
pub struct WordSymbols {
    pub text: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    /// The word's symbols in reading order.
    pub symbols: Vec<SymbolBox>,
}

/// One symbol of a word: its text and its box.
#[derive(Debug, Clone)]
pub struct SymbolBox {
    pub text: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Outcome of a full-page word extraction pass over the `ResultIterator`.
///
/// `skipped` distinguishes "the page has no words" from "words exist but
/// per-word FFI extraction failed for some of them" — both previously
/// collapsed into an empty or partial `Vec<WordData>` with no signal.
#[derive(Debug, Clone, Default)]
pub struct WordExtractionOutcome {
    /// Successfully extracted words, in iterator order.
    pub words: Vec<WordData>,
    /// Count of words for which `extract_word_data_unlocked` returned a
    /// recoverable error (null pointer, invalid parameter, or invalid UTF-8)
    /// and was therefore dropped from `words`.
    pub skipped: usize,
}

pub struct ResultIterator {
    pub handle: Arc<Mutex<*mut c_void>>,
}

struct TessTextGuard<D: FnMut(*mut c_char)> {
    pointer: *mut c_char,
    deleter: D,
}

impl<D: FnMut(*mut c_char)> Drop for TessTextGuard<D> {
    fn drop(&mut self) {
        (self.deleter)(self.pointer);
    }
}

/// Copy a NUL-terminated Tesseract string and release it on every return path. ~keep
///
/// # Safety
///
/// A non-null `pointer` must reference a valid NUL-terminated allocation that remains readable
/// until `deleter` releases it. A null pointer represents no allocation and is rejected without
/// calling `deleter`. `deleter` must accept a non-null allocation exactly once. ~keep
unsafe fn copy_and_delete_tess_text(pointer: *mut c_char, deleter: impl FnMut(*mut c_char)) -> Result<String> {
    if pointer.is_null() {
        return Err(TesseractError::NullPointerError);
    }
    let text = TessTextGuard { pointer, deleter };
    // SAFETY: the caller guarantees that the guarded pointer is readable and NUL-terminated. ~keep
    let c_str = unsafe { CStr::from_ptr(text.pointer) };
    Ok(c_str.to_str()?.to_owned())
}

unsafe impl Send for ResultIterator {}
unsafe impl Sync for ResultIterator {}

impl ResultIterator {
    /// Creates a new instance of the ResultIterator.
    ///
    /// # Arguments
    ///
    /// * `handle` - Pointer to the ResultIterator.
    ///
    /// # Returns
    ///
    /// Returns the new instance of the ResultIterator.
    pub fn new(handle: *mut c_void) -> Self {
        ResultIterator {
            handle: Arc::new(Mutex::new(handle)),
        }
    }

    /// Gets the UTF-8 text of the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the text.
    ///
    /// # Returns
    ///
    /// Returns the UTF-8 text as a `String` if successful, otherwise returns an error.
    pub fn get_utf8_text(&self, level: TessPageIteratorLevel) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let text_ptr = unsafe { TessResultIteratorGetUTF8Text(*handle, level as c_int) };
        if text_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        // SAFETY: Tesseract returned this NUL-terminated allocation and requires TessDeleteText. ~keep
        let result = unsafe {
            copy_and_delete_tess_text(text_ptr, |pointer| {
                // SAFETY: the guard calls this exactly once with the pointer returned by Tesseract. ~keep
                TessDeleteText(pointer);
            })
        }?;
        Ok(result)
    }

    /// Gets the confidence of the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the confidence.
    ///
    /// # Returns
    ///
    /// Returns the confidence as a `f32`.
    pub fn confidence(&self, level: TessPageIteratorLevel) -> Result<f32> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessResultIteratorConfidence(*handle, level as c_int) })
    }

    /// Gets the recognition language of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the recognition language as a `String` if successful, otherwise returns an error.
    pub fn word_recognition_language(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let lang_ptr = unsafe { TessResultIteratorWordRecognitionLanguage(*handle) };
        if lang_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        let c_str = unsafe { CStr::from_ptr(lang_ptr) };
        Ok(c_str.to_str()?.to_owned())
    }

    /// Gets the font attributes of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the font attributes as a tuple if successful, otherwise returns an error.
    pub fn word_font_attributes(&self) -> Result<(bool, bool, bool, bool, bool, bool, i32, i32)> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let mut is_bold = 0;
        let mut is_italic = 0;
        let mut is_underlined = 0;
        let mut is_monospace = 0;
        let mut is_serif = 0;
        let mut is_smallcaps = 0;
        let mut pointsize = 0;
        let mut font_id = 0;

        let result = unsafe {
            TessResultIteratorWordFontAttributes(
                *handle,
                &mut is_bold,
                &mut is_italic,
                &mut is_underlined,
                &mut is_monospace,
                &mut is_serif,
                &mut is_smallcaps,
                &mut pointsize,
                &mut font_id,
            )
        };

        if result.is_null() {
            Err(TesseractError::InvalidParameterError)
        } else {
            Ok((
                is_bold != 0,
                is_italic != 0,
                is_underlined != 0,
                is_monospace != 0,
                is_serif != 0,
                is_smallcaps != 0,
                pointsize,
                font_id,
            ))
        }
    }

    /// Checks if the current iterator is from the dictionary.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is from the dictionary, otherwise returns `false`.
    pub fn word_is_from_dictionary(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessResultIteratorWordIsFromDictionary(*handle) != 0 })
    }

    /// Checks if the current iterator is numeric.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is numeric, otherwise returns `false`.
    pub fn word_is_numeric(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessResultIteratorWordIsNumeric(*handle) != 0 })
    }

    /// Checks if the current iterator is superscript.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is superscript, otherwise returns `false`.
    pub fn symbol_is_superscript(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessResultIteratorSymbolIsSuperscript(*handle) != 0 })
    }

    /// Checks if the current iterator is subscript.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is subscript, otherwise returns `false`.
    pub fn symbol_is_subscript(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessResultIteratorSymbolIsSubscript(*handle) != 0 })
    }

    /// Checks if the current iterator is dropcap.
    ///
    /// # Returns
    ///
    /// Returns `true` if the current iterator is dropcap, otherwise returns `false`.
    pub fn symbol_is_dropcap(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessResultIteratorSymbolIsDropcap(*handle) != 0 })
    }

    /// Moves to the next iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the next iterator.
    ///
    /// # Returns
    ///
    /// Returns `true` if the next iterator exists, otherwise returns `false`.
    pub fn next(&self, level: TessPageIteratorLevel) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessResultIteratorNext(*handle, level as c_int) != 0 })
    }

    /// Gets the current word from the iterator with its bounding box and confidence.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (text, left, top, right, bottom, confidence) if successful
    pub fn get_word_with_bounds(&self) -> Result<(String, i32, i32, i32, i32, f32)> {
        let text = self.get_utf8_text(TessPageIteratorLevel::RIL_WORD)?;
        let (left, top, right, bottom) = self.get_bounding_box(TessPageIteratorLevel::RIL_WORD)?;
        let confidence = self.confidence(TessPageIteratorLevel::RIL_WORD)?;

        Ok((text, left, top, right, bottom, confidence))
    }

    /// Advances the iterator to the next word.
    ///
    /// # Returns
    ///
    /// Returns true if successful, false if there are no more words
    pub fn next_word(&self) -> Result<bool> {
        self.next(TessPageIteratorLevel::RIL_WORD)
    }

    /// Gets the word information for the current position in the iterator.
    /// Should be called before next() to ensure valid data.
    ///
    /// # Returns
    /// Returns a tuple of (text, left, top, right, bottom, confidence) if successful
    pub fn get_current_word(&self) -> Result<(String, i32, i32, i32, i32, f32)> {
        let text = self.get_utf8_text(TessPageIteratorLevel::RIL_WORD)?;
        let (left, top, right, bottom) = self.get_bounding_box(TessPageIteratorLevel::RIL_WORD)?;
        let confidence = self.confidence(TessPageIteratorLevel::RIL_WORD)?;

        Ok((text, left, top, right, bottom, confidence))
    }

    /// Gets the bounding box for the current element.
    pub fn get_bounding_box(&self, level: TessPageIteratorLevel) -> Result<(i32, i32, i32, i32)> {
        let mut left = 0;
        let mut top = 0;
        let mut right = 0;
        let mut bottom = 0;

        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;

        let result = unsafe {
            TessPageIteratorBoundingBox(*handle, level as c_int, &mut left, &mut top, &mut right, &mut bottom)
        };

        if result == 0 {
            Err(TesseractError::InvalidParameterError)
        } else {
            Ok((left, top, right, bottom))
        }
    }

    /// Extracts all word data from the iterator in a single mutex lock.
    ///
    /// Acquires the mutex once and iterates all words, collecting text, bounding box,
    /// confidence, and font attributes for each word. This is more efficient than
    /// calling individual methods in a loop since it avoids repeated mutex acquisitions.
    ///
    /// The iterator is always reset to the beginning before traversal so that partial
    /// prior consumption does not cause words to be missed.
    ///
    /// Per-word extraction failures (null pointer, invalid parameter, invalid UTF-8)
    /// are recoverable and do not abort the pass, but they ARE counted in
    /// `WordExtractionOutcome::skipped` so callers can distinguish "no words on this
    /// page" from "words exist but some were dropped by the iterator" (#192).
    ///
    /// # Returns
    ///
    /// Returns a [`WordExtractionOutcome`], or an error if the mutex cannot be
    /// acquired or an unrecoverable iterator error occurs.
    pub fn extract_all_words(&self) -> Result<WordExtractionOutcome> {
        self.extract_all_words_with_line_starts().map(|(outcome, _)| outcome)
    }

    /// Extracts all word data and a parallel flag for each word that begins a text line.
    ///
    /// The returned flags have exactly the same length and order as `outcome.words`. If the
    /// first word in a line cannot be extracted, the next successfully extracted word begins
    /// that line instead.
    pub fn extract_all_words_with_line_starts(&self) -> Result<(WordExtractionOutcome, Vec<bool>)> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let raw = *handle;
        let mut words = Vec::new();
        let mut line_starts = Vec::new();
        let mut skipped = 0usize;

        unsafe { TessPageIteratorBegin(raw) };

        let mut pending_line_start = false;
        loop {
            let word = carry_line_start(
                extract_word_data_unlocked(raw),
                at_text_line_start_unlocked(raw),
                &mut pending_line_start,
            );
            match word {
                Ok((word, starts_line)) => {
                    record_word_extraction_result(Ok(word), &mut words, &mut skipped)?;
                    line_starts.push(starts_line);
                }
                Err(error) => record_word_extraction_result(Err(error), &mut words, &mut skipped)?,
            }

            let has_next = unsafe { TessResultIteratorNext(raw, TessPageIteratorLevel::RIL_WORD as c_int) != 0 };
            if !has_next {
                break;
            }
        }

        Ok((WordExtractionOutcome { words, skipped }, line_starts))
    }

    /// Extracts the current word's data in a single mutex lock.
    ///
    /// Acquires the mutex once and calls all FFI functions (text, bounding box,
    /// confidence, font attributes) within that lock scope. More efficient than
    /// calling the individual methods separately when all fields are needed.
    ///
    /// # Returns
    ///
    /// Returns a [`WordData`] struct if successful, otherwise returns an error.
    pub fn extract_word_data(&self) -> Result<WordData> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        extract_word_data_unlocked(*handle)
    }

    /// Reads the text and box of each symbol in every word that `wanted` accepts, in a single
    /// mutex lock. The iterator is reset to the beginning first. A word whose text, box or
    /// symbols cannot be read is left out, so an absent word means "unknown", not "no symbols". ~keep
    pub fn extract_word_symbols(&self, wanted: impl Fn(&str) -> bool) -> Result<Vec<WordSymbols>> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let raw = *handle;
        let mut words = Vec::new();

        unsafe { TessPageIteratorBegin(raw) };

        loop {
            if let Ok(text) = iterator_text_unlocked(raw, TessPageIteratorLevel::RIL_WORD)
                && wanted(&text)
                && let Ok(word) = word_symbols_unlocked(raw, text)
            {
                words.push(word);
            }

            let has_next = unsafe { TessResultIteratorNext(raw, TessPageIteratorLevel::RIL_WORD as c_int) != 0 };
            if !has_next {
                break;
            }
        }

        Ok(words)
    }
}

/// Classifies a single per-word extraction attempt and folds it into the running
/// `words`/`skipped` totals used by [`ResultIterator::extract_all_words`].
///
/// Null pointer, invalid parameter, and invalid UTF-8 errors are recoverable: the
/// word is dropped and `skipped` is incremented. Any other error is unrecoverable
/// and is propagated to abort the pass. Extracted as a standalone, FFI-free
/// function so the classification/counting logic itself is unit-testable without
/// a live Tesseract handle (#192).
fn record_word_extraction_result(
    result: Result<WordData>,
    words: &mut Vec<WordData>,
    skipped: &mut usize,
) -> Result<()> {
    match result {
        Ok(word) => words.push(word),
        Err(TesseractError::NullPointerError)
        | Err(TesseractError::InvalidParameterError)
        | Err(TesseractError::Utf8Error(_)) => {
            *skipped += 1;
        }
        Err(e) => return Err(e),
    }
    Ok(())
}

/// Moves a text-line start off a word that failed to extract onto the next word that does, so a
/// failed first word does not merge its line into the previous one.
fn carry_line_start(
    word: Result<WordData>,
    at_line_start: bool,
    pending_line_start: &mut bool,
) -> Result<(WordData, bool)> {
    match word {
        Ok(word) => {
            let pending_line_start = std::mem::take(pending_line_start);
            Ok((word, at_line_start || pending_line_start))
        }
        Err(error) => {
            *pending_line_start |= at_line_start;
            Err(error)
        }
    }
}

/// Extracts word data from a raw iterator handle without acquiring the mutex.
///
/// The caller MUST hold the mutex lock for the `ResultIterator` this handle belongs to
/// before calling this function. Passing a handle that is not mutex-guarded, or calling
/// this function concurrently on the same handle, is undefined behaviour.
fn extract_word_data_unlocked(raw: *mut c_void) -> Result<WordData> {
    let text = iterator_text_unlocked(raw, TessPageIteratorLevel::RIL_WORD)?;
    let (left, top, right, bottom) = bounding_box_unlocked(raw, TessPageIteratorLevel::RIL_WORD)?;

    let confidence = unsafe { TessResultIteratorConfidence(raw, TessPageIteratorLevel::RIL_WORD as c_int) };
    let font_attrs = extract_word_font_attributes(raw);
    let language = extract_word_recognition_language(raw);

    Ok(WordData {
        text,
        left,
        top,
        right,
        bottom,
        confidence,
        font_attrs,
        language,
    })
}

/// Whether the current element begins a text line. The caller MUST hold the iterator's mutex. ~keep
fn at_text_line_start_unlocked(raw: *mut c_void) -> bool {
    unsafe { TessPageIteratorIsAtBeginningOf(raw, TessPageIteratorLevel::RIL_TEXTLINE as c_int) != 0 }
}

/// The text of the current element at `level`. The caller MUST hold the iterator's mutex. ~keep
fn iterator_text_unlocked(raw: *mut c_void, level: TessPageIteratorLevel) -> Result<String> {
    let text_ptr = unsafe { TessResultIteratorGetUTF8Text(raw, level as c_int) };
    if text_ptr.is_null() {
        return Err(TesseractError::NullPointerError);
    }
    // SAFETY: Tesseract returned this NUL-terminated allocation and requires TessDeleteText. ~keep
    unsafe {
        copy_and_delete_tess_text(text_ptr, |pointer| {
            // SAFETY: the guard calls this exactly once with the pointer returned by Tesseract. ~keep
            TessDeleteText(pointer);
        })
    }
}

/// The box `(left, top, right, bottom)` of the current element at `level`. The caller MUST hold
/// the iterator's mutex. ~keep
fn bounding_box_unlocked(raw: *mut c_void, level: TessPageIteratorLevel) -> Result<(i32, i32, i32, i32)> {
    let (mut left, mut top, mut right, mut bottom) = (0, 0, 0, 0);
    let found =
        unsafe { TessPageIteratorBoundingBox(raw, level as c_int, &mut left, &mut top, &mut right, &mut bottom) };
    if found == 0 {
        return Err(TesseractError::InvalidParameterError);
    }
    Ok((left, top, right, bottom))
}

/// The current word with the text and box of each of its symbols. The walk leaves the iterator
/// inside the word, so the caller moves on with a word-level step. The caller MUST hold the
/// iterator's mutex. ~keep
fn word_symbols_unlocked(raw: *mut c_void, text: String) -> Result<WordSymbols> {
    let (left, top, right, bottom) = bounding_box_unlocked(raw, TessPageIteratorLevel::RIL_WORD)?;
    let word_level = TessPageIteratorLevel::RIL_WORD as c_int;
    let symbol_level = TessPageIteratorLevel::RIL_SYMBOL as c_int;
    let mut symbols = Vec::new();
    loop {
        let symbol_text = iterator_text_unlocked(raw, TessPageIteratorLevel::RIL_SYMBOL)?;
        let (left, top, right, bottom) = bounding_box_unlocked(raw, TessPageIteratorLevel::RIL_SYMBOL)?;
        symbols.push(SymbolBox {
            text: symbol_text,
            left,
            top,
            right,
            bottom,
        });
        let at_last_symbol = unsafe { TessPageIteratorIsAtFinalElement(raw, word_level, symbol_level) != 0 };
        if at_last_symbol || unsafe { TessResultIteratorNext(raw, symbol_level) } == 0 {
            break;
        }
    }
    Ok(WordSymbols {
        text,
        left,
        top,
        right,
        bottom,
        symbols,
    })
}

/// The current word's font attributes, or `None` when Tesseract reports it has none (the C API
/// signals this with a null font-name pointer, in which case the flag and `font_id` outputs are
/// undefined and must not be read). Split out of
/// [`extract_word_data_unlocked`] to keep that function under the workspace line-count limit. ~keep
fn extract_word_font_attributes(raw: *mut c_void) -> Option<FontAttributes> {
    let mut is_bold = 0;
    let mut is_italic = 0;
    let mut is_underlined = 0;
    let mut is_monospace = 0;
    let mut is_serif = 0;
    let mut is_smallcaps = 0;
    let mut pointsize = 0;
    let mut font_id = 0;
    let result = unsafe {
        TessResultIteratorWordFontAttributes(
            raw,
            &mut is_bold,
            &mut is_italic,
            &mut is_underlined,
            &mut is_monospace,
            &mut is_serif,
            &mut is_smallcaps,
            &mut pointsize,
            &mut font_id,
        )
    };
    if !result.is_null() {
        Some(FontAttributes {
            is_bold: is_bold != 0,
            is_italic: is_italic != 0,
            is_underlined: is_underlined != 0,
            is_monospace: is_monospace != 0,
            is_serif: is_serif != 0,
            is_smallcaps: is_smallcaps != 0,
            pointsize,
            font_id,
        })
    } else {
        None
    }
}

/// The current word's recognition language (e.g. `"eng"`, `"deu"`).
///
/// `TessResultIteratorWordRecognitionLanguage` returns a pointer owned by Tesseract (not by
/// us), so it must NOT be freed via `TessDeleteText` — matching
/// `ResultIterator::word_recognition_language`. A null pointer means Tesseract could not
/// attribute this word to a specific language (e.g. non-LSTM engines, or a word outside the
/// recognized text); that's a normal, non-fatal case, so it maps to `None`, not an error. Split
/// out of [`extract_word_data_unlocked`] to keep that function under the workspace line-count
/// limit. ~keep
fn extract_word_recognition_language(raw: *mut c_void) -> Option<String> {
    let lang_ptr = unsafe { TessResultIteratorWordRecognitionLanguage(raw) };
    if lang_ptr.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(lang_ptr) }.to_str().ok().map(str::to_owned)
    }
}

impl Drop for ResultIterator {
    fn drop(&mut self) {
        if let Ok(handle) = self.handle.lock() {
            unsafe { TessResultIteratorDelete(*handle) };
        }
    }
}

#[cfg(any(feature = "build-tesseract", feature = "build-tesseract-wasm"))]
ffi_extern! {
    pub fn TessResultIteratorDelete(handle: *mut c_void);
    pub fn TessPageIteratorBegin(handle: *mut c_void);
    pub fn TessResultIteratorGetUTF8Text(handle: *mut c_void, level: c_int) -> *mut c_char;
    pub fn TessResultIteratorConfidence(handle: *mut c_void, level: c_int) -> c_float;
    pub fn TessResultIteratorWordRecognitionLanguage(handle: *mut c_void) -> *const c_char;
    pub fn TessResultIteratorWordFontAttributes(
        handle: *mut c_void,
        is_bold: *mut c_int,
        is_italic: *mut c_int,
        is_underlined: *mut c_int,
        is_monospace: *mut c_int,
        is_serif: *mut c_int,
        is_smallcaps: *mut c_int,
        pointsize: *mut c_int,
        font_id: *mut c_int,
    ) -> *const c_char;
    pub fn TessResultIteratorWordIsFromDictionary(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorWordIsNumeric(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorSymbolIsSuperscript(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorSymbolIsSubscript(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorSymbolIsDropcap(handle: *mut c_void) -> c_int;
    pub fn TessResultIteratorNext(handle: *mut c_void, level: c_int) -> c_int;
    pub fn TessPageIteratorIsAtBeginningOf(handle: *mut c_void, level: c_int) -> c_int;
    pub fn TessPageIteratorIsAtFinalElement(handle: *mut c_void, level: c_int, element: c_int) -> c_int;
    pub fn TessPageIteratorBoundingBox(
        handle: *mut c_void,
        level: c_int,
        left: *mut c_int,
        top: *mut c_int,
        right: *mut c_int,
        bottom: *mut c_int,
    ) -> c_int;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn sample_word(text: &str) -> WordData {
        WordData {
            text: text.to_string(),
            right: 10,
            bottom: 10,
            confidence: 90.0,
            language: Some("eng".to_string()),
            ..Default::default()
        }
    }

    /// Runs `carry_line_start` over `(extracted, at_line_start)` attempts in iterator order and
    /// returns each extracted word's line-start flag, plus the flag left pending at the end.
    fn carried_line_starts(attempts: &[(bool, bool)]) -> (Vec<bool>, bool) {
        let mut pending_line_start = false;
        let starts = attempts
            .iter()
            .filter_map(|&(extracted, at_line_start)| {
                let word = if extracted {
                    Ok(sample_word("w"))
                } else {
                    Err(TesseractError::NullPointerError)
                };
                carry_line_start(word, at_line_start, &mut pending_line_start).ok()
            })
            .map(|(_, starts_line)| starts_line)
            .collect();
        (starts, pending_line_start)
    }

    #[test]
    fn should_not_start_a_line_after_a_failure_inside_a_line() {
        assert_eq!(
            carried_line_starts(&[(true, true), (false, false), (true, false)]),
            (vec![true, false], false)
        );
    }

    #[test]
    fn should_start_the_line_at_the_first_word_after_consecutive_failures() {
        assert_eq!(
            carried_line_starts(&[(true, true), (false, true), (false, false), (true, false)]),
            (vec![true, true], false)
        );
    }

    #[test]
    fn should_leave_the_line_start_pending_when_the_last_word_fails() {
        assert_eq!(carried_line_starts(&[(true, true), (false, true)]), (vec![true], true));
    }

    #[test]
    fn should_hand_a_fully_failed_line_start_to_the_next_line() {
        assert_eq!(
            carried_line_starts(&[(true, true), (false, true), (false, false), (true, true), (true, false)]),
            (vec![true, true, false], false)
        );
    }

    fn invalid_utf8_error() -> TesseractError {
        let invalid_bytes: Vec<u8> = vec![0xFF, 0xFE];
        std::str::from_utf8(&invalid_bytes).unwrap_err().into()
    }

    #[test]
    fn should_delete_iterator_text_when_utf8_conversion_fails() {
        let mut invalid_text = vec![0xff_u8, 0];
        let delete_count = Cell::new(0usize);

        // SAFETY: the vector is NUL-terminated and remains alive for the call; the test deleter does not free it. ~keep
        let result = unsafe {
            copy_and_delete_tess_text(invalid_text.as_mut_ptr().cast(), |_| {
                delete_count.set(delete_count.get() + 1)
            })
        };

        assert!(matches!(result, Err(TesseractError::Utf8Error(_))));
        assert_eq!(delete_count.get(), 1, "the native text deleter must run exactly once");
    }

    #[test]
    fn should_delete_iterator_text_once_after_a_successful_copy() {
        let mut valid_text = b"retained\0".to_vec();
        let delete_count = Cell::new(0usize);

        // SAFETY: the vector is NUL-terminated and remains alive for the call; the test deleter does not free it. ~keep
        let result = unsafe {
            copy_and_delete_tess_text(valid_text.as_mut_ptr().cast(), |_| {
                delete_count.set(delete_count.get() + 1)
            })
        };

        assert_eq!(result.unwrap(), "retained");
        assert_eq!(delete_count.get(), 1, "the native text deleter must run exactly once");
    }

    #[test]
    fn should_not_delete_text_when_tesseract_returns_null() {
        let delete_count = Cell::new(0usize);

        // SAFETY: null represents Tesseract returning no allocation; the helper rejects it before dereferencing. ~keep
        let result =
            unsafe { copy_and_delete_tess_text(std::ptr::null_mut(), |_| delete_count.set(delete_count.get() + 1)) };

        assert!(matches!(result, Err(TesseractError::NullPointerError)));
        assert_eq!(delete_count.get(), 0, "no native allocation exists to delete");
    }

    /// Drives [`record_word_extraction_result`] over a fixed, known sequence of
    /// synthetic per-word outcomes — the same seam `extract_all_words` folds its
    /// FFI-derived results through — and asserts the *exact* resulting `skipped`
    /// count and surviving `words`. No live Tesseract handle is involved: this
    /// isolates the counting/classification logic from the FFI iteration (#192).
    #[test]
    fn should_count_exact_number_of_recoverable_failures_and_keep_successful_words() {
        let results: Vec<Result<WordData>> = vec![
            Ok(sample_word("first")),
            Err(TesseractError::NullPointerError),
            Ok(sample_word("second")),
            Err(TesseractError::InvalidParameterError),
            Err(invalid_utf8_error()),
            Ok(sample_word("third")),
        ];

        let mut words = Vec::new();
        let mut skipped = 0usize;
        for result in results {
            record_word_extraction_result(result, &mut words, &mut skipped).expect("all failures here are recoverable");
        }

        assert_eq!(skipped, 3, "exactly three recoverable failures were fed in");
        assert_eq!(words.len(), 3, "exactly three successful words were fed in");
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );
    }

    /// When nothing goes wrong, `skipped` must stay at exactly zero — not merely
    /// "not incremented past some threshold" — so callers can rely on its absence
    /// as a clean-extraction signal.
    #[test]
    fn should_report_zero_skipped_when_no_recoverable_failures_occur() {
        let results: Vec<Result<WordData>> = vec![Ok(sample_word("only")), Ok(sample_word("word"))];

        let mut words = Vec::new();
        let mut skipped = 0usize;
        for result in results {
            record_word_extraction_result(result, &mut words, &mut skipped).unwrap();
        }

        assert_eq!(skipped, 0);
        assert_eq!(words.len(), 2);
    }

    /// An unrecoverable error (anything outside the three recoverable variants)
    /// must propagate instead of being silently folded into `skipped`.
    #[test]
    fn should_propagate_unrecoverable_error_instead_of_counting_it_as_skipped() {
        let mut words = Vec::new();
        let mut skipped = 0usize;

        let outcome = record_word_extraction_result(Err(TesseractError::MutexLockError), &mut words, &mut skipped);

        assert!(outcome.is_err());
        assert_eq!(skipped, 0, "unrecoverable errors must not be counted as skipped");
        assert!(words.is_empty());
    }
}

#[cfg(test)]
mod native_signature_contract_tests {
    use super::*;

    #[test]
    fn word_font_attributes_returns_a_borrowed_font_name() {
        #[cfg(not(target_arch = "wasm32"))]
        type NativeFunction = unsafe extern "C-unwind" fn(
            *mut c_void,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
        ) -> *const c_char;
        #[cfg(target_arch = "wasm32")]
        type NativeFunction = unsafe extern "C" fn(
            *mut c_void,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
            *mut c_int,
        ) -> *const c_char;

        let _: NativeFunction = super::TessResultIteratorWordFontAttributes;
    }
}
