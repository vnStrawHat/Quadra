// Language rows: the C family, JVM and the compiled languages (lang_spec.v).
module code

const c_kw = 'auto const extern inline register restrict signed sizeof static struct typedef union unsigned volatile enum _Atomic _Static_assert _Alignas _Generic'
const c_ctl = 'if else for while do switch case default break continue return goto'
const c_types = 'int char short long float double void bool _Bool size_t ssize_t ptrdiff_t int8_t int16_t int32_t int64_t uint8_t uint16_t uint32_t uint64_t intptr_t uintptr_t wchar_t FILE'
const js_kw = 'var let const function class extends new delete typeof instanceof in of void this super import export from as async await yield static get set debugger with'
const js_ctl = 'if else for while do switch case default break continue return try catch finally throw'

const lang_c = LangSpec{
	names:         words('c h')
	keywords:      words(c_kw)
	control:       words(c_ctl)
	types:         words(c_types)
	constants:     words('true false NULL nullptr')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	quotes:        '"\''
	preproc:       true
	cap_type:      true
}

const lang_cpp = LangSpec{
	names:         words('cpp c++ cc cxx hpp hh hxx h++ ino cu')
	keywords:      words(c_kw +
		' class namespace template typename using virtual override final public private protected friend operator new delete this explicit constexpr consteval constinit mutable noexcept static_assert decltype alignas alignof concept requires co_await co_yield co_return export import module thread_local')
	control:       words(c_ctl + ' try catch throw')
	types:         words(c_types + ' char8_t char16_t char32_t')
	constants:     words('true false NULL nullptr')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	quotes:        '"\''
	preproc:       true
	cap_type:      true
}

const lang_cs = LangSpec{
	names:         words('cs csharp c# csx')
	keywords:      words('abstract as base class const delegate enum event explicit extern fixed implicit in interface internal is lock namespace new operator out override params private protected public readonly ref sealed sizeof stackalloc static struct this typeof unsafe using virtual volatile async await var get set init record partial where yield nameof required global')
	control:       words('if else for foreach while do switch case default break continue return goto try catch finally throw when')
	types:         words('bool byte sbyte char decimal double float int uint long ulong short ushort object string void dynamic nint nuint')
	constants:     words('true false null')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	quotes:        '"\''
	prefixes:      '@$'
	preproc:       true
	cap_type:      true
}

const lang_java = LangSpec{
	names:         words('java')
	keywords:      words('abstract assert class const enum extends final implements import instanceof interface native new package private protected public static strictfp super synchronized this transient volatile var record sealed permits yield throws')
	control:       words('if else for while do switch case default break continue return try catch finally throw goto')
	types:         words('boolean byte char short int long float double void String Object')
	constants:     words('true false null')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	quotes:        '"\''
	triple:        true
	at_attr:       true
	cap_type:      true
}

const lang_kotlin = LangSpec{
	names:         words('kotlin kt kts')
	keywords:      words('fun val var class object interface enum sealed data inner open abstract override private protected public internal companion import package typealias constructor init this super is as in out by get set lateinit suspend inline reified operator infix tailrec vararg const external annotation where')
	control:       words('if else when for while do return break continue try catch finally throw')
	types:         words('Int Long Short Byte Float Double Boolean Char String Unit Any Nothing')
	constants:     words('true false null')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	nest:          true
	quotes:        '"\''
	triple:        true
	interps:       ['\${']
	interp_in:     '"'
	dollar_var:    true
	at_attr:       true
	cap_type:      true
}

const lang_swift = LangSpec{
	names:         words('swift')
	keywords:      words('func var let class struct enum protocol extension import init deinit self Self super static private fileprivate public internal open final override mutating nonmutating inout typealias associatedtype subscript operator lazy weak unowned convenience required dynamic some any is as in where async await throws rethrows actor nonisolated')
	control:       words('if else guard for while repeat switch case default break continue return fallthrough defer do try catch throw')
	types:         words('Int Int8 Int16 Int32 Int64 UInt UInt8 UInt16 UInt32 UInt64 Float Double Bool String Character Void')
	constants:     words('true false nil')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	nest:          true
	quotes:        '"'
	triple:        true
	interps:       ['\\(']
	interp_in:     '"'
	at_attr:       true
	cap_type:      true
}

const lang_go = LangSpec{
	names:         words('go golang')
	keywords:      words('package import func var const type struct interface map chan go defer select range')
	control:       words('if else for switch case default break continue return goto fallthrough')
	types:         words('int int8 int16 int32 int64 uint uint8 uint16 uint32 uint64 uintptr float32 float64 complex64 complex128 byte rune string bool error any')
	constants:     words('true false nil iota')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	quotes:        '"\''
	multi_quotes:  '`'
	raw_quotes:    '`'
	cap_type:      true
}

// ponytail: Rust raw strings with two or more hashes (r##"..."##) end at the first "#.
const lang_rust = LangSpec{
	names:         words('rust rs')
	keywords:      words('as async await const crate dyn enum extern fn impl let mod move mut pub ref self Self static struct super trait type union unsafe use where')
	control:       words('if else for while loop match return break continue in')
	types:         words('i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool char str String Vec Option Result Box')
	constants:     words('true false None Some Ok Err')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	nest:          true
	quotes:        "'"
	multi_quotes:  '"'
	prefixes:      'rb'
	lifetimes:     true
	macro_bang:    true
	hash_attr:     true
	cap_type:      true
}

// ponytail: regex literals and JSX tags are not recognized; /x/ reads as operators.
const lang_js = LangSpec{
	names:         words('js javascript jsx mjs cjs node')
	keywords:      words(js_kw)
	control:       words(js_ctl)
	constants:     words('true false null undefined NaN Infinity')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	quotes:        '"\''
	multi_quotes:  '`'
	interps:       ['\${']
	interp_in:     '`'
	at_attr:       true
	cap_type:      true
}

const lang_ts = LangSpec{
	names:         words('ts typescript tsx mts cts')
	keywords:      words(js_kw +
		' interface type enum namespace declare abstract implements private protected public readonly keyof infer is asserts satisfies override module')
	control:       words(js_ctl)
	types:         words('string number boolean any unknown never void object bigint symbol')
	constants:     words('true false null undefined NaN Infinity')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	quotes:        '"\''
	multi_quotes:  '`'
	interps:       ['\${']
	interp_in:     '`'
	at_attr:       true
	cap_type:      true
}

const lang_dart = LangSpec{
	names:         words('dart')
	keywords:      words('abstract as assert async await base class const covariant deferred dynamic enum export extends extension external factory final get hide implements import in interface is late library mixin new of on operator part required sealed set show static super sync this typedef var with yield')
	control:       words('if else for while do switch case default break continue return try catch finally throw rethrow')
	types:         words('int double num bool String List Map Set void Object Future Stream Iterable')
	constants:     words('true false null')
	line_comments: ['//']
	block_open:    '/*'
	block_close:   '*/'
	quotes:        '"\''
	triple:        true
	prefixes:      'r'
	interps:       ['\${']
	interp_in:     '"\''
	dollar_var:    true
	at_attr:       true
	cap_type:      true
}
