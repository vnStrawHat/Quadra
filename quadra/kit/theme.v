// Theme tokens (specs/15-design-language.md). Themes are plain values stored as an app
// global. Palettes live in theme_one.v and theme_code.v, the bundled faces in fonts.v.
module kit

import quadra.core
import quadra.prim
import quadra.text

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
	// text
	text           prim.Color
	text_2         prim.Color
	text_3         prim.Color
	text_disabled  prim.Color
	text_on_accent prim.Color
	// accent
	accent       prim.Color
	accent_hover prim.Color
	accent_press prim.Color
	accent_tint  prim.Color
	accent_text  prim.Color
	// feedback, each with a tint for banners and tinted buttons (finish() derives unset ones;
	// accent_tint is the info tint)
	ok         prim.Color
	warn       prim.Color
	error      prim.Color
	info       prim.Color
	ok_tint    prim.Color
	warn_tint  prim.Color
	error_tint prim.Color
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
	text_scale  f32    = 1     // OS text-size setting
	line_height f32    = 1.618 // of UI text
	mono_size   f32    = 15    // the code editor's text
	mono_line   f32    = 1.618 // and its line height
	// chrome
	titlebar_h     f32 = 32
	activity_w     f32 = 40
	tabs_h         f32 = 32
	panel_header_h f32 = 28
	row_h          f32 = 26
	statusbar_h    f32 = 30
	// follow_system tints the accent from the OS accent color when set
	system_accent bool
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
		danger_fg:       p.text_on_accent
		danger_hover:    p.error.lerp(p.canvas, 0.15)
		success:         p.ok
		warning:         p.warn
		info:            p.info
		selection:       p.accent_tint
		link:            p.accent_text
	}
	return r
}

// height is the control height for a size at the theme's density.
pub fn (t &Theme) height(s Size) f32 {
	base := match s {
		.xs { f32(18) }
		.sm { 22 }
		.md { 28 }
		.lg { 32 }
	}

	return if t.density == .comfortable { base + 4 } else { base }
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

// theme reads the global theme during render (tracked: the view re-renders on change).
pub fn theme(cx &core.Cx) &Theme {
	return cx.global[Theme]() or { &Theme{
		...dark()
	} }
}

// theme_of reads the theme at layout time; components resolve tokens there, so a theme
// switch only needs a relayout, never a re-render.
pub fn theme_of(app &core.App) &Theme {
	cx := core.Cx{
		app: unsafe { app }
	}
	return cx.peek_global[Theme]() or { &Theme{
		...dark()
	} }
}

// set_theme replaces the theme and relays out every window.
pub fn set_theme(mut app core.App, t Theme) {
	mut cx := core.Cx{
		app: app
	}
	cx.set_global(t)
	app.text_style = text.TextStyle{
		family:      t.font_family
		size:        t.text_md * t.text_scale
		color:       t.colors.foreground
		line_height: t.line_height
	}
	for mut w in app.windows {
		w.clear_color = t.colors.background
		w.focus_color = t.p.border_focused
		w.scroll_color = t.p.scroll_thumb
		w.invalidate()
	}
}

// follow_system picks light, dark or high contrast from the OS and keeps following it.
// Themes with `system_accent` take the accent from the OS too.
pub fn follow_system(mut app core.App, light_theme Theme, dark_theme Theme) {
	apply := fn [light_theme, dark_theme] (mut app core.App) {
		look := app.plat.appearance()
		mut t := if look.high_contrast {
			high_contrast()
		} else if look.dark {
			dark_theme
		} else {
			light_theme
		}
		if !look.high_contrast && t.system_accent {
			t.p.accent = look.accent
			t.p.accent_hover = look.accent.lerp(prim.white, 0.12)
			t.p.accent_press = look.accent.lerp(prim.black, 0.15)
			t.p.accent_tint = look.accent.alpha(0.1)
			t.p.accent_text = look.accent
			t.p.border_focused = look.accent
			t = t.finish()
		}
		t.text_scale = look.text_scale
		set_theme(mut app, t)
	}
	apply(mut app)
	app.appearance_hooks << apply
}
