// Commands: one enum behind the keys, the menu and the palette (specs/21, Keyboard).
module main

import os
import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.ui

pub enum Cmd {
	open
	save
	save_as
	new_doc
	new_window
	raw
	split
	preview
	outline
	palette
	zoom_in
	zoom_out
	zoom_reset
	theme_system
	theme_dark
	theme_light
	close_tab
	next_tab
	prev_tab
	reveal
	close
	// formatting (format.v)
	bold
	italic
	strike
	code
	math
	link
	image
	para
	h1
	h2
	h3
	bullet
	numbered
	task
	quote
	code_block
	math_block
	diagram
	table
	rule
}

// Do is the action every shortcut sends.
pub struct Do {
pub:
	cmd Cmd
}

struct CmdInfo {
	cmd   Cmd
	title string
	keys  string
}

const commands = [
	CmdInfo{.open, 'Open file…', 'ctrl-o'},
	CmdInfo{.save, 'Save', 'ctrl-s'},
	CmdInfo{.save_as, 'Save as…', 'ctrl-shift-s'},
	CmdInfo{.new_doc, 'New document', 'ctrl-n'},
	CmdInfo{.new_window, 'New window', 'ctrl-shift-n'},
	CmdInfo{.raw, 'Layout: Raw', 'ctrl-1'},
	CmdInfo{.split, 'Layout: Split', 'ctrl-2'},
	CmdInfo{.preview, 'Layout: Preview', 'ctrl-3'},
	CmdInfo{.outline, 'Show or hide the outline', 'ctrl-shift-o'},
	CmdInfo{.palette, 'Command palette', 'ctrl-shift-p'},
	CmdInfo{.zoom_in, 'Preview text larger', 'ctrl-='},
	CmdInfo{.zoom_out, 'Preview text smaller', 'ctrl-minus'},
	CmdInfo{.zoom_reset, 'Preview text default size', 'ctrl-0'},
	CmdInfo{.theme_system, 'Theme: follow the system', ''},
	CmdInfo{.theme_dark, 'Theme: One Dark Pro', ''},
	CmdInfo{.theme_light, 'Theme: One Light', ''},
	CmdInfo{.close_tab, 'Close tab', 'ctrl-w'},
	CmdInfo{.next_tab, 'Next tab', 'ctrl-tab'},
	CmdInfo{.prev_tab, 'Previous tab', 'ctrl-shift-tab'},
	CmdInfo{.reveal, 'Reveal in Explorer', ''},
	CmdInfo{.close, 'Close window', 'ctrl-shift-w'},
]

// all_commands is every command: the app's and the formatting ones.
fn all_commands() []CmdInfo {
	mut all := commands.clone()
	all << format_commands
	return all
}

pub fn bind_keys(mut a core.App) ! {
	for c in all_commands() {
		if c.keys != '' {
			a.bind(c.keys, Do{c.cmd})!
		}
	}
	a.bind('ctrl-pagedown', Do{.next_tab})!
	a.bind('ctrl-pageup', Do{.prev_tab})!
}

// with_actions runs commands sent to the window.
fn (mut f Folio) with_actions(root &ui.Div, mut cx core.ViewCx[Folio]) &ui.Div {
	return root.on_action(cx.listener(fn (mut f Folio, d &Do, mut cx core.ViewCx[Folio]) {
		f.run(d.cmd, mut cx)
	}))
}

pub fn (mut f Folio) run(cmd Cmd, mut cx core.ViewCx[Folio]) {
	match cmd {
		.open { f.open_dialog(mut cx) }
		.save { f.save(mut cx) }
		.save_as { f.save_as(mut cx) }
		.new_doc { f.new_doc(mut cx) }
		.new_window { f.new_window(mut cx) }
		.raw { f.set_layout(.raw, mut cx) }
		.split { f.set_layout(.split, mut cx) }
		.preview { f.set_layout(.preview, mut cx) }
		.outline { f.toggle_outline(mut cx) }
		.palette { f.palette(mut cx) }
		.zoom_in { f.zoom(0.1, mut cx) }
		.zoom_out { f.zoom(-0.1, mut cx) }
		.zoom_reset { f.zoom(0, mut cx) }
		.theme_system { f.set_theme('system', mut cx) }
		.theme_dark { f.set_theme('dark', mut cx) }
		.theme_light { f.set_theme('light', mut cx) }
		.close_tab { f.close_tab(f.cur_id(), mut cx) }
		.next_tab { f.cycle(1, mut cx) }
		.prev_tab { f.cycle(-1, mut cx) }
		.reveal { f.reveal_file(mut cx) }
		.close { cx.request_close() }
		else { f.format(cmd, mut cx) }
	}
}

// RunCmd is a menu item's action: it sends cmd to the window's Folio.
struct RunCmd {
	to  core.Listener[Cmd]
	cmd Cmd
}

pub fn (r RunCmd) run(mut cx core.Cx) {
	c := r.cmd
	cx.app.invoke(r.to.any, &c, cx.win, true)
}

