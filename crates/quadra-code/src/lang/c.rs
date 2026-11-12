//! Rows: the C family, the languages of the JVM and the compiled ones.

use super::{LangSpec, PLAIN};

macro_rules! c_kw {
    () => {
        "auto const extern inline register restrict signed sizeof static struct typedef union unsigned volatile enum _Atomic _Static_assert _Alignas _Generic"
    };
}
macro_rules! c_ctl {
    () => {
        "if else for while do switch case default break continue return goto"
    };
}
macro_rules! c_types {
    () => {
        "int char short long float double void bool _Bool size_t ssize_t ptrdiff_t int8_t int16_t int32_t int64_t uint8_t uint16_t uint32_t uint64_t intptr_t uintptr_t wchar_t FILE"
    };
}
macro_rules! js_kw {
    () => {
        "var let const function class extends new delete typeof instanceof in of void this super import export from as async await yield static get set debugger with"
    };
}
const JS_CTL: &str = "if else for while do switch case default break continue return try catch finally throw";

/// What the C family shares: its comments, its quotes, types by their capital.
const LIKE_C: LangSpec = LangSpec {
    line_comments: &["//"],
    block_open: "/*",
    block_close: "*/",
    quotes: "\"'",
    cap_type: true,
    ..PLAIN
};

pub const C: LangSpec = LangSpec {
    names: "c h",
    keywords: c_kw!(),
    control: c_ctl!(),
    types: c_types!(),
    constants: "true false NULL nullptr",
    preproc: true,
    ..LIKE_C
};

pub const CPP: LangSpec = LangSpec {
    names: "cpp c++ cc cxx hpp hh hxx h++ ino cu",
    keywords: concat!(
        c_kw!(),
        " class namespace template typename using virtual override final public private protected friend operator new delete this explicit constexpr consteval constinit mutable noexcept static_assert decltype alignas alignof concept requires co_await co_yield co_return export import module thread_local"
    ),
    control: concat!(c_ctl!(), " try catch throw"),
    types: concat!(c_types!(), " char8_t char16_t char32_t"),
    constants: "true false NULL nullptr",
    preproc: true,
    ..LIKE_C
};

pub const CS: LangSpec = LangSpec {
    names: "cs csharp c# csx",
    keywords: "abstract as base class const delegate enum event explicit extern fixed implicit in interface internal is lock namespace new operator out override params private protected public readonly ref sealed sizeof stackalloc static struct this typeof unsafe using virtual volatile async await var get set init record partial where yield nameof required global",
    control: "if else for foreach while do switch case default break continue return goto try catch finally throw when",
    types: "bool byte sbyte char decimal double float int uint long ulong short ushort object string void dynamic nint nuint",
    constants: "true false null",
    prefixes: "@$",
    preproc: true,
    ..LIKE_C
};

pub const JAVA: LangSpec = LangSpec {
    names: "java",
    keywords: "abstract assert class const enum extends final implements import instanceof interface native new package private protected public static strictfp super synchronized this transient volatile var record sealed permits yield throws",
    control: "if else for while do switch case default break continue return try catch finally throw goto",
    types: "boolean byte char short int long float double void String Object",
    constants: "true false null",
    triple: true,
    at_attr: true,
    ..LIKE_C
};

pub const KOTLIN: LangSpec = LangSpec {
    names: "kotlin kt kts",
    keywords: "fun val var class object interface enum sealed data inner open abstract override private protected public internal companion import package typealias constructor init this super is as in out by get set lateinit suspend inline reified operator infix tailrec vararg const external annotation where",
    control: "if else when for while do return break continue try catch finally throw",
    types: "Int Long Short Byte Float Double Boolean Char String Unit Any Nothing",
    constants: "true false null",
    nest: true,
    triple: true,
    interps: &["${"],
    interp_in: "\"",
    dollar_var: true,
    at_attr: true,
    ..LIKE_C
};

pub const SWIFT: LangSpec = LangSpec {
    names: "swift",
    keywords: "func var let class struct enum protocol extension import init deinit self Self super static private fileprivate public internal open final override mutating nonmutating inout typealias associatedtype subscript operator lazy weak unowned convenience required dynamic some any is as in where async await throws rethrows actor nonisolated",
    control: "if else guard for while repeat switch case default break continue return fallthrough defer do try catch throw",
    types: "Int Int8 Int16 Int32 Int64 UInt UInt8 UInt16 UInt32 UInt64 Float Double Bool String Character Void",
    constants: "true false nil",
    nest: true,
    quotes: "\"",
    triple: true,
    interps: &["\\("],
    interp_in: "\"",
    at_attr: true,
    ..LIKE_C
};

pub const GO: LangSpec = LangSpec {
    names: "go golang",
    keywords: "package import func var const type struct interface map chan go defer select range",
    control: "if else for switch case default break continue return goto fallthrough",
    types: "int int8 int16 int32 int64 uint uint8 uint16 uint32 uint64 uintptr float32 float64 complex64 complex128 byte rune string bool error any",
    constants: "true false nil iota",
    multi_quotes: "`",
    raw_quotes: "`",
    ..LIKE_C
};

// ponytail: a raw string with two hashes or more (r##"..."##) ends at the first "#.
pub const RUST: LangSpec = LangSpec {
    names: "rust rs",
    keywords: "as async await const crate dyn enum extern fn impl let mod move mut pub ref self Self static struct super trait type union unsafe use where",
    control: "if else for while loop match return break continue in",
    types: "i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool char str String Vec Option Result Box",
    constants: "true false None Some Ok Err",
    nest: true,
    quotes: "'",
    multi_quotes: "\"",
    prefixes: "rb",
    lifetimes: true,
    macro_bang: true,
    hash_attr: true,
    ..LIKE_C
};

// ponytail: a pattern between slashes and tags in code are not known; /x/ reads as operators.
pub const JS: LangSpec = LangSpec {
    names: "js javascript jsx mjs cjs node",
    keywords: js_kw!(),
    control: JS_CTL,
    constants: "true false null undefined NaN Infinity",
    multi_quotes: "`",
    interps: &["${"],
    interp_in: "`",
    at_attr: true,
    ..LIKE_C
};

pub const TS: LangSpec = LangSpec {
    names: "ts typescript tsx mts cts",
    keywords: concat!(
        js_kw!(),
        " interface type enum namespace declare abstract implements private protected public readonly keyof infer is asserts satisfies override module"
    ),
    control: JS_CTL,
    types: "string number boolean any unknown never void object bigint symbol",
    constants: "true false null undefined NaN Infinity",
    multi_quotes: "`",
    interps: &["${"],
    interp_in: "`",
    at_attr: true,
    ..LIKE_C
};

pub const DART: LangSpec = LangSpec {
    names: "dart",
    keywords: "abstract as assert async await base class const covariant deferred dynamic enum export extends extension external factory final get hide implements import in interface is late library mixin new of on operator part required sealed set show static super sync this typedef var with yield",
    control: "if else for while do switch case default break continue return try catch finally throw rethrow",
    types: "int double num bool String List Map Set void Object Future Stream Iterable",
    constants: "true false null",
    triple: true,
    prefixes: "r",
    interps: &["${"],
    interp_in: "\"'",
    dollar_var: true,
    at_attr: true,
    ..LIKE_C
};
