use std::cmp::{Eq, PartialEq, Ordering};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

/// A wrapper around f32 that allow it to be stored in a BTreeMap. This breaks some
/// of the rules of f32 because they only implement PartialOrd and PartialEq.
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct RFloat32(pub f32);
impl Hash for RFloat32 {
    fn hash<H: Hasher>(&self, h: &mut H) { h.write_u32(self.0.to_bits()); }
}
impl Eq for RFloat32 {}
impl Ord for RFloat32 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}
impl Deref for RFloat32 {
    type Target = f32;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl DerefMut for RFloat32 {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
