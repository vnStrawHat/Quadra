// Drag and drop between elements (specs/20). A drag source arms on a left mouse down,
// starts once the pointer moves 4 px, shows its preview under the pointer, and ends on the
// release: over a drop target that accepts its kind, the target's drop handlers run;
// anywhere else, or on escape, nothing happens. The payload is a kind and a value (an id
// the app resolves), so no closure outlives the drag.
module core

import quadra.prim
import quadra.layout

pub struct DragPayload {
pub:
	kind  string // drop targets accept one kind
	value string
}

// DropEvent reaches the target's drop handlers; x and y are in window coordinates.
pub struct DropEvent {
pub:
	payload DragPayload
	x       f32
	y       f32
	bounds  prim.Rect // the target's bounds
}

pub type PreviewFn = fn () Element

// DragSource is what a draggable element registers with its hitbox.
pub struct DragSource {
pub:
	payload DragPayload
	preview PreviewFn = unsafe { nil } // nil: no preview
	// preview_el is a ready preview, used when preview is nil (no closure: specs/14).
	preview_el ?Element
	// tear_off > 0: the drag starts only once the pointer has moved this far vertically, so
	// horizontal moves stay with the element (tabs reorder inside their strip first).
	tear_off f32
}

const drag_threshold = f32(4)

struct DragState {
mut:
	sources []DragSource // this prepaint's, indexed by Hitbox.drag
	armed   u64          // hitbox id of the source under the press, 0 = none
	start   prim.Point
	active  bool
	payload DragPayload
	preview ?Element
	pos     prim.Point
	over    u64 // hitbox id of the accepting target under the pointer, 0 = none
}

// dragging returns the payload of the drag in progress, if any (sources dim themselves).
pub fn (w &Window) dragging() ?DragPayload {
	return if w.drag.active { w.drag.payload } else { none }
}

// is_drop_target reports whether hitbox i is the target under an accepted drag.
pub fn (cx &PaintCx) is_drop_target(i int) bool {
	w := cx.win
	return i >= 0 && i < w.hitboxes.len && w.drag.active && w.drag.over == w.hitboxes[i].id
}

// arm_drag remembers the deepest drag source under a left press.
fn (mut w Window) arm_drag(path []int, p prim.Point) {
	w.drag.armed = 0
	for i in path {
		if w.hitboxes[i].drag >= 0 {
			w.drag.armed = w.hitboxes[i].id
			w.drag.start = p
			return
		}
	}
}

// armed_source finds the armed source in the latest prepaint. Sources are looked up again
// rather than kept from the press: their preview closures belong to the render that made
// them, and a click that re-renders frees those.
fn (w &Window) armed_source() ?DragSource {
	if w.drag.armed == 0 {
		return none
	}
	for hb in w.hitboxes {
		if hb.id == w.drag.armed && hb.drag >= 0 {
			return w.drag.sources[hb.drag]
		}
	}
	return none
}

// drag_move starts or moves a drag; it returns true while a drag is active (moves are not
// dispatched to handlers then).
fn (mut w Window) drag_move(p prim.Point) bool {
	if !w.drag.active {
		src := w.armed_source() or { return false }
		dx := prim.max(p.x - w.drag.start.x, w.drag.start.x - p.x)
		dy := prim.max(p.y - w.drag.start.y, w.drag.start.y - p.y)
		started := if src.tear_off > 0 {
			dy >= src.tear_off
		} else {
			prim.max(dx, dy) >= drag_threshold
		}
		if w.down_path.len == 0 || !started {
			return false
		}
		w.drag.active = true
		w.drag.armed = 0
		w.drag.payload = src.payload
		w.drag.preview = src.preview_el
		if src.preview != unsafe { nil } {
			w.drag.preview = src.preview()
		}
		if w.drag.preview != none {
			// The preview (or the closure making it) comes from a render and is kept for the
			// whole drag: pin the renders, so neither its closures nor its elements (arena.v)
			// are reused.
			mut app := w.app
			app.escaped()
		}
		w.capture = 0
		w.down_path = []u64{} // a drag is not a click
		w.pressed = map[u64]bool{}
	}
	w.drag.pos = p
	w.drag.over = 0
	for i in w.hit_path(p) {
		if w.hitboxes[i].drop_kind == w.drag.payload.kind {
			w.drag.over = w.hitboxes[i].id
			break
		}
	}
	w.invalidate_prepaint() // the preview follows the pointer
	return true
}

// drag_release ends a drag on the mouse up; it returns false when no drag was active.
fn (mut w Window) drag_release(p prim.Point) bool {
	w.drag.armed = 0
	if !w.drag.active {
		return false
	}
	w.drag_move(p)
	over := w.drag.over
	payload := w.drag.payload
	w.end_drag()
	for i, hb in w.hitboxes {
		if hb.id == over {
			ev := DropEvent{payload, p.x, p.y, hb.bounds}
			w.dispatch(.drop, [i], &ev, true)
			break
		}
	}
	return true
}

fn (mut w Window) end_drag() {
	w.drag.active = false
	w.drag.preview = none
	w.drag.over = 0
	w.invalidate_prepaint()
}

// cancel_drag stops a drag without dropping (escape); it returns false when none was active.
fn (mut w Window) cancel_drag() bool {
	if !w.drag.active {
		return false
	}
	w.end_drag()
	return true
}

// prepaint_drag lays out the preview next to the pointer, above every overlay.
fn (mut w Window) prepaint_drag(mut cx PrepaintCx) {
	mut el := w.drag.preview or { return }
	mut lcx := cx.layout_cx()
	node := el.request_layout(mut lcx)
	w.tree.compute_fit(node, w.pw.size())
	sz := w.tree.rect(node).size()
	win := w.pw.size() // kept inside the window near its edges
	x := prim.max(0, prim.min(w.drag.pos.x + 12, win.width - sz.width))
	y := prim.max(0, prim.min(w.drag.pos.y + 8, win.height - sz.height))
	w.defer_draw(&DragPreview{el, node}, prim.rect(x, y, sz.width, sz.height), 1_000_000)
}

// DragPreview paints a drag's preview translucent. It is never prepainted, so it has no
// hitboxes: the pointer finds the targets under it.
struct DragPreview {
mut:
	el   Element
	node layout.NodeId
}

fn (mut d DragPreview) request_layout(mut _ LayoutCx) layout.NodeId {
	return d.node
}

fn (mut d DragPreview) prepaint(_ prim.Rect, mut _ PrepaintCx) {}

fn (mut d DragPreview) paint(bounds prim.Rect, mut cx PaintCx) {
	saved := cx.opacity
	cx.opacity *= 0.85
	d.el.paint(bounds, mut cx)
	cx.opacity = saved
}
