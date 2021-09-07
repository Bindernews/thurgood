use num_bigint::BigInt;
use super::{RFloat32, RbAny, RbSymbol, RbFields, RbClass, RbObject, RbHash};
use crate::RbType;
use base64;

#[cfg(feature = "json")]
use serde_json::{Value, Map, Number};

macro_rules! match_opt {
    ($var:ident { $the_match:pat => $the_result:expr }) => {
        match $var { $the_match => Some($the_result), _ => None }
    };
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RbRef {
    Float(RFloat32),
    BigInt(BigInt),
    /// Array of RbAny
    Array(Vec<RbAny>),
    /// utf-8 or ascii encoded string
    Str(String),
    /// String with some alternate encoding or additional fields
    StrI { content: Vec<u8>, metadata: RbFields },
    /// utf-8 encoded regex
    Regex { content: String, flags: u32 },
    /// Regex with some alternate encoding
    RegexI { content: Vec<u8>, flags: u32, metadata: RbFields },
    /// A ruby hashmap
    Hash(RbHash),
    /// A Struct. Identical to an object except for the type ID
    Struct(RbObject),
    /// An Object. Identical to a struct except for the type ID
    Object(RbObject),
    ClassRef(String),
    ModuleRef(String),
    ClassModuleRef(String),
    /// A Data object
    Data(RbClass),
    /// Subclass of String, Regexp, Array, or Hash
    UserClass(RbClass),
    /// User-defined data, as stored/loaded using `_dump` and `_load` methods
    UserData { name: RbSymbol, data: Vec<u8> },
    /// Class-based user-defined serialization
    UserMarshal(RbClass),
    /// Extended object
    Extended { module: RbSymbol, object: RbAny },
}
impl RbRef {
    pub fn get_type(&self) -> RbType {
        match self {
            RbRef::Float(_) => RbType::Float,
            RbRef::BigInt(_) => RbType::BigInt,
            RbRef::Array(_) => RbType::Array,
            RbRef::Str(_) | RbRef::StrI { .. } => RbType::Str,
            RbRef::Regex { .. } | RbRef::RegexI { .. } => RbType::Regex,
            RbRef::Hash(_) => RbType::Hash,
            RbRef::Struct(_) => RbType::Struct,
            RbRef::Object(_) => RbType::Object,
            RbRef::ClassRef(_) => RbType::ClassRef,
            RbRef::ModuleRef(_) => RbType::ModuleRef,
            RbRef::ClassModuleRef(_) => RbType::ClassModuleRef,
            RbRef::Data(_) => RbType::Data,
            RbRef::UserClass(_) => RbType::UserClass,
            RbRef::UserData { .. } => RbType::UserData,
            RbRef::UserMarshal(_) => RbType::UserMarshal,
            RbRef::Extended { .. } => RbType::Extended,
        }
    }

    /// Convenience method to get the a child of this object. For Arrays, `key` MUST be
    /// an `RbAny::Int`, for `Hash` key can be anything, and for all other objects key MUST
    /// be `RbAny::Symbol`. If the key isn't found or types are invalid, returns None.
    pub fn get_child(&self, key: &RbAny) -> Option<&RbAny> {
        match &self {
            RbRef::Float(_) | RbRef::BigInt(_) | RbRef::Str(_) | RbRef::StrI { .. }
                | RbRef::Regex { .. } | RbRef::RegexI { .. } | RbRef::ClassRef( _ )
                | RbRef::ModuleRef( _ ) | RbRef::ClassModuleRef( _ ) | RbRef::UserData { .. }
                => None,
            RbRef::Data(v) | RbRef::UserClass(v) | RbRef::UserMarshal(v) => {
                v.data.as_rbref().and_then(|c| c.get_child(key))
            },
            RbRef::Extended { object, .. } => {
                object.as_rbref().and_then(|c| c.get_child(key))
            },
            RbRef::Array(v) => {
                key.as_int().and_then(|k| v.get(k as usize))
            },
            RbRef::Hash(v) => v.get(key),
            RbRef::Struct(v) => v.get(key.as_symbol()?),
            RbRef::Object(v) => v.get(key.as_symbol()?),
        }
    }

    pub fn new_regex(content: String, flags: u32) -> RbRef {
        Self::Regex { content, flags }
    }

    pub fn new_object<N: Into<RbSymbol>>(name: N, pairs: &[(RbSymbol, RbAny)]) -> Self {
        Self::Object(RbObject::new_from_slice(name.into(), pairs))
    }

    pub fn into_any(self) -> RbAny {
        RbAny::from(self)
    }


    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Option<Value> {
        use super::helper::json::JsonMapExt;

        let r = match &self {
            Self::Float(v) => Value::Number(Number::from_f64(v.0 as f64)?),
            Self::BigInt(v) => Value::String(v.to_string()),
            Self::Array(v) => {
                let mut ar = Vec::with_capacity(v.capacity());
                for it in v.iter() {
                    ar.push(it.to_json()?);
                }
                Value::Array(ar)
            },
            Self::Str(v) => Value::String(v.clone()),
            Self::StrI { .. } => todo!(),
            // TODO use an object and include flags
            Self::Regex { content, flags } => {
                let mut map = Map::new();
                map.ezset("data", content.clone());
                map.ezset("flags", *flags);
                map.ezset("@", "RegEx");
                Value::Object(map)
            },
            Self::RegexI { content, flags, .. } => {
                let mut map = Map::new();
                map.ezset("data-b64", base64::encode(content));
                map.ezset("flags", *flags);
                map.ezset("@", "RegEx");
                Value::Object(map)
            },
            Self::Hash(hash) => hash.to_json()?,
            Self::Struct(v) => v.to_json()?,
            Self::Object(v) => v.to_json()?,
            Self::ClassRef(v) => Value::from(v.as_str()),
            Self::ModuleRef(v) => Value::from(v.as_str()),
            Self::ClassModuleRef(v) => Value::from(v.as_str()),
            Self::Data(v) => v.to_json()?,
            Self::UserClass(v) => v.to_json()?,
            Self::UserData { name, data } => {
                let mut map = Map::new();
                map.ezset("data", base64::encode(data));
                map.ezset("name", name.to_json()?);
                map.ezset("@", "@userdata@");
                Value::Object(map)
            },
            Self::UserMarshal(v) => v.to_json()?,
            Self::Extended { module, object } => {
                let mut map = Map::new();
                map.ezset("object", object.to_json()?);
                map.ezset("module", module.to_json()?);
                map.ezset("@", "@extended@");
                Value::Object(map)
            }
        };
        Some(r)
    }
}

impl RbRef {
    pub fn as_float(&self) -> Option<&f32> {
        match_opt!(self { RbRef::Float(ref v) => &v.0 })
    }
    pub fn as_float_mut(&mut self) -> Option<&mut f32> {
        match_opt!(self { RbRef::Float(ref mut v) => &mut v.0 })
    }
    pub fn as_array(&self) -> Option<&Vec<RbAny>> {
        match_opt!(self { RbRef::Array(ref v) => v })
    }
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<RbAny>> {
        match_opt!(self { RbRef::Array(ref mut v) => v })
    }
    pub fn as_hash(&self) -> Option<&RbHash> {
        match_opt!(self { RbRef::Hash(ref v) => v })
    }
    pub fn as_hash_mut(&mut self) -> Option<&mut RbHash> {
        match_opt!(self { RbRef::Hash(ref mut v) => v })
    }
    pub fn as_object(&self) -> Option<&RbObject> {
        match_opt!(self { RbRef::Object(ref v) => v })
    }
    pub fn as_object_mut(&mut self) -> Option<&mut RbObject> {
        match_opt!(self { RbRef::Object(ref mut v) => v })
    }
    pub fn as_struct(&self) -> Option<&RbObject> {
        match_opt!(self { RbRef::Struct(ref v) => v })
    }
    pub fn as_struct_mut(&mut self) -> Option<&mut RbObject> {
        match_opt!(self { RbRef::Struct(ref mut v) => v })
    }
    pub fn as_string(&self) -> Option<&String> {
        match_opt!(self { RbRef::Str(ref v) => v })
    }
    pub fn as_string_mut(&mut self) -> Option<&mut String> {
        match_opt!(self { RbRef::Str(ref mut v) => v })
    }
}

impl From<f32> for RbRef { fn from(v: f32) -> Self { RbRef::Float(RFloat32(v)) } }
impl From<RbHash> for RbRef { fn from(v: RbHash) -> Self { RbRef::Hash(v) } }
impl From<RbObject> for RbRef { fn from(v: RbObject) -> Self { RbRef::Object(v) } }
