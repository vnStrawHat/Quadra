// TermExt: what OneTerm adds to one terminal (specs/22, P3): completion, the semantic classes,
// the log file. Reached through closure-free hooks; it travels with a dragged terminal.
module main

import time
import completion
import highlight
import termlog
import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.prim

@[heap]
pub struct TermExt {
pub mut:
	one  &OneTerm      = unsafe { nil }
	pty  &BroadcastPty = unsafe { nil }
	kind string // the shell kind: completion family, highlight profile
	ctl  completion.Controller
	sem  highlight.Overlay
	log  &termlog.TerminalLogController = unsafe { nil }
	// completion
	anchor      term.Pos // (display row, token-start column)
	has_anchor  bool
	last_cursor term.Pos = term.Pos{-1, -1}
	line        string // the input line last read
	// semantic classes, cached per logical line (semantic.v)
	scratch highlight.ScanScratch
	text    []rune // the logical line being rescanned
	owners  []int  // row << 16 | column of each rune of text
	out     []u8
	keys    []u64
	cache   [][]u8
	wraps   []bool
	roles   highlight.RowRoles
	visible []term.Row
	cleared bool
	gutter  Gutter // timestamps and line numbers (gutter.v)
	find    FindBar
}

fn completion_config(c CompletionConfig) completion.Config {
	return completion.Config{
		enabled:                  c.enabled
		accept_tab:               c.accept_tab
		max_history:              c.max_history
		min_prefix_len:           c.min_prefix_len
		max_visible_items:        c.max_visible_items
		sources:                  completion.SourceToggles{c.sources.memory, c.sources.manual, c.sources.external}
		fuzzy:                    c.fuzzy
		inherit_ancestor_options: c.inherit_ancestor_options
		disable_in_alt_screen:    c.disable_in_alt_screen
		require_prompt_region:    c.require_prompt_region
		windows_allow_coreutils:  c.windows_allow_coreutils
		force_family:             c.force_family
		redact_sensitive:         c.redact_sensitive
	}
}

// semantic_tints flattens the shipped semantic styles for the terminal's table.
fn semantic_tints() []term.CellTint {
	styles := highlight.load_default_styles()
	mut out := []term.CellTint{cap: highlight.class_count}
	for i in 0 .. highlight.class_count {
		s := styles.style(u8(i))
		out << term.CellTint{s.has_fg, s.fg, s.has_bg, s.bg, s.bold, s.italic, s.underline, s.override_ansi}
	}
	return out
}

// attach_ext gives a freshly made terminal its extension.
fn (mut o OneTerm) attach_ext(mut s Space, logging bool, identity string, mut cx core.ViewCx[OneTerm]) {
	kind := if s.remote { 'bash' } else { s.shell.kind }
	mut x := &TermExt{
		one:  unsafe { &o }
		kind: kind
		ctl:  completion.new_controller(kind, completion_config(o.cfg.completion))
		sem:  highlight.new_overlay(highlight.profile_for(kind),
			o.cfg.layout.semantic_highlighting != 'off')
		log:  termlog.new_controller(identity)
	}
	s.ext = x
	mut t := cx.peek(s.view.model())
	t.set_hooks(term.Hooks{
		holder:  prim.hold(x)
		key:     ext_key
		overlay: ext_overlay
		rows:    ext_rows
		output:  ext_output
	}, semantic_tints())
	if logging {
		x.start_log(o.log_config()) or {
			kit.notify(mut cx.Cx, .error, 'Terminal logging', err.msg())
			''
		}
	}
}

fn ext_key(holder prim.Holder, mut t term.Terminal, e &core.KeyDownEvent, mut cx core.ViewCx[term.Terminal]) bool {
	mut x := prim.held[TermExt](holder)
	return x.key(mut t, e, mut cx)
}

fn ext_overlay(holder prim.Holder, mut t term.Terminal, mut cx core.ViewCx[term.Terminal]) ?core.Element {
	mut x := prim.held[TermExt](holder)
	x.update(mut t, mut cx)
	return x.layers(mut t, kit.theme(cx), mut cx)
}

fn ext_rows(holder prim.Holder, mut t term.Terminal, top int) {
	mut x := prim.held[TermExt](holder)
	x.fill_classes(mut t, top)
}

enum CompletionKey {
	accept
	dismiss
	select_first
	select_next
	select_prev
}

// intercept is the visible overlay's claim on a key; none means "not bound".
fn (x &TermExt) intercept(e &core.KeyDownEvent) ?CompletionKey {
	ctrl := e.mods.has(.ctrl)
	selected := x.ctl.selected() >= 0
	match e.key {
		.down {
			return if selected { CompletionKey.select_next } else { none }
		}
		.up {
			return if selected { CompletionKey.select_prev } else { none }
		}
		.n {
			return if ctrl && selected { CompletionKey.select_next } else { none }
		}
		.p {
			return if ctrl && selected { CompletionKey.select_prev } else { none }
		}
		.escape {
			return .dismiss
		}
		.enter {
			return if selected { CompletionKey.accept } else { none }
		}
		.tab {
			if x.ctl.accept_tab() {
				return if selected { CompletionKey.accept } else { CompletionKey.select_first }
			}
		}
		else {}
	}

	return none
}

