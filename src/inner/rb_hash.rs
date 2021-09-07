use super::RbAny;
use std::ops::{Deref, DerefMut};
use std::collections::BTreeMap;

#[cfg(feature = "json")]
use serde_json::{Value, Map};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct RbHash {
    pub map: BTreeMap<RbAny, RbAny>,
    pub default: Option<Box<RbAny>>,
}
impl RbHash {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            default: None,
        }
    }

    /// Construct a RbHash from an array of key-value pairs
    pub fn from_pairs(pairs: Vec<(RbAny, RbAny)>) -> Self {
        let mut map = BTreeMap::new();
        for item in pairs {
            map.insert(item.0, item.1);
        }
        Self {
            map,
            default: None
        }
    }

    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Option<Value> {
        use super::helper::json::JsonMapExt;
        let mut pairs = Vec::new();
        for it in self.map.iter() {
            pairs.push( Value::Array(vec![it.0.to_json()?, it.1.to_json()?]) );
        }
        let mut map = Map::new();
        map.ezset("@", "Hash");
        map.ezset("data", pairs);
        if let Some(def) = &self.default {
            map.ezset("default", def.to_json()?);
        }
        Some(Value::Object(map))
    }
}
impl Deref for RbHash {
    type Target = BTreeMap<RbAny, RbAny>;
    fn deref(&self) -> &Self::Target { &self.map }
}
impl DerefMut for RbHash {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.map }
}
