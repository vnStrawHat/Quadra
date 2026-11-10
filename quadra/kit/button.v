// Buttons: button, icon_button, toggle_button.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

pub enum Variant {
	primary
	secondary
	outline
	ghost
	danger
	link
}

@[heap]
pub struct Button {
	Lazy
pub mut:
	id       string
	text     string
	glyph    string // icon body (kit.icon_*)
	variant  Variant = .secondary
	size     Size    = .md
	disabled bool
	loading  bool
	pressed  ?bool // toggle buttons
	square   bool
	full     bool
	muted    bool // the label and icon in the muted text color (status bar buttons)
	caret    bool // a dropdown trigger: a trailing chevron, the input fill (caret())
	compact  bool // narrower padding, at least as wide as tall (compact())
	tip      string
	tip_keys string // the shortcut the tooltip shows after its text
	click    ?core.AnyListener
	toggled  ?core.AnyListener
mut:
	div &ui.Div = unsafe { nil }
}

pub fn button(id string) &Button {
	return &Button{
		id: id
	}
}

// icon_button is a square button showing only an icon (give it a tooltip).
pub fn icon_button(id string, glyph string) &Button {
	return &Button{
		id:      id
		glyph:   glyph
		variant: .ghost
		size:    .sm
		square:  true
	}
}

// toggle_button keeps a pressed state owned by the parent.
pub fn toggle_button(id string, pressed bool) &Button {
	return &Button{
		id:      id
		pressed: pressed
		variant: .ghost
	}
}

// colors maps a variant to its fills (specs/15): ghost has none, secondary and outline are
// outlined, primary and danger tinted. Toggled: the selected step, the label the accent.
fn (b &Button) colors(t &Theme) ButtonColors {
	p := t.p
	clear := prim.transparent
	if t.ctl.solid_buttons {
		return b.solid_colors(t)
	}
	c := match b.variant {
		.primary {
			tinted(p.accent_tint, p.accent, p.text)
		}
		.danger {
			tinted(p.error_tint, p.error, p.text)
		}
		.secondary {
			ButtonColors{clear, p.text, p.panel_hover, p.press, p.line}
		}
		.outline {
			ButtonColors{clear, p.text, p.panel_hover, clear, p.line}
		}
		.ghost {
			ButtonColors{clear, p.text, p.panel_hover, p.press, clear}
		}
		.link {
			ButtonColors{clear, p.accent_text, clear, clear, clear}
		}
	}

	return b.state_colors(c, t)
}

// state_colors applies the toggled, disabled and muted states to a variant's colors.
fn (b &Button) state_colors(c ButtonColors, t &Theme) ButtonColors {
	p := t.p
	on := b.pressed or { false }
	return ButtonColors{
		...c
		bg: if on && !b.square && c.bg.a == 0 { p.press } else { c.bg } // icon toggles: accent only
		fg: if b.disabled {
			p.text_disabled
		} else if on {
			p.accent_text
		} else if b.muted {
			p.text_2
		} else {
			c.fg
		}
	}
}

fn (b &Button) build(t &Theme) &ui.Div {
	col := b.colors(t)
	h := t.height(b.size)
	is_small := b.size == .xs || b.size == .sm
	solid := t.ctl.solid_buttons
	inactive := b.disabled || b.loading
	mut text_sz := Size.md
	if b.size == .xs {
		text_sz = .sm
	} else if solid && b.size == .lg {
		text_sz = .lg
	}
	gap := if solid && !is_small { t.sp(8) } else { t.sp(4) }
	mut d := ui.div().id(b.id).test_id(b.id).flex_row().items_center().justify_center().gap(gap).h(h).shrink_0().rounded(t.radius_md).bg(col.bg).text_color(col.fg).text_size(t.ui_text(text_sz)).role(if b.pressed != none {
		core.Role.switch
	} else {
		core.Role.button
	}).label(if b.text != '' { b.text } else { b.tip }).disabled(b.disabled)
	if b.pressed != none {
		d.checked(b.pressed or { false })
	}
	if b.square {
		d.w(h)
	} else if b.compact {
		d.px(t.sp(prim.min(button_px(b.size, solid), compact_px(b.size)))).min_w(h)
	} else {
		d.px(t.sp(button_px(b.size, solid)))
	}
	if b.full {
		d.w_len(layout.pct(100))
	}
	if col.border.a > 0 {
		// Outlined: the border darkens on hover and turns the focus color on keyboard focus.
		d.border(1).border_color(col.border).focus_ring(ui.Patch{
			border_color: t.p.border_focused
		})
	} else {
		d.focus_ring(ui.Patch{ background: col.hover })
	}
	if !inactive {
		d.cursor(.pointer).hover(ui.Patch{
			background:   col.hover
			border_color: if col.border.a > 0 && !solid {
				?prim.Color(t.p.line_strong)
			} else {
				none
			}
		}).active(ui.Patch{
			background: col.active
		})
		data := Press{b.click, b.toggled, b.pressed or { false }}
		d.on_click(core.handler_with(data, fn (p Press, e &core.ClickEvent, mut cx core.Cx) {
			p.press(e, mut cx)
		}))
		d.on_key_down(core.handler_with(data, fn (p Press, e &core.KeyDownEvent, mut cx core.Cx) {
			if activates(e) {
				ev := core.ClickEvent{}
				p.press(&ev, mut cx)
				cx.stop_propagation()
			}
		}))
	} else if b.disabled {
		d.cursor(.not_allowed)
	}
	glyph := button_icon(b.size, b.square && solid)
	if b.loading {
		d.child(spinner().size(glyph))
	} else if b.glyph != '' {
		d.child(icon(b.glyph).size(glyph))
	}
	if b.text != '' {
		d.child(ui.text(b.text))
	}
	if b.caret {
		d.gap(8).bg(t.p.input_bg).child(icon(icon_chevron_down).size(16).color(col.fg.alpha(0.75)))
	}
	return d
}

pub fn (mut b Button) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut b, mut cx)
	return b.layout_inner(el, mut cx)
}

pub fn (mut b Button) build_tree(mut cx core.LayoutCx) core.Element {
	b.div = b.build(theme_of(cx.app))
	return if b.tip != '' {
		core.Element(tooltip(b.div, b.tip).keys(b.tip_keys))
	} else {
		core.Element(b.div)
	}
}

pub fn (mut b Button) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	if !b.disabled && b.variant != .link {
		b.div.focus_id = stable_focus(mut cx, b.id)
	}
	b.Lazy.prepaint(bounds, mut cx)
}
