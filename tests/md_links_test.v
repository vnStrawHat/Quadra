// Links in the Markdown view (specs/21, Links): every link opens on ctrl-click only, URLs in
// fenced and inline code are links, and a plain click selects instead.
module main

import quadra.core
import quadra.kit
import quadra.kit.md
import quadra.platform
import quadra.testing
import quadra.ui

struct MdHost {
mut:
	links []string
}

const md_src = '[docs](https://a.example/docs)\n\nRun `see https://c.example/x` now.\n\n```bash\ncurl -fsSL https://b.example/install.sh | sh\n```\n\n[top](#intro)\n'

@[markused]
pub fn (mut m MdHost) render(mut cx core.ViewCx[MdHost]) core.Element {
	return ui.div().size_full().p(16).child(md.view(md_src).on_link(cx.listener(fn (mut m MdHost, url &string, mut _ core.ViewCx[MdHost]) {
		m.links << *url
	})))
}

fn open_md() !(&testing.Harness, &MdHost) {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(MdHost{}, width: 800, height: 600)!
	cx := core.Cx{
		app: h.app
	}
	return h, cx.peek(v.model())
}

// link_at is the middle of the k-th link's hitbox: 0 the [docs] link, 1 the URL in inline
// code, 2 the one in the fence (its tokens are one hitbox), the last the anchor.
fn link_at(h &testing.Harness, k int) !(f32, f32) {
	links := h.window().hitboxes.filter(it.cursor == .pointer)
	if k >= links.len {
		return error('${links.len} links')
	}
	r := links[k].bounds
	return r.x + r.width / 2, r.y + r.height / 2
}

fn test_a_plain_click_on_a_link_selects_and_ctrl_click_opens() {
	mut h, m := open_md()!
	x, y := link_at(h, 0)!
	h.move_to(x, y)
	assert h.cursor() != .pointer // the pointer needs ctrl
	h.inject(platform.KeyEvent{ key: .ctrl, mods: .ctrl, down: true })
	assert h.cursor() == .pointer
	h.inject(platform.KeyEvent{ key: .ctrl, mods: platform.Mods.zero(), down: false })
	h.click_at(x, y)
	assert m.links.len == 0
	h.click_mods(x, y, .ctrl)
	assert m.links == ['https://a.example/docs']
}

fn test_urls_in_code_open_on_ctrl_click() {
	mut h, m := open_md()!
	x, y := link_at(h, 2)!
	h.click_at(x, y)
	assert m.links.len == 0
	h.click_mods(x, y, .ctrl)
	assert m.links == ['https://b.example/install.sh']
	// Inline code: the URL inside the backticks is a link too.
	ix, iy := link_at(h, 1)!
	h.click_mods(ix, iy, .ctrl)
	assert m.links.last() == 'https://c.example/x'
}

fn test_resting_on_a_link_shows_a_hint_until_a_click_or_a_wheel() {
	mut h, _ := open_md()!
	x, y := link_at(h, 0)!
	h.move_to(x, y)
	assert !h.has_text(core.hint_ctrl_open)
	h.advance(core.hint_delay_ms + 20)
	assert h.has_text(core.hint_ctrl_open)
	h.click_at(x, y)
	assert !h.has_text(core.hint_ctrl_open)
	// An anchor is followed, not opened.
	ax, ay := link_at(h, h.window().hitboxes.filter(it.cursor == .pointer).len - 1)!
	h.move_to(ax, ay)
	h.advance(core.hint_delay_ms + 20)
	assert h.has_text(core.hint_ctrl_follow) && !h.has_text(core.hint_ctrl_open)
	h.scroll_at(ax, ay, 40)
	assert !h.has_text(core.hint_ctrl_follow)
	h.inject(platform.MouseEvent{ action: .leave })
	h.move_to(x, y)
	h.advance(core.hint_delay_ms + 20)
	assert h.has_text(core.hint_ctrl_open)
	h.inject(platform.MouseEvent{ action: .leave })
	assert !h.has_text(core.hint_ctrl_open)
}
