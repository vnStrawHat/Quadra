// The kitty keyboard protocol and xterm's modifyOtherKeys (specs/17).
module term

import quadra.platform

pub const kitty_disambiguate = u8(1)
pub const kitty_event_types = u8(2)
pub const kitty_alternates = u8(4)
pub const kitty_all_as_esc = u8(8)
pub const kitty_text = u8(16)
const kitty_mask = u8(31)

pub enum KeyAction {
	press
	repeat
	release
}

// FlagStack is one screen's kitty flag stack; live is what the encoder and the query read.
pub struct FlagStack {
pub mut:
	flags [8]u8
	len   int
	live  u8
}

fn (mut f FlagStack) apply(v u8, how int) {
	f.live = match how {
		3 { f.live & ~v }
		2 { f.live | v }
		else { v }
	}
}

fn (mut f FlagStack) push(v u8) {
	if f.len == 8 {
		for i in 0 .. 7 {
			f.flags[i] = f.flags[i + 1]
		}
		f.len--
	}
	f.flags[f.len] = v
	f.len++
	f.live = v
}

fn (mut f FlagStack) pop(n int) {
	f.len = if n >= f.len { 0 } else { f.len - n }
	f.live = if f.len > 0 { f.flags[f.len - 1] } else { u8(0) }
}

pub struct KeyboardStacks {
pub mut:
	active   FlagStack
	inactive FlagStack
}

fn (mut k KeyboardStacks) swap() {
	k.active, k.inactive = k.inactive, k.active
}

// KeyInput is what the encoder needs of the terminal's modes.
pub struct KeyInput {
pub:
	app_cursor bool
	flags      u8
	other      int // modifyOtherKeys level
}

// Form of a key report: a code point (CSI u), a tilde code or a letter.
enum KeyForm {
	codepoint
	tilde
	letter
}

const shifted_chars = '~!@#$%^&*()_+{}|:"<>?'
const unshifted_chars = "`1234567890-=[]\\;',./"

// base_char is the unshifted ASCII a key types (US layout).
fn base_char(key platform.Key) ?u8 {
	k := int(key)
	if k >= int(platform.Key.a) && k <= int(platform.Key.z) {
		return u8(`a`) + u8(k - int(platform.Key.a))
	}
	if k >= int(platform.Key.digit0) && k <= int(platform.Key.digit9) {
		return u8(`0`) + u8(k - int(platform.Key.digit0))
	}
	return match key {
		.minus { `-` }
		.equal { `=` }
		.bracket_left { `[` }
		.bracket_right { `]` }
		.backslash { `\\` }
		.semicolon { `;` }
		.quote { `'` }
		.comma { `,` }
		.period { `.` }
		.slash { `/` }
		.backquote { 0x60 }
		else { none }
	}
}

fn shifted_char(c u8) ?u8 {
	if c >= `a` && c <= `z` {
		return c - 32
	}
	at := unshifted_chars.index_u8(c)
	return if at >= 0 { shifted_chars[at] } else { none }
}

fn key_form(key platform.Key) ?(KeyForm, int) {
	if c := base_char(key) {
		return KeyForm.codepoint, int(c)
	}
	return match key {
		.escape { KeyForm.codepoint, 27 }
		.enter { KeyForm.codepoint, 13 }
		.tab { KeyForm.codepoint, 9 }
		.backspace { KeyForm.codepoint, 127 }
		.up { KeyForm.letter, int(`A`) }
		.down { KeyForm.letter, int(`B`) }
		.right { KeyForm.letter, int(`C`) }
		.left { KeyForm.letter, int(`D`) }
		.home { KeyForm.letter, int(`H`) }
		.end { KeyForm.letter, int(`F`) }
		.insert { KeyForm.tilde, 2 }
		.delete { KeyForm.tilde, 3 }
		.page_up { KeyForm.tilde, 5 }
		.page_down { KeyForm.tilde, 6 }
		.f1 { KeyForm.letter, int(`P`) }
		.f2 { KeyForm.letter, int(`Q`) }
		.f3 { KeyForm.tilde, 13 }
		.f4 { KeyForm.letter, int(`S`) }
		.f5 { KeyForm.tilde, 15 }
		.f6 { KeyForm.tilde, 17 }
		.f7 { KeyForm.tilde, 18 }
		.f8 { KeyForm.tilde, 19 }
		.f9 { KeyForm.tilde, 20 }
		.f10 { KeyForm.tilde, 21 }
		.f11 { KeyForm.tilde, 23 }
		.f12 { KeyForm.tilde, 24 }
		else { none }
	}
}

