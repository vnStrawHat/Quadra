// Checked holders (specs/14): a pointer erased for storage (listener holders, callback
// contexts, store slots) that remembers its type outside -prod builds. `held[T]` panics
// with both type names when the pointer is read back as another type; a wrong
// `unsafe { &T(p) }` would compile and corrupt memory. In -prod builds Holder is the bare
// pointer: same size, no tag, no branch (tests/holder_test.v checks the size).
module prim

// V 0.5.2 has no `$if` inside a struct body, and file suffixes do not follow -prod: a
// top-level `$if` picks the declaration.
$if prod {
	pub struct Holder {
	mut:
		ptr voidptr
	}
} $else {
	pub struct Holder {
	mut:
		ptr voidptr
		tag string // typeof[T]().name of what ptr points to
	}
}

// hold erases p, keeping its type for checked reads.
@[inline]
pub fn hold[T](p &T) Holder {
	mut h := Holder{
		ptr: unsafe { voidptr(p) } // unsafe: V 0.5.2 takes voidptr(p) for a string cast when T is string
	}
	$if !prod {
		h.tag = typeof[T]().name
	}
	return h
}

// hold_named erases p with a type name recorded elsewhere (typeof[T]().name, as a
// type-erased store keeps it). Prefer hold: nothing checks that the name is p's.
@[inline]
pub fn hold_named(p voidptr, type_name string) Holder {
	mut h := Holder{
		ptr: p
	}
	$if !prod {
		h.tag = type_name
	}
	return h
}

// held reads the pointer back as a &T. Outside -prod builds a holder of another type
// panics. An empty holder gives nil. A free function: a generic method call such as
// `h.get[Gen[A, B]]()` fails to type-check inside generic functions in V 0.5.2.
@[inline]
pub fn held[T](h Holder) &T {
	$if !prod {
		if h.ptr != unsafe { nil } && h.tag != typeof[T]().name {
			panic('quadra: a holder of ${h.tag} was read as ${typeof[T]().name} (specs/14, checked holders)')
		}
	}
	// No cast expression: V checks a cast's type is visible here, and T may be private to
	// its module. A voidptr converts to any pointer implicitly.
	return h.ptr
}

// holds reports whether held[T] would succeed. Always true in -prod builds (no tag).
pub fn holds[T](h Holder) bool {
	$if !prod {
		return h.ptr == unsafe { nil } || h.tag == typeof[T]().name
	}
	return true
}

// is_nil reports whether nothing is held.
@[inline]
pub fn (h Holder) is_nil() bool {
	return h.ptr == unsafe { nil }
}

// raw gives the bare pointer, for identity only (comparisons, map keys): never cast it.
@[inline]
pub fn (h Holder) raw() voidptr {
	return h.ptr
}
