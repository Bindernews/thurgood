use std::cmp::{Eq, PartialEq, Ordering};
use indexmap::IndexMap;
use super::{RbAny, RbSymbol, RbRef};
use crate::{
    ThurgoodError as Error,
    RbType,
};

#[cfg(feature = "json")]
use serde_json::{Value, Map};

/// A Ruby Object (or Struct) that has a type name and a set of fields, this is a serialized
/// instance of a class.
/// 
/// This is one of the most common data types, and as such it should be fairly ergonomic to use.
/// Fields are serialized in the order they are added to the object to ensure proper round-tripping.
/// 
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RbObject {
    /// Type name of the Object
    pub name: RbSymbol,
    /// Map of object fields
    pub fields: IndexMap<RbSymbol, RbAny>,
}
impl RbObject {
    /// Construct a new `RbObject` with no fields.
    pub fn new(name: &RbSymbol) -> Self {
        Self { name: name.clone(), fields: IndexMap::new() }
    }

    /// Construct a new Object with the given name and fields.
    pub fn new_from_slice<N, K>(name: N, items: &[(K, RbAny)]) -> Self
    where
        N: Into<RbSymbol>,
        K: Into<RbSymbol> + Clone,
    {
        let mut keys = Vec::new();
        let mut vals = Vec::new();
        for it in items {
            keys.push(it.0.clone().into());
            vals.push(it.1.clone());
        }
        let name = name.into();
        let mut fields = IndexMap::new();
        keys.drain(..).zip(vals.drain(..)).for_each(|it| {
            fields.insert(it.0, it.1);
        });
        Self { name, fields }
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any type which can be converted into the key's type.
    pub fn get<Q: Into<RbSymbol>>(&self, key: Q) -> Option<&RbAny> {
        let key = key.into();
        self.fields.get(&key)
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any type which can be converted into the key's type.
    pub fn get_mut<Q: Into<RbSymbol>>(&mut self, key: Q) -> Option<&mut RbAny> {
        self.fields.get_mut(&key.into())
    }

    /// Returns a mutable reference to the value corresponding to the key, and inserts
    /// a default value if that key doesn't yet exist.
    ///
    /// The key may be any type which can be converted into the key's type.
    pub fn get_mut_insert<Q: Into<RbSymbol>>(&mut self, key: Q) -> Option<&mut RbAny> {
        let key: RbSymbol = key.into();
        if !self.fields.contains_key(&key) {
            self.fields.insert(key.clone(), RbAny::default());
        }
        self.fields.get_mut(&key)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old value is returned.
    pub fn insert<Q: Into<RbSymbol>>(&mut self, key: Q, value: RbAny) -> Option<RbAny> {
        self.fields.insert(key.into(), value)
    }

    /// Assume each pair is an `(RbSymbol, RbAny)` and add each pair to the list of fields.
    /// If one of the keys is not an `RbSymbol` return an error, otherwise return `Ok(())`.
    pub fn extend_from_pairs(&mut self, pairs: &[(RbAny, RbAny)]) -> Result<(), Error> {
        for it in pairs {
            let key = it.0.as_symbol()
                .ok_or_else(|| Error::unexpected_type(RbType::Symbol, it.0.get_type()))?;
            self.insert(key.clone(), it.1.clone());
        }
        Ok(())
    }


    /// Convert this into an `RbRef::Object`.
    pub fn into_object(self) -> RbRef {
        RbRef::Object(self)
    }

    /// Convert this into an `RbRef::Struct`.
    pub fn into_struct(self) -> RbRef {
        RbRef::Struct(self)
    }

    /// Return a new JSON object representing this object.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Option<Value> {
        use super::helper::json::JsonMapExt;
        let mut map = Map::new();
        map.ezset("@", self.name.as_str()?);
        let mut fields: Vec<Option<(String, Value)>> = self.fields.iter()
            .map(|it| {
                let key = it.0.as_str()?.to_owned();
                let val = it.1.to_json()?;
                Some( (key, val) )
            }).collect();
        for it in fields.drain(..) {
            let pair = it?;
            map.insert(pair.0, pair.1);
        }
        Some(Value::Object(map))
    }
}

impl PartialOrd<Self> for RbObject {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let c = self.name.cmp(&other.name);
        if !c.is_eq() { return Some(c); }
        let c = self.fields.len().cmp(&other.fields.len());
        if !c.is_eq() { return Some(c); }
        for it in self.fields.iter().zip(other.fields.iter()) {
            let c = it.0.0.cmp(&it.1.0);
            if !c.is_eq() { return Some(c); }
            let c = it.0.1.cmp(&it.1.1);
            if !c.is_eq() { return Some(c); }
        }
        return Some(Ordering::Equal);
    }
}
impl Ord for RbObject {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

