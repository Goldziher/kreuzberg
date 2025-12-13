//! Implementation of Ruby macros.
//!
//! Since macros are rely on the C preprocessor, or defined as `inline` C
//! functions, they are not available when linking libruby. In order to use the
//! libruby macros from Rust, `rb-sys` implements them using the following
//! strategies:
//!
//! 1. For stable versions of Ruby, the macros are implemented as Rust functions
//! 2. For ruby-head, the macros are implemented as C functions that are linked
//!    into the crate.

#![allow(rustdoc::broken_intra_doc_links)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use crate::rb_data_type_t;
use crate::ruby_value_type;
use crate::stable_api::get_default as api;
use crate::StableApiDefinition;
use crate::VALUE;
use std::ffi::c_void;
use std::os::raw::{c_char, c_long};

/// Emulates Ruby's "if" statement.
///
/// - @param[in]  obj    An arbitrary ruby object.
/// - @retval     false  `obj` is either ::RUBY_Qfalse or ::RUBY_Qnil.
/// - @retval     true   Anything else.
///
/// ```
/// use rb_sys::special_consts::*;
///
/// assert!(!TEST(Qfalse));
/// assert!(!TEST(Qnil));
/// assert!(TEST(Qtrue));
/// ```
#[inline]
pub fn TEST<T: Into<VALUE>>(obj: T) -> bool {
    api().rb_test(obj.into())
}

/// Checks if the given object is nil.
///
/// - @param[in]  obj    An arbitrary ruby object.
/// - @retval     true   `obj` is ::RUBY_Qnil.
/// - @retval     false  Anything else.
///
/// ### Example
///
/// ```
/// use rb_sys::special_consts::*;
///
/// assert!(NIL_P(Qnil));
/// assert!(!NIL_P(Qtrue));
/// ```
#[inline]
pub fn NIL_P<T: Into<VALUE>>(obj: T) -> bool {
    api().nil_p(obj.into())
}

/// Checks if the given object is a so-called Fixnum.
///
/// - @param[in]  obj    An arbitrary ruby object.
/// - @retval     true   `obj` is a Fixnum.
/// - @retval     false  Anything else.
/// - @note       Fixnum was  a thing  in the  20th century, but  it is  rather an implementation detail today.
#[inline]
pub fn FIXNUM_P<T: Into<VALUE>>(obj: T) -> bool {
    api().fixnum_p(obj.into())
}

/// Checks if the given object is a static symbol.
///
/// - @param[in]  obj    An arbitrary ruby object.
/// - @retval     true   `obj` is a static symbol
/// - @retval     false  Anything else.
/// - @see        RB_DYNAMIC_SYM_P()
/// - @see        RB_SYMBOL_P()
/// - @note       These days  there are static  and dynamic symbols, just  like we once had Fixnum/Bignum back in the old days.
#[inline]
pub fn STATIC_SYM_P<T: Into<VALUE>>(obj: T) -> bool {
    api().static_sym_p(obj.into())
}

/// Get the backend storage of a Ruby array.
///
/// ### Safety
///
/// This function is unsafe because it dereferences a raw pointer and returns
/// raw pointers to Ruby memory. The caller must ensure that the pointer stays live
/// for the duration of usage the the underlying array (by either GC marking or
/// keeping the RArray on the stack).
///
/// - @param[in]  a  An object of ::RArray.
/// - @return     Its backend storage.
#[inline]
pub unsafe fn RARRAY_CONST_PTR<T: Into<VALUE>>(obj: T) -> *const VALUE {
    api().rarray_const_ptr(obj.into())
}

/// Get the length of a Ruby array.
///
/// ### Safety
///
/// This function is unsafe because it dereferences a raw pointer in order to
/// access internal Ruby memory.
///
/// - @param[in]  a  An object of ::RArray.
/// - @return     Its length.
#[inline]
pub unsafe fn RARRAY_LEN<T: Into<VALUE>>(obj: T) -> c_long {
    api().rarray_len(obj.into())
}

