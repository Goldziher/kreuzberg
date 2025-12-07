use crate::error::{Result, TesseractError};
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::sync::Arc;
use std::sync::Mutex;

use crate::result_iterator::{
    TessResultIteratorConfidence, TessResultIteratorGetUTF8Text, TessResultIteratorNext,
    TessResultIteratorSymbolIsDropcap, TessResultIteratorSymbolIsSubscript, TessResultIteratorSymbolIsSuperscript,
    TessResultIteratorWordFontAttributes, TessResultIteratorWordIsFromDictionary, TessResultIteratorWordIsNumeric,
    TessResultIteratorWordRecognitionLanguage,
};

pub struct MutableIterator {
    handle: Arc<Mutex<*mut c_void>>,
}

unsafe impl Send for MutableIterator {}
unsafe impl Sync for MutableIterator {}

impl MutableIterator {
    /// Creates a new instance of the MutableIterator.
    ///
    /// # Arguments
    ///
    /// * `handle` - Pointer to the MutableIterator.
    pub fn new(handle: *mut c_void) -> Self {
        MutableIterator {
            handle: Arc::new(Mutex::new(handle)),
        }
    }

    /// Gets the UTF-8 text for the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the text.
    pub fn get_utf8_text(&self, level: i32) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        let text_ptr = unsafe { TessResultIteratorGetUTF8Text(*handle, level) };
        if text_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        unsafe { TessDeleteText(text_ptr as *mut c_char) };
        Ok(result)
    }

    /// Gets the confidence of the current iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the confidence.
    pub fn confidence(&self, level: i32) -> Result<f32> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        Ok(unsafe { TessResultIteratorConfidence(*handle, level) })
    }

    /// Gets the recognition language of the current iterator.
    ///
    /// # Returns
    ///
    /// Returns the recognition language as a `String` if successful, otherwise returns an error.
    pub fn word_recognition_language(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
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
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
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

        if result == 0 {
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

    /// Checks if the current word is from the dictionary.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current word is from the dictionary, otherwise returns `Ok(false)`.
    pub fn word_is_from_dictionary(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        Ok(unsafe { TessResultIteratorWordIsFromDictionary(*handle) != 0 })
    }

    /// Checks if the current word is numeric.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current word is numeric, otherwise returns `Ok(false)`.
    pub fn word_is_numeric(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        Ok(unsafe { TessResultIteratorWordIsNumeric(*handle) != 0 })
    }

    /// Checks if the current symbol is superscript.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current symbol is superscript, otherwise returns `Ok(false)`.
    pub fn symbol_is_superscript(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        Ok(unsafe { TessResultIteratorSymbolIsSuperscript(*handle) != 0 })
    }

    /// Checks if the current symbol is subscript.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current symbol is subscript, otherwise returns `Ok(false)`.
    pub fn symbol_is_subscript(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        Ok(unsafe { TessResultIteratorSymbolIsSubscript(*handle) != 0 })
    }

    /// Checks if the current symbol is dropcap.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the current symbol is dropcap, otherwise returns `Ok(false)`.
    pub fn symbol_is_dropcap(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        Ok(unsafe { TessResultIteratorSymbolIsDropcap(*handle) != 0 })
    }

    /// Gets the next iterator.
    ///
    /// # Arguments
    ///
    /// * `level` - Level of the iterator.
    ///
    /// # Returns
    ///
    /// Returns `true` if the next iterator is successful, otherwise returns `false`.
    pub fn next(&self, level: i32) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexError)?;
        Ok(unsafe { TessResultIteratorNext(*handle, level) != 0 })
    }
}

impl Drop for MutableIterator {
    fn drop(&mut self) {
        if let Ok(handle) = self.handle.lock() {
            unsafe { TessResultIteratorDelete(*handle) };
        }
    }
}

unsafe extern "C" {
    pub fn TessResultIteratorDelete(handle: *mut c_void);
    pub fn TessDeleteText(text: *mut c_char);
}
