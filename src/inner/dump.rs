use super::*;
use std::fmt::{self, Write};
use std::io;

struct Dumper<'a, 'b> {
    f: &'a mut fmt::Formatter<'b>,
    max_depth: usize,
}

impl<'a, 'b: 'a> Dumper<'a, 'b> {
    fn dump_rec(&mut self, e: &RbAny, depth: usize) -> fmt::Result {
        match e {
            RbAny::Int(_) | RbAny::True | RbAny::False | RbAny::Nil | RbAny::Symbol(_) => {
                write!(self.f, "{:?}", e)
            },
            RbAny::Ref(v) => {
                match v.as_ref() {
                    RbRef::Object(o) => {
                        write!(self.f, "Object {:?} {{\n", o.name)?;
                        if depth < self.max_depth {
                            for (key, val) in &o.fields {
                                self.print_spaces(depth + 1)?;
                                write!(self.f, "{:?} = ", key)?;
                                self.dump_rec(val, depth + 1)?;
                                write!(self.f, "\n")?;
                            }
                        }
                        self.print_spaces(depth)?;
                        write!(self.f, "}}")
                    },
                    RbRef::Hash(h) => {
                        write!(self.f, "Hash {{\n")?;
                        if depth < self.max_depth {
                            for (key, val) in h.map.iter() {
                                self.print_spaces(depth + 1)?;
                                self.dump_rec(key, depth + 1)?;
                                write!(self.f, " = ")?;
                                self.dump_rec(val, depth + 1)?;
                                write!(self.f, "\n")?;
                            }
                        }
                        self.print_spaces(depth)?;
                        write!(self.f, "}}")
                    },
                    RbRef::Array(ar) => {
                        write!(self.f, "[\n")?;
                        if depth < self.max_depth {
                            for it in ar.iter() {
                                self.print_spaces(depth + 1)?;
                                self.dump_rec(it, depth + 1)?;
                                write!(self.f, "\n")?;
                            }
                        }
                        self.print_spaces(depth)?;
                        write!(self.f, "]")
                    },
                    RbRef::Str(s) => {
                        write!(self.f, "\"{}\"", s)
                    },
                    _ => {
                        write!(self.f, "todo!()")
                    }
                }
            }
        }
    }

    fn print_spaces(&mut self, s: usize) -> fmt::Result {
        for _ in 0..(s * 2) {
            self.f.write_char(' ')?;
        }
        Ok(())
    }
}

struct DumperWrap<'a> {
    root: &'a RbAny,
    max_depth: usize,
}

impl<'a> fmt::Display for DumperWrap<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = Dumper { f, max_depth: self.max_depth };
        d.dump_rec(self.root, 0)
    }
}

/// Pretty-print the Ruby object in a textual format, with the given maximum recursive depth.
///
/// This is intended for debug purposes and is NOT fully implemented. Prefer `to_json()` for
/// a more complete information dump, but note that the JSON conversion doesn't preserve all data.
pub fn dump_ruby_pretty<W: io::Write>(mut dst: W, root: &RbAny, max_depth: usize) -> io::Result<()> {
    let d = DumperWrap { root, max_depth };
    write!(dst, "{}", d)
}
