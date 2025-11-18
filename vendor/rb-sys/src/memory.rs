/// Prevents premature destruction of local objects.
///
/// Ruby's garbage collector is conservative; it scans the C level machine stack as well.
/// Possible in-use Ruby objects must remain visible on stack, to be properly marked as such.
/// However, Rust's compiler optimizations might remove the references to these objects from
/// the stack when they are not being used directly.
///
/// Consider the following example:
///
/// ```ignore
/// use rb_sys::{rb_str_new_cstr, rb_str_cat_cstr, RSTRING_PTR, rb_gc_guard};
///
/// unsafe {
///     let s = rb_str_new_cstr(" world\0".as_ptr() as _);
///     let sptr = RSTRING_PTR(s);
///     let t = rb_str_new_cstr("hello,\0".as_ptr() as _); // Possible GC invocation
///     let u = rb_str_cat_cstr(t, sptr);
///     rb_gc_guard!(s); // ensure `s` (and thus `sptr`) do not get GC-ed
/// }
/// ```
///
/// In this example, without the `rb_gc_guard!`, the last use of `s` is before the last use
/// of `sptr`. Compilers could think `s` and `t` are allowed to overlap. That would
/// eliminate `s` from the stack, while `sptr` is still in use. If our GC runs at that
/// very moment, `s` gets swept out, which also destroys `sptr`.
///
/// In order to prevent this scenario, `rb_gc_guard!` must be placed after the last use
/// of `sptr`. Placing `rb_gc_guard!` before dereferencing `sptr` would be of no use.
///
/// Using the `rb_gc_guard!` macro has the following advantages:
///
/// - the intent of the macro use is clear.
///
/// - `rb_gc_guard!` only affects its call site, without negatively affecting other systems.
///
/// # Example
/// ```no_run
/// use rb_sys::{rb_utf8_str_new_cstr, rb_gc_guard};
///
/// let my_string = unsafe { rb_utf8_str_new_cstr("hello world\0".as_ptr() as _) };
/// let _ = rb_gc_guard!(my_string);
/// ```
#[macro_export]
macro_rules! rb_gc_guard {
    ($v:expr) => {{
        unsafe {
            let val: $crate::VALUE = $v;
            let rb_gc_guarded_ptr = std::ptr::read_volatile(&&val);
            std::arch::asm!("/* {0} */", in(reg) rb_gc_guarded_ptr);
            *rb_gc_guarded_ptr
        }
    }};
}
