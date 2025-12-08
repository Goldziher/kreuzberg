/// Finds or creates a symbol for the given static string. This macro will
/// memoize the ID to avoid repeated calls to libruby. You should prefer this
/// macro over [`rb_intern3`] when the string is known at compile time.
///
/// # Safety
///
/// This macro is safe under two conditions:
///   - Ruby VM is initialized and that thus safe to call into libruby
///   - The first call to this macro will be done inside of a managed Ruby thread (i.e. not a native thread)
///
/// # Example
///
/// ```no_run
/// use rb_sys::{symbol::rb_intern, rb_funcall, rb_utf8_str_new};
///
/// unsafe {
///   let reverse_id = rb_intern!("reverse");
///   let msg = rb_utf8_str_new("nice one".as_ptr() as *mut _, 4);
///   rb_funcall(msg, reverse_id, 0);
/// }
/// ```
#[macro_export]
macro_rules! rb_intern {
    ($s:literal) => {{
        static mut ID: $crate::ID = 0;
        if ID == 0 {
            ID = $crate::rb_intern3($s.as_ptr() as _, $s.len() as _, $crate::rb_utf8_encoding());
        }
        ID
    }};
}
