// Display helpers: heading, label, badge, tag, kbd, avatar, card, divider.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

@[heap]
pub struct Styled {
	Lazy
mut:
	make  fn (t &Theme) core.Element = unsafe { nil }
	bound core.BoundRef[Theme, core.Element]
}

// themed defers building an element until layout, when the theme is known. For app code;
// the kit uses themed_with, which needs no closure.
pub fn themed(make fn (t &Theme) core.Element) &Styled {
	return &Styled{
		make: make
	}
}

// themed_with is themed with the data the builder needs passed in, so the builder captures
// nothing (specs/14, avoiding closures).
pub fn themed_with[D](data D, make fn (d D, t &Theme) core.Element) &Styled {
	return &Styled{
		bound: core.bind_fn_ref[D, Theme, core.Element](data, make)
	}
}

pub fn (mut s Styled) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut s, mut cx)
	return s.layout_inner(el, mut cx)
}

pub fn (mut s Styled) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	return if s.bound.is_set() { s.bound.run(t) } else { s.make(t) }
}

struct HeadingArgs {
	s     string
	level int
}

pub fn heading(s string, level int) &Styled {
	return themed_with(HeadingArgs{s, level}, fn (a HeadingArgs, t &Theme) core.Element {
		s, level := a.s, a.level
		size := match level {
			1 { t.text_2xl }
			2 { t.text_xl }
			3 { t.text_lg }
			else { t.text_md }
		}

		// Headlines are regular weight (specs/15); size alone ranks them.
		return ui.div().role(.heading).label(s).child(ui.text(s).size(t.scaled(size)).color(t.p.text))
	})
}

// label is muted secondary text.
pub fn label(s string) &Styled {
	return themed_with(s, fn (s string, t &Theme) core.Element {
		return ui.text(s).size(t.font_size(.sm)).color(t.p.text_2)
	})
}

pub enum Tone {
	neutral
	primary
	success
	warning
	danger
	info
}

fn (t &Theme) tone(tone Tone) prim.Color {
	p := t.p
	return match tone {
		.neutral { p.text_2 }
		.primary { p.accent_text }
		.success { p.ok }
		.warning { p.warn }
		.danger { p.error }
		.info { p.info }
	}
}

struct BadgeArgs {
	s    string
	tone Tone
}

pub fn badge(s string, tone Tone) &Styled {
	return themed_with(BadgeArgs{s, tone}, fn (a BadgeArgs, t &Theme) core.Element {
		s := a.s
		col := t.tone(a.tone)
		// A chip (specs/15): 10 px mono text, 4 padding, a faint tint of the tone. The outer
		// row keeps it at its text's width in grid cells, which stretch their children.
		return ui.div().flex_row().child(ui.div().px(4).h(16).flex_row().items_center().rounded(t.radius_md).border(1).border_color(col.alpha(0.2)).bg(col.alpha(0.1)).text_color(col).text_size(t.font_size(.xs)).font(t.mono_family).child(ui.text(s)))
	})
}

pub fn tag(s string) &Styled {
	return themed_with(s, fn (s string, t &Theme) core.Element {
		return ui.div().px(4).h(18).flex_row().items_center().rounded(t.radius_md).border(1).border_color(t.p.line_strong).bg(t.p.raised).text_color(t.p.text).text_size(t.font_size(.xs)).font(t.mono_family).child(ui.text(s))
	})
}

// kbd shows a keyboard shortcut like "ctrl-s" as plain muted text, "Ctrl-S" (specs/15).
pub fn kbd(keys string) &Styled {
	return themed_with(keys, fn (keys string, t &Theme) core.Element {
		return ui.text(key_label(keys)).color(t.p.text_2)
	})
}

// avatar shows an image or the initials of a name in a circle.
struct AvatarArgs {
	name  string
	size  f32
	image string
}

