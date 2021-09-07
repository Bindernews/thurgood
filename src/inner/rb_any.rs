use std::fmt;
use super::{RbSymbol, RbRef, RbHash, RbObject, RcType};
use crate::RbType;
use std::fmt::Formatter;

#[cfg(feature = "json")]
use serde_json::Value;

macro_rules! match_opt {
    ($var:ident { $the_match:pat => $the_result:expr }) => {
        match $var { $the_match => Some($the_result), _ => None }
    };
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RbAny {
    Int(i32),
    True,
    False,
    Nil,
    Symbol(RbSymbol),
    Ref(RcType<RbRef>),
}
impl RbAny {
    pub fn symbol_from(name: &str) -> RbAny {
        let bytes = Vec::from(name.as_bytes());
        RbAny::Symbol(RbSymbol::new(bytes))
    }

    pub fn get_bool(&self) -> Option<bool> {
        if self == &RbAny::True { return Some(true); }
        if self == &RbAny::False { return Some(false); }
        return None;
    }

    pub fn get_type(&self) -> RbType {
        match self {
            RbAny::Int(_) => RbType::Int,
            RbAny::True | RbAny::False => RbType::Bool,
            RbAny::Nil => RbType::Nil,
            RbAny::Symbol(_) => RbType::Symbol,
            RbAny::Ref(o) => o.get_type(),
        }
    }

    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Option<Value> {
        let r = match &self {
            Self::Int(v) => Value::from(*v),
            Self::True => Value::Bool(true),
            Self::False => Value::Bool(false),
            Self::Nil => Value::Null,
            Self::Symbol(sym) => Value::String(sym.as_str()?.to_owned()),
            Self::Ref(r) => r.to_json()?,
        };
        Some(r)
    }
}

impl RbAny {
    pub fn is_nil(&self) -> bool {
        match self { Self::Nil => true, _ => false }
    }

    pub fn as_int(&self) -> Option<i32> {
        match_opt!(self { RbAny::Int(v) => *v })
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            RbAny::True => Some(true),
            RbAny::False => Some(false),
            _ => None,
        }
    }

    pub fn as_symbol(&self) -> Option<&RbSymbol> {
        match self { Self::Symbol(r) => Some(r), _ => None }
    }

    pub fn as_rbref(&self) -> Option<&RbRef> {
        match self { RbAny::Ref(ref r) => Some(r), _ => None }
    }
    pub fn as_rbref_mut(&mut self) -> Option<&mut RbRef> {
        match self { RbAny::Ref(ref mut r) => RcType::get_mut(r), _ => None }
    }

    pub fn as_array(&self) -> Option<&Vec<RbAny>> {
        self.as_rbref().and_then(|v| v.as_array())
    }
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<RbAny>> {
        self.as_rbref_mut().and_then(|v| v.as_array_mut())
    }
    pub fn as_hash(&self) -> Option<&RbHash> {
        self.as_rbref().and_then(|v| v.as_hash())
    }
    pub fn as_hash_mut(&mut self) -> Option<&mut RbHash> {
        self.as_rbref_mut().and_then(|v| v.as_hash_mut())
    }
    pub fn as_object(&self) -> Option<&RbObject> {
        self.as_rbref().and_then(|v| v.as_object())
    }
    pub fn as_object_mut(&mut self) -> Option<&mut RbObject> {
        self.as_rbref_mut().and_then(|v| v.as_object_mut())
    }
    pub fn as_string(&self) -> Option<&String> {
        self.as_rbref().and_then(|v| v.as_string())
    }

    pub fn find_child<'a, I>(root: &'a RbAny, path: I) -> Option<&'a RbAny>
        where I: IntoIterator<Item=&'a RbAny>
    {
        let mut current = root;
        for key in path {
            current = current.as_rbref()?.get_child(key)?;
        }
        Some(current)
    }
}

impl fmt::Debug for RbAny {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(v) => write!(f, "Int({})", v),
            Self::True => write!(f, "True"),
            Self::False => write!(f, "False"),
            Self::Nil => write!(f, "Nil"),
            Self::Symbol(v) => write!(f, "{:?}", v),
            Self::Ref(v) => v.fmt(f)
        }
    }
}

impl Default for RbAny {
    fn default() -> Self {
        Self::Nil
    }
}

impl From<i32> for RbAny { fn from(v: i32) -> Self { RbAny::Int(v) } }
impl From<f32> for RbAny { fn from(v: f32) -> Self { Self::from(RbRef::from(v)) } }
impl From<bool> for RbAny { fn from(v: bool) -> Self { if v { RbAny::True } else { RbAny::False } } }
impl From<String> for RbAny { fn from(v: String) -> Self { Self::from(RbRef::Str(v)) } }
impl From<&str> for RbAny { fn from(v: &str) -> Self { Self::from(RbRef::Str(v.to_owned())) } }
impl From<RbRef> for RbAny { fn from(v: RbRef) -> Self { RbAny::Ref(RcType::new(v)) } }
impl From<Vec<RbAny>> for RbAny { fn from(v: Vec<RbAny>) -> Self { Self::from(RbRef::Array(v)) } }
impl From<RbHash> for RbAny { fn from(v: RbHash) -> Self { Self::from(RbRef::Hash(v)) } }
impl From<RbSymbol> for RbAny { fn from(v: RbSymbol) -> Self { RbAny::Symbol(v) } }
impl From<&RbSymbol> for RbAny { fn from(v: &RbSymbol) -> Self { RbAny::Symbol(v.clone()) } }
