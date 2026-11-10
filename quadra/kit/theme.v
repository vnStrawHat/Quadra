// Theme tokens (specs/15), an app global. Palettes: theme_one.v, theme_code.v; faces: fonts.v.
module kit

import quadra.prim

pub enum Mode {
	light
	dark
	high_contrast
}

pub enum Density {
	compact // the default: developer tools are dense
	comfortable
}

pub enum Size {
	xs
	sm
	md
	lg
}

// Palette holds the design roles (specs/15). Themes define these; colors() derives the
// older names the kit still uses.
pub struct Palette {
pub mut:
	// planes, back to front
	canvas         prim.Color
	panel          prim.Color
	panel_hover    prim.Color
	raised         prim.Color
	raised_hover   prim.Color
	press          prim.Color
	overlay_bg     prim.Color
	scrim          prim.Color
	line           prim.Color
	line_strong    prim.Color
	border_focused prim.Color // finish() makes it the accent when unset
	chrome         prim.Color // title and status bars; finish() makes it the panel when unset
	status_bg      prim.Color // the status bar when it differs from the title bar (else chrome)
	tab_bg         prim.Color // tab strips and inactive tabs (else panel)
	scroll_thumb   prim.Color // scroll thumbs (else text at 30 %)
	input_bg       prim.Color // text fields, steppers, dropdown triggers (else canvas)
	// roles a theme may split off; finish() derives the unset ones (derived_roles)
	selection          prim.Color // selected UI text (else accent_tint in fields, the editor selection elsewhere)
	tab_fg             prim.Color // inactive tab labels (else text_2)
	tab_active_fg      prim.Color // the active tab's label, and any tab's under the pointer (else text)
	table_head_bg      prim.Color // table headers (else panel)
	table_head_fg      prim.Color // their labels (else text_2)
	table_row_border   prim.Color // lines between table rows (else line_strong)
	list_even          prim.Color // even rows of striped lists (else transparent: no stripes)
	scroll_thumb_hover prim.Color // a thumb under the pointer or dragged (else scroll_thumb)
	// text
	text           prim.Color
	text_2         prim.Color
	text_3         prim.Color
	text_disabled  prim.Color
	text_on_accent prim.Color
	text_on_status prim.Color // on ok, warn, error and info fills (else text_on_accent)
	// accent
	accent       prim.Color
	accent_hover prim.Color
	accent_press prim.Color
	accent_tint  prim.Color
	accent_text  prim.Color
	// feedback, each with a tint for banners and tinted buttons (finish() derives unset ones;
	// accent_tint is the info tint)
	ok          prim.Color
	warn        prim.Color
	error       prim.Color
	info        prim.Color
	ok_tint     prim.Color
	warn_tint   prim.Color
	error_tint  prim.Color
	error_hover prim.Color // a filled error control under the pointer (else error toward canvas)
	cyan        prim.Color // a cyan hue for kinds and marks (else the syntax type color)
	// workload status (shape + color, specs/15)
	running     prim.Color
	pending     prim.Color
	failed      prim.Color
	succeeded   prim.Color
	terminating prim.Color
	unknown     prim.Color
}

// Colors are the kit's original token names, derived from the Palette.
pub struct Colors {
pub mut:
	background      prim.Color
	foreground      prim.Color
	surface         prim.Color
	surface_hover   prim.Color
	muted           prim.Color
	muted_fg        prim.Color
	border          prim.Color
	input           prim.Color // borders of form controls
	ring            prim.Color
	overlay         prim.Color
	primary         prim.Color
	primary_fg      prim.Color
	primary_hover   prim.Color
	primary_active  prim.Color
	secondary       prim.Color
	secondary_hover prim.Color
	danger          prim.Color
	danger_fg       prim.Color
	danger_hover    prim.Color
	success         prim.Color
	warning         prim.Color
	info            prim.Color
	selection       prim.Color
	link            prim.Color
}

