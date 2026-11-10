// Keymap: keystrokes and chords bound to typed actions, scoped by key contexts.
module core

import quadra.platform

pub struct Keystroke {
pub:
	key  platform.Key
	mods platform.Mods
}

pub fn (k Keystroke) str() string {
	mut parts := []string{}
	if k.mods.has(.ctrl) {
		parts << 'ctrl'
	}
	if k.mods.has(.alt) {
		parts << 'alt'
	}
	if k.mods.has(.shift) {
		parts << 'shift'
	}
	if k.mods.has(.super) {
		parts << 'super'
	}
	parts << key_name(k.key)
	return parts.join('-')
}

fn key_name(k platform.Key) string {
	s := k.str()
	return if s.starts_with('digit') { s[5..] } else { s.replace('_', '') }
}

// key_symbols lets bindings use the printed character: "ctrl-," for ctrl-comma.
// ("-" separates parts, so the minus key is always spelled "minus".)
const key_symbols = {
	'=':  'equal'
	'[':  'bracketleft'
	']':  'bracketright'
	r'\': 'backslash'
	';':  'semicolon'
	"'":  'quote'
	',':  'comma'
	'.':  'period'
	'/':  'slash'
	'`':  'backquote'
}

// parse_keystroke parses "ctrl-shift-z", "alt-enter", "f5", "pageup".
pub fn parse_keystroke(s string) !Keystroke {
	mut mods := platform.Mods.zero()
	parts := s.to_lower().split('-')
	for p in parts[..parts.len - 1] {
		match p {
			'ctrl', 'control' { mods.set(.ctrl) }
			'alt' { mods.set(.alt) }
			'shift' { mods.set(.shift) }
			'super', 'win', 'cmd' { mods.set(.super) }
			else { return error('unknown modifier "${p}" in "${s}"') }
		}
	}
	last := key_symbols[parts.last()] or { parts.last() }
	for k in 1 .. int(platform.Key.backquote) + 1 {
		key := unsafe { platform.Key(k) }
		if key_name(key) == last {
			return Keystroke{key, mods}
		}
	}
	return error('unknown key "${last}" in "${s}"')
}

// ContextPredicate is a conjunction like "TextInput && !ReadOnly".
struct ContextPredicate {
	required  []string
	forbidden []string
}

fn parse_context(s string) ContextPredicate {
	mut req, mut forb := []string{}, []string{}
	for t in s.split('&&') {
		tok := t.trim_space()
		if tok == '' {
			continue
		}
		if tok.starts_with('!') {
			forb << tok[1..].trim_space()
		} else {
			req << tok
		}
	}
	return ContextPredicate{req, forb}
}

fn (p ContextPredicate) matches(contexts []string) bool {
	return p.required.all(it in contexts) && !p.forbidden.any(it in contexts)
}

pub struct Binding {
pub:
	keys        []Keystroke
	action_type string
	action      voidptr
	context     string
mut:
	pred ContextPredicate
}

pub struct Keymap {
mut:
	bindings []Binding
}

@[params]
pub struct BindOptions {
pub:
	context string
}

// bind maps a keystroke sequence ("ctrl-k ctrl-s") to an action value.
pub fn (mut app App) bind[A](keys string, action A, opts BindOptions) ! {
	app.escaped_in_render()
	mut seq := []Keystroke{}
	for part in keys.fields() {
		seq << parse_keystroke(part)!
	}
	app.keymap.bindings << Binding{
		keys:        seq
		action_type: typeof[A]().name
		action:      heap_copy(action)
		context:     opts.context
		pred:        parse_context(opts.context)
	}
}

// unbind removes every binding of an action type, so an app can rebind its keys.
pub fn (mut app App) unbind[A]() {
	name := typeof[A]().name
	app.keymap.bindings = app.keymap.bindings.filter(it.action_type != name)
}

pub enum MatchResult {
	none
	pending
	matched
}

// resolve finds the binding for a keystroke sequence given the context stack of the
// focus path (deepest last). Deeper contexts win; among equals, later bindings win.
fn (km &Keymap) resolve(seq []Keystroke, contexts [][]string) (MatchResult, Binding) {
	mut pending := false
	for depth := contexts.len; depth >= 0; depth-- {
		mut ctx := []string{}
		for d in 0 .. depth {
			ctx << contexts[d]
		}
		for i := km.bindings.len - 1; i >= 0; i-- {
			b := km.bindings[i]
			if !b.pred.matches(ctx) || (b.context == '' && depth != 0 && contexts.len > 0) {
				continue
			}
			if b.keys.len >= seq.len && b.keys[..seq.len] == seq {
				if b.keys.len == seq.len {
					return MatchResult.matched, b
				}
				pending = true
			}
		}
	}
	return if pending { MatchResult.pending } else { MatchResult.none }, Binding{}
}

// bindings_for lists the keystrokes bound to an action type, for menus and tooltips.
pub fn (app &App) bindings_for[A]() []string {
	name := typeof[A]().name
	return app.keymap.bindings.filter(it.action_type == name).map(it.keys.map(it.str()).join(' '))
}
