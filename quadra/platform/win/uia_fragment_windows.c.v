// UI Automation fragment navigation of UiaNode (uia_windows.c.v).
module win

import quadra.prim

// --- IRawElementProviderFragment ---

fn (u &Uia) frag(i int, out &voidptr) int {
	if i < 0 {
		return give_none(out)
	}
	mut uu := unsafe { u }
	return give(out, &uu.node(u.nodes[i].id).fragment)
}

fn frag_navigate(this voidptr, dir int, out &voidptr) int {
	n := face_node(this)
	u := n.uia
	if n.id == 0 {
		return match dir {
			3 { u.frag(u.first[u.nodes.len], out) }
			4 { u.frag(u.last[u.nodes.len], out) }
			else { give_none(out) }
		}
	}
	i := u.index[n.id] or { return uia_e_elementnotavailable }
	p := u.nodes[i].parent
	return match dir {
		0 {
			if p >= 0 {
				u.frag(p, out)
			} else {
				give(out, &u.root.fragment)
			}
		}
		1 {
			u.frag(u.next[i], out)
		}
		2 {
			u.frag(u.prev[i], out)
		}
		3 {
			u.frag(u.first[i], out)
		}
		4 {
			u.frag(u.last[i], out)
		}
		else {
			give_none(out)
		}
	}
}

fn frag_runtime_id(this voidptr, out &voidptr) int {
	n := face_node(this)
	if n.id == 0 {
		return give_none(out)
	}
	sa := n.uia.api.array(vt_i4, 0, 3)
	parts := [3, int(u32(n.id)), int(u32(n.id >> 32))] // 3: UiaAppendRuntimeId
	for k in 0 .. 3 {
		at := k
		n.uia.api.array_put(sa, &at, &parts[k])
	}
	unsafe {
		*out = sa
	}
	return 0
}

// screen converts a logical client rect to screen pixels.
fn (w &WinWindow) screen(r prim.Rect) UiaRect {
	pt := C.POINT{0, 0}
	C.ScreenToClient(w.hwnd, &pt) // the client origin, negated
	s := f64(w.scale_)
	return UiaRect{f64(r.x) * s - pt.x, f64(r.y) * s - pt.y, f64(r.width) * s, f64(r.height) * s}
}

fn frag_rect(this voidptr, out &UiaRect) int {
	n := face_node(this)
	mut r := unsafe { &UiaRect(out) }
	d := n.data() or {
		unsafe {
			*r = UiaRect{}
		}
		return 0
	}
	unsafe {
		*r = n.uia.w.screen(d.bounds)
	}
	return 0
}

fn frag_embedded(_ voidptr, out &voidptr) int {
	return give_none(out)
}

fn frag_set_focus(this voidptr) int {
	return face_node(this).act(.focus)
}

fn frag_root(this voidptr, out &voidptr) int {
	return give(out, &face_node(this).uia.root.root)
}
