use super::RbAny;
use std::{cmp::Ordering, ops::{Deref, DerefMut}};
use indexmap::IndexMap;

#[derive(Clone, Eq, Debug)]
pub struct RbHash {
    pub map: IndexMap<RbAny, RbAny>,
    pub default: Option<Box<RbAny>>,
}
impl RbHash {
    pub fn new() -> Self {
        Self {
            map: IndexMap::new(),
            default: None,
        }
    }

    /// Construct a RbHash from an array of key-value pairs
    pub fn from_pairs(pairs: Vec<(RbAny, RbAny)>) -> Self {
        let mut map = IndexMap::new();
        for item in pairs {
            map.insert(item.0, item.1);
        }
        Self {
            map,
            default: None
        }
    }
}

impl PartialEq for RbHash {
    fn eq(&self, other: &Self) -> bool {
        if self.map.len() != other.map.len() {
            return false;
        }
        for (k,v) in self.map.iter() {
            if other.map.get(k) != Some(v) {
                return false;
            }
        }
        return true;
    }
}
impl Ord for RbHash {
    fn cmp(&self, other: &Self) -> Ordering {
        let c0 = self.map.len().cmp(&other.map.len());
        if c0.is_ne() { return c0; }
        for i in 0..self.map.len() {
            let lh = self.map.get_index(i).unwrap();
            let rh = other.map.get_index(i).unwrap();
            let c0 = lh.0.cmp(rh.0);
            if c0.is_ne() { return c0; }
            let c1 = lh.1.cmp(rh.1);
            if c1.is_ne() { return c1; }
        }
        return Ordering::Equal;
    }
}
impl PartialOrd for RbHash {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Deref for RbHash {
    type Target = IndexMap<RbAny, RbAny>;
    fn deref(&self) -> &Self::Target { &self.map }
}
impl DerefMut for RbHash {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.map }
}
