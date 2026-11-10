// Key encoding: what the terminal sends to the program for a key press (xterm style).
// Printable text arrives as text input instead and is sent as UTF-8. Also the view's key events.
module term

import quadra.core
import quadra.platform

// encode_key returns the bytes for a key, or none when the key is plain text (sent by the
// text input event) or not a terminal key.
pub fn encode_key(key platform.Key, mods platform.Mods, app_cursor bool) ?[]u8 {
	ctrl, alt, shift := mods.has(.ctrl), mods.has(.alt), mods.has(.shift)
	m := 1 + (if shift { 1 } else { 0 }) + (if alt { 2 } else { 0 }) + (if ctrl { 4 } else { 0 })
	esc := if alt { [u8(0x1b)] } else { []u8{} }
	match key {
		.enter {
			return '${esc.bytestr()}\r'.bytes()
		}
		.backspace {
			mut out := esc.clone()
			out << if ctrl { u8(0x08) } else { u8(0x7f) }
			return out
		}
		.tab {
			return if shift { '\x1b[Z'.bytes() } else { '\t'.bytes() }
		}
		.escape {
			return [u8(0x1b)]
		}
		.up, .down, .right, .left, .home, .end {
			f := match key {
				.up { `A` }
				.down { `B` }
				.right { `C` }
				.left { `D` }
				.home { `H` }
				else { `F` }
			}

			if m > 1 {
				return '\x1b[1;${m}${rune(f)}'.bytes()
			}
			return if app_cursor { [u8(0x1b), `O`, f] } else { [u8(0x1b), `[`, f] }
		}
		.insert, .delete, .page_up, .page_down, .f5, .f6, .f7, .f8, .f9, .f10, .f11, .f12 {
			n := match key {
				.insert { 2 }
				.delete { 3 }
				.page_up { 5 }
				.page_down { 6 }
				.f5 { 15 }
				.f6 { 17 }
				.f7 { 18 }
				.f8 { 19 }
				.f9 { 20 }
				.f10 { 21 }
				.f11 { 23 }
				else { 24 }
			}

			return if m > 1 { '\x1b[${n};${m}~'.bytes() } else { '\x1b[${n}~'.bytes() }
		}
		.f1, .f2, .f3, .f4 {
			f := u8(`P`) + u8(int(key) - int(platform.Key.f1))
			return if m > 1 { '\x1b[1;${m}${rune(f)}'.bytes() } else { [u8(0x1b), `O`, f] }
		}
		else {}
	}

	if ctrl {
		mut out := esc.clone()
		if int(key) >= int(platform.Key.a) && int(key) <= int(platform.Key.z) {
			out << u8(int(key) - int(platform.Key.a) + 1)
			return out
		}
		c := match key {
			.space, .digit2 { u8(0) }
			.bracket_left, .digit3 { u8(0x1b) }
			.backslash, .digit4 { u8(0x1c) }
			.bracket_right, .digit5 { u8(0x1d) }
			.digit6 { u8(0x1e) }
			.minus, .slash, .digit7 { u8(0x1f) }
			else { return none }
		}

		out << c
		return out
	}
	if alt {
		ch := key_char(key, shift) or { return none }
		return [u8(0x1b), ch]
	}
	return none
}

// key_char is the ASCII a letter or digit key types (for alt+key, which sends no text).
fn key_char(key platform.Key, shift bool) ?u8 {
	if int(key) >= int(platform.Key.a) && int(key) <= int(platform.Key.z) {
		base := if shift { u8(`A`) } else { u8(`a`) }
		return base + u8(int(key) - int(platform.Key.a))
	}
	if int(key) >= int(platform.Key.digit0) && int(key) <= int(platform.Key.digit9) {
		return u8(`0`) + u8(int(key) - int(platform.Key.digit0))
	}
	return none
}

// paste_bytes wraps pasted text for programs that asked for bracketed paste; newlines
// become CR, as typed.
pub fn paste_bytes(s string, bracketed bool) []u8 {
	if bracketed {
		return '\x1b[200~${s.replace('\x1b[201~', '').replace('\x1b', '').replace('\x03', '')}\x1b[201~'.bytes()
	}
	return s.replace('\r\n', '\r').replace('\n', '\r').bytes()
}

fn (mut t Terminal) key(e &core.KeyDownEvent, mut cx core.ViewCx[Terminal]) {
	if t.hooks.key != unsafe { nil } && t.hooks.key(t.hooks.holder, mut t, e, mut cx) {
		cx.notify()
		cx.stop_propagation()
		return
	}
	if t.view_key(e, mut cx) {
		cx.notify()
		cx.stop_propagation()
		return
	}
	s := t.screen
	input := KeyInput{s.app_cursor, s.keyboard.active.live, s.modify_other}
	action := if e.repeat { KeyAction.repeat } else { KeyAction.press }
	if b := encode_key_event(e.key, e.mods, action, input) {
		// With flag 8 even printable keys go as CSI u: the text event that follows is dropped.
		t.eat_text = input.flags & kitty_all_as_esc != 0 && key_generates_text(e.key, e.mods)
		t.type_bytes(b)
		t.key_held[int(e.key)] = true
		cx.notify()
		cx.stop_propagation()
	}
}

// key_up sends a kitty release for a key whose press reached the program (specs/17).
fn (mut t Terminal) key_up(e &core.KeyUpEvent) {
	if int(e.key) !in t.key_held {
		return
	}
	t.key_held.delete(int(e.key))
	s := t.screen
	if b := encode_key_event(e.key, e.mods, .release, KeyInput{s.app_cursor, s.keyboard.active.live, s.modify_other}) {
		t.send(b)
	}
}

// view_key handles the keys the terminal keeps for itself: copy and paste, scrolling, zoom.
fn (mut t Terminal) view_key(e &core.KeyDownEvent, mut cx core.ViewCx[Terminal]) bool {
	cs := platform.Mods.ctrl | .shift
	n := t.screen.scrollback.len()
	if t.b.clipboard_keys && e.mods == cs && e.key == .c {
		t.copy(mut cx)
	} else if t.b.clipboard_keys && e.mods == cs && e.key == .v {
		t.paste_clipboard(mut cx.Cx)
	} else if e.mods == .shift && e.key == .insert {
		t.paste_clipboard(mut cx.Cx)
	} else if e.mods.has(.shift) && e.key == .page_up {
		t.offset = clamp(t.offset + t.screen.rows - 1, 0, n)
	} else if e.mods.has(.shift) && e.key == .page_down {
		t.offset = clamp(t.offset - t.screen.rows + 1, 0, n)
	} else if e.mods == .shift && e.key == .home {
		t.offset = n
	} else if e.mods == .shift && e.key == .end {
		t.offset = 0
	} else if e.mods.has(.ctrl) && !e.mods.has(.alt) && e.key == .equal {
		t.zoom += 1 // shift too: `ctrl-+`
	} else if e.mods.has(.ctrl) && !e.mods.has(.alt) && e.key == .minus {
		t.zoom -= 1
	} else if e.mods.has(.ctrl) && !e.mods.has(.alt) && e.key == .digit0 {
		t.zoom = 0
	} else {
		return false
	}
	return true
}
