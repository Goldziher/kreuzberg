//! Helper to define the `ruby_abi_version` function needed for extensions.
//!
//! Since Ruby 3.2, gems are required to define a `ruby_abi_version` function.
//! For C extensions, this is done transparently by including `ruby.h`, but for
//! Rust we have to define it ourselves. This is enabled automatically by when
//! compiling a gem.

#[doc(hidden)]
#[cfg(not(has_ruby_abi_version))]
pub const __RB_SYS_RUBY_ABI_VERSION: std::os::raw::c_ulonglong = 0;

#[doc(hidden)]
#[cfg(has_ruby_abi_version)]
pub const __RB_SYS_RUBY_ABI_VERSION: std::os::raw::c_ulonglong = crate::RUBY_ABI_VERSION as _;

#[doc(hidden)]
#[no_mangle]
#[allow(unused)]
pub extern "C" fn ruby_abi_version() -> std::os::raw::c_ulonglong {
    __RB_SYS_RUBY_ABI_VERSION
}

#[doc(hidden)]
#[no_mangle]
#[allow(unused)]
#[cfg(ruby_engine = "truffleruby")]
pub extern "C" fn rb_tr_abi_version() -> *const std::os::raw::c_char {
    crate::TRUFFLERUBY_ABI_VERSION.as_ptr() as *const _
}

#[deprecated(
    since = "0.9.102",
    note = "You no longer need to invoke this macro, the `ruby_abi_version` function is defined automatically."
)]
#[macro_export]
macro_rules! ruby_abi_version {
    () => {};
}
