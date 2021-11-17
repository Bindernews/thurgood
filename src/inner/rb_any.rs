use std::{cmp::Ordering, fmt, hash::{Hash, Hasher}};
use super::{RbHash, RbObject, RbRef, RbSymbol, RcType, rb_compare::RbCompare, rc_get_ptr};
use crate::RbType;
use std::fmt::Formatter;

macro_rules! match_opt {
    ($var:ident { $the_match:pat => $the_result:expr }) => {
        match $var { $the_match => Some($the_result), _ => None }
    };
}

/// Represents any valid Ruby value.
/// 
#[derive(Clone, Eq, PartialOrd, Ord)]
pub enum RbAny {
    Int(i32),
    True,
    False,
    Nil,
    Symbol(RbSymbol),
    Ref(RcType<RbRef>),
}
impl RbAny {
    /// Construct a new `RbSymbol` from the given string and return it wrapped in an `RbAny`.
    pub fn symbol_from(name: &str) -> RbAny {
        let bytes = Vec::from(name.as_bytes());
        RbAny::Symbol(RbSymbol::new(bytes))
    }

    /// Returns the generic type of the Ruby object.
    pub fn get_type(&self) -> RbType {
        match self {
            RbAny::Int(_) => RbType::Int,
            RbAny::True | RbAny::False => RbType::Bool,
            RbAny::Nil => RbType::Nil,
            RbAny::Symbol(_) => RbType::Symbol,
            RbAny::Ref(o) => o.get_type(),
        }
    }

    /// Returns true if this RbAny is Nil.
    pub fn is_nil(&self) -> bool {
        match self { Self::Nil => true, _ => false }
    }

    /// If `Any` is an int, returns the value, otherwise returns None.
    pub fn as_int(&self) -> Option<i32> {
        match_opt!(self { RbAny::Int(v) => *v })
    }

    /// If `Any` is a boolean, returns the value, otherwise returns None.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            RbAny::True => Some(true),
            RbAny::False => Some(false),
            _ => None,
        }
    }

    /// If `Any` is a Symbol, returns the symbol, otherwise returns None.
    pub fn as_symbol(&self) -> Option<&RbSymbol> {
        match self { Self::Symbol(r) => Some(r), _ => None }
    }

    /// If `Any` is an object reference, returns a reference to it `RbRef`, otherwise returns None.
    pub fn as_rbref(&self) -> Option<&RbRef> {
        match self { RbAny::Ref(ref r) => Some(r), _ => None }
    }

    /// If `Any` is an object reference, returns a mutable reference to it, otherwise returns None.
    pub fn as_rbref_mut(&mut self) -> Option<&mut RbRef> {
        match self { RbAny::Ref(ref mut r) => RcType::get_mut(r), _ => None }
    }

    pub fn as_rc(&self) -> Option<&RcType<RbRef>> {
        match self { RbAny::Ref(r) => Some(r), _ => None }
    }
    pub fn as_rc_mut(&mut self) -> Option<&mut RcType<RbRef>> {
        match self { RbAny::Ref(r) => Some(r), _ => None }
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

    pub fn deep_cmp(&self, other: &Self) -> Ordering {
        RbCompare::new().cmp(self, other)
    }

    pub fn deep_eq(&self, other: &Self) -> bool {
        self.deep_cmp(other).is_eq()
    }

    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Option<serde_json::Value> {
        super::rb_json::RbToJson::new().to_json(self)
    }
}

impl PartialEq for RbAny {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Symbol(l0), Self::Symbol(r0)) => l0 == r0,
            (Self::Ref(l0), Self::Ref(r0)) => {
                if let Some(result) = l0.partial_eq(r0) {
                    result
                } else {
                    rc_get_ptr(l0) == rc_get_ptr(r0)
                }
            },
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
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
            Self::Ref(v) => {
                if v.contains_ref() {
                    write!(f, "{:?}", rc_get_ptr(v))
                } else {
                    v.fmt(f)
                }
            },                
        }
    }
}

impl Default for RbAny {
    fn default() -> Self {
        Self::Nil
    }
}

impl Hash for RbAny {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Ref(r) => {
                state.write_usize(rc_get_ptr(r) as usize);
            },
            _ => {
                core::mem::discriminant(self).hash(state);
            },
        }
    }
}

impl From<i32> for RbAny { fn from(v: i32) -> Self { RbAny::Int(v) } }
impl From<f32> for RbAny { fn from(v: f32) -> Self { Self::from(RbRef::from(v)) } }
impl From<f64> for RbAny { fn from(v: f64) -> Self { Self::from(RbRef::from(v)) } }
impl From<bool> for RbAny { fn from(v: bool) -> Self { if v { RbAny::True } else { RbAny::False } } }
impl From<String> for RbAny { fn from(v: String) -> Self { Self::from(RbRef::Str(v)) } }
impl From<&str> for RbAny { fn from(v: &str) -> Self { Self::from(RbRef::Str(v.to_owned())) } }
impl From<RbRef> for RbAny { fn from(v: RbRef) -> Self { RbAny::Ref(RcType::new(v)) } }
impl From<Vec<RbAny>> for RbAny { fn from(v: Vec<RbAny>) -> Self { Self::from(RbRef::Array(v)) } }
impl From<RbHash> for RbAny { fn from(v: RbHash) -> Self { Self::from(RbRef::Hash(v)) } }
impl From<RbSymbol> for RbAny { fn from(v: RbSymbol) -> Self { RbAny::Symbol(v) } }
impl From<&RbSymbol> for RbAny { fn from(v: &RbSymbol) -> Self { RbAny::Symbol(v.clone()) } }
