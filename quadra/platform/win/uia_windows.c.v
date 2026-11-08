// UI Automation server (specs/20): the window answers WM_GETOBJECT with a fragment root
// whose children are the app's semantic nodes. Each node is one V object with a tear-off
// face per interface; UIA calls arrive on the window's thread. Nodes are looked up by id
// in the latest tree, so a provider UIA still holds after its element went away answers
// "element not available".
module win

import quadra.platform

const uia_root_object_id = -25
const uia_e_elementnotavailable = int(u32(0x80040201))
const uia_focus_changed_event = 20005

const iid_simple = guid('d6dd68d1-86fd-4332-8666-9abedea2d24c')
const iid_fragment = guid('f7063da8-8359-439c-9297-bbc5299a7d87')
const iid_fragment_root = guid('620ce2a5-ab8f-40a9-86cb-de3c75599b58')
const iid_invoke = guid('54fcb24b-e18e-47a2-b4d3-eccbe77599a2')
const iid_toggle = guid('56d00bd0-c4f4-433c-a836-1a52a57e0892')
const iid_selection_item = guid('2acad808-b2d4-452d-a407-91ff1ad167b2')
const iid_expand = guid('d847d3a5-cab0-4a98-8c32-ecb45c59ad24')
const iid_value = guid('c7935180-6fb3-4201-b174-7df73adbf64a')

type FnReturnProvider = fn (hwnd voidptr, wparam usize, lparam isize, el voidptr) isize

type FnHostFromHwnd = fn (hwnd voidptr, out &voidptr) int

type FnRaiseEvent = fn (el voidptr, id int) int

type FnListening = fn () int

type FnSysAlloc = fn (s &u16) voidptr

type FnSafeArrayVector = fn (vt u16, lbound int, n u32) voidptr

type FnSafeArrayPut = fn (sa voidptr, index &int, v voidptr) int

struct UiaApi {
	ok        bool
	ret       FnReturnProvider  = unsafe { nil }
	host      FnHostFromHwnd    = unsafe { nil }
	raise     FnRaiseEvent      = unsafe { nil }
	listening FnListening       = unsafe { nil }
	sys_alloc FnSysAlloc        = unsafe { nil }
	array     FnSafeArrayVector = unsafe { nil }
	array_put FnSafeArrayPut    = unsafe { nil }
}

fn load_uia_api() UiaApi {
	core := load_system_dll('uiautomationcore.dll') or { return UiaApi{} }
	ole := load_system_dll('oleaut32.dll') or { return UiaApi{} }
	return UiaApi{
		ok:        true
		ret:       FnReturnProvider(dll_proc(core, 'UiaReturnRawElementProvider') or {
			return UiaApi{}
		})
		host:      FnHostFromHwnd(dll_proc(core, 'UiaHostProviderFromHwnd') or { return UiaApi{} })
		raise:     FnRaiseEvent(dll_proc(core, 'UiaRaiseAutomationEvent') or { return UiaApi{} })
		listening: FnListening(dll_proc(core, 'UiaClientsAreListening') or { return UiaApi{} })
		sys_alloc: FnSysAlloc(dll_proc(ole, 'SysAllocString') or { return UiaApi{} })
		array:     FnSafeArrayVector(dll_proc(ole, 'SafeArrayCreateVector') or { return UiaApi{} })
		array_put: FnSafeArrayPut(dll_proc(ole, 'SafeArrayPutElement') or { return UiaApi{} })
	}
}

// Face is one interface of a UiaNode: COM sees the vtable pointer at its start.
struct Face {
mut:
	vtbl &voidptr = unsafe { nil }
	node &UiaNode = unsafe { nil }
}

@[heap]
struct UiaNode {
mut:
	simple   Face
	fragment Face
	root     Face
	invoke   Face
	toggle   Face
	select   Face
	expand   Face
	value    Face
	refs     u32
	id       u64 // 0: the fragment root
	uia      &Uia = unsafe { nil }
}

