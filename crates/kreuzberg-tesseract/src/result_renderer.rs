use crate::TesseractAPI;
use crate::error::{Result, TesseractError};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::Arc;
use std::sync::Mutex;

pub struct TessResultRenderer {
    handle: Arc<Mutex<*mut c_void>>,
}

unsafe impl Send for TessResultRenderer {}
unsafe impl Sync for TessResultRenderer {}

impl TessResultRenderer {
    /// Creates a new instance of the TessResultRenderer.
    ///
    /// # Arguments
    ///
    /// * `outputbase` - Output base path.
    ///
    /// # Returns
    ///
    /// Returns the new instance of the TessResultRenderer.
    pub fn new_text_renderer(outputbase: &str) -> Result<Self> {
        let outputbase = CString::new(outputbase).unwrap();
        let handle = unsafe { TessTextRendererCreate(outputbase.as_ptr()) };
        if handle.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(TessResultRenderer {
                handle: Arc::new(Mutex::new(handle)),
            })
        }
    }

    /// Creates a new instance of the TessResultRenderer for HOCR.
    ///
    /// # Arguments
    ///
    /// * `outputbase` - Output base path.
    ///
    /// # Returns
    ///
    /// Returns the new instance of the TessResultRenderer.
    pub fn new_hocr_renderer(outputbase: &str) -> Result<Self> {
        let outputbase = CString::new(outputbase).unwrap();
        let handle = unsafe { TessHOcrRendererCreate(outputbase.as_ptr()) };
        if handle.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(TessResultRenderer {
                handle: Arc::new(Mutex::new(handle)),
            })
        }
    }

    /// Creates a new instance of the TessResultRenderer for PDF.
    ///
    /// # Arguments
    ///
    /// * `outputbase` - Output base path.
    /// * `datadir` - Data directory path.
    /// * `textonly` - Whether to include text only.
    ///
    /// # Returns
    ///
    /// Returns the new instance of the TessResultRenderer.
    pub fn new_pdf_renderer(outputbase: &str, datadir: &str, textonly: bool) -> Result<Self> {
        let outputbase = CString::new(outputbase).unwrap();
        let datadir = CString::new(datadir).unwrap();
        let handle = unsafe { TessPDFRendererCreate(outputbase.as_ptr(), datadir.as_ptr(), textonly as c_int) };
        if handle.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            Ok(TessResultRenderer {
                handle: Arc::new(Mutex::new(handle)),
            })
        }
    }

    /// Begins a new document.
    ///
    /// # Arguments
    ///
    /// * `title` - Title of the document.
    ///
    /// # Returns
    ///
    /// Returns `true` if the document was created successfully, otherwise returns `false`.
    pub fn begin_document(&self, title: &str) -> bool {
        let title = CString::new(title).unwrap();
        let handle = self.handle.lock().unwrap();
        unsafe { TessResultRendererBeginDocument(*handle, title.as_ptr()) != 0 }
    }

    /// Adds an image to the document.
    ///
    /// # Arguments
    ///
    /// * `api` - The TesseractAPI instance.
    ///
    /// # Returns
    ///
    /// Returns `true` if the image was added successfully, otherwise returns `false`.
    pub fn add_image(&self, api: &TesseractAPI) -> bool {
        let api_handle = api.handle.lock().unwrap();
        let handle = self.handle.lock().unwrap();
        unsafe { TessResultRendererAddImage(*handle, *api_handle) != 0 }
    }

    /// Ends the document.
    ///
    /// # Returns
    ///
    /// Returns `true` if the document was ended successfully, otherwise returns `false`.
    pub fn end_document(&self) -> bool {
        let handle = self.handle.lock().unwrap();
        unsafe { TessResultRendererEndDocument(*handle) != 0 }
    }

    /// Gets the extension of the document.
    ///
    /// # Returns
    ///
    /// Returns the extension as a `String` if successful, otherwise returns an error.
    pub fn get_extension(&self) -> Result<String> {
        let handle = self.handle.lock().unwrap();
        let ext_ptr = unsafe { TessResultRendererExtention(*handle) };
        if ext_ptr.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            let c_str = unsafe { CStr::from_ptr(ext_ptr) };
            Ok(c_str.to_str()?.to_owned())
        }
    }

    /// Gets the title of the document.
    ///
    /// # Returns
    ///
    /// Returns the title as a `String` if successful, otherwise returns an error.
    pub fn get_title(&self) -> Result<String> {
        let handle = self.handle.lock().unwrap();
        let title_ptr = unsafe { TessResultRendererTitle(*handle) };
        if title_ptr.is_null() {
            Err(TesseractError::NullPointerError)
        } else {
            let c_str = unsafe { CStr::from_ptr(title_ptr) };
            Ok(c_str.to_str()?.to_owned())
        }
    }

    /// Gets the number of images in the document.
    ///
    /// # Returns
    ///
    /// Returns the number of images as an `i32`.
    pub fn get_image_num(&self) -> i32 {
        let handle = self.handle.lock().unwrap();
        unsafe { TessResultRendererImageNum(*handle) }
    }
}

impl Drop for TessResultRenderer {
    fn drop(&mut self) {
        let handle = self.handle.lock().unwrap();
        unsafe { TessDeleteResultRenderer(*handle) };
    }
}

unsafe extern "C" {
    pub fn TessTextRendererCreate(outputbase: *const c_char) -> *mut c_void;
    pub fn TessHOcrRendererCreate(outputbase: *const c_char) -> *mut c_void;
    pub fn TessPDFRendererCreate(outputbase: *const c_char, datadir: *const c_char, textonly: c_int) -> *mut c_void;
    pub fn TessDeleteResultRenderer(renderer: *mut c_void);
    pub fn TessResultRendererBeginDocument(renderer: *mut c_void, title: *const c_char) -> c_int;
    pub fn TessResultRendererAddImage(renderer: *mut c_void, api: *mut c_void) -> c_int;
    pub fn TessResultRendererEndDocument(renderer: *mut c_void) -> c_int;
    pub fn TessResultRendererExtention(renderer: *mut c_void) -> *const c_char;
    pub fn TessResultRendererTitle(renderer: *mut c_void) -> *const c_char;
    pub fn TessResultRendererImageNum(renderer: *mut c_void) -> c_int;
}