/// Get the length of a Ruby string.
///
/// ### Safety
///
/// This function is unsafe because it dereferences a raw pointer in order to
/// access internal Ruby memory.
///
/// - @param[in]  a  An object of ::RString.
/// - @return     Its length.
#[inline]
pub unsafe fn RSTRING_LEN<T: Into<VALUE>>(obj: T) -> c_long {
    api().rstring_len(obj.into())
}

/// Get the backend storage of a Ruby string.
///
/// ### Safety
///
/// This function is unsafe because it dereferences a raw pointer and returns
/// raw pointers to Ruby memory.
///
/// - @param[in]  a  An object of ::RString.
/// - @return     Its backend storage
#[inline]
pub unsafe fn RSTRING_PTR<T: Into<VALUE>>(obj: T) -> *const c_char {
    api().rstring_ptr(obj.into())
}

/// Checks if the given object is a so-called Flonum.
///
/// @param[in]  obj    An arbitrary ruby object.
/// @retval     true   `obj` is a Flonum.
/// @retval     false  Anything else.
/// @see        RB_FLOAT_TYPE_P()
/// @note       These days there are Flonums and non-Flonum floats, just like we
///             once had Fixnum/Bignum back in the old days.
#[inline]
pub fn FLONUM_P<T: Into<VALUE>>(#[allow(unused)] obj: T) -> bool {
    api().flonum_p(obj.into())
}

/// Checks if  the given  object is  an immediate  i.e. an  object which  has no
/// corresponding storage inside of the object space.
///
/// @param[in]  obj    An arbitrary ruby object.
/// @retval     true   `obj` is a Flonum.
/// @retval     false  Anything else.
/// @see        RB_FLOAT_TYPE_P()
/// @note       The concept of "immediate" is purely C specific.
#[inline]
pub fn IMMEDIATE_P<T: Into<VALUE>>(obj: T) -> bool {
    api().immediate_p(obj.into())
}

/// Checks if the given object is of enum ::ruby_special_consts.
///
/// @param[in]  obj    An arbitrary ruby object.
/// @retval     true   `obj` is a special constant.
/// @retval     false  Anything else.
///
/// ### Example
///
/// ```
/// use rb_sys::special_consts::*;
///
/// assert!(SPECIAL_CONST_P(Qnil));
/// assert!(SPECIAL_CONST_P(Qtrue));
/// assert!(SPECIAL_CONST_P(Qfalse));
/// ```
#[inline]
pub fn SPECIAL_CONST_P<T: Into<VALUE>>(obj: T) -> bool {
    api().special_const_p(obj.into())
}

/// Queries the type of the object.
///
/// @param[in]  obj  Object in question.
/// @pre        `obj` must not be a special constant.
/// @return     The type of `obj`.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// attemping to access the underlying [`RBasic`] struct.
#[inline]
pub unsafe fn RB_BUILTIN_TYPE(obj: VALUE) -> ruby_value_type {
    api().builtin_type(obj)
}

/// Queries if the object is an instance of ::rb_cInteger.
///
/// @param[in]  obj    Object in question.
/// @retval     true   It is.
/// @retval     false  It isn't.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// attemping to access the underlying [`RBasic`] struct.
#[inline]
pub unsafe fn RB_INTEGER_TYPE_P(obj: VALUE) -> bool {
    api().integer_type_p(obj)
}

/// Queries if the object is a dynamic symbol.
///
/// @param[in]  obj    Object in question.
/// @retval     true   It is.
/// @retval     false  It isn't.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// attemping to access the underlying [`RBasic`] struct.
#[inline]
pub unsafe fn RB_DYNAMIC_SYM_P(obj: VALUE) -> bool {
    api().dynamic_sym_p(obj)
}