// encode_key_event is the entry point: enhanced protocols first, the legacy table after.
// A release returns none unless a protocol asked to hear it.
pub fn encode_key_event(key platform.Key, mods platform.Mods, action KeyAction, m KeyInput) ?[]u8 {
	flags := m.flags
	all_esc := flags & kitty_all_as_esc != 0
	event_types := flags & kitty_event_types != 0
	legacy_c0 := key in [.enter, .tab, .backspace]
	text := key_generates_text(key, mods)
	reportable := event_types && (all_esc || !(legacy_c0 || text))
	if action == .release && !reportable {
		return none
	}
	mut applies := all_esc
	if !applies && flags != 0 && !legacy_c0 {
		applies = (action != .press && reportable) || (flags & kitty_disambiguate != 0 && !text)
	}
	if applies {
		return csi_u(key, mods, action, flags)
	}
	if b := modify_other(key, mods, m.other) {
		return b
	}
	return encode_key(key, mods, m.app_cursor)
}

fn key_generates_text(key platform.Key, mods platform.Mods) bool {
	_ := base_char(key) or { return false }
	return !mods.has(.ctrl) && !mods.has(.alt)
}

fn modifier_byte(mods platform.Mods) int {
	return 1 + (if mods.has(.shift) {
		1
	} else {
		0
	}) + (if mods.has(.alt) {
		2
	} else {
		0
	}) + (if mods.has(.ctrl) {
		4
	} else {
		0
	})
}

fn csi_u(key platform.Key, mods platform.Mods, action KeyAction, flags u8) ?[]u8 {
	form, code := key_form(key) or { return none }
	modifier := modifier_byte(mods)
	kind := if flags & kitty_event_types == 0 {
		1
	} else {
		match action {
			.press { 1 }
			.repeat { 2 }
			.release { 3 }
		}
	}
	field2 := match true {
		modifier == 1 && kind == 1 { '' }
		kind == 1 { '${modifier}' }
		else { '${modifier}:${kind}' }
	}

	mut seq := ''
	match form {
		.tilde {
			seq = if field2 == '' { '\x1b[${code}~' } else { '\x1b[${code};${field2}~' }
		}
		.letter {
			seq = if field2 == '' { '\x1b[${rune(code)}' } else { '\x1b[1;${field2}${rune(code)}' }
		}
		.codepoint {
			mut alt := ''
			if flags & kitty_alternates != 0 && mods.has(.shift) && code >= 32 && code < 127 {
				if sh := shifted_char(u8(code)) {
					alt = ':${int(sh)}'
				}
			}
			mut fields := ['${code}${alt}', field2, kitty_text_field(code, mods, action, flags)]
			for fields.last() == '' {
				fields.pop()
			}
			seq = '\x1b[${fields.join(';')}u'
		}
	}

	return seq.bytes()
}

// kitty_text_field is the associated text of a code-point report: nothing without the flag and
// report-all, on a release, with ctrl or alt, or for a control code. Quadra's keys carry one
// scalar, so the original's 32-scalar cap cannot be reached.
fn kitty_text_field(code int, mods platform.Mods, action KeyAction, flags u8) string {
	if flags & kitty_text == 0 || flags & kitty_all_as_esc == 0 || action == .release
		|| mods.has(.ctrl) || mods.has(.alt) || code < 32
		|| (code >= 0x7f && code <= 0x9f) {
		return ''
	}
	if mods.has(.shift) && code < 127 {
		if sh := shifted_char(u8(code)) {
			return '${int(sh)}'
		}
	}
	return '${code}'
}

fn modify_other(key platform.Key, mods platform.Mods, level int) ?[]u8 {
	if level == 0 || !(mods.has(.ctrl) || mods.has(.alt)) {
		return none
	}
	code := match key {
		.enter { 13 }
		.tab { 9 }
		.backspace { 127 }
		.escape { 27 }
		else { int(base_char(key) or { return none }) }
	}

	if level == 1 {
		if b := encode_key(key, mods, false) {
			last := if b.len > 1 && b[0] == 0x1b { b[1] } else { b[0] }
			if b.len <= 2 && (last < 0x20 || last == 0x7f) {
				return none
			}
		}
	}
	return '\x1b[27;${modifier_byte(mods)};${code}~'.bytes()
}
