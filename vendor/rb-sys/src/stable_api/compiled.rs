use super::StableApiDefinition;
use crate::{RUBY_API_VERSION_MAJOR, RUBY_API_VERSION_MINOR, VALUE, ruby_value_type, timeval};
use std::{
    ffi::c_void,
    os::raw::{c_char, c_long},
    ptr::NonNull,
    time::Duration,
};

#[allow(dead_code)]
extern "C" {
    #[link_name = "impl_rstring_len"]
    fn impl_rstring_len(str: VALUE) -> c_long;

    #[link_name = "impl_rstring_ptr"]
    fn impl_rstring_ptr(str: VALUE) -> *const c_char;

    #[link_name = "impl_rarray_len"]
    fn impl_rarray_len(ary: VALUE) -> c_long;

    #[link_name = "impl_rarray_const_ptr"]
    fn impl_rarray_const_ptr(ary: VALUE) -> *const VALUE;

    #[link_name = "impl_rbasic_class"]
    fn impl_rbasic_class(obj: VALUE) -> VALUE;

    #[link_name = "impl_frozen_p"]
    fn impl_frozen_p(obj: VALUE) -> bool;

    #[link_name = "impl_special_const_p"]
    fn impl_special_const_p(value: VALUE) -> bool;

    #[link_name = "impl_bignum_positive_p"]
    fn impl_bignum_positive_p(obj: VALUE) -> bool;

    #[link_name = "impl_bignum_negative_p"]
    fn impl_bignum_negative_p(obj: VALUE) -> bool;

    #[link_name = "impl_builtin_type"]
    fn impl_builtin_type(obj: VALUE) -> ruby_value_type;

    #[link_name = "impl_nil_p"]
    fn impl_nil_p(obj: VALUE) -> bool;

    #[link_name = "impl_fixnum_p"]
    fn impl_fixnum_p(obj: VALUE) -> bool;

    #[link_name = "impl_static_sym_p"]
    fn impl_static_sym_p(obj: VALUE) -> bool;

    #[link_name = "impl_flonum_p"]
    fn impl_flonum_p(obj: VALUE) -> bool;

    #[link_name = "impl_immediate_p"]
    fn impl_immediate_p(obj: VALUE) -> bool;

    #[link_name = "impl_rb_test"]
    fn impl_rb_test(obj: VALUE) -> bool;

    #[link_name = "impl_type_p"]
    fn impl_type_p(obj: VALUE, ty: ruby_value_type) -> bool;

    #[link_name = "impl_dynamic_sym_p"]
    fn impl_dynamic_sym_p(obj: VALUE) -> bool;

    #[link_name = "impl_symbol_p"]
    fn impl_symbol_p(obj: VALUE) -> bool;

    #[link_name = "impl_float_type_p"]
    fn impl_float_type_p(obj: VALUE) -> bool;

    #[link_name = "impl_rb_type"]
    fn impl_rb_type(obj: VALUE) -> ruby_value_type;

    #[link_name = "impl_integer_type_p"]
    fn impl_integer_type_p(obj: VALUE) -> bool;

    #[link_name = "impl_rstring_interned_p"]
    fn impl_rstring_interned_p(obj: VALUE) -> bool;

    #[link_name = "impl_thread_sleep"]
    fn impl_thread_sleep(interval: timeval);

    #[link_name = "impl_rtypeddata_p"]
    fn impl_rtypeddata_p(obj: VALUE) -> bool;

    #[link_name = "impl_rtypeddata_embedded_p"]
    fn impl_rtypeddata_embedded_p(obj: VALUE) -> bool;

    #[link_name = "impl_rtypeddata_type"]
    fn impl_rtypeddata_type(obj: VALUE) -> *const crate::rb_data_type_t;

    #[link_name = "impl_rtypeddata_get_data"]
    fn impl_rtypeddata_get_data(obj: VALUE) -> *mut c_void;
}

pub struct Definition;

impl StableApiDefinition for Definition {
    const VERSION_MAJOR: u32 = RUBY_API_VERSION_MAJOR;
    const VERSION_MINOR: u32 = RUBY_API_VERSION_MINOR;

    #[inline]
    unsafe fn rstring_len(&self, obj: VALUE) -> std::os::raw::c_long {
        impl_rstring_len(obj)
    }

    #[inline]
    unsafe fn rstring_ptr(&self, obj: VALUE) -> *const std::os::raw::c_char {
        impl_rstring_ptr(obj)
    }

