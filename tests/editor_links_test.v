// URLs in the code editor (specs/18, Mouse): ctrl-hover shows the pointer, ctrl-click hands
// the URL to on_link without touching the cursors, a plain click places the cursor.
module main

import quadra.core
import quadra.kit
import quadra.kit.code
import quadra.platform
import quadra.testing
import quadra.ui

struct LinkHost {
mut:
	ed    core.View[code.Editor]
	text  string
	hook  bool = true
	links []string
}

@[markused]
pub fn (mut h LinkHost) render(mut cx core.ViewCx[LinkHost]) core.Element {
	if h.ed.id == 0 {
		h.ed = code.new_editor(mut cx.Cx, id: 'ed', text: h.text, path: 'run.sh')
		if h.hook {
			code.on_link(mut cx.Cx, h.ed, cx.listener(fn (mut h LinkHost, url &string, mut _ core.ViewCx[LinkHost]) {
				h.links << *url
			}))
		}
	}
	return ui.div().size_full().child(cx.child_view(h.ed))
}

fn open_links(text string, hook bool) !(&testing.Harness, &LinkHost, &code.Editor) {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(LinkHost{ text: text, hook: hook },
		width:  800
		height: 400
	)!
	mut cx := core.Cx{
		app: h.app
	}
	host := cx.peek(v.model())
	return h, host, cx.peek(host.ed.model())
}

// pt is the window point of line n, column col.
fn pt(e &code.Editor, n int, col int) (f32, f32) {
	return e.bounds.x + f32(col) * e.cell.width + 1, e.bounds.y + f32(n) * e.cell.height -
		e.scroll.y + e.cell.height / 2
}

const sample = 'echo hi\ncurl -L https://example.com/a_(b) | sh\nend'

fn test_ctrl_click_on_a_url_calls_the_hook_and_keeps_the_cursor() {
	mut h, host, e := open_links(sample, true)!
	x, y := pt(e, 1, 12)
	h.click_mods(x, y, .ctrl)
	assert host.links == ['https://example.com/a_(b)']
	assert e.doc.cursors.len == 1 && e.doc.primary().head == 0 // neither moved nor added
	// Ctrl-click off a URL still adds a cursor.
	ox, oy := pt(e, 0, 3)
	h.click_mods(ox, oy, .ctrl)
	assert e.doc.cursors.len == 2 && host.links.len == 1
}

fn test_a_plain_click_on_a_url_places_the_cursor() {
	mut h, host, e := open_links(sample, true)!
	x, y := pt(e, 1, 12)
	h.click_at(x, y)
	assert host.links.len == 0
	assert e.doc.cursors.len == 1 && e.doc.primary().head == 8 + 12 // line 1 starts at 8
	assert h.plat.opened.len == 0
}

fn test_the_pointer_shows_only_while_ctrl_is_held() {
	mut h, _, e := open_links(sample, true)!
	x, y := pt(e, 1, 12)
	h.move_to(x, y)
	assert h.cursor() == .ibeam
	h.inject(platform.KeyEvent{ key: .ctrl, mods: .ctrl, down: true })
	assert h.cursor() == .pointer // no mouse move needed
	h.inject(platform.KeyEvent{ key: .ctrl, mods: platform.Mods.zero(), down: false })
	assert h.cursor() == .ibeam
	h.inject(platform.MouseEvent{ action: .move, x: x, y: y, mods: .ctrl })
	assert h.cursor() == .pointer
	// Off the URL (the `| sh` after it) the editor keeps its text cursor.
	sx, sy := pt(e, 1, 36)
	h.inject(platform.MouseEvent{ action: .move, x: sx, y: sy, mods: .ctrl })
	assert h.cursor() == .ibeam
}

fn test_without_a_hook_the_url_opens_in_the_browser() {
	mut h, _, e := open_links(sample, false)!
	x, y := pt(e, 1, 12)
	h.click_mods(x, y, .ctrl)
	assert h.plat.opened == ['https://example.com/a_(b)']
}

fn test_resting_on_a_url_shows_the_hint_until_a_click_or_leave() {
	mut h, _, e := open_links(sample, true)!
	x, y := pt(e, 1, 12)
	h.move_to(x, y)
	assert !h.has_text(core.hint_ctrl_open)
	h.advance(core.hint_delay_ms + 20)
	assert h.has_text(core.hint_ctrl_open)
	h.click_at(x, y)
	assert !h.has_text(core.hint_ctrl_open)
	ox, oy := pt(e, 0, 3) // off the URL, then back
	h.move_to(ox, oy)
	h.move_to(x, y)
	h.advance(core.hint_delay_ms + 20)
	assert h.has_text(core.hint_ctrl_open)
	h.inject(platform.MouseEvent{ action: .leave })
	assert !h.has_text(core.hint_ctrl_open)
}
