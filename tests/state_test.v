// Models, dependency tracking, ownership, observe/subscribe/emit and owned timers.
module main

import quadra.core
import quadra.testing
import quadra.ui

struct Doc {
mut:
	title string
	saves int
}

struct Saved {
	path string
}

struct Shell {
mut:
	doc       core.Model[Doc]
	reader    core.View[Reader]
	bystander core.View[Bystander]
	log       []string
}

struct Reader {
	doc core.Model[Doc]
}

struct Bystander {}

@[markused]
pub fn (mut s Shell) render(mut cx core.ViewCx[Shell]) core.Element {
	if s.reader.id == 0 {
		s.doc = cx.new_model(Doc{
			title: 'draft'
		})
		s.reader = cx.new_view(Reader{s.doc})
		s.bystander = cx.new_view(Bystander{})
		core.observe(mut cx, s.doc, fn (mut s Shell, m core.Model[Doc], mut cx core.ViewCx[Shell]) {
			s.log << 'observed ${cx.peek(m).title}'
		})
		cx.subscribe(s.doc, fn (mut s Shell, _ core.Model[Doc], e &Saved, mut _ core.ViewCx[Shell]) {
			s.log << 'saved ${e.path}'
		})
	}
	return ui.div().flex_col().child(cx.child_view(s.reader)).child(cx.child_view(s.bystander))
}

@[markused]
pub fn (mut r Reader) render(mut cx core.ViewCx[Reader]) core.Element {
	return ui.text('title: ${cx.read(r.doc).title}')
}

@[markused]
pub fn (mut b Bystander) render(mut cx core.ViewCx[Bystander]) core.Element {
	return ui.text('static')
}

fn setup() (&testing.Harness, core.View[Shell]) {
	mut h := testing.new_harness() or { panic(err) }
	v := h.open(Shell{}) or { panic(err) }
	return h, v
}

fn shell(h &testing.Harness, v core.View[Shell]) &Shell {
	cx := core.Cx{
		app: h.app
	}
	return cx.peek(v.model())
}

fn test_only_readers_rerender() {
	mut h, v := setup()
	s := shell(h, v)
	assert h.has_text('title: draft')
	r0, b0 := h.renders_of(s.reader), h.renders_of(s.bystander)
	mut cx := core.Cx{
		app: h.app
	}
	cx.update(s.doc, fn (mut d Doc, mut _ core.ViewCx[Doc]) {
		d.title = 'final'
	})
	h.frame()
	assert h.has_text('title: final')
	assert h.renders_of(s.reader) == r0 + 1
	assert h.renders_of(s.bystander) == b0 // did not read the doc
	cx.update_silent(s.doc, fn (mut d Doc, mut _ core.ViewCx[Doc]) {
		d.saves++
	})
	h.frame()
	assert h.renders_of(s.reader) == r0 + 1 // silent updates notify nobody
}

fn test_events_are_delivered_before_observers() {
	mut h, v := setup()
	s := shell(h, v)
	mut cx := core.Cx{
		app: h.app
	}
	cx.update(s.doc, fn (mut d Doc, mut mcx core.ViewCx[Doc]) {
		d.title = 'v2'
		mcx.emit(Saved{'a.txt'})
	})
	assert shell(h, v).log == ['saved a.txt', 'observed v2']
}

fn test_release_cascades_to_owned_models() {
	mut h, v := setup()
	s := shell(h, v)
	doc := s.doc
	reader := s.reader
	assert h.app.is_alive(doc.id, doc.gen)
	mut cx := core.Cx{
		app: h.app
	}
	cx.release(v.model())
	assert !h.app.is_alive(doc.id, doc.gen)
	assert !h.app.is_alive(reader.id, reader.gen)
	if _ := cx.try_read(doc) {
		assert false, 'stale handle must not resolve'
	}
}

struct Ticker {
mut:
	ticks int
}

@[markused]
pub fn (mut t Ticker) render(mut cx core.ViewCx[Ticker]) core.Element {
	if t.ticks == 0 {
		cx.set_interval(100, fn (mut t Ticker, mut _ core.ViewCx[Ticker]) {
			t.ticks++
		})
		t.ticks = 1
	}
	return ui.text('ticks ${t.ticks}')
}

fn test_owned_timers_run_and_die_with_their_owner() {
	mut h := testing.new_harness()!
	v := h.open(Ticker{})!
	h.advance(350)
	assert h.has_text('ticks 4') // 1 + three intervals
	assert h.plat.timer_count() == 1
	mut cx := core.Cx{
		app: h.app
	}
	cx.release(v.model())
	assert h.plat.timer_count() == 0
}
