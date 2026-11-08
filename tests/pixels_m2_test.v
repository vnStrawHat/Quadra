// Visual check of M2 features on real pixels (WARP + DirectWrite).
module main

import os
import stbi
import quadra.core
import quadra.prim
import quadra.testing
import quadra.ui
import quadra.vector

const search_icon = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>'
const heart_icon = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/></svg>'
const badge_svg = '<svg viewBox="0 0 40 40"><circle cx="20" cy="20" r="18" fill="#16A34A"/><path d="M12 20l6 6 10-12" fill="none" stroke="#ffffff" stroke-width="4"/></svg>'

struct Showcase {
	image string
mut:
	focus core.FocusHandle
}

@[markused]
pub fn (mut s Showcase) render(mut cx core.ViewCx[Showcase]) core.Element {
	if s.focus.id == 0 {
		s.focus = cx.focus_handle()
	}
	chart := ui.canvas(fn (mut p ui.Painter, size prim.Size) {
		vals := [f32(12), 30, 22, 40, 34, 52, 46]
		for i, v in vals {
			p.fill_rect(prim.rect(f32(i) * 34 + 8, size.height - v * 2, 24, v * 2),
				prim.rgb(0xBFDBFE), 4)
		}
		mut line := vector.new_path()
		for i, v in vals {
			x, y := f32(i) * 34 + 20, size.height - v * 2
			if i == 0 {
				line.move_to(x, y)
			} else {
				line.line_to(x, y)
			}
		}
		p.stroke_path(line, 2.5, prim.rgb(0x2563EB))
	}).size(250, 120)
	mut list :=
		ui.div().flex_col().h(110).w(220).overflow_y_scroll().border(1).border_color(prim.rgb(0xE5E7EB)).rounded(8)
	for i in 0 .. 12 {
		list.child(ui.div().px(10).py(6).child(ui.text('Mục số ${i + 1}')))
	}
	return ui.div().size_full().p(24).gap(24).bg(prim.rgb(0xF9FAFB)).flex_row().child(ui.div().flex_col().gap(16).child(ui.div().flex_row().gap(12).items_center().text_color(prim.rgb(0x374151)).child(ui.icon(search_icon).size(24)).child(ui.icon(heart_icon).size(24).color(prim.rgb(0xDC2626))).child(ui.svg(badge_svg).size(32)).child(ui.text('Icons'))).child(chart).child(ui.div().test_id('btn').track_focus(s.focus).px(14).py(8).rounded(6).bg(prim.rgb(0x2563EB)).text_color(prim.white).child(ui.text('Focus ring')))).child(ui.div().flex_col().gap(16).child(list).child(ui.img(s.image).size(120,
		80).fit(.cover).rounded(10)).child(ui.anchored(ui.div().p(10).bg(prim.white).rounded(8).border(1).border_color(prim.rgb(0xE5E7EB)).shadow_level(3).child(ui.text('Popover overlay'))).at(prim.rect(300,
		330, 80, 20))))
}

fn test_m2_visuals() {
	out := os.join_path(os.dir(@FILE), '..', '.tmp', 'out')
	os.mkdir_all(out)!
	// A gradient test image, written with stbi.
	mut px := []u8{len: 64 * 32 * 4}
	for y in 0 .. 32 {
		for x in 0 .. 64 {
			i := (y * 64 + x) * 4
			px[i], px[i + 1], px[i + 2], px[i + 3] = u8(x * 4), u8(y * 8), 200, 255
		}
	}
	img_path := os.join_path(out, 'gradient.png')
	stbi.stbi_write_png(img_path, 64, 32, 4, px.data, 64 * 4)!
	mut h := testing.new_pixel_harness()!
	h.open(Showcase{
		image: img_path
	},
		width:  560
		height: 380
		scale:  1.5
	)!
	h.wait_images()!
	h.press('tab')! // keyboard focus shows the ring
	h.scroll_at(330, 80, -50)
	h.save_png(os.join_path(out, 'm2_showcase.png'))!
	assert (h.find('btn') or { panic('') }).width > 0
}
