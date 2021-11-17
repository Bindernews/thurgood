
/*
This module implements a way to serialize/deserialize Ruby's Marshal format. Unfortunately a complete implementation is impossible
given that Ruby classes can deserialize themselves, so without a full Ruby VM AND the original source we can't deserialize everything.
That being said this library supports callbacks when such information is encountered, so that the caller may handle things appropriately.
The primary goal is an implementation which doesn't lose any information and can easily be used in other projects.

References:
- https://docs.ruby-lang.org/en/2.1.0/marshal_rdoc.html
- https://ilyabylich.svbtle.com/ruby-marshalling-from-a-to-z
- Calling `Marshal.dump` on various things in Ruby
*/

use std::io;
use std::convert::TryInto;
use num_bigint::{BigInt, Sign};
use crate::{
    consts::*,
    error::*,
    RbType,
};
use super::{RbAny, RbClass, RbFields, RbHash, RbObject, RbRef, RbSymbol, RbUserData, rc_get_ptr};

fn bytes_to_string(buf: &[u8]) -> TResult<String> {
    Ok(std::str::from_utf8(buf)?.to_owned())
}


#[derive(Clone)]
pub struct RbReader<R> {
    src: R,
    symbols: Vec<RbSymbol>,
    objects: Vec<RbAny>,
    sym_e: RbSymbol,
}

