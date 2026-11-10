// Terminal links in the harness (specs/22, Links): the pointer and the underline need ctrl, as
// in the editor and the Markdown preview, and resting on a link shows "Ctrl+click to open".
module main

import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.platform
import quadra.testing
import quadra.ui

struct LinkTermHost {
mut:
	term core.View[term.Terminal]
	fake &term.FakePty = unsafe { nil }
}

@[markused]
pub fn (mut h LinkTermHost) render(mut cx core.ViewCx[LinkTermHost]) core.Element {
	if h.term.id == 0 {
		h.term = term.new_terminal(mut cx.Cx, id: 't')
		h.fake = &term.FakePty{
			prompt: '$ '
		}
		term.start(mut cx.Cx, h.term, h.fake.spawner(), platform.PtyOptions{}) or { panic(err) }
	}
	return ui.div().size_full().child(cx.child_view(h.term))
}

// open_term shows `$ see https://w.org/Foo_(bar) now`: the URL is columns 6..29 of row 0.
fn open_term() !(&testing.Harness, &term.Terminal) {
	h, _, t := open_term_host()!
	return h, t
}

fn open_term_host() !(&testing.Harness, &term.FakePty, &term.Terminal) {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(LinkTermHost{}, width: 640, height: 400)!
	mut cx := core.Cx{
		app: h.app
	}
	host := cx.peek(v.model())
	mut fake := host.fake
	fake.output('see https://w.org/Foo_(bar) now\r\n')
	h.frame()
	return h, fake, cx.peek(host.term.model())
}

fn cell_pt(t &term.Terminal, col int) (f32, f32) {
	return t.bounds.x + t.cell.width * (f32(col) + 0.5), t.bounds.y + t.cell.height * 0.5
}

const ctrl_down = platform.KeyEvent{
	key:  .ctrl
	mods: .ctrl
	down: true
}

fn test_the_pointer_shows_only_while_ctrl_is_held() {
	mut h, t := open_term()!
	x, y := cell_pt(t, 10)
	h.move_to(x, y)
	assert t.hover.url == 'https://w.org/Foo_(bar)' // the balanced `)` stays
	assert h.cursor() == .ibeam
	h.inject(ctrl_down)
	assert h.cursor() == .pointer // no mouse move needed
	h.inject(platform.KeyEvent{ key: .ctrl, mods: platform.Mods.zero(), down: false })
	assert h.cursor() == .ibeam
	h.inject(platform.MouseEvent{ action: .move, x: x, y: y, mods: .ctrl })
	assert h.cursor() == .pointer
	// Off the link (the `now` after it) the text cursor stays.
	nx, ny := cell_pt(t, 32)
	h.inject(platform.MouseEvent{ action: .move, x: nx, y: ny, mods: .ctrl })
	assert h.cursor() == .ibeam
}

fn test_resting_on_a_link_shows_the_hint_until_a_click_or_leave() {
	mut h, t := open_term()!
	x, y := cell_pt(t, 10)
	h.move_to(x, y)
	assert !h.has_text(core.hint_ctrl_open)
	h.advance(core.hint_delay_ms + 20)
	assert h.has_text(core.hint_ctrl_open)
	h.inject(ctrl_down) // the hint stays while ctrl is held
	assert h.has_text(core.hint_ctrl_open)
	h.inject(platform.KeyEvent{ key: .ctrl, mods: platform.Mods.zero(), down: false })
	h.click_at(x, y)
	assert !h.has_text(core.hint_ctrl_open)
	h.advance(core.hint_delay_ms + 20) // closed until the pointer leaves
	assert !h.has_text(core.hint_ctrl_open)
	h.inject(platform.MouseEvent{ action: .leave })
	h.move_to(x, y)
	h.advance(core.hint_delay_ms + 20)
	assert h.has_text(core.hint_ctrl_open)
	h.inject(platform.MouseEvent{ action: .leave })
	assert !h.has_text(core.hint_ctrl_open)
}

fn test_a_program_reading_the_mouse_gets_no_hint() {
	mut h, mut fake, t := open_term_host()!
	fake.output('\x1b[?1000h') // mode 1000: the program reads clicks
	h.frame()
	assert t.screen.mouse != .off
	x, y := cell_pt(t, 10)
	h.move_to(x, y)
	h.advance(core.hint_delay_ms + 20)
	assert !h.has_text(core.hint_ctrl_open)
}
