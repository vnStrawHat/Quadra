// Limits that need the view and its clock (specs/17, Limits): the timing of mode 2026 and the
// notifications of one chunk.
module main

import quadra.core
import quadra.kit
import quadra.kit.term
import quadra.platform
import quadra.testing
import quadra.ui

struct Host {
mut:
	term  core.View[term.Terminal]
	fake  &term.FakePty = unsafe { nil }
	infos []term.TerminalInfo
}

@[markused]
pub fn (mut h Host) render(mut cx core.ViewCx[Host]) core.Element {
	if h.term.id == 0 {
		h.term = term.new_terminal(mut cx.Cx, id: 't')
		h.fake = &term.FakePty{}
		term.start(mut cx.Cx, h.term, h.fake.spawner(), platform.PtyOptions{}) or { panic(err) }
		cx.subscribe(h.term.model(), fn (mut h Host, _ core.Model[term.Terminal], e &term.TerminalInfo, mut _ core.ViewCx[Host]) {
			if e.kind == .notification {
				h.infos << *e
			}
		})
	}
	return ui.div().size_full().child(cx.child_view(h.term))
}

fn open() !(&testing.Harness, &Host, &term.Terminal) {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Host{}, width: 640, height: 400)!
	mut cx := core.Cx{
		app: h.app
	}
	host := cx.peek(v.model())
	return h, host, cx.peek(host.term.model())
}

// send delivers output and lets the view take it at the current time.
fn send(mut h testing.Harness, mut fake term.FakePty, s string) {
	fake.output(s)
	h.frame()
}

const begin = '\x1b[?2026h'
const end = '\x1b[?2026l'

fn test_a_hold_lapses_150_ms_after_the_last_begin() {
	mut h, host, t := open()!
	mut fake := host.fake
	send(mut h, mut fake, begin + 'abc')
	h.advance(100)
	assert !t.screen.text().contains('abc')
	send(mut h, mut fake, begin + 'def') // refreshes, though it arrives held back
	h.advance(100)
	assert !t.screen.text().contains('abc')
	h.advance(60)
	assert t.screen.text().contains('abcdef') // frames resume
	assert t.screen.sync
	send(mut h, mut fake, 'x')
	assert t.screen.text().contains('abcdefx')
}

fn test_the_watchdog_ends_the_mode_a_second_after_the_first_begin() {
	mut h, host, t := open()!
	mut fake := host.fake
	send(mut h, mut fake, begin)
	for _ in 0 .. 9 { // refreshed all the time: the watchdog does not move
		h.advance(100)
		send(mut h, mut fake, begin + 'a')
	}
	assert t.screen.sync
	h.advance(110)
	assert !t.screen.sync
	assert t.screen.text().contains('aaaaaaaaa')
}

fn test_end_clears_the_watchdog() {
	mut h, host, t := open()!
	mut fake := host.fake
	send(mut h, mut fake, begin)
	h.advance(500)
	send(mut h, mut fake, end + begin) // a new update: its own second
	h.advance(600)
	assert t.screen.sync
	h.advance(410)
	assert !t.screen.sync
}

fn test_a_begin_after_a_lapse_holds_for_150_ms_again() {
	mut h, host, t := open()!
	mut fake := host.fake
	send(mut h, mut fake, begin)
	h.advance(200) // lapsed; the next timer is the watchdog's
	send(mut h, mut fake, begin + 'z')
	assert !t.screen.text().contains('z')
	h.advance(160)
	assert t.screen.text().contains('z') && t.screen.sync
}

fn notice(s string) string {
	return '\x1b]9;${s}\x07'
}

fn test_one_chunk_shows_the_last_five_notifications_and_says_how_many_went() {
	mut h, host, _ := open()!
	mut fake := host.fake
	mut out := ''
	for i in 0 .. 7 {
		out += notice('n${i}')
	}
	send(mut h, mut fake, out)
	assert host.infos.len == 6
	assert host.infos[0].warn && host.infos[0].text.starts_with('2 terminal notifications')
	assert host.infos[1..].map(it.text) == ['n2', 'n3', 'n4', 'n5', 'n6']
}

fn test_the_rate_limit_counts_only_what_survives_the_sanitizer() {
	mut h, host, _ := open()!
	mut fake := host.fake
	send(mut h, mut fake, notice('\x01').repeat(20)) // empty once cleaned: costs nothing
	assert host.infos.len == 0
	for i in 0 .. 12 {
		send(mut h, mut fake, notice('n${i}'))
	}
	assert host.infos.len == 10
}