    #[inline]
    unsafe fn rarray_len(&self, obj: VALUE) -> std::os::raw::c_long {
        impl_rarray_len(obj)
    }

    #[inline]
    unsafe fn rarray_const_ptr(&self, obj: VALUE) -> *const VALUE {
        impl_rarray_const_ptr(obj)
    }

    #[inline]
    unsafe fn rbasic_class(&self, obj: VALUE) -> Option<NonNull<VALUE>> {
        NonNull::<VALUE>::new(impl_rbasic_class(obj) as _)
    }

    unsafe fn frozen_p(&self, obj: VALUE) -> bool {
        impl_frozen_p(obj)
    }

    #[inline]
    fn special_const_p(&self, value: VALUE) -> bool {
        unsafe { impl_special_const_p(value) }
    }

    #[inline]
    unsafe fn bignum_positive_p(&self, obj: VALUE) -> bool {
        impl_bignum_positive_p(obj)
    }

    #[inline]
    unsafe fn bignum_negative_p(&self, obj: VALUE) -> bool {
        impl_bignum_negative_p(obj)
    }

    #[inline]
    unsafe fn builtin_type(&self, obj: VALUE) -> ruby_value_type {
        impl_builtin_type(obj)
    }

    #[inline]
    fn nil_p(&self, obj: VALUE) -> bool {
        unsafe { impl_nil_p(obj) }
    }

    #[inline]
    fn fixnum_p(&self, obj: VALUE) -> bool {
        unsafe { impl_fixnum_p(obj) }
    }

    #[inline]
    fn static_sym_p(&self, obj: VALUE) -> bool {
        unsafe { impl_static_sym_p(obj) }
    }

    #[inline]
    fn flonum_p(&self, obj: VALUE) -> bool {
        unsafe { impl_flonum_p(obj) }
    }

    #[inline]
    fn immediate_p(&self, obj: VALUE) -> bool {
        unsafe { impl_immediate_p(obj) }
    }

    #[inline]
    fn rb_test(&self, obj: VALUE) -> bool {
        unsafe { impl_rb_test(obj) }
    }

    #[inline]
    unsafe fn type_p(&self, obj: VALUE, ty: ruby_value_type) -> bool {
        impl_type_p(obj, ty)
    }

    #[inline]
    unsafe fn dynamic_sym_p(&self, obj: VALUE) -> bool {
        impl_dynamic_sym_p(obj)
    }

    #[inline]
    unsafe fn symbol_p(&self, obj: VALUE) -> bool {
        impl_symbol_p(obj)
    }

    #[inline]
    unsafe fn float_type_p(&self, obj: VALUE) -> bool {
        impl_float_type_p(obj)
    }

    #[inline]
    unsafe fn rb_type(&self, obj: VALUE) -> crate::ruby_value_type {
        impl_rb_type(obj)
    }

    #[inline]
    unsafe fn integer_type_p(&self, obj: VALUE) -> bool {
        impl_integer_type_p(obj)
    }

    #[inline]
    unsafe fn rstring_interned_p(&self, obj: VALUE) -> bool {
        impl_rstring_interned_p(obj)
    }

    #[inline]
    fn thread_sleep(&self, duration: Duration) {
        let seconds = duration.as_secs() as _;
        let microseconds = duration.subsec_micros() as _;

        let time = crate::timeval {
            tv_sec: seconds,
            tv_usec: microseconds,
        };

        unsafe { impl_thread_sleep(time) }
    }

    #[inline]
    unsafe fn rtypeddata_p(&self, obj: VALUE) -> bool {
        impl_rtypeddata_p(obj)
    }

    #[inline]
    #[cfg(ruby_gte_3_3)]
    unsafe fn rtypeddata_embedded_p(&self, obj: VALUE) -> bool {
        impl_rtypeddata_embedded_p(obj)
    }

    #[inline]
    #[cfg(ruby_lt_3_3)]
    unsafe fn rtypeddata_embedded_p(&self, _obj: VALUE) -> bool {
        false
    }

    #[inline]
    unsafe fn rtypeddata_type(&self, obj: VALUE) -> *const crate::rb_data_type_t {
        impl_rtypeddata_type(obj)
    }

    #[inline]
    unsafe fn rtypeddata_get_data(&self, obj: VALUE) -> *mut c_void {
        impl_rtypeddata_get_data(obj)
    }
}
