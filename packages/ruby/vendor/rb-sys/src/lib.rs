#![allow(rustdoc::bare_urls)]
#![allow(unpredictable_function_pointer_comparisons)]
#![doc = include_str!("../readme.md")]

pub mod bindings;
#[cfg(feature = "stable-api")]
pub mod macros;
pub mod memory;
pub mod special_consts;
#[cfg(feature = "stable-api")]
pub mod stable_api;
pub mod symbol;
pub mod tracking_allocator;
pub mod value_type;

mod hidden;
mod ruby_abi_version;
mod utils;

pub use bindings::*;
#[cfg(feature = "stable-api")]
pub use macros::*;
pub use ruby_abi_version::*;
pub use special_consts::*;
#[cfg(feature = "stable-api")]
pub use stable_api::StableApiDefinition;
pub use value_type::*;

#[deprecated(since = "0.9.79", note = "Use `VALUE` instead")]
pub type Value = VALUE;
#[deprecated(since = "0.9.79", note = "Use `VALUE` instead")]
pub type RubyValue = VALUE;

#[cfg(use_global_allocator)]
set_global_tracking_allocator!();
