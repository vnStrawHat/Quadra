// Controller: the per-terminal state machine: gating, selection, accept, capture. Holds no
// history: the UI passes the shared store on every call.
module completion

// Accept is what applying a suggestion types into the shell.
pub struct Accept {
pub:
	backspaces int // chars to erase first (Cmd/PowerShell case correction)
	text       string
}

// bytes is the PTY form: 0x7f per backspace, then the text.
pub fn (a Accept) bytes() []u8 {
	mut out := []u8{len: a.backspaces, init: 0x7f}
	out << a.text.bytes()
	return out
}

// Controller belongs to one terminal; `new_controller` builds it.
pub struct Controller {
mut:
	engine                Engine
	family                ShellFamily
	params                Params
	enabled               bool
	accept_tab            bool
	disable_in_alt_screen bool
	require_prompt_region bool
	on_alt_screen         bool
	in_prompt_region      bool
	suggestions           []Suggestion
	selected              int
	typed_prefix          string
	last_line             string
	last_cursor           int
	dirty                 bool
	snapshot              Config
}

fn resolve_family(kind string, c Config) ShellFamily {
	if f := family_from_config(c.force_family) {
		return f
	}
	return family_from_kind(kind)
}

// new_controller builds a controller for a shell kind with the live settings.
pub fn new_controller(kind string, c Config) Controller {
	return Controller{
		engine:                new_engine()
		family:                resolve_family(kind, c)
		params:                params_from_config(c)
		enabled:               c.enabled
		accept_tab:            c.accept_tab
		disable_in_alt_screen: c.disable_in_alt_screen
		require_prompt_region: c.require_prompt_region
		in_prompt_region:      true
		selected:              -1
		last_cursor:           -1
		dirty:                 true
		snapshot:              c
	}
}

// sync_settings re-applies settings when they changed; cheap per frame.
pub fn (mut c Controller) sync_settings(kind string, cfg Config) {
	if c.snapshot == cfg {
		return
	}
	c.family = resolve_family(kind, cfg)
	c.params = params_from_config(cfg)
	c.enabled = cfg.enabled
	c.accept_tab = cfg.accept_tab
	c.disable_in_alt_screen = cfg.disable_in_alt_screen
	c.require_prompt_region = cfg.require_prompt_region
	c.snapshot = cfg
	c.dirty = true
}

pub fn (c &Controller) family() ShellFamily {
	return c.family
}

pub fn (c &Controller) accept_tab() bool {
	return c.accept_tab
}

pub fn (c &Controller) enabled() bool {
	return c.enabled
}

// set_alt_screen updates the alternate-screen signal; turning it on dismisses the overlay.
pub fn (mut c Controller) set_alt_screen(on bool) {
	if c.on_alt_screen != on {
		c.on_alt_screen = on
		c.dirty = true
	}
	if on {
		c.dismiss()
	}
}

pub fn (mut c Controller) set_in_prompt_region(in_region bool) {
	if c.in_prompt_region != in_region {
		c.in_prompt_region = in_region
		c.dirty = true
	}
}

// gating_allows is the full gate.
pub fn (c &Controller) gating_allows() bool {
	return c.enabled && !(c.disable_in_alt_screen && c.on_alt_screen)
		&& (!c.require_prompt_region || c.in_prompt_region)
}

// pre_gate_ok is the cheap gate, before the input line is read.
pub fn (c &Controller) pre_gate_ok() bool {
	return c.enabled && !(c.disable_in_alt_screen && c.on_alt_screen)
}

pub fn (c &Controller) is_visible() bool {
	return c.suggestions.len > 0
}

pub fn (c &Controller) suggestions() []Suggestion {
	return c.suggestions
}

// selected returns the selected row, -1 until the user navigates.
pub fn (c &Controller) selected() int {
	return c.selected
}

pub fn (c &Controller) max_visible() int {
	return if c.params.max_visible_items < 1 { 1 } else { c.params.max_visible_items }
}

// typed_len is the width in chars of what the suggestions extend (the overlay's anchor).
pub fn (c &Controller) typed_len() int {
	return c.typed_prefix.runes().len
}

