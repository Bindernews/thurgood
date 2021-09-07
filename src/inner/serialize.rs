use std::io;
use std::collections::BTreeMap;
use crate::consts::*;
use crate::error::{TResult};
use super::{RbAny, RbRef, RbSymbol, RbObject, RFloat32, RcType};
use num_traits::sign::Signed;

#[derive(Clone)]
pub struct RbWriter<W> {
    dst: W,
    symbol_map: BTreeMap<RbSymbol, usize>,
    symbol_next: usize,
    object_map: BTreeMap<RcType<RbRef>, usize>,
    object_next: usize,
    sym_e: RbSymbol,
}

impl<W> RbWriter<W> where
    W: io::Write
{
    pub fn new(dst: W) -> Self {
        Self {
            dst,
            symbol_map: BTreeMap::new(),
            symbol_next: 0,
            object_map: BTreeMap::new(),
            object_next: 0,
            sym_e: RbSymbol::from("E"),
        }
    }

    pub fn write(&mut self, data: &RbAny) -> TResult<usize> {
        let header = [4u8, 8u8];
        self.dst.write(&header)?;
        Ok(self.write_entry(data)? + 2)
    }

    fn write_entry(&mut self, entry: &RbAny) -> TResult<usize> {
        match entry {
            RbAny::Int(v) => Ok(self.write_byte(T_INT)? + self.write_int(*v)?),
            RbAny::True => self.write_byte(T_TRUE),
            RbAny::False => self.write_byte(T_FALSE),
            RbAny::Nil => self.write_byte(T_NIL),
            RbAny::Symbol(v) => self.write_symbol(v),
            RbAny::Ref(v) => self.write_ref(v),
        }
    }

    fn write_ref(&mut self, entry: &RcType<RbRef>) -> TResult<usize> {
        if let Some(obj_index) = self.object_map.get(entry) {
            let obj_index = *obj_index;
            Ok(self.write_byte(T_OBJECT_REF)? + self.write_int(obj_index as i32)?)
        } else {
            self.object_map.insert(entry.clone(), self.object_next);
            self.object_next += 1;
            match entry.as_ref() {
                RbRef::Float(v) => {
                    let mut sz = 0;
                    sz += self.write_byte(T_FLOAT)?;
                    sz += self.write_float(v)?;
                    Ok(sz)
                },

                // Write a BigInt
                RbRef::BigInt(v) => {
                    let mut sz = 0;
                    let (_, bytes) = v.to_bytes_le();
                    let b2 = [T_BIGNUM, if v.is_negative() { '-' } else { '+' } as u8];
                    self.dst.write_all(&b2)?;
                    sz += b2.len();
                    sz += self.write_int((bytes.len() / 2) as i32)?;
                    self.dst.write_all(&bytes)?;
                    sz += bytes.len();
                    Ok(sz)
                },

                // Write an array
                RbRef::Array(v) => {
                    let mut sz = 0;
                    sz += self.write_byte(T_ARRAY)?;
                    sz += self.write_int(v.len() as i32)?;
                    for item in v.iter() {
                        sz += self.write_entry(item)?;
                    }
                    Ok(sz)
                },

                // Write a string. Actually we're writing a String Instance
                RbRef::Str(v) => {
                    let prefix = [T_INSTANCE, T_STRING];
                    let mut sz = 0;
                    self.dst.write_all(&prefix)?;
                    sz += prefix.len();
                    sz += self.write_len_bytes(v.as_bytes())?;
                    // One field, key is :E, value is True
                    sz += self.write_et()?;
                    Ok(sz)
                },

                // Write an instance string with unknown encoding
                RbRef::StrI { content, metadata } => {
                    let prefix = [T_INSTANCE, T_STRING];
                    let mut sz = 0;
                    self.dst.write_all(&prefix)?;
                    sz += prefix.len();
                    sz += self.write_len_bytes(&content)?;
                    sz += self.write_pairs(metadata)?;
                    Ok(sz)
                },

                // Write an instance regex with default encoding
                RbRef::Regex { content, flags } => {
                    let prefix = [T_INSTANCE, T_REGEX];
                    let mut sz = 0;
                    self.dst.write_all(&prefix)?;
                    sz += prefix.len();
                    sz += self.write_len_bytes(content.as_bytes())?;
                    // Write regex flags
                    sz += self.write_byte(*flags as u8)?;
                    // One field, key is :E, value is True
                    sz += self.write_et()?;
                    Ok(sz)
                },

                // Write an instance regex with unknown encoding or extra metadata
                RbRef::RegexI { content, flags, metadata } => {
                    let prefix = [T_INSTANCE, T_REGEX];
                    let mut sz = 0;
                    self.dst.write_all(&prefix)?;
                    sz += prefix.len();
                    sz += self.write_len_bytes(content.as_slice())?;
                    // Write regex flags
                    sz += self.write_byte(*flags as u8)?;
                    sz += self.write_pairs(metadata)?;
                    Ok(sz)
                },

                // Write a hash
                RbRef::Hash(v) => {
                    let mut sz = 0;
                    // Write type byte
                    sz += if v.default.is_some() {
                        self.write_byte(T_HASH_DEFAULT)?
                    } else {
                        self.write_byte(T_HASH)?
                    };
                    // Write entries
                    sz += self.write_int(v.len() as i32)?;
                    for (key, val) in v.iter() {
                        sz += self.write_entry(key)?;
                        sz += self.write_entry(val)?;
                    }
                    // Optionally write default value
                    if let Some(ref def) = v.default {
                        sz += self.write_entry(def)?;
                    }
                    Ok(sz)
                },

                RbRef::Object(v) => {
                    let mut sz = 0;
                    sz += self.write_byte(T_OBJECT)?;
                    sz += self.write_object(v)?;
                    Ok(sz)
                },

                RbRef::Struct(v) => {
                    let mut sz = 0;
                    sz += self.write_byte(T_STRUCT)?;
                    sz += self.write_object(v)?;
                    Ok(sz)
                },

                RbRef::ClassRef( v ) => {
                    let mut sz = 0;
                    sz += self.write_byte(T_CLASS)?;
                    sz += self.write_len_bytes(v.as_bytes())?;
                    Ok(sz)
                },

                RbRef::ModuleRef( v ) => {
                    let mut sz = 0;
                    sz += self.write_byte(T_MODULE)?;
                    sz += self.write_len_bytes(v.as_bytes())?;
                    Ok(sz)
                },

                RbRef::ClassModuleRef( v) => {
                    let mut sz = 0;
                    sz += self.write_byte(T_CLASS_MODULE)?;
                    sz += self.write_len_bytes(v.as_bytes())?;
                    Ok(sz)
                },

                RbRef::Data( v) => {
                    self.write_typed_data(&v.name, &v.data, T_DATA)
                },

                RbRef::UserClass( v ) => {
                    self.write_typed_data(&v.name, &v.data, T_USER_CLASS)
                },
                RbRef::UserData { name, data } => {
                    let mut sz = 0;
                    sz += self.write_byte(T_USER_DEFINED)?;
                    sz += self.write_symbol(name)?;
                    sz += self.write_len_bytes(&data)?;
                    Ok(sz)
                },
                RbRef::UserMarshal( v ) => {
                    self.write_typed_data(&v.name, &v.data, T_USER_MARSHAL)
                },
                RbRef::Extended { module, object } => {
                    self.write_typed_data(module, object, T_EXTENDED)
                },
            }
        }
    }

    fn write_symbol(&mut self, sym: &RbSymbol) -> TResult<usize> {
        if let Some(sym_index) = self.symbol_map.get(sym) {
            // If we already have this symbol, just write a reference
            let sym_index = *sym_index;
            Ok(self.write_byte(T_SYMBOL_REF)? + self.write_int(sym_index as i32)?)
        } else {
            // Otherwise write a new symbol and add it to the symbol map
            self.symbol_map.insert(sym.clone(), self.symbol_next);
            self.symbol_next += 1;
            // Write to the stream
            let mut sz = 0;
            sz += self.write_byte(T_SYMBOL)?;
            sz += self.write_len_bytes(sym.as_bytes())?;
            Ok(sz)
        }
    }

    fn write_et(&mut self) -> TResult<usize> {

        let mut sz = 0;
        sz += self.write_byte(0x06)?;
        sz += self.write_symbol(&(self.sym_e.clone()))?;
        sz += self.write_byte(T_TRUE)?;
        Ok(sz)
    }

    fn write_int(&mut self, v: i32) -> TResult<usize> {
        let mut buf = [0u8; 5];

        fn count_bytes(b: &[u8]) -> usize {
            for i in 1..b.len() {
                if b[i] == 0 { return i; }
            }
            return b.len();
        }

        match v {
            0 => self.write_byte(0),
            1 ..= 122 => self.write_byte((v as u8) + 5),
            -123 ..= -1 => self.write_byte((v - 5) as u8),
            _ => {
                buf[1..].copy_from_slice(&v.to_le_bytes());
                // Count how many bytes we need
                let sz = count_bytes(&buf) as i32;
                if v > 0 {
                    buf[0] = (sz - 1) as u8;
                } else {
                    buf[0] = (-sz + 1) as u8;
                }
                self.dst.write_all(&buf[0..sz as usize])?;
                Ok(sz as usize)
            },
        }
    }

    fn write_float(&mut self, v: &RFloat32) -> TResult<usize> {
        if v.0.is_infinite() {
            if v.0.is_sign_negative() {
                self.write_len_bytes("-inf".as_bytes())
            } else {
                self.write_len_bytes("inf".as_bytes())
            }
        } else if v.0.is_nan() {
            self.write_len_bytes("nan".as_bytes())
        } else {
            self.write_len_bytes(v.0.to_string().as_bytes())
        }
    }

    /// Write a varint (n) denoting the number of *pairs* and then (n * 2) objects:
    /// the key, value pairs. Returns the number of bytes written.
    fn write_pairs(&mut self, pairs: &Vec<(RbAny, RbAny)>) -> TResult<usize> {
        let mut sz = 0;
        sz += self.write_int(pairs.len() as i32)?;
        for (key, val) in pairs.iter() {
            sz += self.write_entry(key)?;
            sz += self.write_entry(val)?;
        }
        Ok(sz)
    }

    fn write_object(&mut self, obj: &RbObject) -> TResult<usize> {
        let mut sz = 0;
        sz += self.write_symbol(&obj.name)?;
        sz += self.write_int(obj.fields.len() as i32)?;
        for (key, val) in obj.fields.iter() {
            sz += self.write_symbol(key)?;
            sz += self.write_entry(val)?;
        }
        Ok(sz)
    }

    fn write_typed_data(&mut self, name: &RbSymbol, data: &RbAny, type_byte: u8) -> TResult<usize> {
        let mut sz = 0;
        sz += self.write_byte(type_byte)?;
        sz += self.write_symbol(name)?;
        sz += self.write_entry(data)?;
        Ok(sz)
    }

    /// Writes the number of bytes in `data` as a variable-length integer then writes `data`.
    /// Returns the total size of bytes written.
    fn write_len_bytes(&mut self, data: &[u8]) -> TResult<usize> {
        let sz = self.write_int(data.len() as i32)?;
        self.dst.write_all(data)?;
        Ok(data.len() + sz)
    }

    fn write_byte(&mut self, b: u8) -> TResult<usize> {
        let buf = [b];
        self.dst.write_all(&buf)?;
        Ok(1)
    }
}

/// Serialize an `RbAny` to an IO stream.
/// 
pub fn to_writer<W: io::Write>(dst: W, value: &RbAny) -> TResult<usize> {
    let mut wr = RbWriter::new(dst);
    wr.write(value)
}
