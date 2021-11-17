mod rb_any;
mod rb_compare;
mod rb_ref;
mod rb_hash;
mod rb_float;
mod rb_misc;
mod rb_object;
mod helper;
mod deserialize;
mod serialize;
pub mod dump;

#[cfg(feature = "json")]
mod rb_json;

// This is so we can safely define the ref type in the parent module
pub use super::{RcType, rc_get_ptr};

pub use rb_any::RbAny;
pub use rb_float::RbFloat;
pub use rb_hash::RbHash;
pub use rb_misc::{RbClass, RbFields, RbSymbol, RbUserData};
pub use rb_ref::RbRef;
pub use rb_object::RbObject;
pub use serialize::to_writer;
pub use deserialize::from_reader;

// Re-export error type for convenience
pub use crate::error::ThurgoodError as Error;
