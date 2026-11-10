// Headless platform: in-memory windows, a virtual clock and scripted input.
// Pure V: needs no window station, desktop, GPU or OS at all.
module headless

import sync
import quadra.prim
import quadra.platform

struct NopSink {}

fn (mut s NopSink) on_event(_ platform.WindowId, _ platform.Event) {}

fn (mut s NopSink) on_frame(_ platform.WindowId) {}

fn (mut s NopSink) on_timer(_ platform.TimerId) {}

fn (mut s NopSink) hit_test(_ platform.WindowId, _ f32, _ f32) platform.HitRegion {
	return .client
}

struct Timer {
mut:
	id       platform.TimerId
	due      f64
	interval f64
	repeat   bool
}

pub type Task = fn ()

@[heap]
pub struct HeadlessPlatform {
mut:
	sink       platform.EventSink = NopSink{}
	timers     []Timer
	next_timer u64
	posted     []Task
	post_lock  &sync.Mutex = sync.new_mutex()
	next_id    u32
pub mut:
	clock     f64 // virtual ms
	windows   []&HeadlessWindow
	clipboard string
	// picks are the answers pick_files gives, oldest first; opened records open_external and
	// reveal calls.
	picks  [][]string
	opened []string
	// answers are what message_box returns, oldest first (cancel when none); messages records
	// the boxes shown.
	answers  []platform.MessageResult
	messages []platform.MessageBox
	tray     string   // the tray tooltip, '' = no icon
	notices  []string // 'title: body' of each notify
	quit_    bool
	look     platform.Appearance
}

pub fn new() &HeadlessPlatform {
	return &HeadlessPlatform{}
}

pub fn (mut p HeadlessPlatform) set_sink(mut sink platform.EventSink) {
	p.sink = sink
}

pub fn (mut p HeadlessPlatform) run() {
	// Nothing to block on: tests drive the platform with advance() and pump_frames().
}

pub fn (mut p HeadlessPlatform) quit() {
	p.quit_ = true
}

pub fn (mut p HeadlessPlatform) open_window(opts platform.WindowOptions) !platform.PlatformWindow {
	p.next_id++
	w := &HeadlessWindow{
		id:    p.next_id
		title: opts.title
		sz:    if opts.placement.valid {
			prim.size(opts.placement.width, opts.placement.height)
		} else {
			prim.size(opts.width, opts.height)
		}
		place: opts.placement
	}
	p.windows << w
	return w
}

// post may be called from worker threads (task completions).
pub fn (mut p HeadlessPlatform) post(task fn ()) {
	p.post_lock.@lock()
	p.posted << task
	p.post_lock.unlock()
}

pub fn (mut p HeadlessPlatform) set_timer(ms int, repeat bool) platform.TimerId {
	p.next_timer++
	p.timers << Timer{p.next_timer, p.clock + f64(ms), f64(ms), repeat}
	return p.next_timer
}

pub fn (mut p HeadlessPlatform) cancel_timer(id platform.TimerId) {
	p.timers = p.timers.filter(it.id != id)
}

pub fn (p &HeadlessPlatform) now_ms() f64 {
	return p.clock
}

pub fn (p &HeadlessPlatform) clipboard_read() ?string {
	return if p.clipboard == '' { none } else { p.clipboard }
}

pub fn (mut p HeadlessPlatform) clipboard_write(s string) {
	p.clipboard = s
}

// claim_single_instance: a headless app is always the only instance.
pub fn (mut p HeadlessPlatform) claim_single_instance(_ string, _ []string) bool {
	return true
}

// install_crash_handler: tests report crashes through the test runner instead.
pub fn (mut p HeadlessPlatform) install_crash_handler(_ string, _ fn () string) {}

pub fn (p &HeadlessPlatform) is_headless() bool {
	return true
}