// key is the terminal's first look at a key press (the original's row 0).
fn (mut x TermExt) key(mut t term.Terminal, e &core.KeyDownEvent, mut cx core.ViewCx[term.Terminal]) bool {
	if t.screen.alt_active && x.ctl.is_visible() {
		x.dismiss()
	}
	if x.one.zoom_key(e, mut cx.Cx) {
		return true
	}
	if x.ctl.is_visible() {
		if k := x.intercept(e) {
			x.one.last_input = time.now()
			x.apply_key(k, mut t)
			return true
		}
	}
	if e.key == .space && e.mods.has(.ctrl) && e.mods.has(.shift) {
		x.one.last_input = time.now()
		x.trigger(mut t)
		return true
	}
	if e.key == .enter {
		x.capture(mut t) // Enter with nothing selected runs the typed command
	}
	return false
}

fn (mut x TermExt) dismiss() {
	x.ctl.dismiss()
	x.has_anchor = false
}

fn (mut x TermExt) apply_key(k CompletionKey, mut t term.Terminal) {
	match k {
		.accept { x.accept(mut t) }
		.dismiss { x.dismiss() }
		.select_first { x.ctl.select_first_if_none() }
		.select_next { x.ctl.select_next() }
		.select_prev { x.ctl.select_prev() }
	}
}

// accept writes the suggestion to this terminal alone: never broadcast, as in the original.
fn (mut x TermExt) accept(mut _ term.Terminal) {
	if a := x.ctl.accept() {
		data := a.bytes()
		if data.len > 0 && x.pty != unsafe { nil } {
			x.pty.inner.write(data) or {}
		}
	}
	x.dismiss()
}

struct CursorCommand {
	line      string
	in_region bool
}

// cursor_of is the cursor as (display row, column).
fn cursor_of(t &term.Terminal) term.Pos {
	return term.Pos{t.screen.y, t.screen.x}
}

// read_command reads the command line under the cursor (specs/22, Input line).
fn (x &TermExt) read_command(t &term.Terminal) CursorCommand {
	line_no := t.screen.scrollback.len() + t.screen.y
	row := t.row_at(line_no)
	col := if t.screen.x < row.cells.len { t.screen.x } else { row.cells.len }
	if t.shell_marks_at(line_no).marks & term.mark_b != 0 {
		start := t.input_start(line_no)
		if start >= 0 && start <= col {
			return CursorCommand{cells_text(row.cells[start..col]), t.in_command_input()}
		}
	}
	command, found := completion.strip_prompt(cells_text(row.cells[..col]))
	return CursorCommand{command, found}
}

fn cells_text(cells []term.Cell) string {
	mut out := []rune{cap: cells.len}
	for c in cells {
		if c.flags & term.f_spacer == 0 {
			out << if c.ch == 0 { ` ` } else { rune(c.ch) }
		}
	}
	return out.string()
}

// update is the per-render step: gating, then a recompute only when something moved.
fn (mut x TermExt) update(mut t term.Terminal, mut _ core.ViewCx[term.Terminal]) {
	x.ctl.sync_settings(x.kind, completion_config(x.one.cfg.completion))
	if !x.ctl.enabled() || t.exited != none {
		x.dismiss()
		return
	}
	x.ctl.set_alt_screen(t.screen.alt_active)
	if !x.ctl.pre_gate_ok() {
		x.dismiss()
		return
	}
	cursor := cursor_of(t)
	if !x.ctl.wants_recompute(cursor != x.last_cursor) {
		return
	}
	x.last_cursor = cursor
	cmd := x.read_command(t)
	x.ctl.set_in_prompt_region(cmd.in_region)
	if !x.ctl.gating_allows() {
		x.dismiss()
		return
	}
	x.line = cmd.line
	x.ctl.recompute(cmd.line, cmd.line.len, u64(time.ticks()), x.one.history, false)
	x.anchor_at(cursor)
}

// trigger force-opens the overlay at the cursor, bypassing min_prefix_len.
fn (mut x TermExt) trigger(mut t term.Terminal) {
	x.ctl.set_alt_screen(t.screen.alt_active)
	if t.screen.alt_active {
		return
	}
	cmd := x.read_command(t)
	x.ctl.set_in_prompt_region(cmd.in_region)
	x.line = cmd.line
	x.ctl.recompute(cmd.line, cmd.line.len, u64(time.ticks()), x.one.history, true)
	x.anchor_at(cursor_of(t))
}

fn (mut x TermExt) anchor_at(cursor term.Pos) {
	x.has_anchor = x.ctl.is_visible()
	col := cursor.col - x.ctl.typed_len()
	x.anchor = term.Pos{cursor.line, if col > 0 { col } else { 0 }}
}

// capture records the line being run; the overlay closes.
fn (mut x TermExt) capture(mut t term.Terminal) {
	cmd := x.read_command(t)
	if cmd.line.trim_space() == '' {
		return
	}
	x.ctl.capture(cmd.line, u64(time.ticks()), mut x.one.history)
	x.dismiss()
}