/// Queries if the object is an instance of ::rb_cSymbol.
///
/// @param[in]  obj    Object in question.
/// @retval     true   It is.
/// @retval     false  It isn't.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// attemping to access the underlying [`RBasic`] struct.
#[inline]
pub unsafe fn RB_SYMBOL_P(obj: VALUE) -> bool {
    api().symbol_p(obj)
}

/// Identical to RB_BUILTIN_TYPE(), except it can also accept special constants.
///
/// @param[in]  obj  Object in question.
/// @return     The type of `obj`.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// attemping to access the underlying [`RBasic`] struct.
#[inline]
pub unsafe fn RB_TYPE(value: VALUE) -> ruby_value_type {
    api().rb_type(value)
}

/// Queries if the given object is of given type.
///
/// @param[in]  obj    An object.
/// @param[in]  t      A type.
/// @retval     true   `obj` is of type `t`.
/// @retval     false  Otherwise.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// attemping to access the underlying [`RBasic`] struct.
#[inline]
#[cfg(ruby_engine = "mri")] // truffleruby provides its own implementation
pub unsafe fn RB_TYPE_P(obj: VALUE, ty: ruby_value_type) -> bool {
    api().type_p(obj, ty)
}

/// Queries if the object is an instance of ::rb_cFloat.
///
/// @param[in]  obj    Object in question.
/// @retval     true   It is.
/// @retval     false  It isn't.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// attemping to access the underlying [`RBasic`] struct.
#[inline]
pub unsafe fn RB_FLOAT_TYPE_P(obj: VALUE) -> bool {
    api().float_type_p(obj)
}

/// Checks if the given object is an RTypedData.
///
/// @param[in]  obj    Object in question.
/// @retval     true   It is an RTypedData.
/// @retval     false  It isn't an RTypedData.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// accessing the underlying data structure.
#[inline]
pub unsafe fn RTYPEDDATA_P(obj: VALUE) -> bool {
    api().rtypeddata_p(obj)
}

/// Checks if the given RTypedData is embedded.
///
/// @param[in]  obj    An RTypedData object.
/// @retval     true   The data is embedded in the object itself.
/// @retval     false  The data is stored separately.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// accessing the underlying data structure. The caller must ensure the object
/// is a valid RTypedData.
#[inline]
pub unsafe fn RTYPEDDATA_EMBEDDED_P(obj: VALUE) -> bool {
    api().rtypeddata_embedded_p(obj)
}

/// Gets the data type information from an RTypedData object.
///
/// @param[in]  obj    An RTypedData object.
/// @return     Pointer to the rb_data_type_t structure for this object.
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer to get
/// access to the underlying data type. The caller must ensure the object
/// is a valid RTypedData.
#[inline]
pub unsafe fn RTYPEDDATA_TYPE(obj: VALUE) -> *const rb_data_type_t {
    api().rtypeddata_type(obj)
}

/// Gets the data pointer from an RTypedData object.
///
/// @param[in]  obj    An RTypedData object.
/// @return     Pointer to the wrapped C struct.
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer to get
/// access to the underlying data. The caller must ensure the object
/// is a valid RTypedData.
#[inline]
pub unsafe fn RTYPEDDATA_GET_DATA(obj: VALUE) -> *mut c_void {
    api().rtypeddata_get_data(obj)
}

/// Checks if the bignum is positive.
///
/// @param[in]  b      An object of RBignum.
/// @retval     false  `b` is less than zero.
/// @retval     true   Otherwise.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// accessing the underlying bignum structure.
#[inline]
pub unsafe fn RBIGNUM_POSITIVE_P(b: VALUE) -> bool {
    api().bignum_positive_p(b)
}

/// Checks if the bignum is negative.
///
/// @param[in]  b      An object of RBignum.
/// @retval     true   `b` is less than zero.
/// @retval     false  Otherwise.
///
/// # Safety
/// This function is unsafe because it could dereference a raw pointer when
/// accessing the underlying bignum structure.
#[inline]
pub unsafe fn RBIGNUM_NEGATIVE_P(b: VALUE) -> bool {
    api().bignum_negative_p(b)
}
