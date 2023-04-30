//! Thurgood implements (de)serialization for Ruby's Marshal format.
//! 
//! Thurgood implements a full model for Ruby's Marshal format and fully supports round-tripping data,
//! with a few minor exceptions (see Errata below). Thurgood uses reference-counting to reduce memory
//! usage with large data sets and to ensure that object references can be properly represented during
//! deserialization and serialization.
//! 
//! Thurgood places an emphasis on the use-case of loading some data, manipulating parts of it, then serializing
//! it back to the Marshal format. Generating your own Ruby data from scratch is supported, but it's not
//! the primary use-case.
//! 
//! # Examples
//! Load a binary string, convert it to JSON, and pretty-print it (this requires the "json" feature).
//! ```rust
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     use thurgood::rc::{from_reader, RbAny, Error};
//! 
//!     let inp = concat!("\x04\x08[\x07o:\x08Foo\x07:\n@nameI\"\tJack\x06:\x06ET:",
//!         "\t@agei\x1Eo;\x00\x07;\x06I\"\tJane\x06;\x07T;\x08i\x1D");
//!     let value_ruby = from_reader( inp.as_bytes() ).expect("Parsing failed");
//!     let value_json = value_ruby.to_json().unwrap();
//!     println!("{}", serde_json::to_string_pretty( &value_json )?);
//!     Ok(())
//! }
//! ```
//! 
//! # `Rc` vs `Arc`
//! To support both single and multi-threaded uses Thurgood provides two implementations. They're
//! identical except for one using `Rc` and the other using `Arc` for all internal refernce-counting.
//! There are NO `RefMut`, `RwLock`, or `Mutex` in the code. If you want to mutate an object
//! use `Rc::get_mut`, `Rc::make_mut`, or the equivalent `Arc` functions. 
//! 
//! ## Errata
//! * Floats are stored as strings, however due to the way the spec is written, they may be either
//!   length-terminated OR NULL-terminated. Thurgood can parse either, but will only produce
//!   length-terminated floats.
//! * Standard Ruby strings are encoded as an `Instance` containing a string and one field: `:E => true`,
//!   Or as just a raw string. Thurgood assumes ALL instance strings with `:E => true` are UTF-8 strings
//!   and will ignore extra instance fields. In practice this shouldn't be an issue, and any string
//!   with a non-standard encoding is stored appropriately, but this is a potential source of difference
//!   when trying to round-trip data.
//! * If `RbReader.allow_bin_strings` is set to true the reader will produce `RbRef::StrI` instances
//!   when the input is a normal string, but not in UTF-8 encoding. This may impact round-trip byte-compatibility.
//! 
pub mod consts;
pub mod error;
mod rb_type;
pub use rb_type::RbType;
pub use error::{ThurgoodError, TResult};

pub mod rc;
/// This module is the same as rc but using Arc instead of Rc for situations where you need thread-safety.
#[cfg(not(doctest))]
#[cfg(feature = "arc")]
pub mod arc;

#[cfg(test)]
mod tests {
    use std::io;

    use crate::{rc::*, consts::{T_STRING, T_INSTANCE}};
    // use crate::inner::*;

    /// Parse a string into an `RbAny`
    fn reader_parse(s: &str) -> RbAny {
        from_reader(io::Cursor::new(s.as_bytes())).expect("Parsing error")
    }

    fn reader_parse_loose(s: &[u8]) -> RbAny {
        let mut rd = RbReader::new(io::Cursor::new(s));
        rd.allow_bin_strings = true;
        rd.read().expect("Parsing error")
    }

    /// Writes `value` to a `Vec<u8>` and returns it.
    fn writer_write(value: &RbAny) -> Vec<u8> {
        let mut buf = Vec::new();
        to_writer(&mut buf, value).expect("Writing error");
        buf
    }

    fn assert_write(value: &RbAny, expected: &[u8]) {
        assert_eq!(escape_str(writer_write(value).as_slice()), escape_str(expected));
    }

    #[test]
    fn array_string_hash() {
        let inp = "\x04\x08[\x07I\"\ttest\x06:\x06ET{\x06:\x06aI\"\x06b\x06;\x00T";
        let exp = RbAny::from(vec![
                RbAny::from("test"),
                RbAny::from(RbHash::from_pairs(vec![
                    ( RbSymbol::from("a").into(), RbAny::from("b") )
                ])),
            ]);
        assert!(reader_parse(inp).deep_eq(&exp));
        assert_eq!(writer_write(&exp).as_slice(), inp.as_bytes());
    }

    #[test]
    fn class_and_int() {
        let inp = "\x04\x08[\x07o:\x08Foo\x07:\n@nameI\"\tJack\x06:\x06ET:\t@agei\x1Eo;\x00\x07;\x06I\"\tJane\x06;\x07T;\x08i\x1D";
        let sym_name = RbSymbol::from("@name");
        let sym_age = RbSymbol::from("@age");
        let exp = RbAny::from(vec![
            RbObject::new_from_slice("Foo", &vec![
                ("@name", "Jack".into()),
                ("@age", 25.into()),
            ]).into_object().into(),
            RbRef::new_object("Foo", &vec![
                (sym_name.clone(), "Jane".into()),
                (sym_age.clone(), 24.into()),
            ]).into_any(),
        ]);
        assert!(reader_parse(inp).deep_eq(&exp));
        assert_write(&exp, inp.as_bytes());
    }

