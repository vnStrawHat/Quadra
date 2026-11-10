// Control shapes that differ between design languages (specs/15, Controls). The defaults
// are One's; a theme file sets them under "controls", e.g. {"plain_toggles": true}.
module kit

import x.json2
import quadra.core
import quadra.prim

pub struct Controls {
pub mut:
	// switch: a 36 x 20 track (`line` off, `accent` on) with a 16 thumb in `text`, no ring;
	// checkbox: radius 4, `input_bg` fill, checked = an `accent` fill with a 12 white check
	plain_toggles bool
	// select trigger: `input_bg` fill, padding 10, gap 8, a 16 chevron-down (else One's
	// `raised` fill and 12 up-down chevron)
	select_input bool
	// buttons: a solid accent primary, danger and outlined fills that tint by state, padding
	// xs 4 / sm 8 / md 10 / lg 12, gap 8 from md, icon squares with 12 / 14 / 16 icons
	solid_buttons bool
	// text inputs: single-line ones height(.md) tall (32), padding 10 x 8, gap 6, `text_2`
	// placeholder, a 3 px `border_focused` 50 % ring outside the border while focused
	roomy_inputs bool
	// menus: 26 rows 2 apart, text 12 from the edge, min width 128, radius_md with a ring
	// instead of a border, 2 px separators, 12 icons, checks in the row color, muted shortcuts
	// at 12; tooltips: padding 8 x 2, radius_md, at the pointer
	roomy_menus bool
	// shortcuts as "Ctrl+Shift+C": `+`, modifiers Ctrl, Alt, Shift, Win, "Page Up"
	plus_keys bool
	// dialogs: canvas fill, 1/10 down the window, padding 16, semibold title, a close button
	// at the top right, footer without a line; alerts have no close button and ignore
	// outside clicks
	card_dialogs bool
	// notifications and toasts: one stack at the bottom right, 16 from the edges, 382 wide,
	// tinted with their kind, the close button only while hovered; they hide after 5 s unless
	// they have actions
	tinted_notices bool
	// chrome: a title bar shading down to `chrome` with 34 caption buttons and 14 icons, tabs
	// with a 2 px accent line on top and seams around the active one, 12 status dividers in
	// `line`, resize handles with a `text_2` pill on hover and drag (resize_line)
	accent_chrome bool
	// scroll thumbs always shown, `scroll_thumb_hover` and 8 wide under the pointer; a
	// terminal's thumb grey, 8 wide, shown 2 s
	fixed_scrollbars bool
}

// controls_into sets the fields of c named by the keys of o.
fn controls_into(mut c Controls, o map[string]json2.Any) ! {
	for k, v in o {
		mut found := false
		$for f in Controls.fields {
			if f.name == k {
				found = true
				$if f.typ is bool {
					if v !is bool {
						return error('controls.${k}: true or false')
					}
					c.$(f.name) = v.bool()
				}
			}
		}
		if !found {
			return error('controls.${k}: not a control setting')
		}
	}
}

fn control_lines(c Controls) []string {
	mut out := []string{}
	$for f in Controls.fields {
		$if f.typ is bool {
			v := c.$(f.name)
			out << '"${f.name}": ${v}'
		}
	}
	return out
}

// thumb_style is the look of the window's scroll thumbs: One's, or with fixed_scrollbars
// always shown, radius 4, 8 wide and `scroll_thumb_hover` under the pointer.
fn thumb_style(t &Theme) core.ThumbStyle {
	if !t.ctl.fixed_scrollbars {
		return core.ThumbStyle{}
	}
	return core.ThumbStyle{
		wide:   8
		radius: 4
		hover:  t.p.scroll_thumb_hover
		always: true
	}
}

// term_thumb is a terminal's scroll thumb: the window's, or with fixed_scrollbars an 8 px
// grey thumb 2 from the edge, shown 2 s and fading over 1 s.
pub fn term_thumb(t &Theme) core.ThumbStyle {
	if !t.ctl.fixed_scrollbars {
		return thumb_style(t)
	}
	return core.ThumbStyle{
		width:   8
		wide:    8
		inset:   2
		min_len: 24
		radius:  t.sp(2)
		color:   prim.rgba(0x808080CC)
		hold_ms: 2000
		fade_ms: 1000
	}
}
