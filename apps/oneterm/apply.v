// Settings into effect: the theme, the keys and every running terminal.
module main

import os
import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.prim

fn weight_of(s string) int {
	return match s.to_lower() {
		'thin' { 100 }
		'extra_light', 'extralight' { 200 }
		'light' { 300 }
		'normal', 'regular' { 400 }
		'medium' { 500 }
		'semibold' { 600 }
		'bold' { 700 }
		'extra_bold', 'extrabold' { 800 }
		'black' { 900 }
		else { 400 }
	}
}

const default_min_contrast = f32(4.5)

// behavior is the settings as terminals use them, with the live zoom on the shared size.
fn (o &OneTerm) behavior() term.Behavior {
	mut b := behavior_of(o.cfg)
	if o.font_zoom != 0 {
		base := if o.cfg.font.size > 0 { o.cfg.font.size } else { term.font_size }
		b.size = zoomed(base + o.font_zoom)
	}
	return b
}

fn zoomed(v f32) f32 {
	return if v < 6 {
		6
	} else if v > 100 {
		100
	} else {
		v
	}
}

// zoom_key is ctrl+= (or ctrl++), ctrl+- and ctrl+0: the shared font size of every terminal,
// live only (the original persists nothing until the settings UI saves).
fn (mut o OneTerm) zoom_key(e &core.KeyDownEvent, mut cx core.Cx) bool {
	if !e.mods.has(.ctrl) || e.mods.has(.alt)
		|| (e.key != .equal && e.key != .minus && e.key != .digit0) {
		return false
	}
	base := if o.cfg.font.size > 0 { o.cfg.font.size } else { term.font_size }
	match e.key {
		.equal { o.font_zoom = zoomed(base + o.font_zoom + 1) - base }
		.minus { o.font_zoom = zoomed(base + o.font_zoom - 1) - base }
		else { o.font_zoom = 0 }
	}

	b := o.behavior()
	for tab in o.tabs {
		for s in tab.spaces {
			if s.view.id != 0 {
				mut t := cx.peek(s.view.model())
				t.b.size = b.size
				cx.touch(s.view.model())
			}
		}
	}
	return true
}

fn behavior_of(c TerminalConfig) term.Behavior {
	p := c.layout.padding
	mut cursor_color := prim.Color{}
	if c.cursor.color != '' {
		set_color(mut cursor_color, c.cursor.color)
	}
	min_contrast := if c.colors.min_contrast > 0 {
		c.colors.min_contrast
	} else {
		default_min_contrast
	}
	return term.Behavior{
		family:           c.font.family
		size:             c.font.size
		weight:           weight_of(c.font.weight)
		line_height:      c.layout.line_height
		padding:          prim.Edges{p.top, p.right, p.bottom, p.left}
		cursor:           match c.cursor.shape {
			'bar' { term.CursorChoice.bar }
			'underline' { term.CursorChoice.underline }
			else { term.CursorChoice.block }
		}
		cursor_blink:     c.cursor.blink
		copy_on_select:   c.mouse.copy_on_select
		middle_paste:     c.mouse.middle_click_paste
		wheel:            c.scroll.multiplier
		alternate_scroll: c.scroll.alternate_scroll
		bell:             c.bell.enabled
		clipboard_keys:   false // the app binds terminal_copy and terminal_paste
		ligatures:        c.font.ligatures
		fallbacks:        c.font.fallbacks
		features:         c.font.features
		cursor_color:     cursor_color
		cell_width:       c.layout.cell_width
		min_contrast:     min_contrast
	}
}

fn set_color(mut to prim.Color, s string) {
	if c := prim.parse_hex(s) {
		to = c
	}
}

// themed applies the `colors` block of terminal.json over a theme.
fn themed(base kit.Theme, c ColorsConfig, ui_font_size f32) kit.Theme {
	mut t := base
	// The original's sizes are in rem, rem = ui_font_size (16 by default): its type scale
	// 12 / 14 / 16 / 18 / 20 with body text at 16 and controls one step down (text_step),
	// controls 20 / 24 / 32 tall, paddings scaled the same way. The title bar is absolute.
	rem := ui_font_size / 16
	t.text_scale, t.ui_scale, t.text_step = rem, rem, 1
	t.text_xs, t.text_sm, t.text_md, t.text_lg, t.text_xl = 12, 14, 16, 18, 20
	t.h_xs, t.h_sm, t.h_md, t.h_lg = 20, 24, 32, 32
	t.titlebar_h, t.statusbar_h, t.panel_header_h = 34, 28 * rem + 1, 32 * rem
	t.radius_md, t.radius_lg, t.radius_xl = 4, 4, 6 // controls and fields 4, overlays 6
	t.font_family, t.mono_family = 'Segoe UI', 'Consolas' // the system faces it uses on Windows
	t.ctl.plain_toggles, t.ctl.select_input = true, true
	t.ctl.solid_buttons, t.ctl.roomy_inputs, t.ctl.roomy_menus, t.ctl.plus_keys = true, true, true, true
	t.ctl.card_dialogs, t.ctl.tinted_notices, t.ctl.accent_chrome, t.ctl.fixed_scrollbars = true, true, true, true
	if t.mode == .dark { // its fields are the input border at 30 %
		t.p.input_bg = t.p.line.alpha(0.3)
	}
	set_color(mut t.code.terminal.foreground, c.foreground)
	set_color(mut t.code.terminal.background, c.background)
	set_color(mut t.code.terminal.cursor, c.cursor)
	set_color(mut t.code.terminal.selection, c.selection)
	for i, s in c.ansi {
		if i < 16 {
			if col := prim.parse_hex(s) {
				t.code.terminal.ansi[i] = col
			}
		}
	}
	return t
}

// apply puts the current settings into effect. Shell settings wait for the next terminal.
pub fn (mut o OneTerm) apply(mut cx core.Cx) {
	kit.set_theme(mut cx.app, theme_for(o.dir, o.ui, o.cfg.colors))
	b := o.behavior()
	o.history.set_capacity(o.cfg.completion.max_history)
	semantic := o.cfg.layout.semantic_highlighting != 'off'
	o.push_panel_settings(mut cx)
	for tab in o.tabs {
		for s in tab.spaces {
			if s.view.id != 0 {
				mut t := cx.peek(s.view.model())
				t.b = b
				if s.ext != unsafe { nil } {
					mut x := s.ext
					x.sem.enabled = semantic
					x.keys = []u64{len: x.keys.len, init: ~u64(0)}
				}
				t.screen.set_scrollback(o.cfg.scroll.scrollback_history)
				t.screen.allow_osc52_read = o.cfg.security.allow_clipboard_read
				cx.touch(s.view.model())
			}
		}
	}
}

// write_config replaces a file through a temporary one and keeps the old one as .bak.
fn write_config(path string, text string) ! {
	os.mkdir_all(os.dir(path))!
	tmp := '${path}.tmp'
	os.write_file(tmp, text)!
	if os.exists(path) {
		os.cp(path, '${path}.bak')!
	}
	os.mv(tmp, path, overwrite: true)!
}