// run_posted executes queued cross-thread tasks.
pub fn (mut p HeadlessPlatform) run_posted() {
	for {
		p.post_lock.@lock()
		tasks := p.posted
		p.posted = []Task{}
		p.post_lock.unlock()
		if tasks.len == 0 {
			break
		}
		for t in tasks {
			t()
		}
	}
}

// advance moves the virtual clock forward, firing timers in due order.
pub fn (mut p HeadlessPlatform) advance(ms f64) {
	target := p.clock + ms
	for {
		mut next := -1
		for i, t in p.timers {
			if t.due <= target && (next < 0 || t.due < p.timers[next].due) {
				next = i
			}
		}
		if next < 0 {
			break
		}
		t := p.timers[next]
		p.clock = t.due
		if t.repeat {
			p.timers[next].due += t.interval
		} else {
			p.timers.delete(next)
		}
		p.sink.on_timer(t.id)
		p.run_posted()
	}
	p.clock = target
}

// pump_frames delivers on_frame to every window that requested one. Returns how many.
pub fn (mut p HeadlessPlatform) pump_frames() int {
	mut n := 0
	for mut w in p.windows {
		if w.frame_requested && !w.closed {
			w.frame_requested = false
			p.sink.on_frame(w.id)
			n++
		}
	}
	return n
}

pub fn (p &HeadlessPlatform) has_pending_frames() bool {
	return p.windows.any(it.frame_requested && !it.closed)
}

pub fn (p &HeadlessPlatform) timer_count() int {
	return p.timers.len
}

// inject delivers an event as if the OS had produced it.
pub fn (mut p HeadlessPlatform) inject(id platform.WindowId, e platform.Event) {
	p.sink.on_event(id, e)
	p.run_posted()
}

// resize changes a window's logical size and reports it.
pub fn (mut p HeadlessPlatform) resize(id platform.WindowId, w f32, h f32) {
	for mut win in p.windows {
		if win.id == id {
			win.sz = prim.size(w, h)
		}
	}
	p.inject(id, platform.ResizeEvent{w, h})
}

pub fn (mut p HeadlessPlatform) set_scale(id platform.WindowId, s f32) {
	for mut win in p.windows {
		if win.id == id {
			win.scale_ = s
		}
	}
	p.inject(id, platform.ScaleEvent{s})
}

pub fn (p &HeadlessPlatform) appearance() platform.Appearance {
	return p.look
}

// set_appearance changes the simulated system look and notifies every window.
pub fn (mut p HeadlessPlatform) set_appearance(a platform.Appearance) {
	p.look = a
	for w in p.windows {
		p.sink.on_event(w.id, platform.AppearanceChanged{})
	}
	p.run_posted()
}

pub fn (p &HeadlessPlatform) pty_host() string {
	return ''
}

pub fn (mut p HeadlessPlatform) spawn_pty(opts platform.PtyOptions) !platform.Pty {
	return error('headless: no pseudo terminal (use term.FakePty in tests)')
}

// pick_files returns the next queued answer (tests push to `picks`), or none: cancelled.
pub fn (mut p HeadlessPlatform) pick_files(_ platform.FileDialog, _ platform.WindowId) []string {
	if p.picks.len == 0 {
		return []
	}
	return p.picks.pop_left()
}

pub fn (mut p HeadlessPlatform) open_external(target string) bool {
	p.opened << target
	return true
}

pub fn (mut p HeadlessPlatform) reveal(path string) bool {
	p.opened << 'reveal:' + path
	return true
}

pub fn (mut p HeadlessPlatform) message_box(m platform.MessageBox, _ platform.WindowId) platform.MessageResult {
	p.messages << m
	return if p.answers.len == 0 { .cancel } else { p.answers.pop_left() }
}

pub fn (mut p HeadlessPlatform) set_tray(tooltip string) bool {
	p.tray = tooltip
	return true
}

pub fn (mut p HeadlessPlatform) notify(title string, body string) bool {
	p.notices << '${title}: ${body}'
	return true
}
