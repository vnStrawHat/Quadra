// Elements and the phase contexts they run in (specs/06-core.md).
module core

import hash.fnv1a
import quadra.prim
import quadra.layout
import quadra.platform
import quadra.text

pub interface Element {
mut:
	request_layout(mut cx LayoutCx) layout.NodeId
	prepaint(bounds prim.Rect, mut cx PrepaintCx)
	paint(bounds prim.Rect, mut cx PaintCx)
}

pub struct LayoutCx {
pub mut:
	app        &App             = unsafe { nil }
	win        &Window          = unsafe { nil }
	tree       &layout.Tree     = unsafe { nil }
	text       &text.TextSystem = unsafe { nil }
	text_style text.TextStyle
}

pub struct Hitbox {
pub:
	id              u64 // stable across frames (path of element keys)
	bounds          prim.Rect
	parent          int
	cursor          platform.Cursor
	has_cursor      bool
	hover_sensitive bool
	hover_prepaint  bool               // hover changes need a prepaint (tooltips)
	focus_id        u64                // focus target activated by clicking here
	region          platform.HitRegion // custom titlebar role (caption, window buttons)
	drag            int = -1 // index of this hitbox's drag source (drag.v), -1 = none
	drop_kind       string // the drag kind this hitbox accepts as a drop target
pub mut:
	handlers_start int
	handlers_end   int
	patch_site     int = -1 // paint patch of this hitbox's element in the last paint
}

pub struct HandlerEntry {
pub:
	kind     HandlerKind
	capture  bool
	listener AnyListener
}

pub struct HitboxOptions {
pub:
	cursor          ?platform.Cursor
	hover_sensitive bool
	hover_prepaint  bool
	focus_id        u64
	region          platform.HitRegion
	drag            ?DragSource // this hitbox starts drags (drag.v)
	drop_kind       string      // this hitbox accepts drops of this kind
}

pub struct PrepaintCx {
pub mut:
	app        &App             = unsafe { nil }
	win        &Window          = unsafe { nil }
	tree       &layout.Tree     = unsafe { nil }
	text       &text.TextSystem = unsafe { nil }
	text_style text.TextStyle
mut:
	ids      []u64 = [u64(0)]
	counters []int = [0]
	hitboxes []int
	keys     []int
	sems     []int
	clips    []prim.Rect = [prim.rect(-1.0e6, -1.0e6, 2.0e6, 2.0e6)]
}

// enter scopes an element: key is an explicit id, or empty for its position among siblings.
pub fn (mut cx PrepaintCx) enter(key string) u64 {
	n := cx.counters.last()
	cx.counters[cx.counters.len - 1] = n + 1
	local := if key == '' { '#${n}' } else { key }
	id := fnv1a.sum64_string('${cx.ids.last()}/${local}')
	cx.ids << id
	cx.counters << 0
	return id
}

pub fn (mut cx PrepaintCx) leave() {
	cx.ids.pop()
	cx.counters.pop()
}

// request_prepaint_after prepaints this window again after ms, without a closure (hover
// delays). Requests coalesce: a caller woken early asks again for the time left.
pub fn (mut cx PrepaintCx) request_prepaint_after(ms int) {
	mut w := cx.win
	if w.prepaint_timer != 0 {
		return
	}
	mut app := cx.app
	w.prepaint_timer = app.plat.set_timer(ms, false)
	app.timers[w.prepaint_timer] = TimerEntry{
		reprepaint: w.id
	}
}

pub fn (cx &PrepaintCx) now_ms() f64 {
	app := cx.app
	return app.plat.now_ms()
}

pub fn (cx &PrepaintCx) is_hovered_id(id u64) bool {
	return cx.win.hovered[id]
}

pub fn (cx &PrepaintCx) current_id() u64 {
	return cx.ids.last()
}

pub fn (cx &PrepaintCx) clip() prim.Rect {
	return cx.clips.last()
}

pub fn (mut cx PrepaintCx) push_clip(r prim.Rect) {
	cx.clips << cx.clips.last().intersect(r)
}

pub fn (mut cx PrepaintCx) pop_clip() {
	cx.clips.pop()
}

