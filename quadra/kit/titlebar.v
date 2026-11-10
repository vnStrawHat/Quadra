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
	size     f32        // the title's text size; 0: 12 px (title_size)
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

// title_size makes the title a window caption at `px` (body size, 16), 12 from the left
// edge, in a bar 34 tall unless a height is set.
pub fn (self_ &Titlebar) title_size(px f32) &Titlebar {
	mut t := unsafe { self_ }
	t.size = px
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
	h := if tb.height > 0 {
		tb.height
	} else if tb.size > 0 {
		f32(34)
	} else {
		t.titlebar_h
	}
	maximized := cx.win != unsafe { nil } && cx.win.pw.is_maximized()
	// The chrome tone, an optional hamburger menu, 12 px text, the caption buttons drawn with
	// the system's caption glyph font (specs/15).
	mut bar :=
		ui.div().test_id('titlebar').window_region(.caption).flex_row().items_center().h(h).shrink_0().border_b(1)
	if t.ctl.accent_chrome { // shades down from chrome 45 % toward the canvas to chrome
		bar.gradient(t.p.chrome.lerp(t.p.canvas, 0.45), t.p.chrome, 90)
	} else {
		bar.bg(t.p.chrome)
	}
	bar.border_color(t.p.line_strong).text_size(t.ui_text(.sm)).text_color(t.p.text)
	if tb.menu.len > 0 {
		bar.child(ui.div().pl(4).child(menu_button('titlebar-menu', '', tb.menu).icon(icon_menu).size(.sm).tooltip(strings_of(cx.app).app_menu)))
	}
	if tb.icon != '' {
		bar.child(ui.div().pl(8).child(icon(tb.icon).size(14).color(t.p.text_2)))
	}
	if tb.title != '' && tb.size > 0 {
		bar.child(ui.div().pl(12).pr(8).child(ui.text(tb.title).size(t.scaled(tb.size))))
	} else if tb.title != '' {
		bar.child(ui.div().px(8).child(ui.text(tb.title)))
	}
	bar.child(ui.div().flex_row().items_center().gap(4).grow().children(tb.controls))
	face := if cx.text.has_family(fluent_face) { fluent_face } else { caption_face }
	bar.child(window_button('win-min', .minimize, '', false, h, face, false))
	bar.child(window_button('win-max', .maximize, if maximized { '' } else { '' }, false, h,
		face, maximized))
	bar.child(window_button('win-close', .close, '', true, h, face, false))
	return bar
}

// The caption glyphs: Segoe Fluent Icons on Windows 11, else Segoe MDL2 Assets (Windows
// 10), which has the same code points.
const fluent_face = 'Segoe Fluent Icons'
const caption_face = 'Segoe MDL2 Assets'

// window_button is a 36 px caption button with a 10 px glyph; close turns red on hover.
fn window_button(id string, region platform.HitRegion, glyph string, danger bool, h f32, face string, restore bool) core.Element {
	return themed_with(WindowButton{id, region, glyph, danger, h, face, restore}, fn (b WindowButton, t &Theme) core.Element {
		if t.ctl.accent_chrome {
			return drawn_caption(b, t)
		}
		id, region, glyph, danger, h := b.id, b.region, b.glyph, b.danger, b.h
		red := prim.rgb(0xE81120)
		return ui.div().test_id(id).window_region(region).flex_row().items_center().justify_center().w(36).h(h).text_color(t.p.text).hover(ui.Patch{
			background: if danger { red } else { t.p.panel_hover }
			text_color: if danger { prim.white } else { t.p.text }
		}).active(ui.Patch{
			background: if danger { red.alpha(0.8) } else { t.p.press }
		}).role(.button).label(id).child(ui.text(glyph).family(b.face).size(10)).on_click(core.handler_with(region,
			caption_click))
	})
}

// drawn_caption is a caption button of Controls.accent_chrome: 34 wide, a 14 px icon,
// hover `raised_hover`, close in `error`.
fn drawn_caption(b WindowButton, t &Theme) core.Element {
	p := t.p
	glyph := match b.region {
		.minimize {
			icon_minus
		}
		.maximize {
			if b.restore { icon_window_restore } else { icon_window_maximize }
		}
		else {
			icon_x
		}
	}

	hover := if b.danger { p.error } else { p.raised_hover }
	return ui.div().test_id(b.id).window_region(b.region).flex_row().items_center().justify_center().w(34).h(b.h).text_color(p.text).hover(ui.Patch{
		background: hover
		text_color: if b.danger { p.text_on_status } else { p.text }
	}).active(ui.Patch{
		background: if b.danger { p.error.lerp(prim.black, 0.08) } else { hover }
	}).role(.button).label(b.id).child(icon(glyph).size(14)).on_click(core.handler_with(b.region,
		caption_click))
}

fn caption_click(region platform.HitRegion, _ &core.ClickEvent, mut cx core.Cx) {
	match region {
		.minimize { cx.win.pw.minimize() }
		.maximize { cx.win.pw.toggle_maximize() }
		.close { cx.request_close() } // runs the close handlers ("save changes?")
		else {}
	}
}

struct WindowButton {
	id      string
	region  platform.HitRegion
	glyph   string
	danger  bool
	h       f32
	face    string
	restore bool
}