@[heap]
struct Uia {
mut:
	api        UiaApi
	w          &WinWindow = unsafe { nil }
	nodes      []platform.A11yNode
	index      map[u64]int
	first      []int // first child of node i, -1 = none (index nodes.len: the root)
	next       []int
	prev       []int
	last       []int
	providers  map[u64]&UiaNode
	root       &UiaNode = unsafe { nil }
	vt         [8]&voidptr
	last_focus u64
	tid        u32 // the UI thread
	foreign    int // calls that arrived on another thread (tests check none do)
}

fn new_uia(mut w WinWindow) ?&Uia {
	api := load_uia_api()
	if !api.ok {
		return none
	}
	mut u := &Uia{
		api: api
		w:   w
		tid: C.GetCurrentThreadId()
	}
	u.vt = uia_vtables()
	u.root = u.node(0)
	u.update([])
	return u
}

// node returns the provider object for an id, made on first use and kept: UIA holds
// pointers into it that the GC cannot see.
fn (mut u Uia) node(id u64) &UiaNode {
	if n := u.providers[id] {
		return n
	}
	mut n := &UiaNode{
		id:  id
		uia: u
	}
	faces := [&n.simple, &n.fragment, &n.root, &n.invoke, &n.toggle, &n.select, &n.expand, &n.value]
	for k, f in faces {
		unsafe {
			*f = Face{u.vt[k], n}
		}
	}
	u.providers[id] = n
	return n
}

// update takes the tree of a new prepaint and links children for navigation.
fn (mut u Uia) update(nodes []platform.A11yNode) {
	n := nodes.len
	u.nodes = nodes
	u.index = map[u64]int{}
	u.first = []int{len: n + 1, init: -1}
	u.last = []int{len: n + 1, init: -1}
	u.next = []int{len: n, init: -1}
	u.prev = []int{len: n, init: -1}
	for i, node in nodes {
		u.index[node.id] = i
		p := if node.parent >= 0 { node.parent } else { n }
		if u.last[p] >= 0 {
			u.next[u.last[p]] = i
			u.prev[i] = u.last[p]
		} else {
			u.first[p] = i
		}
		u.last[p] = i
	}
	// ponytail: providers of vanished nodes are dropped only once UIA released them all;
	// an app churning through millions of ids keeps those UIA still references.
	for id, p in u.providers {
		if id != 0 && p.refs == 0 && id !in u.index {
			u.providers.delete(id)
		}
	}
	// ponytail: only focus changes are raised; property and structure changes are not, so a
	// screen reader sees a new toggle state when it next reads the node. Raise them when
	// live regions (toasts, logs) need announcing.
	mut focus := u64(0)
	for node in nodes {
		if node.focused {
			focus = node.id
		}
	}
	if focus != u.last_focus {
		u.last_focus = focus
		if focus != 0 && u.api.listening() != 0 {
			mut f := u.node(focus)
			u.api.raise(&f.simple, uia_focus_changed_event)
		}
	}
}

// --- PlatformWindow ---

pub fn (w &WinWindow) a11y_active() bool {
	return w.uia != unsafe { nil }
}

pub fn (mut w WinWindow) update_a11y(nodes []platform.A11yNode) {
	if w.uia != unsafe { nil } {
		w.uia.update(nodes)
	}
}

// uia_get_object answers WM_GETOBJECT for the UIA root; the first ask turns a11y on.
fn (mut w WinWindow) uia_get_object(wparam usize, lparam isize) ?isize {
	if i32(u32(u64(lparam) & 0xFFFF_FFFF)) != uia_root_object_id {
		return none
	}
	if w.uia == unsafe { nil } {
		if !w.plat.ole().ok { // the UI thread becomes a COM apartment calls are marshaled to
			return none
		}
		w.uia = new_uia(mut w) or { return none }
		w.request_frame() // the next prepaint sends the tree
	}
	return w.uia.api.ret(w.hwnd, wparam, lparam, &w.uia.root.simple)
}

fn (mut w WinWindow) uia_disconnect() {
	if w.uia != unsafe { nil } {
		w.uia.api.ret(w.hwnd, 0, 0, unsafe { nil })
	}
}

// uia_foreign_calls counts provider calls made off the UI thread (tests).
pub fn (w &WinWindow) uia_foreign_calls() int {
	return if w.uia == unsafe { nil } { -1 } else { w.uia.foreign }
}
