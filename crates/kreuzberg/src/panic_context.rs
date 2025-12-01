use std::any::Any;
use std::time::{SystemTime, UNIX_EPOCH};

/// Context information captured when a panic occurs.
///
/// This struct stores detailed information about where and when a panic happened,
/// enabling better error reporting across FFI boundaries.
#[derive(Debug, Clone)]
pub struct PanicContext {
    /// Source file where the panic occurred
    pub file: &'static str,
    /// Line number where the panic occurred
    pub line: u32,
    /// Function name where the panic occurred
    pub function: &'static str,
    /// Panic message extracted from the panic payload
    pub message: String,
    /// Timestamp when the panic was captured
    pub timestamp: SystemTime,
}

impl PanicContext {
    /// Creates a new PanicContext with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `file` - Source file path
    /// * `line` - Line number
    /// * `function` - Function name
    /// * `panic_info` - The panic payload to extract message from
    pub fn new(file: &'static str, line: u32, function: &'static str, panic_info: &dyn Any) -> Self {
        // Catch panic from SystemTime::now() to prevent panic-in-panic scenario
        let timestamp = std::panic::catch_unwind(SystemTime::now).unwrap_or(UNIX_EPOCH);

        Self {
            file,
            line,
            function,
            message: extract_panic_message(panic_info),
            timestamp,
        }
    }

    /// Formats the panic context as a human-readable string.
    pub fn format(&self) -> String {
        format!(
            "Panic at {}:{}:{} - {}",
            self.file, self.line, self.function, self.message
        )
    }
}

/// Maximum panic message length to prevent DoS attacks
const MAX_PANIC_MESSAGE_LEN: usize = 4096;

/// Extracts a human-readable message from a panic payload.
///
/// Attempts to downcast the panic payload to common types (String, &str)
/// to extract a meaningful error message.
///
/// Message is truncated to 4KB to prevent DoS attacks via extremely large panic messages.
///
/// # Arguments
///
/// * `panic_info` - The panic payload from catch_unwind
///
/// # Returns
///
/// A string representation of the panic message (truncated if necessary)
pub fn extract_panic_message(panic_info: &dyn Any) -> String {
    let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = panic_info.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "Unknown panic payload".to_string()
    };

    // Truncate to prevent DoS via extremely large panic messages
    if msg.len() > MAX_PANIC_MESSAGE_LEN {
        // Use floor_char_boundary to avoid panicking on multi-byte UTF-8 character boundaries
        let truncate_at = msg.floor_char_boundary(MAX_PANIC_MESSAGE_LEN);
        format!("{}... [truncated]", &msg[..truncate_at])
    } else {
        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_panic_message_string() {
        let panic_msg = "test panic".to_string();
        let msg = extract_panic_message(&panic_msg);
        assert_eq!(msg, "test panic");
    }

    #[test]
    fn test_extract_panic_message_str() {
        let panic_msg: &str = "test panic";
        let msg = extract_panic_message(&panic_msg);
        assert_eq!(msg, "test panic");
    }

    #[test]
    fn test_extract_panic_message_unknown() {
        let panic_msg = 42i32;
        let msg = extract_panic_message(&panic_msg);
        assert_eq!(msg, "Unknown panic payload");
    }

    #[test]
    fn test_panic_context_format() {
        let panic_msg = "test error".to_string();
        let ctx = PanicContext::new("test.rs", 42, "test_function", &panic_msg);

        let formatted = ctx.format();
        assert!(formatted.contains("test.rs"));
        assert!(formatted.contains("42"));
        assert!(formatted.contains("test_function"));
        assert!(formatted.contains("test error"));
    }

    #[test]
    fn test_panic_message_truncation() {
        let long_msg = "x".repeat(5000);
        let msg = extract_panic_message(&long_msg);
        assert!(msg.len() <= MAX_PANIC_MESSAGE_LEN + 20); // +20 for "... [truncated]"
        assert!(msg.ends_with("... [truncated]"));
    }

    #[test]
    fn test_panic_message_truncation_utf8_boundary() {
        // Create a message with multi-byte UTF-8 character near 4096 boundary
        // This tests that truncation doesn't panic on char boundaries
        let mut msg = "x".repeat(4093); // 4093 bytes
        msg.push('🦀'); // Rust crab emoji: 4 bytes in UTF-8 (F0 9F A6 80)
        msg.push_str("yyy"); // 3 more bytes = 4100 total

        // This should NOT panic, even though byte 4096 is mid-character
        let truncated = extract_panic_message(&msg);

        // Should be truncated at valid UTF-8 boundary
        assert!(truncated.ends_with("... [truncated]"));

        // Verify it's valid UTF-8 (would panic on invalid UTF-8)
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());

        // The emoji and trailing "yyy" should not be present (they were after the truncation point)
        assert!(!truncated.contains("🦀"));
        assert!(!truncated.contains("yyy"));
    }

    #[test]
    fn test_panic_message_no_truncation_needed() {
        let short_msg = "short".to_string();
        let msg = extract_panic_message(&short_msg);
        assert_eq!(msg, "short");
        assert!(!msg.contains("[truncated]"));
    }
}
