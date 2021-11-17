use serde_json::{Value, Map, Number};
use std::collections::HashMap;
use super::{RbAny, RbClass, RbHash, RbObject, RbRef, RbUserData, rc_get_ptr};

pub struct RbToJson {
    seen: HashMap<*const RbRef, usize>,
    next_id: usize,
}

impl RbToJson {
    pub fn new() -> Self {
        Self {
            seen: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn to_json(&mut self, value: &RbAny) -> Option<Value> {
        self.conv_any(value)
    }

    /// Returns a JSON value representing this Any, or None if the conversion failed.
    fn conv_any(&mut self, value: &RbAny) -> Option<Value> {
        let r = match value {
            RbAny::Int(v) => Value::from(*v),
            RbAny::True => Value::Bool(true),
            RbAny::False => Value::Bool(false),
            RbAny::Nil => Value::Null,
            RbAny::Symbol(sym) => Value::String(sym.as_str()?.to_owned()),
            RbAny::Ref(r) => {
                if r.contains_ref() {
                    let ptr = rc_get_ptr(r);
                    if let Some(obj_id) = self.seen.get(&ptr) {
                        Value::String(format!("@{}", obj_id))
                    } else {
                        self.seen.insert(ptr, self.next_id);
                        self.next_id += 1;
                        self.conv_ref(r)?
                    }
                } else {
                    self.conv_ref(r)?
                }
            }
        };
        Some(r)
    }

    fn conv_ref(&mut self, value: &RbRef) -> Option<Value> {
        let obj_id = self.next_id - 1;
        let r = match value {
            RbRef::Float(v) => Value::Number(Number::from_f64(v.0 as f64)?),
            RbRef::BigInt(v) => Value::String(v.to_string()),
            RbRef::Array(v) => {
                let mut map = Map::new();
                map.ezset("@", "Array");
                map.ezset("@id", obj_id);
                let mut ar = Vec::with_capacity(v.capacity());
                for it in v.iter() {
                    ar.push(self.conv_any(it)?);
                }
                map.ezset("data", Value::Array(ar));
                Value::Object(map)
            },
            RbRef::Str(v) => Value::String(v.clone()),
            RbRef::StrI { .. } => todo!(),
            // TODO use an object and include flags
            RbRef::Regex { content, flags } => {
                let mut map = Map::new();
                map.ezset("data", content.clone());
                map.ezset("flags", *flags);
                map.ezset("@", "RegEx");
                map.ezset("@id", obj_id);
                Value::Object(map)
            },
            RbRef::RegexI { content, flags, .. } => {
                let mut map = Map::new();
                map.ezset("data-b64", base64::encode(content));
                map.ezset("flags", *flags);
                map.ezset("@", "RegEx");
                map.ezset("@id", obj_id);
                Value::Object(map)
            },
            RbRef::Hash(hash) => self.conv_hash(hash)?,
            RbRef::Struct(v) => self.conv_object(v)?,
            RbRef::Object(v) => self.conv_object(v)?,
            RbRef::ClassRef(v) => Value::from(v.as_str()),
            RbRef::ModuleRef(v) => Value::from(v.as_str()),
            RbRef::ClassModuleRef(v) => Value::from(v.as_str()),
            RbRef::Data(v) => self.conv_class(v)?,
            RbRef::UserClass(v) => self.conv_class(v)?,
            RbRef::UserData(v) => self.conv_user_data(v)?,
            RbRef::UserMarshal(v) => self.conv_class(v)?,
            RbRef::Extended { module, object } => {
                let mut map = Map::new();
                map.ezset("object", self.conv_any(object)?);
                map.ezset("module", module.to_json()?);
                map.ezset("@", "@extended@");
                Value::Object(map)
            }
        };
        Some(r)
    }

    fn conv_class(&mut self, value: &RbClass) -> Option<Value> {
        let mut map = Map::new();
        map.ezset("@", value.name.as_str()?);
        map.ezset("data", self.conv_any(&value.data)?);
        Some(Value::Object(map))
    }

    /// Return a new JSON object representing this object.
    fn conv_object(&mut self, value: &RbObject) -> Option<Value> {
        let mut map = Map::new();
        map.ezset("@", value.name.as_str()?);
        map.ezset("@id", self.next_id - 1);
        let mut fields = Map::new();
        for it in value.fields.iter() {
            let key = it.0.as_str()?.to_owned();
            let val = self.conv_any(&it.1)?;
            fields.insert(key, val);
        }
        map.ezset("fields", fields);
        Some(Value::Object(map))
    }

    fn conv_hash(&mut self, value: &RbHash) -> Option<Value> {
        let mut map = Map::new();
        map.ezset("@", "Hash");
        map.ezset("@id", self.next_id - 1);

        let mut pairs = Vec::new();
        for it in value.map.iter() {
            pairs.push( Value::Array(vec![self.conv_any(it.0)?, self.conv_any(it.1)?]) );
        }
        map.ezset("data", pairs);
        if let Some(def) = &value.default {
            map.ezset("default", self.conv_any(def)?);
        }
        Some(Value::Object(map))
    }

    fn conv_user_data(&mut self, value: &RbUserData) -> Option<Value> {
        let mut map = Map::new();
        map.ezset("data", base64::encode(&value.data));
        map.ezset("name", value.name.to_json()?);
        map.ezset("@", "@userdata@");
        map.ezset("@id", self.next_id - 1);
        Some(Value::Object(map))
    }
}

pub trait JsonMapExt {
    fn ezset<K, V>(&mut self, key: K, value: V) where K: AsRef<str>, V: Into<Value>;
}
impl JsonMapExt for Map<String, Value> {
    fn ezset<K, V>(&mut self, key: K, value: V) where K: AsRef<str>, V: Into<Value> {
        self.insert(key.as_ref().to_owned(), value.into());
    }
}