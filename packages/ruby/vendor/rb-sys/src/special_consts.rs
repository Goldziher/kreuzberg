#![allow(rustdoc::broken_intra_doc_links)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

//! Definitions for Ruby's special constants.
//!
//! Makes it easier to reference important Ruby constants, without having to dig
//! around in bindgen's output.

use std::ffi::c_long;

use crate::{VALUE, ruby_special_consts};

pub const Qfalse: ruby_special_consts = ruby_special_consts::RUBY_Qfalse;
pub const Qtrue: ruby_special_consts = ruby_special_consts::RUBY_Qtrue;
pub const Qnil: ruby_special_consts = ruby_special_consts::RUBY_Qnil;
pub const Qundef: ruby_special_consts = ruby_special_consts::RUBY_Qundef;
pub const IMMEDIATE_MASK: ruby_special_consts = ruby_special_consts::RUBY_IMMEDIATE_MASK;
pub const FIXNUM_FLAG: ruby_special_consts = ruby_special_consts::RUBY_FIXNUM_FLAG;
pub const FIXNUM_MIN: c_long = c_long::MIN / 2;
pub const FIXNUM_MAX: c_long = c_long::MAX / 2;
pub const FLONUM_MASK: ruby_special_consts = ruby_special_consts::RUBY_FLONUM_MASK;
pub const FLONUM_FLAG: ruby_special_consts = ruby_special_consts::RUBY_FLONUM_FLAG;
pub const SYMBOL_FLAG: ruby_special_consts = ruby_special_consts::RUBY_SYMBOL_FLAG;

#[allow(clippy::from_over_into)]
impl Into<VALUE> for ruby_special_consts {
    fn into(self) -> VALUE {
        self as VALUE
    }
}
