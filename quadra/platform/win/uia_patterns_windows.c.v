// UI Automation fragment root and pattern providers of UiaNode (uia_windows.c.v).
module win

import quadra.prim

// --- IRawElementProviderFragmentRoot ---

fn root_from_point(this voidptr, x f64, y f64, out &voidptr) int {
	u := face_node(this).uia
	pt := C.POINT{0, 0}
	C.ScreenToClient(u.w.hwnd, &pt)
	s := f64(u.w.scale_)
	p := prim.pt(f32((x + pt.x) / s), f32((y + pt.y) / s))
	for i := u.nodes.len - 1; i >= 0; i-- { // later nodes are on top
		if u.nodes[i].bounds.contains(p) {
			return u.frag(i, out)
		}
	}
	return give_none(out)
}

fn root_focus(this voidptr, out &voidptr) int {
	u := face_node(this).uia
	for i, d in u.nodes {
		if d.focused {
			return u.frag(i, out)
		}
	}
	return give_none(out)
}

// --- patterns ---

fn put_int(out &int, v bool) int {
	unsafe {
		*out = if v { 1 } else { 0 }
	}
	return 0
}

fn invoke_invoke(this voidptr) int {
	return face_node(this).act(.invoke)
}

fn toggle_toggle(this voidptr) int {
	return face_node(this).act(.toggle)
}

fn toggle_state(this voidptr, out &int) int {
	d := face_node(this).data() or { return uia_e_elementnotavailable }
	return put_int(out, d.checked)
}

fn select_select(this voidptr) int {
	return face_node(this).act(.select)
}

fn select_remove(_ voidptr) int {
	return 0
}

fn select_is_selected(this voidptr, out &int) int {
	d := face_node(this).data() or { return uia_e_elementnotavailable }
	return put_int(out, d.selected)
}

fn select_container(this voidptr, out &voidptr) int {
	n := face_node(this)
	d := n.data() or { return uia_e_elementnotavailable }
	if d.parent < 0 {
		return give_none(out)
	}
	mut u := n.uia
	return give(out, &u.node(u.nodes[d.parent].id).simple)
}

fn expand_expand(this voidptr) int {
	d := face_node(this).data() or { return uia_e_elementnotavailable }
	return if d.expanded { 0 } else { face_node(this).act(.expand) }
}

fn expand_collapse(this voidptr) int {
	d := face_node(this).data() or { return uia_e_elementnotavailable }
	return if d.expanded { face_node(this).act(.collapse) } else { 0 }
}

fn expand_state(this voidptr, out &int) int {
	d := face_node(this).data() or { return uia_e_elementnotavailable }
	return put_int(out, d.expanded)
}

fn value_set(_ voidptr, _ &u16) int {
	return e_notimpl
}

fn value_get(this voidptr, out &voidptr) int {
	n := face_node(this)
	d := n.data() or { return uia_e_elementnotavailable }
	unsafe {
		*out = voidptr(n.bstr(d.value))
	}
	return 0
}

fn value_read_only(_ voidptr, out &int) int {
	return put_int(out, true)
}