// recompute refreshes the suggestions from the input line and cursor; no-op (keeping the
// selection) when nothing changed. `force` bypasses `min_prefix_len`.
pub fn (mut c Controller) recompute(line string, cursor_col int, now_ms u64, history &History, force bool) {
	if !c.gating_allows() {
		c.dismiss()
		return
	}
	cursor := if cursor_col > line.len { line.len } else { cursor_col }
	if !force && !c.dirty && line == c.last_line && cursor == c.last_cursor {
		return
	}
	c.last_line = line
	c.last_cursor = cursor
	c.dirty = false
	c.typed_prefix = current_typed_prefix(line, cursor)
	mut params := c.params
	if force {
		params.min_prefix_len = 0
		params.suggest_on_empty = true
	}
	ctx := Context{c.family, line, cursor, now_ms}
	c.suggestions = c.engine.suggest(history, ctx, params)
	if c.suggestions.len > c.params.hard_cap() {
		c.suggestions = unsafe { c.suggestions[..c.params.hard_cap()] }
	}
	if has_sole_exact_match(c.suggestions, line, cursor) {
		c.suggestions = []
	}
	c.selected = -1
}

// wants_recompute lets the view skip reading the grid on idle frames.
pub fn (c &Controller) wants_recompute(cursor_moved bool) bool {
	return c.dirty || cursor_moved
}

pub fn (mut c Controller) dismiss() {
	c.suggestions = []
	c.selected = -1
}

// select_first_if_none selects the first row when nothing is selected; true when it changed.
pub fn (mut c Controller) select_first_if_none() bool {
	if c.selected >= 0 || c.suggestions.len == 0 {
		return false
	}
	c.selected = 0
	return true
}

pub fn (mut c Controller) select_next() {
	if c.selected >= 0 && c.selected < c.suggestions.len - 1 {
		c.selected++
	}
}

pub fn (mut c Controller) select_prev() {
	if c.selected > 0 {
		c.selected--
	}
}

// accept returns what to type for the selected suggestion, none when nothing is selected
// or it would need a fuzzy (non-prefix) replacement.
pub fn (c &Controller) accept() ?Accept {
	if c.selected < 0 || c.selected >= c.suggestions.len {
		return none
	}
	s := c.suggestions[c.selected]
	typed := replacement_text(c.last_line, c.last_cursor, s) or { return none }
	if s.text.starts_with(typed) {
		return Accept{0, s.text[typed.len..]}
	}
	if c.family != .unix && s.is_prefix_of_typed(typed) {
		return case_corrected(s.text, typed)
	}
	if c.params.allow_fuzzy_accept {
		return Accept{0, s.text}
	}
	return none
}

// capture records a just-run command line, redacting first.
pub fn (c &Controller) capture(raw_command string, now_ms u64, mut history History) {
	if !c.enabled || !c.params.sources.memory || (c.disable_in_alt_screen && c.on_alt_screen) {
		return
	}
	line := if c.params.redact_sensitive { redact(raw_command) } else { raw_command.trim_space() }
	if line.len == 0 {
		return
	}
	history.record(c.family, line, now_ms)
}

// breadcrumb returns the resolved command path of the line (the overlay's header).
pub fn (mut c Controller) breadcrumb(line string, cursor int) []string {
	return c.engine.breadcrumb(Context{c.family, line, cursor, 0}, c.params)
}

fn has_sole_exact_match(suggestions []Suggestion, line string, cursor int) bool {
	if suggestions.len != 1 {
		return false
	}
	typed := replacement_text(line, cursor, suggestions[0]) or { return false }
	return typed == suggestions[0].text
}

fn replacement_text(line string, cursor int, s Suggestion) ?string {
	if s.replace_from > cursor || cursor > line.len {
		return none
	}
	typed := line[s.replace_from..cursor]
	return if s.replace_from == 0 { typed.trim_left(' \t\n\r\v\f') } else { typed }
}

fn case_corrected(suggestion string, typed string) ?Accept {
	limit := if suggestion.len < typed.len { suggestion.len } else { typed.len }
	mut mismatch := -1
	for i in 0 .. limit {
		if suggestion[i] != typed[i] {
			mismatch = i
			break
		}
	}
	if mismatch < 0 {
		return none
	}
	return Accept{typed[mismatch..].runes().len, suggestion[mismatch..]}
}

fn current_typed_prefix(line string, cursor int) string {
	p := parse_line(line, cursor)
	if p.token.len == 0 {
		return line[..cursor].trim_left(' \t\n\r\v\f')
	}
	return p.token
}