// insert_hitbox registers an interactive area for the current element and returns its index.
pub fn (mut cx PrepaintCx) insert_hitbox(bounds prim.Rect, opts HitboxOptions) int {
	parent := if cx.hitboxes.len > 0 { cx.hitboxes.last() } else { -1 }
	mut w := cx.win
	mut drag := -1
	if src := opts.drag {
		w.drag.sources << src
		drag = w.drag.sources.len - 1
	}
	w.hitboxes << Hitbox{
		id:              cx.current_id()
		bounds:          bounds.intersect(cx.clip())
		parent:          parent
		cursor:          opts.cursor or { platform.Cursor.arrow }
		has_cursor:      opts.cursor != none
		hover_sensitive: opts.hover_sensitive
		focus_id:        opts.focus_id
		hover_prepaint:  opts.hover_prepaint
		region:          opts.region
		drag:            drag
		drop_kind:       opts.drop_kind
		handlers_start:  w.handlers.len
		handlers_end:    w.handlers.len
	}
	return w.hitboxes.len - 1
}

// on_mouse adds a handler to the most recently inserted hitbox.
pub fn (mut cx PrepaintCx) on_mouse(hitbox int, kind HandlerKind, l AnyListener, capture bool) {
	mut w := cx.win
	w.handlers << HandlerEntry{kind, capture, l}
	w.hitboxes[hitbox].handlers_end = w.handlers.len
}

pub fn (mut cx PrepaintCx) push_hitbox(i int) {
	cx.hitboxes << i
}

pub fn (mut cx PrepaintCx) pop_hitbox() {
	cx.hitboxes.pop()
}

// insert_key_node adds a node to the key-dispatch tree (focus target, key context, key handlers).
pub fn (mut cx PrepaintCx) insert_key_node(focus_id u64, context string, tab_index int, bounds prim.Rect) int {
	parent := if cx.keys.len > 0 { cx.keys.last() } else { -1 }
	mut w := cx.win
	w.key_nodes << KeyNode{
		parent:         parent
		focus_id:       focus_id
		context:        context
		tab_index:      tab_index
		bounds:         bounds
		handlers_start: w.key_handlers.len
		handlers_end:   w.key_handlers.len
	}
	return w.key_nodes.len - 1
}

pub fn (mut cx PrepaintCx) on_key(node int, kind KeyHandlerKind, action_type string, l AnyListener) {
	mut w := cx.win
	w.key_handlers << KeyHandler{kind, action_type, l}
	w.key_nodes[node].handlers_end = w.key_handlers.len
}

pub fn (mut cx PrepaintCx) push_key_node(i int) {
	cx.keys << i
}

pub fn (mut cx PrepaintCx) pop_key_node() {
	cx.keys.pop()
}

pub fn (mut cx PrepaintCx) defer_draw(el Element, bounds prim.Rect, priority int) {
	mut w := cx.win
	w.defer_draw(el, bounds, priority)
}

// layout_cx makes a layout context for laying out an extra root (overlays) during prepaint.
pub fn (cx &PrepaintCx) layout_cx() LayoutCx {
	return LayoutCx{
		app:        cx.app
		win:        cx.win
		tree:       cx.tree
		text:       cx.text
		text_style: cx.text_style
	}
}

pub fn (mut cx PrepaintCx) register_test_id(id string, bounds prim.Rect) {
	cx.win.test_ids[id] = bounds
}

pub fn (mut cx PrepaintCx) register_text(s string, bounds prim.Rect) {
	cx.win.texts << TextEntry{s, bounds}
}

pub struct TextEntry {
pub:
	text   string
	bounds prim.Rect
}

// ViewElement places a child view; it renders the view when laid out.
pub struct ViewElement {
	view AnyView
mut:
	inner ?Element
	fresh bool
}

pub fn (mut e ViewElement) request_layout(mut cx LayoutCx) layout.NodeId {
	mut app := cx.app
	before := cx.win.stats.renders[e.view.id]
	mut el := app.render_any(e.view.id, cx.win)
	e.fresh = cx.win.stats.renders[e.view.id] != before
	e.inner = el
	return el.request_layout(mut cx)
}

pub fn (mut e ViewElement) prepaint(bounds prim.Rect, mut cx PrepaintCx) {
	if e.fresh {
		mut w := cx.win
		w.fresh_views << bounds
	}
	cx.enter('v${e.view.id}')
	if mut el := e.inner {
		el.prepaint(bounds, mut cx)
	}
	cx.leave()
}

pub fn (mut e ViewElement) paint(bounds prim.Rect, mut cx PaintCx) {
	if mut el := e.inner {
		el.paint(bounds, mut cx)
	}
}