impl<R> RbReader<R> where
    R: io::Read
{
    pub fn new(src: R) -> Self {
        Self {
            src,
            symbols: Vec::new(),
            // Documentation says that object indexes start at 1, actually the root object is
            // at index 0, and since objects can't recursively reference, this works out.
            objects: vec![],
            // Cached copy of this symbol so we can easily test for string encodings
            sym_e: RbSymbol::from_str("E"),
        }
    }

    pub fn read(&mut self) -> TResult<RbAny> {
        let mut buf2 = [0u8;2];
        self.src.read_exact(&mut buf2)?;
        if !(buf2[0] == 4 && buf2[1] == 8) {
            return Err(ThurgoodError::Version(format!("{}.{}", buf2[0], buf2[1])));
        }
        self.read_entry()
    }

    fn read_entry(&mut self) -> TResult<RbAny> {
        let c = self.read_byte()?;
        match c {
            T_TRUE => { Ok(RbAny::True) },
            T_FALSE => { Ok(RbAny::False) },
            T_NIL => { Ok(RbAny::Nil) },
            T_INT => { Ok(RbAny::Int(self.read_int()?)) },
            T_SYMBOL => { self.read_symbol() },
            T_SYMBOL_REF => { self.read_symbol_ref() },
            T_OBJECT_REF => { self.read_object_ref() },
            _ => self.read_ref(c),
        }
    }

    fn read_ref(&mut self, type_byte: u8) -> TResult<RbAny> {
        if type_byte == T_EXTENDED {
            let module = self.read_entry_symbol()?;
            let object = self.read_entry()?;
            Ok(RbRef::Extended { module, object }.into_any())
        } else {
            let o_index = self.alloc_object();
            let obj = match type_byte {
                T_INSTANCE => {
                    self.read_instance()
                },
                T_ARRAY => {
                    self.read_array()
                },
                T_BIGNUM => {
                    self.read_bignum()
                },
                T_CLASS => {
                    Ok(RbRef::ClassRef(self.read_class_mod_ref()?))
                },
                T_MODULE => {
                    Ok(RbRef::ModuleRef(self.read_class_mod_ref()?))
                },
                T_CLASS_MODULE => {
                    Ok(RbRef::ClassModuleRef(self.read_class_mod_ref()?))
                },
                T_DATA => {
                    Ok(RbRef::Data(self.read_rb_class()?))
                },
                T_FLOAT => {
                    Ok(RbRef::from(self.read_float()?))
                },
                T_HASH => { self.read_hash(false) },
                T_HASH_DEFAULT => { self.read_hash(true) },
                T_REGEX => { self.read_regex() },
                T_STRING => {
                    self.read_string()
                },
                T_OBJECT => {
                    Ok(RbRef::Object(self.read_rb_object()?))
                },
                T_STRUCT => {
                    Ok(RbRef::Struct(self.read_rb_object()?))
                },
                T_USER_CLASS => {
                    self.read_user_class()
                },
                T_USER_DEFINED => {
                    let name = self.read_entry_symbol()?;
                    let data = self.read_len_bytes()?;
                    Ok(RbRef::UserData(RbUserData { name, data }))
                },
                T_USER_MARSHAL => {
                    Ok(RbRef::UserMarshal(self.read_rb_class()?))
                },
                _ => { Err(ThurgoodError::BadTypeByte(type_byte)) }
            }?;
            Ok(self.set_object(o_index, obj))
        }
    }

    /// Allocate space for an object in the object list.
    /// The object will start out as Nil and MUST be replaced later.
    fn alloc_object(&mut self) -> usize {
        let n = self.objects.len();
        self.objects.push(RbAny::Nil);
        // println!("Alloc: {}", n);
        n
    }

    /// Replace the object at the given index with an RbRef and return the newly-created RbAny.
    fn set_object(&mut self, index: usize, obj: RbRef) -> RbAny {
        // If there an no extra references, set it the easy way
        if self.objects[index].is_nil() {
            self.objects[index] = RbAny::from(obj);
        } else {
            // Bypass mutability rules here. This is safe because no other code has access to
            // any of the Rc/Arc/etc. created here until the read() function returns. Until that
            // point this RbReader is the only "real" owner, no matter how many references there
            // are to this object.
            unsafe {
                let raw_ptr = rc_get_ptr(self.objects[index].as_rc().unwrap());
                *(raw_ptr as *mut RbRef) = obj;
            }
        }
        self.objects[index].clone()
    }

    /// Read and return variable-sized integer from the data stream.
    /// This does NOT parse a type byte as there are many varints used in the encoding.
    fn read_int(&mut self) -> TResult<i32> {
        let mut buf = [0u8;4];
        self.src.read_exact(&mut buf[0..1])?;
        let is_neg = buf[0] >= 128;
        // Special cases for 0 or multi-byte values
        if buf[0] <= 0x04 || buf[0] >= 0xfc {
            let bytes_to_read = (if is_neg { -(buf[0] as i8) } else { buf[0] as i8 }) as usize;
            // If it's 0x00 then we just return 0
            if bytes_to_read == 0 {
                return Ok(0);
            }
            // Read the correct number of bytes. The rest will still be 0 so it's fine to convert using little-endian
            self.src.read_exact(&mut buf[0..bytes_to_read])?;
            let u_val = u32::from_le_bytes(buf[0..4].try_into()
                .expect("Something is VERY wrong, maybe a hardware error.")) as i32;
            
            // Return the resulting value
            if is_neg {
                Ok(-u_val)
            } else {
                Ok(u_val)
            }
        // General case of single-byte value
        } else {
            let b0 = buf[0] as i8;
            if is_neg {
                Ok((b0 as i32) + 5)
            } else {
                Ok((b0 as i32) - 5)
            }
        }
    }

    /// Parse a new symbol (no type byte)
    fn read_symbol(&mut self) -> TResult<RbAny> {
        let symbol_len = self.read_int()? as usize;
        let mut buf = vec![0; symbol_len];
        self.src.read_exact(&mut buf)?;
        self.symbols.push(RbSymbol::new(buf));
        Ok(RbAny::Symbol(self.symbols[self.symbols.len() - 1].clone()))
    }

    /// Parse a symbol reference (no type byte)
    fn read_symbol_ref(&mut self) -> TResult<RbAny> {
        let symbol_index = self.read_int()? as usize;
        if symbol_index < self.symbols.len() {
            Ok(RbAny::Symbol(self.symbols[symbol_index].clone()))
        } else {
            Err(ThurgoodError::BadSymbolRef(symbol_index))
        }
    }

    /// Read the next entry (including type byte) and assert that it's a symbol.
    /// Returns a reference to the symbol instead of an RbAny.
    fn read_entry_symbol(&mut self) -> TResult<RbSymbol> {
        let r = self.read_entry()?;
        if let RbAny::Symbol(s) = r { 
            Ok(s)
        } else {
            Err(ThurgoodError::UnexpectedType { expected: RbType::Symbol, found: r.get_type() })
        }
    }

    fn read_object_ref(&mut self) -> TResult<RbAny> {
        let index = self.read_int()? as usize;
        if index < self.objects.len() {
            // println!("Object # {}", index);
            let base = &mut self.objects[index];
            // If the base is nil, we need to make it an Rc and use unsafe hackery later to set the value
            if base.is_nil() {
                *base = RbRef::from(1.0f32).into_any();
            }
            Ok(base.clone())
        } else {
            Err(ThurgoodError::BadObjectRef(index))
        }
    }

    /// Parse and return an object/string/regex with extra fields.
    /// It's important to note that instanced strings and regexes basically get added
    /// to the object array TWICE.
    fn read_instance(&mut self) -> TResult<RbRef> {
        let type_byte = self.read_byte()?;
        match type_byte {
            T_OBJECT => {
                let mut obj = self.read_rb_object()?;
                let num_pairs = self.read_int()? as usize;
                let pairs = self.read_pairs(num_pairs)?;
                // Append fields to object
                obj.extend_from_pairs(&pairs)?;
                Ok(RbRef::Object(obj))
            },
            T_STRING => {
                // Read the string data
                let data = self.read_len_bytes()?;
                // Gather extra pairs of data so we can confirm the string type
                let num_fields = self.read_int()? as usize;
                let pairs = self.read_pairs(num_fields)?;
                let obj = if self.is_utf8(&pairs) {
                    RbRef::Str(bytes_to_string(&data)?)
                } else {
                    RbRef::StrI { content: data, metadata: pairs }
                };
                Ok(obj)
            },
            T_REGEX => {
                // Read the regex data
                let data = self.read_len_bytes()?;
                let flags = self.read_int()? as u32;
                // Parse the remaining fields
                let num_fields = self.read_int()? as usize;
                let pairs = self.read_pairs(num_fields)?;
                let obj = if self.is_utf8(&pairs) {
                    RbRef::Regex { content: bytes_to_string(&data)?, flags }
                } else {
                    RbRef::RegexI { content: data, flags, metadata: pairs }
                };
                Ok(obj)
            },
            _ => {
                Err(ThurgoodError::BadInstanceType(type_byte as char))
            }
        }
    }

    fn is_utf8(&self, pairs: &RbFields) -> bool {
        for it in pairs.iter() {
            if it.0 == &self.sym_e && it.1 == &RbAny::True {
                return true
            }
        }
        return false;
    }

    /// Read a string (no specified encoding) from the data stream
    fn read_string(&mut self) -> TResult<RbRef> {
        let data = self.read_len_bytes()?;
        Ok(RbRef::Str(bytes_to_string(&data)?))
    }

    /// Read `count` key-value pairs from the stream, storing them in and returning an RbHash.
    /// The keys may be anything.
    fn read_pairs(&mut self, count: usize) -> TResult<RbFields> {
        let mut result = RbFields::new();
        for _ in 0..count {
            let key = self.read_entry()?;
            let key_sym = key.as_symbol()
                .ok_or_else(|| ThurgoodError::unexpected_type(RbType::Symbol, key.get_type()))?;
            let val = self.read_entry()?;
            result.insert(key_sym.clone(), val);
        }
        return Ok(result);
    }

    /// Read, track, and return an array of values (no type byte)
    fn read_array(&mut self) -> TResult<RbRef> {
        // Read the data for real
        let array_size = self.read_int()?;
        let mut data = Vec::new();
        for _ in 0..array_size {
            data.push(self.read_entry()?);
        }
        Ok(RbRef::Array(data))
    }

    fn read_bignum(&mut self) -> TResult<RbRef> {
        let c_sign = self.read_byte()? as char;
        let data_len = self.read_int()? as usize * 2;
        let mut buf = vec![0u8; data_len];
        self.src.read_exact(&mut buf)?;
        let v_sign = if c_sign == '+' { Sign::Plus } else { Sign::Minus };
        // return the object
        Ok(RbRef::BigInt(BigInt::from_bytes_le(v_sign, &buf)))
    }

    /// Read and return an RbClass instance (!! NOT RbAny)
    /// This is a helper function for the many things that are formatted the same.
    fn read_rb_class(&mut self) -> TResult<RbClass> {
        let name = self.read_entry_symbol()?;
        let data = self.read_entry()?;
        Ok(RbClass { name, data })
    }

    fn read_class_mod_ref(&mut self) -> TResult<String> {
        let buf = self.read_len_bytes()?;
        Ok(bytes_to_string(&buf)?)
    }

    fn read_float(&mut self) -> TResult<f64> {
        let buf = self.read_len_bytes()?;
        // Apparently this CAN be a C string, so we need to check for a NULL terminator.
        // Default to the buffer length.
        let last = buf.iter().position(|e| *e == 0).unwrap_or(buf.len());
        let decoded = std::str::from_utf8(&buf[0..last])?;
        match decoded {
            "inf" => Ok(f64::INFINITY),
            "-inf" => Ok(f64::NEG_INFINITY),
            "nan" => Ok(f64::NAN),
            _ => Ok(decoded.parse::<f64>()?),
        }
    }

    /// Read a hash from the stream (no type byte). If `has_default` is true then read
    /// an additional default value from the stream.
    fn read_hash(&mut self, has_default: bool) -> TResult<RbRef> {
        // Read the hash
        let num_pairs = self.read_int()? as usize;
        let mut nhash = RbHash::new();
        for _ in 0..num_pairs {
            let key = self.read_entry()?;
            let val = self.read_entry()?;
            nhash.insert(key, val);
        }
        if has_default {
            nhash.default = Some(Box::new(self.read_entry()?));
        }
        // Insert the real object
        Ok(RbRef::Hash(nhash))
    }

    /// Read a regex assuming UTF-8 / ASCII encoding.
    fn read_regex(&mut self) -> TResult<RbRef> {
        let content = self.read_len_bytes()?;
        let flags = self.read_byte()? as u32;
        // Track and return object
        Ok(RbRef::Regex { content: bytes_to_string(&content)?, flags })
    }

    /// Read a variable-sized integer, then read that number of bytes and return it as a Vec<u8>
    fn read_len_bytes(&mut self) -> TResult<Vec<u8>> {
        let str_len = self.read_int()? as usize;
        let mut buf = vec![0u8; str_len];
        self.src.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_rb_object(&mut self) -> TResult<RbObject> {
        let name = self.read_entry_symbol()?;
        let pair_count = self.read_int()? as usize;
        let fields = self.read_pairs(pair_count)?;
        let mut obj = RbObject::new(&name);
        obj.extend_from_pairs(&fields)?;
        Ok(obj)
    }

    fn read_user_class(&mut self) -> TResult<RbRef> {
        Ok(RbRef::UserClass(self.read_rb_class()?))
    }

    /// Read a string byte from the stream. Convenience method.
    fn read_byte(&mut self) -> TResult<u8> {
        let mut buf = [0u8; 1];
        self.src.read_exact(&mut buf)?;
        Ok(buf[0])
    }
}

/// Deserialize an `RbAny` from an IO stream.
/// 
/// Thurgood does check for the proper header bytes, and will refuse to deserialize
/// an incomplete or corrupted data stream.
pub fn from_reader<R: io::Read>(src: R) -> TResult<RbAny> {
    let mut de = RbReader::new(src);
    de.read()
}
