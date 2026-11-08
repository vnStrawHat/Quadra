// Minimal COM support: calling methods by vtable slot, and implementing
// COM interfaces in V. Slot numbers come from tools/com_slots.sh.
module win

pub struct Guid {
pub:
	d1 u32
	d2 u16
	d3 u16
	d4 [8]u8
}

// guid parses "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" as copied from the SDK headers.
pub fn guid(s string) Guid {
	h := s.replace('-', '')
	mut d4 := [8]u8{}
	for i in 0 .. 8 {
		d4[i] = u8(h[16 + i * 2..18 + i * 2].parse_uint(16, 8) or { 0 })
	}
	return Guid{
		d1: u32(h[0..8].parse_uint(16, 32) or { 0 })
		d2: u16(h[8..12].parse_uint(16, 16) or { 0 })
		d3: u16(h[12..16].parse_uint(16, 16) or { 0 })
		d4: d4
	}
}

pub fn (a &Guid) equals(b &Guid) bool {
	return a.d1 == b.d1 && a.d2 == b.d2 && a.d3 == b.d3 && a.d4 == b.d4
}

pub const iid_iunknown = guid('00000000-0000-0000-c000-000000000046')

pub const e_nointerface = int(u32(0x80004002))
pub const e_notimpl = int(u32(0x80004001))

// slot returns the function pointer at `index` in the object's vtable.
@[inline]
pub fn slot(obj voidptr, index int) voidptr {
	unsafe {
		vt := *(&&voidptr(obj))
		return vt[index]
	}
}

type FnRelease = fn (this voidptr) u32

type FnQueryInterface = fn (this voidptr, riid &Guid, out &voidptr) int

pub fn release(obj voidptr) {
	if obj != unsafe { nil } {
		f := unsafe { FnRelease(slot(obj, 2)) }
		f(obj)
	}
}

pub fn query_interface(obj voidptr, iid &Guid) !voidptr {
	f := unsafe { FnQueryInterface(slot(obj, 0)) }
	mut out := unsafe { nil }
	check(f(obj, iid, &out), 'QueryInterface')!
	return out
}

pub fn check(hr int, what string) ! {
	if hr < 0 {
		return error('${what} failed: HRESULT 0x${u32(hr).hex()}')
	}
}

// ComObject is the header every COM object implemented in V starts with.
// The vtable is a heap array of V function pointers; slots 0-2 are IUnknown.
pub struct ComObject {
pub mut:
	vtbl &voidptr = unsafe { nil }
	refs u32      = 1
	iid  Guid // the one interface this object implements besides IUnknown
}

pub fn new_vtable(methods []voidptr) &voidptr {
	mut all := []voidptr{cap: 3 + methods.len}
	all << voidptr(com_query_interface)
	all << voidptr(com_add_ref)
	all << voidptr(com_release)
	all << methods
	// Plain function pointers: no-scan memory is fine, the GC never needs to follow them.
	mut table := unsafe { &voidptr(malloc_noscan(all.len * int(sizeof(voidptr)))) }
	for i, m in all {
		unsafe {
			table[i] = m
		}
	}
	return table
}

fn com_query_interface(this voidptr, riid &Guid, out &voidptr) int {
	obj := unsafe { &ComObject(this) }
	if riid.equals(&iid_iunknown) || riid.equals(&obj.iid) {
		unsafe {
			*out = this
		}
		com_add_ref(this)
		return 0
	}
	unsafe {
		*out = nil
	}
	return e_nointerface
}

fn com_add_ref(this voidptr) u32 {
	mut obj := unsafe { &ComObject(this) }
	obj.refs++
	return obj.refs
}

// The V side owns the object (it lives on the GC heap), so Release never frees.
fn com_release(this voidptr) u32 {
	mut obj := unsafe { &ComObject(this) }
	if obj.refs > 0 {
		obj.refs--
	}
	return obj.refs
}
