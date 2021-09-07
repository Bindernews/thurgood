use std::cmp::{Eq, PartialEq};
use std::fmt;
use super::{RbAny, RcType};
#[cfg(feature = "json")]
use serde_json::{Value, Map};

/// A Ruby Symbol (e.g. :key, :value).
/// 
/// Symbols are very common, and often re-used. Thus multiple `RbSymbol`s may share their
/// data internally. Calling `clone()` is cheap.
/// 
/// Most `Symbol`s will be a UTF-8 string, however the Ruby specification places no definite
/// bounds, meaning that 
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RbSymbol {
    /// Raw data representing the symbol name. Specifically does NOT have to have an encoding
    data: RcType<Vec<u8>>,
}
impl RbSymbol {
    /// Construct an RbSymbol from raw data
    pub fn new(data: Vec<u8>) -> RbSymbol {
        Self {
            data: RcType::new(data),
        }
    }

    /// Get the raw bytes of the symbol.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Attempt to get the symbol as a UTF-8 string.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }

    /// Return a clone of this, converted to an `RbAny`.
    pub fn as_any(&self) -> RbAny {
        RbAny::Symbol(self.clone())
    }

    /// Construct an RbSymbol from a string.
    pub fn from_str<S: AsRef<str>>(v: S) -> Self {
        Self { data: RcType::new(Vec::from(v.as_ref().as_bytes())) }
    }

    /// Construct a JSON value from this object.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Option<Value> {
        Some(Value::String(self.as_str()?.to_owned()))
    }
}
impl Default for RbSymbol {
    fn default() -> Self {
        Self { data: RcType::new(Vec::new()) }
    }
}

impl From<&str> for RbSymbol {
    fn from(v: &str) -> Self { Self::from_str(v) }
}
impl From<String> for RbSymbol {
    fn from(v: String) -> Self { Self::from_str(&v) }
}
impl Into<RbSymbol> for &RbSymbol {
    fn into(self) -> RbSymbol { self.clone() }
}

impl fmt::Debug for RbSymbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if let Some(s) = self.as_str() {
            write!(f, "RbSymbol(\"{}\")", s)
        } else {
            write!(f, "RbSymbol({:?})", self.data)
        }
    }
}

/// An ordered list of key-value pairs.
pub type RbFields = Vec<(RbAny, RbAny)>;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct RbClass {
    pub name: RbSymbol,
    pub data: RbAny,
}
impl RbClass {

    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Option<Value> {
        use super::helper::json::JsonMapExt;
        let mut map = Map::new();
        map.ezset("@", self.name.as_str()?);
        map.ezset("data", self.data.to_json()?);
        Some(Value::Object(map))
    }
}
