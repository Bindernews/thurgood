use num_bigint::BigInt;
use super::{RbFloat, RbAny, RbSymbol, RbFields, RbClass, RbObject, RbHash, RbUserData};
use crate::RbType;

macro_rules! match_opt {
    ($var:ident { $the_match:pat => $the_result:expr }) => {
        match $var { $the_match => Some($the_result), _ => None }
    };
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RbRef {
    Float(RbFloat),
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
    UserData(RbUserData),
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
            RbRef::UserData(_) => RbType::UserData,
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
                | RbRef::ModuleRef( _ ) | RbRef::ClassModuleRef( _ ) | RbRef::UserData(_)
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


    /// Returns a number representing the relative "order" of different `RbRef` types.
    /// 
    /// Note that the order isn't relative to anything important. This is simply to
    /// make disparate data types "comparable" even when they're not.
    pub fn ordinal(&self) -> usize {
        match self {
            Self::Array(_) => 0,
            Self::BigInt(_) => 1,
            Self::ClassModuleRef(_) => 2,
            Self::ClassRef(_) => 3,
            Self::Data(_) => 4,
            Self::Extended { .. } => 5,
            Self::Float(_) => 6,
            Self::Hash(_) => 7,
            Self::ModuleRef(_) => 8,
            Self::Object(_) => 9,
            Self::Regex { .. } => 10,
            Self::RegexI { .. } => 11,
            Self::Str(_) => 12,
            Self::StrI { .. } => 13,
            Self::Struct(_) => 14,
            Self::UserClass(_) => 15,
            Self::UserData { .. } => 16,
            Self::UserMarshal(_) => 17,
        }
    }

    /// Test for equality ONLY with variants that aren't containers.
    /// 
    /// This will return a value for BigInt, ClassModuleRef, ClassRef, Float, ModuleRef, Regex, Str, UserData.
    /// If types are incompatible it will return false, and if the types are not one of those listed above
    /// this function will return None.
    pub fn partial_eq(&self, other: &Self) -> Option<bool> {
        match (self, other) {
            (RbRef::BigInt(l0), RbRef::BigInt(r0)) => Some(l0 == r0),
            (
                RbRef::ClassModuleRef(l0),
                RbRef::ClassModuleRef(r0)|RbRef::ClassRef(r0)|RbRef::ModuleRef(r0)
            ) => Some(l0 == r0),
            (RbRef::ClassRef(l0)|RbRef::ModuleRef(l0), RbRef::ClassModuleRef(r0)) => Some(l0 == r0),
            (RbRef::Float(l0), RbRef::Float(r0)) => Some(l0 == r0),
            (
                RbRef::Regex { content: l_con, flags: l_flags },
                RbRef::Regex { content: r_con, flags: r_flags },
            ) => Some(l_con == r_con && l_flags == r_flags),
            (RbRef::Str(l0), RbRef::Str(r0)) => Some(l0 == r0),
            (RbRef::UserData(l0), RbRef::UserData(r0)) => Some(l0 == r0),
            _ => {
                if std::mem::discriminant(self) != std::mem::discriminant(other) {
                    Some(false)
                } else {
                    None
                }
            },
        }
    }

    /// Returns true if this type may contain a (potentially recursive) reference.
    pub fn contains_ref(&self) -> bool {
        match self {
            Self::BigInt(_)|Self::ClassModuleRef(_)|Self::ClassRef(_)|Self::ModuleRef(_)|
                Self::Float(_)|Self::Regex {..}|Self::Str(_)|Self::UserData(_) => false,
            _ => true
        }
    }
}

impl RbRef {
    pub fn as_float(&self) -> Option<&RbFloat> {
        match_opt!(self { RbRef::Float(ref v) => v })
    }
    pub fn as_float_mut(&mut self) -> Option<&mut RbFloat> {
        match_opt!(self { RbRef::Float(ref mut v) => v })
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

impl From<f32> for RbRef { fn from(v: f32) -> Self { Self::from(v as f64) } }
impl From<f64> for RbRef { fn from(v: f64) -> Self { RbRef::Float(RbFloat(v)) } }
impl From<RbHash> for RbRef { fn from(v: RbHash) -> Self { RbRef::Hash(v) } }
impl From<RbObject> for RbRef { fn from(v: RbObject) -> Self { RbRef::Object(v) } }
