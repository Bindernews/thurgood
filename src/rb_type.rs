/// Indicates the type of a Ruby Any. This is intended to make debugging
/// or displaying information easier, and has no bearing on (de)serialization.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RbType {
    Int,
    Bool,
    Float,
    Nil,
    BigInt,
    Symbol,
    Array,
    Str,
    Regex,
    Hash,
    Struct,
    Object,
    ClassRef,
    ModuleRef,
    ClassModuleRef,
    Data,
    UserClass,
    UserData,
    UserMarshal,
    ObjectRef,
    Extended,
}
