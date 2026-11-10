// What the view does with the sequences a chunk produced, and two font behaviors.
module term

import time
import quadra.core
import quadra.text

const sync_timeout_ms = 150

// drain_events turns the screen's event queue into TerminalInfo events.
fn (mut t Terminal) drain_events(mut cx core.ViewCx[Terminal]) {
	if t.screen.events.len == 0 {
		return
	}
	mut events := t.screen.events.clone()
	t.screen.events.clear()
	for ev in events {
		if ev.kind == .notification {
			if !t.rate_allows() {
				continue
			}
			cx.emit(TerminalInfo{
				kind: .notification
				text: sanitize_notification(ev.text)
			})
		} else {
			cx.emit(ev)
		}
	}
}

// rate_allows limits OSC 9 toasts to ten per second.
fn (mut t Terminal) rate_allows() bool {
	now := time.ticks()
	if t.toast_n >= 10 && now - t.toast_at[t.toast_n % 10] < 1000 {
		return false
	}
	t.toast_at[t.toast_n % 10] = now
	t.toast_n++
	return true
}

// sanitize_notification drops control and BiDi-override characters and caps the length.
pub fn sanitize_notification(s string) string {
	mut out := []rune{cap: s.len}
	for r in s.runes() {
		if r == `\t` || r == `\n` || r == `\r`
			|| (r >= 32 && r != 0x7f && !(r >= 0x80 && r < 0xa0) && !(r >= 0x202a && r <= 0x202e)
			&& !(r >= 0x2066 && r <= 0x2069)) {
			out << r
		}
	}
	mut txt := out.string()
	if txt.len > 8192 {
		mut end := 8192
		for end > 0 && txt[end] & 0xc0 == 0x80 {
			end--
		}
		txt = txt[..end]
	}
	return txt
}

// hold_sync starts the timer that ends a synchronized update (mode 2026) a program never
// closed. While one is open the parser holds the bytes back (Parser.feed), so the screen
// stays at the frame before it.
fn (mut t Terminal) hold_sync(mut cx core.ViewCx[Terminal]) {
	if t.parser.held_on && !t.sync_timer {
		t.sync_timer = true
		cx.set_timeout(sync_timeout_ms, fn (mut t Terminal, mut cx core.ViewCx[Terminal]) {
			t.sync_timer = false
			t.flush_sync()
			cx.notify()
		})
	}
}

// flush_sync gives up on an open synchronized update and parses what was held back.
pub fn (mut t Terminal) flush_sync() {
	if t.parser.held_on {
		t.screen.sync, t.parser.held_on = false, false
		held := t.parser.held
		t.parser.held = []u8{}
		t.feed(held)
	}
}

// fallback_layout lays a non-ASCII cell out in the first fallback family that has its
// glyphs; the system fallback is the last resort, as without any configured.
fn (t &Terminal) fallback_layout(s string, st text.TextStyle, mut cx core.PaintCx) &text.LineLayout {
	mut ts := cx.text
	first := ts.layout_line(s, st)
	if layout_has_face(first, ts.face_for(st)) {
		return first
	}
	for fam in t.b.fallbacks {
		style := text.TextStyle{
			...st
			family: fam
		}
		l := ts.layout_line(s, style)
		if layout_has_face(l, ts.face_for(style)) {
			return l
		}
	}
	return first
}

fn layout_has_face(l &text.LineLayout, face text.FaceId) bool {
	for g in l.glyphs {
		if g.face != face {
			return false
		}
	}
	return true
}
