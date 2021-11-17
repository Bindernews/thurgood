use std::cmp::{Eq, PartialEq, Ordering};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

/// A wrapper around f32 that allow it to be stored in a BTreeMap. This breaks some
/// of the rules of f32 because they only implement PartialOrd and PartialEq.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct RbFloat(pub f64);
impl Hash for RbFloat {
    fn hash<H: Hasher>(&self, h: &mut H) { h.write_u64(self.0.to_bits()); }
}
impl Eq for RbFloat {}
impl Ord for RbFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}
impl Deref for RbFloat {
    type Target = f64;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl DerefMut for RbFloat {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
