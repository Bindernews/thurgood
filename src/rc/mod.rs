pub use std::rc::Rc as RcType;
pub fn rc_get_ptr<T>(reff: &RcType<T>) -> *const T {
    RcType::as_ptr(reff)
}
#[path="../inner/mod.rs"]
mod inner;
pub use inner::*;