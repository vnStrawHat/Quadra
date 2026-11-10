// Button builder methods (button.v).
module kit

import quadra.core
import quadra.prim

pub fn (self_ &Button) label(s string) &Button {
	mut b := unsafe { self_ }
	b.text = s
	return b
}

pub fn (self_ &Button) icon(glyph string) &Button {
	mut b := unsafe { self_ }
	b.glyph = glyph
	return b
}

pub fn (self_ &Button) variant(v Variant) &Button {
	mut b := unsafe { self_ }
	b.variant = v
	return b
}

pub fn (self_ &Button) primary() &Button {
	return self_.variant(.primary)
}

pub fn (self_ &Button) danger() &Button {
	return self_.variant(.danger)
}

pub fn (self_ &Button) ghost() &Button {
	return self_.variant(.ghost)
}

pub fn (self_ &Button) outline() &Button {
	return self_.variant(.outline)
}

pub fn (self_ &Button) link() &Button {
	return self_.variant(.link)
}

pub fn (self_ &Button) size(s Size) &Button {
	mut b := unsafe { self_ }
	b.size = s
	return b
}

pub fn (self_ &Button) muted() &Button {
	mut b := unsafe { self_ }
	b.muted = true
	return b
}

pub fn (self_ &Button) disabled(on bool) &Button {
	mut b := unsafe { self_ }
	b.disabled = on
	return b
}

pub fn (self_ &Button) loading(on bool) &Button {
	mut b := unsafe { self_ }
	b.loading = on
	return b
}

pub fn (self_ &Button) full_width() &Button {
	mut b := unsafe { self_ }
	b.full = true
	return b
}

pub fn (self_ &Button) tooltip(s string) &Button {
	mut b := unsafe { self_ }
	b.tip = s
	return b
}

// tooltip_keys is a tooltip that names the button's shortcut (key_label form).
pub fn (self_ &Button) tooltip_keys(s string, keys string) &Button {
	mut b := unsafe { self_ }
	b.tip, b.tip_keys = s, keys
	return b
}

pub fn (self_ &Button) on_click(l core.Listener[core.ClickEvent]) &Button {
	mut b := unsafe { self_ }
	b.click = l.any
	return b
}

// selected shows the button pressed (a toggle whose state the parent owns, e.g. a panel
// that is open).
pub fn (self_ &Button) selected(on bool) &Button {
	mut b := unsafe { self_ }
	b.pressed = on
	return b
}

pub fn (self_ &Button) on_toggle(l core.Listener[bool]) &Button {
	mut b := unsafe { self_ }
	b.toggled = l.any
	return b
}

// compact narrows a labelled button's padding (sm 6, md and lg 8) and keeps it at least as
// wide as it is tall (a title-bar menu).
pub fn (self_ &Button) compact() &Button {
	mut b := unsafe { self_ }
	b.compact = true
	return b
}

fn compact_px(s Size) f32 {
	return match s {
		.xs { f32(4) }
		.sm { 6 }
		else { 8 }
	}
}

// button_px is a labelled button's side padding: 4 / 4 / 8 / 8, or 4 / 8 / 10 / 12 with
// solid buttons (Controls.solid_buttons).
fn button_px(s Size, solid bool) f32 {
	return match s {
		.xs {
			f32(4)
		}
		.sm {
			if solid {
				f32(8)
			} else {
				4
			}
		}
		.md {
			if solid {
				f32(10)
			} else {
				8
			}
		}
		.lg {
			if solid {
				f32(12)
			} else {
				8
			}
		}
	}
}

// button_icon is the icon size: 14 small, 16 from md; solid icon squares 12 / 14 / 16.
fn button_icon(s Size, solid_square bool) f32 {
	if solid_square {
		return match s {
			.xs { f32(12) }
			.sm { 14 }
			else { 16 }
		}
	}
	return if s == .xs || s == .sm { f32(14) } else { 16 }
}

// solid_colors are the fills of Controls.solid_buttons: a solid accent primary; danger,
// outlined and ghost fills that deepen with hover and press.
fn (b &Button) solid_colors(t &Theme) ButtonColors {
	p := t.p
	clear := prim.transparent
	mut c := match b.variant {
		.primary {
			ButtonColors{p.accent, p.text_on_accent, p.accent_hover, p.accent_press, clear}
		}
		.danger {
			ButtonColors{p.error.alpha(0.2), p.error, p.error.alpha(0.3), p.error.alpha(0.4), clear}
		}
		.secondary, .outline {
			ButtonColors{p.line.alpha(0.3), p.text, p.line.alpha(0.5), p.line.alpha(0.7), p.line}
		}
		.ghost {
			ButtonColors{clear, p.text, p.panel_hover.alpha(0.5), p.line.alpha(0.7), clear}
		}
		.link {
			ButtonColors{clear, p.accent_text, clear, clear, clear}
		}
	}

	if b.disabled {
		c = ButtonColors{
			...c
			bg: if b.variant in [.primary, .danger] { c.bg.alpha(0.15) } else { c.bg }
		}
	}
	mut r := b.state_colors(c, t)
	if b.disabled {
		r = ButtonColors{
			...r
			fg: p.text_2.alpha(0.5)
		}
	}
	return r
}