pub fn avatar(name string, size f32, image string) &Styled {
	return themed_with(AvatarArgs{name, size, image}, fn (a AvatarArgs, t &Theme) core.Element {
		name, size, image := a.name, a.size, a.image
		if image != '' {
			return ui.img(image).size(size, size).fit(.cover).rounded(size / 2)
		}
		initials := name.fields().map(it[0..1].to_upper()).join('')
		hue := f32(name.hash() % 360)
		return ui.div().size(size).rounded_full().center().bg(prim.hsla(hue, 0.55, 0.55, 1)).text_color(prim.white).text_size(size * 0.4).font_weight(600).label(name).child(ui.text(if initials.len > 2 {
			initials[..2]
		} else {
			initials
		}))
	})
}

struct CardArgs {
	title       string
	description string
	body        core.Element
}

pub fn card(title string, description string, body core.Element) &Styled {
	return themed_with(CardArgs{title, description, body}, fn (a CardArgs, t &Theme) core.Element {
		title, description, body := a.title, a.description, a.body
		mut d :=
			ui.div().flex_col().gap(8).p(12).bg(t.p.panel).rounded(t.radius_md).border(1).border_color(t.p.line)
		if title != '' {
			mut head := ui.div().flex_col().gap(2).child(ui.text(title).size(t.scaled(t.text_lg)))
			if description != '' {
				head.child(ui.text(description).size(t.font_size(.sm)).color(t.p.text_2))
			}
			d.child(head)
		}
		return d.child(body)
	})
}

pub fn divider() &Styled {
	return themed(fn (t &Theme) core.Element {
		return ui.div().h(1).w_full().bg(t.p.line).shrink_0()
	})
}

pub fn vdivider() &Styled {
	return themed(fn (t &Theme) core.Element {
		return ui.div().w(1).h_full().bg(t.p.line).shrink_0()
	})
}

// section_label names a group (sidebar and panel sections): 12 px muted text (specs/15).
pub fn section_label(s string, t &Theme) &ui.Div {
	return ui.div().child(ui.text(s).size(t.font_size(.sm)).color(t.p.text_2))
}

// key_label writes a binding like "ctrl-shift-p" the way the kit shows shortcuts (specs/15):
// "Ctrl-Shift-P", modifiers in the order Ctrl, Win, Alt, Shift; chords keep their spaces.
pub fn key_label(keys string) string {
	return keys.split(' ').map(stroke_label(it)).join(' ')
}

fn stroke_label(stroke string) string {
	parts := stroke.to_lower().split('-') // 'Ctrl-C' and 'ctrl-c' alike
	mut out := []string{cap: parts.len}
	for m in ['ctrl', 'win', 'alt', 'shift'] {
		if m in parts[..parts.len - 1] {
			out << m.capitalize()
		}
	}
	out << key_part(parts.last())
	return out.join('-')
}

fn key_part(k string) string {
	match k {
		'minus' { return '-' }
		'plus' { return '+' }
		'pageup' { return 'PageUp' }
		'pagedown' { return 'PageDown' }
		'super', 'cmd' { return 'Win' }
		else {}
	}

	if k.len == 1 {
		return k.to_upper()
	}
	return k.capitalize()
}

// key_hint is a keyboard hint (specs/15): the keys in a small box tinted toward the
// accent, then what they do, in mono italic 12 px disabled text.
pub fn key_hint(keys string, text string) &Styled {
	return themed_with([keys, text], fn (a []string, t &Theme) core.Element {
		p := t.p
		k := if t.mode == .light { f32(0.5) } else { f32(1) }
		key := ui.div().px(2).rounded(t.radius_md).border(1).border_color(p.panel.lerp(p.text,
			0.16 * k)).bg(p.panel.lerp(p.accent_text, 0.1 * k)).child(ui.text(key_label(a[0])).size(10).italic())
		return ui.div().flex_row().items_center().gap(4).font(t.mono_family).text_color(p.text_disabled).child(key).child(ui.text(a[1]).size(t.font_size(.sm)).italic())
	})
}