fn (mut f Folio) menu_items(mut cx core.ViewCx[Folio]) []kit.MenuItem {
	to := cx.listener(fn (mut f Folio, c &Cmd, mut cx core.ViewCx[Folio]) {
		f.run(*c, mut cx)
	})
	mut items := []kit.MenuItem{}
	for c in commands {
		if c.cmd in [.raw, .outline, .zoom_in, .theme_system, .close_tab] { // group boundaries
			items << kit.separator()
		}
		items << kit.MenuItem{
			label:    c.title
			shortcut: c.keys
			action:   RunCmd{to, c.cmd}
		}
	}
	return items
}

fn (mut f Folio) palette(mut cx core.ViewCx[Folio]) {
	to := cx.listener(fn (mut f Folio, c &Cmd, mut cx core.ViewCx[Folio]) {
		f.run(*c, mut cx)
	})
	mut list := []input.Command{}
	for c in all_commands() {
		if c.cmd == .palette {
			continue
		}
		r := RunCmd{to, c.cmd}
		list << input.Command{c.title, c.keys, fn [r] (mut cx core.Cx) {
			r.run(mut cx)
		}}
	}
	input.open_command_palette(mut cx.Cx, list)
}

fn (mut f Folio) set_layout(l Layout, mut cx core.ViewCx[Folio]) {
	name := layouts[int(l)]
	f.prefs.update(mut cx.Cx, fn [name] (mut p Prefs, mut _ core.ViewCx[Prefs]) {
		p.layout = name
	})
	if f.tabs.len > 0 {
		f.cur().sync.editor_top = -1 // sync the preview to the editor again
	}
}

fn (mut f Folio) toggle_outline(mut cx core.ViewCx[Folio]) {
	f.prefs.update(mut cx.Cx, fn (mut p Prefs, mut _ core.ViewCx[Prefs]) {
		p.outline = !p.outline
	})
}

fn (mut f Folio) zoom(step f32, mut cx core.ViewCx[Folio]) {
	f.prefs.update(mut cx.Cx, fn [step] (mut p Prefs, mut _ core.ViewCx[Prefs]) {
		p.scale = if step == 0 { f32(1) } else { kit_clamp(p.scale + step, 0.7, 1.8) }
	})
}

fn kit_clamp(v f32, lo f32, hi f32) f32 {
	return if v < lo {
		lo
	} else if v > hi {
		hi
	} else {
		v
	}
}

fn (mut f Folio) set_theme(name string, mut cx core.ViewCx[Folio]) {
	f.prefs.update(mut cx.Cx, fn [name] (mut p Prefs, mut _ core.ViewCx[Prefs]) {
		p.theme = name
	})
	apply_theme(mut cx.app, *f.prefs.get(cx.Cx))
}

fn (mut f Folio) new_window(mut cx core.ViewCx[Folio]) {
	cx.app.open_window(new_window_options(), Folio{
		prefs:   f.prefs
		windows: f.windows
		fresh:   true
	}) or { kit.push_toast(mut cx.Cx, .error, err.msg()) }
}

// reveal_line scrolls the editor and the preview to a source line and puts the cursor
// there (outline clicks).
fn (mut f Folio) reveal_line(line int, mut cx core.ViewCx[Folio]) {
	mut tab := f.cur()
	mut e := cx.peek(tab.editor.model())
	n := if line < e.doc.buf.line_count() { line } else { e.doc.buf.line_count() - 1 }
	off := e.doc.buf.line_start(n)
	e.doc.set_cursor(off, off, false)
	e.scroll_to_line(n)
	tab.sync.editor_top = -1
	tab.scroll.y = tab.pos.top_of(line) + preview_pad - 8
	cx.touch(tab.editor.model())
	cx.win.invalidate_prepaint()
}

// follow_link opens web links in the browser, Markdown files in Folio and #anchors in
// this document.
fn (mut f Folio) follow_link(url string, mut cx core.ViewCx[Folio]) {
	if url.starts_with('http://') || url.starts_with('https://') || url.starts_with('mailto:') {
		cx.open_external(url)
		return
	}
	if url.starts_with('#') {
		for h in f.cur().heads {
			if h.anchor == url[1..] {
				f.reveal_line(h.line, mut cx)
				return
			}
		}
		return
	}
	target := url.all_before('#')
	base := if f.cur().path == '' { os.getwd() } else { os.dir(f.cur().path) }
	path := if os.is_abs_path(target) { target } else { os.join_path(base, target) }
	if os.is_file(path) {
		f.open_path(path, mut cx)
	} else {
		kit.push_toast(mut cx.Cx, .warning, 'Not found: ${target}')
	}
}

fn (f &Folio) cur_id() int {
	return if f.tabs.len > 0 { f.cur().id } else { -1 }
}

// reveal_file shows the shown file in Explorer.
fn (mut f Folio) reveal_file(mut cx core.ViewCx[Folio]) {
	if f.tabs.len > 0 && f.cur().path != '' {
		cx.reveal(f.cur().path)
	}
}
