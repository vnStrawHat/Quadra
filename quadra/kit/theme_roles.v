// Palette roles a theme may split off from the main ones (specs/15, Planes and borders).
// Unset (alpha 0) they follow the role they come from, in finish(). Code that changes a
// finished theme's main roles clears these first (clear_derived), as high_contrast() does.
module kit

import quadra.prim

// derived_roles are left unset when a theme file does not list them, so they follow that
// theme's own colors rather than the base theme's.
const derived_roles = ['input_bg', 'selection', 'tab_fg', 'tab_active_fg', 'table_head_bg',
	'table_head_fg', 'table_row_border', 'list_even', 'scroll_thumb_hover', 'text_on_status',
	'error_hover', 'cyan']

// selection stays unset unless a theme gives it: text fields then select in accent_tint
// and selectable text in the editor selection (Colors.selection, Host).
fn (mut p Palette) derive_roles(type_color prim.Color) {
	if p.tab_fg.a == 0 {
		p.tab_fg = p.text_2
	}
	if p.tab_active_fg.a == 0 {
		p.tab_active_fg = p.text
	}
	if p.table_head_bg.a == 0 {
		p.table_head_bg = p.panel
	}
	if p.table_head_fg.a == 0 {
		p.table_head_fg = p.text_2
	}
	if p.table_row_border.a == 0 {
		p.table_row_border = p.line_strong
	}
	if p.text_on_status.a == 0 {
		p.text_on_status = p.text_on_accent
	}
	if p.error_hover.a == 0 {
		h := p.error.lerp(p.canvas, 0.15)
		p.error_hover = prim.parse_hex(h.hex()) or { h } // whole bytes: theme files round-trip
	}
	if p.cyan.a == 0 {
		p.cyan = type_color
	}
	if p.scroll_thumb_hover.a == 0 {
		p.scroll_thumb_hover = p.scroll_thumb // finish() sets scroll_thumb first
	}
	// list_even stays transparent: no stripes
}

// clear_derived unsets the derived roles a theme file does not list.
fn (mut p Palette) clear_derived(listed map[string]bool) {
	$for f in Palette.fields {
		if f.name in derived_roles && f.name !in listed {
			$if f.typ is prim.Color {
				p.$(f.name) = prim.Color{}
			}
		}
	}
}
