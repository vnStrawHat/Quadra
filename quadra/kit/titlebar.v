// Titlebar for windows opened with `custom_titlebar: true`: a drag area with the title,
// the app's own controls, and minimize / maximize / close. The OS drives the buttons
// through their window regions (snap layouts over maximize on Windows 11); their click
// handlers cover platforms that deliver clicks instead (headless tests).
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.platform
import quadra.ui

pub struct Titlebar {
	Lazy
pub mut:
	title    string
	icon     string
	controls []core.Element
	menu     []MenuItem // shown by a hamburger button before the title
	height   f32        // 0: the theme's titlebar_h
}

pub fn titlebar(title string) &Titlebar {
	return &Titlebar{
		title: title
	}
}

pub fn (self_ &Titlebar) icon(glyph string) &Titlebar {
	mut t := unsafe { self_ }
	t.icon = glyph
	return t
}

// menu puts the application menu behind a hamburger button at the left end.
pub fn (self_ &Titlebar) menu(items []MenuItem) &Titlebar {
	mut t := unsafe { self_ }
	t.menu = items
	return t
}

// controls places app elements (search, menus, tabs) between the title and the buttons.
// They stay clickable: only the bar's empty parts drag the window.
pub fn (self_ &Titlebar) controls(els []core.Element) &Titlebar {
	mut t := unsafe { self_ }
	t.controls = els
	return t
}

pub fn (mut tb Titlebar) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut tb, mut cx)
	return tb.layout_inner(el, mut cx)
}

pub fn (mut tb Titlebar) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	h := if tb.height > 0 { tb.height } else { t.titlebar_h }
	maximized := cx.win != unsafe { nil } && cx.win.pw.is_maximized()
	// The chrome tone, an optional hamburger menu, 12 px text, the caption buttons drawn with
	// the system's caption glyph font (specs/15).
	mut bar :=
		ui.div().test_id('titlebar').window_region(.caption).flex_row().items_center().h(h).shrink_0().bg(t.p.chrome).border_b(1).border_color(t.p.line_strong).text_size(t.font_size(.sm)).text_color(t.p.text)
	if tb.menu.len > 0 {
		bar.child(ui.div().pl(4).child(menu_button('titlebar-menu', '', tb.menu).icon(icon_menu).size(.sm).tooltip(strings_of(cx.app).app_menu)))
	}
	if tb.icon != '' {
		bar.child(ui.div().pl(8).child(icon(tb.icon).size(14).color(t.p.text_2)))
	}
	if tb.title != '' {
		bar.child(ui.div().px(8).child(ui.text(tb.title)))
	}
	bar.child(ui.div().flex_row().items_center().gap(4).grow().children(tb.controls))
	face := if cx.text.has_family(fluent_face) { fluent_face } else { caption_face }
	bar.child(window_button('win-min', .minimize, '', false, h, face))
	bar.child(window_button('win-max', .maximize, if maximized { '' } else { '' }, false, h,
		face))
	bar.child(window_button('win-close', .close, '', true, h, face))
	return bar
}

// The caption glyphs: Segoe Fluent Icons on Windows 11, else Segoe MDL2 Assets (Windows
// 10), which has the same code points.
const fluent_face = 'Segoe Fluent Icons'
const caption_face = 'Segoe MDL2 Assets'

// window_button is a 36 px caption button with a 10 px glyph; close turns red on hover.
fn window_button(id string, region platform.HitRegion, glyph string, danger bool, h f32, face string) core.Element {
	return themed_with(WindowButton{id, region, glyph, danger, h, face}, fn (b WindowButton, t &Theme) core.Element {
		id, region, glyph, danger, h := b.id, b.region, b.glyph, b.danger, b.h
		red := prim.rgb(0xE81120)
		return ui.div().test_id(id).window_region(region).flex_row().items_center().justify_center().w(36).h(h).text_color(t.p.text).hover(ui.Patch{
			background: if danger { red } else { t.p.panel_hover }
			text_color: if danger { prim.white } else { t.p.text }
		}).active(ui.Patch{
			background: if danger { red.alpha(0.8) } else { t.p.press }
		}).role(.button).label(id).child(ui.text(glyph).family(b.face).size(10)).on_click(core.handler_with(region, fn (region platform.HitRegion, _ &core.ClickEvent, mut cx core.Cx) {
			match region {
				.minimize { cx.win.pw.minimize() }
				.maximize { cx.win.pw.toggle_maximize() }
				.close { cx.request_close() } // runs the close handlers ("save changes?")
				else {}
			}
		}))
	})
}

struct WindowButton {
	id     string
	region platform.HitRegion
	glyph  string
	danger bool
	h      f32
	face   string
}
