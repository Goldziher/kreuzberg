use crate::api::TessDeleteText;
use crate::error::{Result, TesseractError};
use std::ffi::CStr;
use std::os::raw::{c_char, c_float, c_int, c_void};
use std::sync::{Arc, Mutex};

pub struct ChoiceIterator {
    handle: Arc<Mutex<*mut c_void>>,
}

unsafe impl Send for ChoiceIterator {}
unsafe impl Sync for ChoiceIterator {}

impl ChoiceIterator {
    /// Creates a new instance of the ChoiceIterator.
    ///
    /// # Arguments
    ///
    /// * `handle` - Pointer to the ChoiceIterator.
    pub fn new(handle: *mut c_void) -> Self {
        ChoiceIterator {
            handle: Arc::new(Mutex::new(handle)),
        }
    }

    /// Gets the next choice.
    ///
    /// # Returns
    ///
    /// Returns `true` if the next choice is successful, otherwise returns `false`.
    pub fn next(&self) -> Result<bool> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessChoiceIteratorNext(*handle) != 0 })
    }

    /// Gets the UTF-8 text for the current choice.
    ///
    /// # Returns
    ///
    /// Returns the UTF-8 text as a `String` if successful, otherwise returns an error.
    pub fn get_utf8_text(&self) -> Result<String> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        let text_ptr = unsafe { TessChoiceIteratorGetUTF8Text(*handle) };
        if text_ptr.is_null() {
            return Err(TesseractError::NullPointerError);
        }
        let c_str = unsafe { CStr::from_ptr(text_ptr) };
        let result = c_str.to_str()?.to_owned();
        unsafe { TessDeleteText(text_ptr) };
        Ok(result)
    }

    /// Gets the confidence of the current choice.
    ///
    /// # Returns
    ///
    /// Returns the confidence as a `f32`.
    pub fn confidence(&self) -> Result<f32> {
        let handle = self.handle.lock().map_err(|_| TesseractError::MutexLockError)?;
        Ok(unsafe { TessChoiceIteratorConfidence(*handle) })
    }
}

impl Drop for ChoiceIterator {
    fn drop(&mut self) {
        if let Ok(handle) = self.handle.lock() {
            unsafe { TessChoiceIteratorDelete(*handle) };
        }
    }
}

unsafe extern "C" {
    fn TessChoiceIteratorDelete(handle: *mut c_void);
    fn TessChoiceIteratorNext(handle: *mut c_void) -> c_int;
    fn TessChoiceIteratorGetUTF8Text(handle: *mut c_void) -> *mut c_char;
    fn TessChoiceIteratorConfidence(handle: *mut c_void) -> c_float;
}
