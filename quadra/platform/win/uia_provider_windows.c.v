// The UI Automation interfaces of UiaNode (uia_windows.c.v). Slots from tools/com_slots.sh
// IRawElementProviderSimple, IRawElementProviderFragment, IRawElementProviderFragmentRoot,
// and the pattern providers.
module win

import quadra.platform

struct Variant {
mut:
	vt u16
	r1 u16
	r2 u16
	r3 u16
	v1 u64
	v2 u64
}

struct UiaRect {
mut:
	left   f64
	top    f64
	width  f64
	height f64
}

const vt_i4 = u16(3)
const vt_bstr = u16(8)
const vt_bool = u16(11)

fn uia_table(methods []voidptr) &voidptr {
	mut all := [voidptr(uia_qi), voidptr(uia_add_ref), voidptr(uia_release)]
	all << methods
	mut table := unsafe { &voidptr(malloc_noscan(all.len * int(sizeof(voidptr)))) }
	for i, m in all {
		unsafe {
			table[i] = m
		}
	}
	return table
}

// uia_vtables in the order of UiaNode's faces.
fn uia_vtables() [8]&voidptr {
	return [
		uia_table([voidptr(simple_options), voidptr(simple_pattern), voidptr(simple_property),
			voidptr(simple_host)]),
		uia_table([voidptr(frag_navigate), voidptr(frag_runtime_id), voidptr(frag_rect),
			voidptr(frag_embedded), voidptr(frag_set_focus), voidptr(frag_root)]),
		uia_table([voidptr(root_from_point), voidptr(root_focus)]),
		uia_table([voidptr(invoke_invoke)]),
		uia_table([voidptr(toggle_toggle), voidptr(toggle_state)]),
		uia_table([voidptr(select_select), voidptr(select_select), voidptr(select_remove),
			voidptr(select_is_selected), voidptr(select_container)]),
		uia_table([voidptr(expand_expand), voidptr(expand_collapse), voidptr(expand_state)]),
		uia_table([voidptr(value_set), voidptr(value_get), voidptr(value_read_only)]),
	]!
}

@[inline]
fn face_node(this voidptr) &UiaNode {
	f := unsafe { &Face(this) }
	mut n := unsafe { &UiaNode(voidptr(f.node)) }
	if C.GetCurrentThreadId() != n.uia.tid {
		n.uia.foreign++ // must stay 0: UseComThreading brings calls to the UI thread
	}
	return n
}

fn (n &UiaNode) data() ?platform.A11yNode {
	i := n.uia.index[n.id] or { return none }
	return n.uia.nodes[i]
}

// pattern_face returns the face for a pattern the node's role supports.
fn (n &UiaNode) pattern_face(id int) ?&Face {
	d := n.data() or { return none }
	ok := match id {
		10000 { d.role in [.button, .link, .menuitem] }
		10015 { d.role in [.checkbox, .switch] }
		10010 { d.role in [.tab, .radio, .listitem, .treeitem, .row] }
		10005 { d.role == .treeitem }
		10002 { d.role in [.textbox, .slider, .progressbar] }
		else { false }
	}

	if !ok {
		return none
	}
	return match id {
		10000 { &n.invoke }
		10015 { &n.toggle }
		10010 { &n.select }
		10005 { &n.expand }
		else { &n.value }
	}
}

fn give(out &voidptr, f &Face) int {
	mut n := f.node
	n.refs++
	unsafe {
		*out = voidptr(f)
	}
	return 0
}

fn give_none(out &voidptr) int {
	unsafe {
		*out = nil
	}
	return 0
}

const pattern_iids = [iid_invoke, iid_toggle, iid_selection_item, iid_expand, iid_value]
const pattern_ids = [10000, 10015, 10010, 10005, 10002]

fn uia_qi(this voidptr, riid &Guid, out &voidptr) int {
	n := face_node(this)
	if riid.equals(&iid_iunknown) || riid.equals(&iid_simple) {
		return give(out, &n.simple)
	}
	if riid.equals(&iid_fragment) {
		return give(out, &n.fragment)
	}
	if riid.equals(&iid_fragment_root) && n.id == 0 {
		return give(out, &n.root)
	}
	for k, iid in pattern_iids {
		if riid.equals(&iid) {
			if f := n.pattern_face(pattern_ids[k]) {
				return give(out, f)
			}
		}
	}
	give_none(out)
	return e_nointerface
}

fn uia_add_ref(this voidptr) u32 {
	mut f := unsafe { &Face(this) }
	f.node.refs++
	return f.node.refs
}

fn uia_release(this voidptr) u32 {
	mut f := unsafe { &Face(this) }
	if f.node.refs > 0 {
		f.node.refs--
	}
	return f.node.refs
}

// act posts an action for the app: UIA wants pattern calls to return at once.
fn (n &UiaNode) act(action platform.A11yAction) int {
	if n.data() == none {
		return uia_e_elementnotavailable
	}
	mut w := n.uia.w
	ev := platform.A11yEvent{n.id, action}
	w.plat.post(fn [mut w, ev] () {
		w.emit(ev)
	})
	return 0
}

// --- IRawElementProviderSimple ---

fn simple_options(_ voidptr, out &int) int {
	unsafe {
		*out = 0x21 // ServerSideProvider, UseComThreading: COM brings every call to the UI thread
	}
	return 0
}

fn simple_pattern(this voidptr, id int, out &voidptr) int {
	f := face_node(this).pattern_face(id) or { return give_none(out) }
	return give(out, f)
}

fn control_type(r platform.A11yRole) int {
	return match r {
		.button { 50000 }
		.checkbox, .switch { 50002 }
		.textbox { 50004 }
		.link { 50005 }
		.image { 50006 }
		.listitem { 50007 }
		.listbox { 50008 }
		.menu { 50009 }
		.menuitem { 50011 }
		.progressbar { 50012 }
		.radio { 50013 }
		.slider { 50015 }
		.tablist { 50018 }
		.tab { 50019 }
		.tooltip { 50022 }
		.tree { 50023 }
		.treeitem { 50024 }
		.group { 50026 }
		.row { 50029 }
		.dialog { 50032 }
		.table { 50036 }
		.none, .heading, .cell, .alert { 50020 }
	}
}

fn (n &UiaNode) bstr(s string) u64 {
	return u64(n.uia.api.sys_alloc(s.to_wide()))
}

fn vbool(b bool) u64 {
	return if b { u64(0xFFFF) } else { 0 } // VARIANT_TRUE is -1
}

fn simple_property(this voidptr, id int, out &Variant) int {
	n := face_node(this)
	mut v := unsafe { &Variant(out) }
	unsafe {
		*v = Variant{}
	}
	if id == 30024 { // FrameworkId
		v.vt, v.v1 = vt_bstr, n.bstr('Quadra')
		return 0
	}
	d := n.data() or { return 0 }
	match id {
		30003 { v.vt, v.v1 = vt_i4, u64(control_type(d.role)) }
		30005 { v.vt, v.v1 = vt_bstr, n.bstr(d.name) }
		30011 { v.vt, v.v1 = vt_bstr, n.bstr(d.automation_id) }
		30008 { v.vt, v.v1 = vt_bool, vbool(d.focused) }
		30009 { v.vt, v.v1 = vt_bool, vbool(d.focusable) }
		30010 { v.vt, v.v1 = vt_bool, vbool(!d.disabled) }
		else {}
	}

	return 0
}

fn simple_host(this voidptr, out &voidptr) int {
	n := face_node(this)
	if n.id != 0 {
		return give_none(out)
	}
	return n.uia.api.host(n.uia.w.hwnd, out)
}
