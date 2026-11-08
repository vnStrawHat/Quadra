module headless

import quadra.platform

struct Recorder {
mut:
	timers []u64
	frames int
	events int
}

fn (mut r Recorder) on_event(_ platform.WindowId, _ platform.Event) {
	r.events++
}

fn (mut r Recorder) on_frame(_ platform.WindowId) {
	r.frames++
}

fn (mut r Recorder) on_timer(id platform.TimerId) {
	r.timers << id
}

fn (mut r Recorder) hit_test(_ platform.WindowId, _ f32, _ f32) platform.HitRegion {
	return .client
}

fn test_virtual_clock_fires_timers_in_order() {
	mut p := new()
	mut rec := &Recorder{}
	p.set_sink(mut rec)
	a := p.set_timer(100, false)
	b := p.set_timer(30, true)
	p.advance(95)
	assert rec.timers == [u64(b), u64(b), u64(b)] // 30, 60, 90
	p.advance(10)
	assert rec.timers == [u64(b), u64(b), u64(b), u64(a)] // 100
	p.cancel_timer(b)
	p.advance(1000)
	assert rec.timers.len == 4
	assert p.now_ms() == 1105
}

fn test_frames_only_when_requested() {
	mut p := new()
	mut rec := &Recorder{}
	p.set_sink(mut rec)
	mut w := p.open_window(platform.WindowOptions{})!
	assert p.pump_frames() == 0
	w.request_frame()
	w.request_frame() // coalesced
	assert p.pump_frames() == 1
	assert p.pump_frames() == 0
	assert rec.frames == 1
}
