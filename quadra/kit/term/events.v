// What the view does with the sequences a chunk produced, and two font behaviors.
module term

import time
import quadra.core
import quadra.kit
import quadra.text

const sync_timeout_ms = 150
const sync_watchdog_ms = 1000
const max_title = 4096
const max_cwd = 8192
const max_product = 64
const max_notices_per_s = 10
const max_queued_notices = 5

// NoticeGate is the OSC 9 rate limit: ten in each fixed one-second window.
struct NoticeGate {
mut:
	start i64 = -1000
	n     int
}

fn (mut g NoticeGate) allow(now i64) bool {
	if now - g.start >= 1000 {
		g.start, g.n = now, 0
	}
	if g.n >= max_notices_per_s {
		return false
	}
	g.n++
	return true
}

// drain_events turns the screen's event queue into TerminalInfo events.
fn (mut t Terminal) drain_events(mut cx core.ViewCx[Terminal]) {
	if t.screen.events.len == 0 {
		return
	}
	mut events := t.screen.events.clone()
	t.screen.events.clear()
	mut shown := []string{}
	mut dropped := 0
	for ev in events {
		if ev.kind != .notification {
			cx.emit(ev)
			continue
		}
		body := sanitize_notification(ev.text)
		if body == '' || !t.notices.allow(time.ticks()) {
			continue
		}
		if shown.len >= max_queued_notices {
			shown.delete(0)
			dropped++
		}
		shown << body
	}
	if dropped > 0 {
		cx.emit(TerminalInfo{
			kind: .notification
			warn: true
			text: '${dropped} terminal notifications were dropped while the UI was busy.'
		})
	}
	for body in shown {
		cx.emit(TerminalInfo{ kind: .notification, text: body })
	}
}

// clean_text removes C0 controls (tab, CR and LF stay with keep_ws), DEL, C1 and BiDi controls
// and cuts to max bytes on a UTF-8 boundary: what a program may put in the UI.
pub fn clean_text(s string, max int, keep_ws bool) string {
	mut out := []rune{cap: s.len}
	for r in s.runes() {
		if (keep_ws && (r == `\t` || r == `\n` || r == `\r`))
			|| (r >= 32 && r != 0x7f && !(r >= 0x80 && r < 0xa0) && !is_bidi_control(u32(r))) {
			out << r
		}
	}
	txt := out.string()
	if txt.len <= max {
		return txt
	}
	mut end := max
	for end > 0 && txt[end] & 0xc0 == 0x80 {
		end--
	}
	return txt[..end]
}

// sanitize_notification is the text of an OSC 9 notification.
pub fn sanitize_notification(s string) string {
	return clean_text(s, 8192, true)
}

// clean_product is a product name a reply may carry: no controls, 64 bytes.
fn clean_product(s string, fallback string) string {
	mut out := ''
	for r in s.runes() {
		if r < 0x20 || (r >= 0x7f && r <= 0x9f) {
			continue
		}
		if out.len + r.length_in_bytes() > max_product {
			break
		}
		out += r.str()
	}
	return if out == '' { fallback } else { out }
}

// SyncClock times mode 2026 (specs/17, Parser): `until` is when the hold lapses, `watch` when
// the mode is forced off, `next` when the latest timer fires (0: none).
struct SyncClock {
mut:
	seen  u32
	until f64
	watch f64
	next  f64
}

// hold_sync notes what a chunk did to mode 2026 and sets the timer.
fn (mut t Terminal) hold_sync(mut cx core.ViewCx[Terminal]) {
	if t.screen.sync_ended {
		t.screen.sync_ended = false
		t.sync.watch = 0
	}
	if !t.screen.sync {
		return
	}
	now := cx.app.plat.now_ms()
	if t.sync.watch == 0 {
		t.sync.watch = now + sync_watchdog_ms
	}
	if t.screen.sync_opens != t.sync.seen {
		t.sync.seen = t.screen.sync_opens
		t.sync.until = now + sync_timeout_ms
	}
	t.arm_sync(now, mut cx)
}

// arm_sync starts a timer for the next deadline unless one fires by then. A timer that turns
// out early only looks at the clock.
fn (mut t Terminal) arm_sync(now f64, mut cx core.ViewCx[Terminal]) {
	at := if t.screen.sync_hold && t.sync.until < t.sync.watch { t.sync.until } else { t.sync.watch }
	if t.sync.next != 0 && t.sync.next <= at {
		return
	}
	t.sync.next = at
	cx.set_timeout(int(at - now) + 1, sync_tick)
}

fn sync_tick(mut t Terminal, mut cx core.ViewCx[Terminal]) {
	now := cx.app.plat.now_ms()
	if now >= t.sync.next {
		t.sync.next = 0
	}
	if !t.screen.sync {
		t.sync.watch = 0
		return
	}
	watchdog := now >= t.sync.watch
	if watchdog || (t.screen.sync_hold && now >= t.sync.until) {
		t.flush_sync()
		t.screen.answer_queries(kit.theme_of(cx.app).code.terminal)
		t.flush_replies()
		t.drain_events(mut cx)
		cx.emit(TerminalInfo{})
		cx.notify()
	}
	if watchdog {
		t.screen.sync, t.sync.watch = false, 0
		return
	}
	t.arm_sync(now, mut cx)
}

// flush_sync ends the hold of a synchronized update and parses what it kept; the mode stays
// set until `l` or the watchdog.
pub fn (mut t Terminal) flush_sync() {
	if t.parser.held_on {
		t.screen.sync_replay = true
		t.feed(t.parser.take_held())
		t.screen.sync_replay = false
	}
	t.screen.sync_hold = false
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
