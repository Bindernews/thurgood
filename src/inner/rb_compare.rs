use std::{cmp::Ordering, collections::HashMap};
use super::{RbAny, RbHash, RbObject, RbRef, RbSymbol, RbFields, rc_get_ptr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RefPair(*const RbRef, *const RbRef);
impl RefPair {
    pub fn new(a: *const RbRef, b: *const RbRef) -> Self {
        if b < a {
            Self(b, a)
        } else {
            Self(a, b)
        }
    }
}

pub struct RbCompare {
    seen: HashMap<RefPair, Option<Ordering>>
}

impl RbCompare {
    pub fn new() -> Self {
        Self {
            seen: HashMap::new(),
        }
    }

    pub fn cmp(&mut self, lhs: &RbAny, rhs: &RbAny) -> Ordering {
        self.cmp_any(lhs, rhs).unwrap()
    }

    fn cmp_any(&mut self, lhs: &RbAny, rhs: &RbAny) -> Option<Ordering> {
        match (lhs, rhs) {
            (RbAny::Int(l0), RbAny::Int(r0)) => Some(l0.cmp(r0)),
            (RbAny::Symbol(l0), RbAny::Symbol(r0)) => Some(l0.cmp(r0)),
            (RbAny::Ref(l0), RbAny::Ref(r0)) => {
                let l_ptr = rc_get_ptr(l0);
                let r_ptr = rc_get_ptr(r0);
                let pair = RefPair::new(l_ptr, r_ptr);
                let current = self.seen.get(&pair);
                if let Some(cur) = current {
                    return *cur;
                } else {
                    self.seen.insert(pair, None);
                    let new_ord = self.cmp_ref(&l0, &r0);
                    if new_ord.is_some() {
                        self.seen.insert(pair, new_ord);
                        new_ord
                    } else {
                        let new_ord = Some(l_ptr.cmp(&r_ptr));
                        self.seen.insert(pair, new_ord);
                        new_ord
                    }
                }
            },
            _ => Self::any_id(lhs).cmp(&Self::any_id(rhs)).into(),
        }
    }

    fn cmp_ref(&mut self, lhs: &RbRef, rhs: &RbRef) -> Option<Ordering> {
        use super::RbRef as En;
        match (lhs, rhs) {
            (En::Array(l0), En::Array(r0)) =>
                self.cmp_array(l0, r0),
            (En::BigInt(l0), En::BigInt(r0)) =>
                l0.partial_cmp(r0),
            (En::ClassModuleRef(l0), En::ClassModuleRef(r0)) =>
                l0.partial_cmp(r0),
            (En::ClassRef(l0), En::ClassRef(r0)) =>
                l0.partial_cmp(r0),
            (En::Data(l0), En::Data(r0)) =>
                self.cmp_symbol_any(&l0.name, &l0.data, &r0.name, &r0.data),
            (En::Extended { module: l0mod, object: l0obj }, En::Extended { module: r0mod, object: r0obj}) =>
                self.cmp_symbol_any(l0mod, l0obj, r0mod, r0obj),
            (En::Float(a), En::Float(b)) =>
                a.partial_cmp(b),
            (En::Hash(l0), En::Hash(r0)) =>
                self.cmp_hash(l0, r0),
            (En::ModuleRef(l0), En::ModuleRef(r0)) =>
                l0.partial_cmp(r0),
            (En::Object(l0), En::Object(r0)) =>
                self.cmp_object(l0, r0),
            (En::Regex { content: l_con, flags: l_flags }, En::Regex { content: r_con, flags: r_flags }) =>
                self.cmp_regex(l_con.as_bytes(), *l_flags, None,  r_con.as_bytes(), *r_flags, None),
            (
                En::RegexI { content: l_con, flags: l_flags, metadata: l_meta },
                En::RegexI { content: r_con, flags: r_flags, metadata: r_meta }
            ) =>
                self.cmp_regex(l_con, *l_flags, Some(l_meta), r_con, *r_flags, Some(r_meta)),
            (En::Str(l0), En::Str(r0)) =>
                l0.partial_cmp(r0),
            (
                En::StrI { content: l_con, metadata: l_meta },
                En::StrI { content: r_con, metadata: r_meta }
            ) => {
                let c0 = l_con.cmp(r_con);
                if c0.is_eq() { self.cmp_fields(l_meta, r_meta) } else { Some(c0) }
            },
            (En::Struct(l0), En::Struct(r0)) =>
                self.cmp_object(l0, r0),
            (En::UserClass(l0), En::UserClass(r0)) =>
                self.cmp_symbol_any(&l0.name, &l0.data, &r0.name, &r0.data),
            (En::UserData(l0), En::UserData(r0)) => {
                let c0 = l0.name.cmp(&r0.name);
                if c0.is_eq() { l0.data.partial_cmp(&r0.data) } else { Some(c0) }
            },
            (En::UserMarshal(l0), En::UserMarshal(r0)) =>
                self.cmp_symbol_any(&l0.name, &l0.data, &r0.name, &r0.data),
            _ => Some(lhs.ordinal().cmp(&rhs.ordinal())),
        }
    }

    fn cmp_symbol_any(&mut self, l_sym: &RbSymbol, l_any: &RbAny, r_sym: &RbSymbol, r_any: &RbAny) -> Option<Ordering> {
        let c0 = l_sym.cmp(r_sym);
        if c0.is_eq() {
            self.cmp_any(l_any, r_any)
        } else {
            Some(c0)
        }
    }

    fn cmp_array(&mut self, l0: &Vec<RbAny>, r0: &Vec<RbAny>) -> Option<Ordering> {
        let c0 = l0.len().cmp(&r0.len());
        if c0.is_ne() {
            return Some(c0);
        }
        for (a, b) in l0.iter().zip(r0.iter()) {
            let c0 = self.cmp_any(a, b).unwrap_or(Ordering::Equal);
            if c0.is_ne() { return Some(c0); }
        }
        Some(Ordering::Equal)
    }

    fn cmp_hash(&mut self, l0: &RbHash, r0: &RbHash) -> Option<Ordering> {
        for ((lkey, lval), (rkey, rval)) in l0.iter().zip(r0.iter()) {
            let c0 = self.cmp_any(lkey, rkey).unwrap_or(Ordering::Equal);
            if c0.is_ne() { return Some(c0); }
            let c1 = self.cmp_any(lval, rval).unwrap_or(Ordering::Equal);
            if c1.is_ne() { return Some(c1); }
        }
        return Some(Ordering::Equal);
    }

    fn cmp_object(&mut self, l0: &RbObject, r0: &RbObject) -> Option<Ordering> {
        let c0 = l0.name.cmp(&r0.name);
        if c0.is_ne() { return Some(c0); }
        self.cmp_fields(&l0.fields, &r0.fields)
    }

    fn cmp_regex(&mut self, l_con: &[u8], l_flags: u32, l_meta: Option<&RbFields>, 
                r_con: &[u8], r_flags: u32, r_meta: Option<&RbFields>) -> Option<Ordering> {
        let c0 = l_con.cmp(r_con);
        if c0.is_eq() {
            let c1 = l_flags.cmp(&r_flags);
            if c1.is_eq() && l_meta.is_some() && r_meta.is_some() {
                self.cmp_fields(l_meta.unwrap(), r_meta.unwrap())
            } else {
                Some(c1)
            }
        } else {
            Some(c0)
        }
    }

    fn cmp_fields(&mut self, l_meta: &RbFields, r_meta: &RbFields) -> Option<Ordering> {
        let c0 = l_meta.len().cmp(&r_meta.len());
        if c0.is_ne() { return Some(c0); }
        for i in 0..l_meta.len() {
            let lh_o = l_meta.get_index(i);
            let rh_o = r_meta.get_index(i);
            let lh = if let Some(v) = lh_o { v } else { return Some(Ordering::Less); };
            let rh = if let Some(v) = rh_o { v } else { return Some(Ordering::Greater); };
            let c0 = lh.0.cmp(rh.0);
            if c0.is_ne() { return Some(c0); }
            let c1 = self.cmp_any(&lh.1, &rh.1).unwrap_or(Ordering::Equal);
            if c1.is_ne() { return Some(c1); }
        }
        return Some(Ordering::Equal);
    }

    fn any_id(a: &RbAny) -> i32 {
        match a {
            RbAny::Nil => 0,
            RbAny::False => 1,
            RbAny::True => 2,
            RbAny::Int(_) => 3,
            RbAny::Symbol(_) => 4,
            RbAny::Ref(_) => 5,
        }
    }
}
