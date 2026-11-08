// Bound: a function together with its data, what a closure would be, without one
// (specs/14, avoiding closures). f captures nothing; the data rides in a generic holder,
// and the one cast back to the holder's type is in call_bound, made for the same types.
//
// V 0.5.2 mangles a pointer to a generic function wrongly when a type argument is itself a
// pointer (`call_bound[D, &Theme, R]` is referred to as `call_bound_T_D_Theme*_R` but
// generated as `call_bound_T_D___ptr__Theme_R`). So type arguments are never pointers:
// a function taking a pointer argument uses BoundRef, whose A is the pointed-to type.
// Not named `bind`: V 0.5.2 then mixes this up with the generic method `App.bind[A]`
// (`typeof[A]().name` came out as "A" and actions got the wrong type).
module core

pub struct Bound[A, R] {
	holder voidptr
	call   fn (holder voidptr, a A) R = unsafe { nil }
}

struct BoundHolder[D, A, R] {
	data D
	f    fn (d D, a A) R = unsafe { nil }
}

fn call_bound[D, A, R](holder voidptr, a A) R {
	h := unsafe { &BoundHolder[D, A, R](holder) }
	return h.f(h.data, a)
}

// bind_fn pairs data with a function of it: `core.bind_fn(rows, fn (r Rows, i int) Element {...})`.
// A must not be a pointer type (see above): use bind_fn_ref.
pub fn bind_fn[D, A, R](data D, f fn (d D, a A) R) Bound[A, R] {
	return Bound[A, R]{
		holder: voidptr(&BoundHolder[D, A, R]{data, f})
		call:   call_bound[D, A, R]
	}
}

// run calls the function with its data.
pub fn (b Bound[A, R]) run(a A) R {
	return b.call(b.holder, a)
}

// is_set reports whether a function was bound.
pub fn (b Bound[A, R]) is_set() bool {
	return b.call != unsafe { nil }
}

// BoundRef is Bound for a function whose argument is a pointer (`&A`).
pub struct BoundRef[A, R] {
	holder voidptr
	call   fn (holder voidptr, a &A) R = unsafe { nil }
}

struct BoundRefHolder[D, A, R] {
	data D
	f    fn (d D, a &A) R = unsafe { nil }
}

fn call_bound_ref[D, A, R](holder voidptr, a &A) R {
	h := unsafe { &BoundRefHolder[D, A, R](holder) }
	return h.f(h.data, a)
}

// bind_fn_ref pairs data with a function taking a pointer: `core.bind_fn_ref(args, fn (a Args,
// t &Theme) Element {...})` gives a BoundRef[Theme, Element].
pub fn bind_fn_ref[D, A, R](data D, f fn (d D, a &A) R) BoundRef[A, R] {
	return BoundRef[A, R]{
		holder: voidptr(&BoundRefHolder[D, A, R]{data, f})
		call:   call_bound_ref[D, A, R]
	}
}

pub fn (b BoundRef[A, R]) run(a &A) R {
	return b.call(b.holder, a)
}

pub fn (b BoundRef[A, R]) is_set() bool {
	return b.call != unsafe { nil }
}