    #[test]
    fn modules() {
        let inp = "\x04\x08{\x07:\x07aao:\x0EBar::BazA\x00:\x07bbo:\x0EBar::BazB\x00";
        let sym_aa = RbSymbol::from("aa");
        let sym_bar_baz_a = RbSymbol::from("Bar::BazA");
        let sym_bb = RbSymbol::from("bb");
        let sym_bar_baz_b = RbSymbol::from("Bar::BazB");
        let exp = RbHash::from_pairs(vec![
                (sym_aa.clone().into(), RbRef::new_object(&sym_bar_baz_a, &vec![]).into()),
                (sym_bb.clone().into(), RbRef::new_object(&sym_bar_baz_b, &vec![]).into()),
            ]).into();
        assert!(reader_parse(inp).deep_eq(&exp));
        assert_write(&exp, inp.as_bytes());
    }

    #[test]
    fn object_ref_count_1() {
        let inp = "\x04\x08[\no:\x08Foo\x07:\n@nameI\"\tJack\x06:\x06ET:\t@agei\x1E@\x06{\x06:\x08key@\x06o;\x00\x07;\x06I\"\tJane\x06;\x07T;\x08i\x1D@\t";
        let sym_name: RbSymbol = "@name".into();
        let sym_age: RbSymbol = "@age".into();
        let sym_key: RbSymbol = "key".into();
        let ob_1 = RbRef::new_object("Foo", &vec![
            (sym_name.clone(), RbAny::from("Jack") ),
            (sym_age.clone(), RbAny::Int(25) ),
            ]).into_any();
        let ob_2 = RbRef::new_object("Foo", &vec![
            (sym_name.clone(), RbAny::from("Jane") ),
            (sym_age.clone(), RbAny::Int(24) ),
            ]).into_any();
        let exp = RbRef::Array(vec![
                ob_1.clone(),
                ob_1.clone(),
                RbHash::from_pairs(vec![
                    (sym_key.as_any(), ob_1.clone() )
                ]).into(),
                ob_2.clone(),
                ob_2.clone(),
            ]).into_any();
        assert!(reader_parse(inp).deep_eq(&exp));
        assert_write(&exp, inp.as_bytes());
    }

    #[test]
    fn object_ref_count_2() {
        let inp = "\x04\x08[\x07[\x06I\"\tTest\x06:\x06ET@\x06";
        let ob_1 = RbRef::Array(vec![ RbAny::from("Test") ]).into_any();
        let exp = RbRef::Array(vec![
            ob_1.clone(),
            ob_1.clone(),
        ]).into_any();
        assert!(reader_parse(inp).deep_eq(&exp));
        assert_write(&exp, inp.as_bytes());
    }

    #[test]
    fn read_extended() {
        let inp = "\x04\x08e:\x08Bar[\x00";
        let sym_bar = RbSymbol::from("Bar");
        let exp = RbRef::Extended {
            module: sym_bar.clone(),
            object: RbRef::Array(vec![]).into(),
        }.into_any();
        assert!(reader_parse(inp).deep_eq(&exp));
        assert_write(&exp, inp.as_bytes());
    }

    /// Technically floats can be stored as NULL-terminated C-strings. This is dumb, but here's
    /// a test for it anyways. This also tests normal floats.
    #[test]
    fn float_types() {
        let inp = "\x04\x08[\x08f\x0D0.123\x00NOf\n1.234f\x10-1196073.75";
        let out = "\x04\x08[\x08f\n0.123f\n1.234f\x10-1196073.75";
        let exp = RbAny::from(vec![
            RbAny::from(0.123f64),
            RbAny::from(1.234f64),
            RbAny::from(-1196073.75f64),
        ]);
        assert!(reader_parse(inp).deep_eq(&exp));
        assert_write(&exp, out.as_bytes());
    }

    #[test]
    fn invalid_utf8_string_allowed() {
        let inp = vec![0x04u8, 0x08, T_STRING, 0x08, 0xc3, 0x28, 0x34];
        let out = vec![0x04u8, 0x08, T_INSTANCE, T_STRING, 0x08, 0xc3, 0x28, 0x34, 0x00];
        let exp = RbRef::StrI { content: vec![0xc3, 0x28, 0x34], metadata: RbFields::new() }.into_any();
        assert!(reader_parse_loose(&inp).deep_eq(&exp));
        assert_write(&exp, &out);
    }

    fn escape_str(src: &[u8]) -> String {
        let mut out = String::new();
        for b in src {
            let c = char::from(*b);
            if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() {
                out.push(c);
            } else {
                out.push_str(&format!("\\x{:02X}", *b));
            }
        }
        out
    }
}
