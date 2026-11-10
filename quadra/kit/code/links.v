// Links: URLs in the text open on ctrl-click (specs/18, Mouse). They are found on the line
// under the pointer as it moves (text.find_urls), never for the whole document or per frame.
module code

import math
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.kit
import quadra.platform
import quadra.text

// LinkHover is the URL under the pointer: bytes a..b of line n at a document version.
struct LinkHover {
	line    int = -1
	a       int
	b       int
	url     string
	version int
}

// on_link calls l with a URL ctrl-clicked in the text. Without it the editor opens the URL in
// the browser (find_urls gives http, https and mailto URLs only).
pub fn on_link(mut cx core.Cx, ed core.View[Editor], l core.Listener[string]) {
	mut w := cx.peek(ed.model())
	w.link = l.any
}

// link_rects are the row pieces of line n's bytes a..b, in window coordinates.
fn (e &Editor) link_rects(ts &text.TextSystem, n int, a int, b int) []prim.Rect {
	d := e.doc
	dl := display_line(d.buf.line(n), d.tab_size)
	mut sys := unsafe { ts }
	l := sys.layout_mono(dl.shown, e.style, e.cell.width)
	s0, s1 := dl.to_shown(a), dl.to_shown(b)
	mut out := []prim.Rect{}
	for r in e.first_row(n) .. e.first_row(n + 1) {
		ra, rb := e.row_span(n, r)
		x0, x1 := math.max(s0, ra), math.min(s1, math.min(rb, dl.shown.len))
		if x0 >= x1 {
			continue
		}
		left := e.bounds.x - e.scroll.x - l.x_for_index(ra) + l.x_for_index(x0)
		out << prim.rect(left, e.row_y(r), l.x_for_index(x1) - l.x_for_index(x0), e.cell.height)
	}
	return out
}

// link_under is the URL at window point (x, y).
fn (mut e Editor) link_under(ts &text.TextSystem, x f32, y f32) ?LinkHover {
	if x < e.bounds.x || e.cell.height <= 0 {
		return none
	}
	e.sync_rows()
	n := e.line_at(y)
	p := prim.pt(x, y)
	for u in text.find_urls(e.doc.buf.line(n)) {
		for r in e.link_rects(ts, n, u.start, u.end) {
			if r.contains(p) {
				return LinkHover{n, u.start, u.end, u.url, e.doc.version}
			}
		}
	}
	return none
}

// hover_link follows the pointer when no button is held; the view changes only when the URL
// under it does.
fn (mut e Editor) hover_link(x f32, y f32, mut cx core.ViewCx[Editor]) {
	h := e.link_under(cx.app.text, x, y) or { LinkHover{} }
	if h != e.hover {
		e.hover = h
		cx.notify()
	}
}

// ctrl_click opens the URL under a ctrl-click without touching the cursors; false when there
// is none (ctrl-click then adds a cursor). The second click of a double click opens nothing.
fn (mut e Editor) ctrl_click(ev &core.MouseDownEvent, mut cx core.ViewCx[Editor]) bool {
	h := e.link_under(cx.app.text, ev.x, ev.y) or { return false }
	if ev.clicks == 1 {
		url := h.url
		if l := e.link {
			cx.app.invoke(l, &url, cx.win, true)
		} else {
			cx.open_external(url)
		}
	}
	cx.stop_propagation()
	return true
}

// link_hitboxes give the hovered URL the pointer while ctrl is held (core cursor_mods) and
// the hint "Ctrl+click to open".
fn (mut e Editor) link_hitboxes(mut cx core.PrepaintCx) {
	e.link_hbs.clear()
	if e.hover.line < 0 || e.hover.version != e.doc.version
		|| e.hover.line >= e.doc.buf.line_count() {
		return
	}
	for k, r in e.link_rects(cx.text, e.hover.line, e.hover.a, e.hover.b) {
		cx.enter('link${k}')
		e.link_hbs << cx.insert_hitbox(r.intersect(e.bounds), core.HitboxOptions{
			cursor:         platform.Cursor.pointer
			cursor_mods:    platform.Mods.ctrl
			hint:           core.hint_ctrl_open
			hover_prepaint: true
		})
		cx.leave()
	}
}

// paint_link underlines the hovered URL while ctrl is held.
fn (e &Editor) paint_link(th &kit.Theme, mut cx core.PaintCx) {
	if e.link_hbs.len == 0 || !cx.win.mods.has(.ctrl) || !e.link_hbs.any(cx.is_hovered(it)) {
		return
	}
	pos, thick := cx.text.underline(e.style)
	for r in e.link_rects(cx.text, e.hover.line, e.hover.a, e.hover.b) {
		cx.quad(gpu.Quad{
			bounds:     prim.rect(r.x, r.y + e.base + pos, r.width, prim.max(1, thick))
			background: th.p.accent_text
		})
	}
}