pub struct Theme {
pub mut:
	name    string
	mode    Mode
	p       Palette
	colors  Colors // derived from p by finish(); kept for the existing components
	code    CodeColors
	series  []prim.Color // categorical chart colors (kit/viz), tuned per theme
	density Density
	ctl     Controls // control shapes (theme_controls.v)
	// shape
	radius_sm f32 = 2 // checkboxes, keys
	radius_md f32 = 4 // buttons, rows, chips
	radius_lg f32 = 6 // inputs, groups, code blocks
	radius_xl f32 = 8 // overlays and dialogs
	// type (specs/15); the faces are bundled (fonts.v)
	font_family string = ui_face
	mono_family string = mono_face
	text_xs     f32    = 10
	text_sm     f32    = 12
	text_md     f32    = 14
	text_lg     f32    = 16
	text_xl     f32    = 18
	text_2xl    f32    = 23
	text_3xl    f32    = 28
	text_scale  f32    = 1 // OS text-size setting
	text_step   int // 1: components take their text one step down the scale (ui_text), for scales whose md is body text (16)
	line_height f32 = 1.618 // of UI text
	mono_size   f32 = 15    // the code editor's text
	mono_line   f32 = 1.618 // and its line height
	// size (specs/15, Size): control heights by Size, and the factor that scales them and the
	// paddings of components (a UI font-size setting; 1 at One's sizes)
	h_xs     f32 = 18
	h_sm     f32 = 22
	h_md     f32 = 28
	h_lg     f32 = 32
	ui_scale f32 = 1
	// chrome
	titlebar_h     f32 = 32
	activity_w     f32 = 40
	tabs_h         f32 = 32
	panel_header_h f32 = 28
	row_h          f32 = 26
	statusbar_h    f32 = 30
	system_accent  bool // follow_system tints the accent from the OS accent color
}

// finish derives the original token names from the palette.
pub fn (t Theme) finish() Theme {
	mut r := t
	if r.p.chrome.a == 0 {
		r.p.chrome = r.p.panel
	}
	if r.p.status_bg.a == 0 {
		r.p.status_bg = r.p.chrome
	}
	if r.p.tab_bg.a == 0 {
		r.p.tab_bg = r.p.panel
	}
	if r.p.scroll_thumb.a == 0 {
		r.p.scroll_thumb = r.p.text.alpha(0.3)
	}
	if r.p.input_bg.a == 0 {
		r.p.input_bg = r.p.canvas
	}
	r.p.derive_roles(r.code.syntax.typ)
	if r.code.editor.foreground.a == 0 {
		r.code.editor.foreground = r.p.text
	}
	if r.p.border_focused.a == 0 {
		r.p.border_focused = r.p.accent
	}
	if r.p.ok_tint.a == 0 {
		r.p.ok_tint = r.p.ok.alpha(0.1)
	}
	if r.p.warn_tint.a == 0 {
		r.p.warn_tint = r.p.warn.alpha(0.1)
	}
	if r.p.error_tint.a == 0 {
		r.p.error_tint = r.p.error.alpha(0.1)
	}
	p := r.p
	r.colors = Colors{
		background:      p.canvas
		foreground:      p.text
		surface:         p.panel
		surface_hover:   p.panel_hover
		muted:           p.raised_hover
		muted_fg:        p.text_2
		border:          p.line
		input:           p.line_strong
		ring:            p.border_focused
		overlay:         p.scrim
		primary:         p.accent
		primary_fg:      p.text_on_accent
		primary_hover:   p.accent_hover
		primary_active:  p.accent_press
		secondary:       p.raised
		secondary_hover: p.raised_hover
		danger:          p.error
		danger_fg:       p.text_on_status
		danger_hover:    p.error_hover
		success:         p.ok
		warning:         p.warn
		info:            p.info
		selection:       if p.selection.a > 0 { p.selection } else { p.accent_tint }
		link:            p.accent_text
	}
	return r
}

// height is the control height for a size at the theme's density.
pub fn (t &Theme) height(s Size) f32 {
	base := match s {
		.xs { t.h_xs }
		.sm { t.h_sm }
		.md { t.h_md }
		.lg { t.h_lg }
	}

	return (if t.density == .comfortable {
		base + 4
	} else {
		base
	}) * t.ui_scale
}

// sp scales a component's padding, gap or fixed size by ui_scale.
pub fn (t &Theme) sp(v f32) f32 {
	return v * t.ui_scale
}

// ui_text is the text size kit components use for size s: font_size, one step down the
// scale when text_step is 1 (a scale whose md is body text, 16, keeps controls at 14).
pub fn (t &Theme) ui_text(s Size) f32 {
	if t.text_step == 0 {
		return t.font_size(s)
	}
	return match s {
		.xs, .sm { t.text_xs * t.text_scale }
		.md { t.text_sm * t.text_scale }
		.lg { t.text_md * t.text_scale }
	}
}

// font_size is the text size for a control size, scaled by the OS text setting.
pub fn (t &Theme) font_size(s Size) f32 {
	v := match s {
		.xs { t.text_xs }
		.sm { t.text_sm }
		.md { t.text_md }
		.lg { t.text_lg }
	}

	return v * t.text_scale
}

pub fn (t &Theme) scaled(v f32) f32 {
	return v * t.text_scale
}
