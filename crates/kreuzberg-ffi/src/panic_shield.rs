use kreuzberg::panic_context::PanicContext;
use std::cell::RefCell;

/// Structured error that includes both the error message and optional panic context.
#[derive(Debug, Clone)]
pub struct StructuredError {
    /// The error message
    pub message: String,
    /// Optional panic context if this error originated from a panic
    pub panic_context: Option<PanicContext>,
    /// Error code for programmatic error handling
    pub code: ErrorCode,
}

/// Error codes for different types of errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    /// No error
    Success = 0,
    /// Generic error
    GenericError = 1,
    /// Panic was caught
    Panic = 2,
    /// Invalid argument
    InvalidArgument = 3,
    /// IO error
    IoError = 4,
    /// Parsing error
    ParsingError = 5,
    /// OCR error
    OcrError = 6,
    /// Missing dependency
    MissingDependency = 7,
}

impl StructuredError {
    /// Creates a new StructuredError from a panic context.
    pub fn from_panic(context: PanicContext) -> Self {
        Self {
            message: context.format(),
            panic_context: Some(context),
            code: ErrorCode::Panic,
        }
    }

    /// Creates a new StructuredError from a regular error message.
    pub fn from_message(message: String, code: ErrorCode) -> Self {
        Self {
            message,
            panic_context: None,
            code,
        }
    }

    /// Returns the full error message including panic context if available.
    pub fn full_message(&self) -> String {
        if let Some(ref ctx) = self.panic_context {
            format!("{} (at {}:{}:{})", self.message, ctx.file, ctx.line, ctx.function)
        } else {
            self.message.clone()
        }
    }
}

thread_local! {
    static LAST_STRUCTURED_ERROR: RefCell<Option<StructuredError>> = const { RefCell::new(None) };
}

/// Sets the last structured error.
pub fn set_structured_error(error: StructuredError) {
    LAST_STRUCTURED_ERROR.with(|last| *last.borrow_mut() = Some(error));
}

/// Gets the last structured error message (for compatibility with existing code).
pub fn get_last_error_message() -> Option<String> {
    LAST_STRUCTURED_ERROR.with(|last| last.borrow().as_ref().map(|e| e.full_message()))
}

/// Gets the last error code.
pub fn get_last_error_code() -> ErrorCode {
    LAST_STRUCTURED_ERROR.with(|last| {
        last.borrow()
            .as_ref()
            .map(|e| e.code)
            .unwrap_or(ErrorCode::Success)
    })
}

/// Gets the last panic context if the last error was a panic.
pub fn get_last_panic_context() -> Option<PanicContext> {
    LAST_STRUCTURED_ERROR.with(|last| last.borrow().as_ref().and_then(|e| e.panic_context.clone()))
}

/// Clears the last structured error.
pub fn clear_structured_error() {
    LAST_STRUCTURED_ERROR.with(|last| *last.borrow_mut() = None);
}

/// Macro to wrap FFI functions with panic catching.
///
/// This macro catches panics at FFI boundaries and converts them to structured errors.
/// It captures file, line, and function information for better error reporting.
///
/// # Usage
///
/// ```rust,ignore
/// #[no_mangle]
/// pub extern "C" fn my_ffi_function(arg: *const c_char) -> *mut ExtractionResult {
///     ffi_panic_guard!("my_ffi_function", {
///         // Your FFI function body here
///         // Return the result normally
///     })
/// }
/// ```
///
/// For bool-returning functions:
///
/// ```rust,ignore
/// #[no_mangle]
/// pub extern "C" fn my_bool_function(arg: *const c_char) -> bool {
///     ffi_panic_guard_bool!("my_bool_function", {
///         // Your FFI function body here
///         // Return true or false normally
///     })
/// }
/// ```
///
/// The macro will:
/// - Catch any panics that occur in the wrapped code
/// - Create a PanicContext with file/line/function information
/// - Store the structured error in thread-local storage
/// - Return a null pointer (for pointer-returning functions) or false (for bool-returning functions) to indicate failure
#[macro_export]
macro_rules! ffi_panic_guard {
    ($function_name:expr, $body:expr) => {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(panic_info) => {
                let context = kreuzberg::panic_context::PanicContext::new(
                    file!(),
                    line!(),
                    $function_name,
                    panic_info.as_ref(),
                );
                $crate::panic_shield::set_structured_error($crate::panic_shield::StructuredError::from_panic(
                    context,
                ));
                std::ptr::null_mut()
            }
        }
    }};
}

/// Macro to wrap FFI functions that return bool with panic catching.
///
/// This variant of ffi_panic_guard returns false on panic (suitable for bool-returning functions).
#[macro_export]
macro_rules! ffi_panic_guard_bool {
    ($function_name:expr, $body:expr) => {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(panic_info) => {
                let context = kreuzberg::panic_context::PanicContext::new(
                    file!(),
                    line!(),
                    $function_name,
                    panic_info.as_ref(),
                );
                $crate::panic_shield::set_structured_error($crate::panic_shield::StructuredError::from_panic(
                    context,
                ));
                false
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_error_from_panic() {
        let panic_msg = "test panic".to_string();
        let ctx = PanicContext::new("test.rs", 42, "test_fn", &panic_msg);
        let err = StructuredError::from_panic(ctx);

        assert_eq!(err.code, ErrorCode::Panic);
        assert!(err.panic_context.is_some());
        assert!(err.message.contains("test panic"));
    }

    #[test]
    fn test_structured_error_from_message() {
        let err = StructuredError::from_message("test error".to_string(), ErrorCode::GenericError);

        assert_eq!(err.code, ErrorCode::GenericError);
        assert!(err.panic_context.is_none());
        assert_eq!(err.message, "test error");
    }

    #[test]
    fn test_error_storage() {
        clear_structured_error();
        assert!(get_last_error_message().is_none());

        let err = StructuredError::from_message("test".to_string(), ErrorCode::IoError);
        set_structured_error(err);

        assert_eq!(get_last_error_message(), Some("test".to_string()));
        assert_eq!(get_last_error_code(), ErrorCode::IoError);

        clear_structured_error();
        assert!(get_last_error_message().is_none());
    }

    #[test]
    fn test_panic_context_extraction() {
        clear_structured_error();

        let panic_msg = "panic message".to_string();
        let ctx = PanicContext::new("file.rs", 10, "func", &panic_msg);
        let err = StructuredError::from_panic(ctx);
        set_structured_error(err);

        let retrieved_ctx = get_last_panic_context();
        assert!(retrieved_ctx.is_some());

        let ctx = retrieved_ctx.unwrap();
        assert_eq!(ctx.file, "file.rs");
        assert_eq!(ctx.line, 10);
        assert_eq!(ctx.function, "func");
    }

    #[test]
    fn test_ffi_panic_guard_success() {
        let result = crate::ffi_panic_guard!("test_success", {
            Box::into_raw(Box::new(42))
        });
        assert!(!result.is_null());
        unsafe {
            assert_eq!(*result, 42);
            let _ = Box::from_raw(result);
        }
    }

    #[test]
    fn test_ffi_panic_guard_panic() {
        clear_structured_error();

        let result: *mut i32 = crate::ffi_panic_guard!("test_panic", {
            panic!("intentional panic");
            #[allow(unreachable_code)]
            Box::into_raw(Box::new(42))
        });

        assert!(result.is_null());
        assert!(get_last_error_message().is_some());
        assert_eq!(get_last_error_code(), ErrorCode::Panic);

        let msg = get_last_error_message().unwrap();
        assert!(msg.contains("intentional panic"));
        assert!(msg.contains("test_panic"));
    }
}
