// Headless test harness: an App on the in-memory platform, the null GPU device and the
// deterministic text backend. Runs without any window station, desktop or GPU.
module testing

import time
import quadra.core
import quadra.prim
import quadra.gpu
import quadra.gpu.null
import quadra.platform
import quadra.platform.headless
import quadra.text
import quadra.text.fake

@[heap]
pub struct Harness {
pub mut:
	app  &core.App
	plat &headless.HeadlessPlatform
	dev  &null.NullDevice
	win  platform.WindowId
}

@[params]
pub struct HarnessOptions {
pub:
	width  f32 = 800
	height f32 = 600
	scale  f32 = 1
	text   ?text.Backend // default: the deterministic fake backend
}

pub fn new_harness(opts HarnessOptions) !&Harness {
	plat := headless.new()
	dev := null.new()
	tb := opts.text or { text.Backend(fake.new()) }
	app := core.new_app(core.Backends{
		platform: plat
		device:   dev
		text:     tb
	}, core.AppOptions{
		text_style: text.TextStyle{
			size:        14
			line_height: 1.5
		}
	})!
	return &Harness{
		app:  app
		plat: plat
		dev:  dev
	}
}

// open opens the test window with `root` as its root view and renders the first frame.
pub fn (mut h Harness) open[T](root T, opts HarnessOptions) !core.View[T] {
	v := h.app.open_window(platform.WindowOptions{
		width:  opts.width
		height: opts.height
	}, root)!
	h.win = h.app.windows.last().id
	if opts.scale != 1 {
		h.plat.set_scale(h.win, opts.scale)
	}
	h.frame()
	return v
}

pub fn (h &Harness) window() &core.Window {
	return h.app.window(h.win) or { panic('testing: no open window') }
}

// frame delivers pending frames until the app is idle; returns how many ran.
// Posted work (inbox deliveries, finished tasks) runs first, so streams that were pushed
// on the main thread (fake PTYs) show without waiting.
pub fn (mut h Harness) frame() int {
	mut n := 0
	for _ in 0 .. 16 {
		h.plat.run_posted()
		got := h.plat.pump_frames()
		if got == 0 {
			break
		}
		n += got
	}
	return n
}

// advance moves the virtual clock (timers fire in order), then renders.
pub fn (mut h Harness) advance(ms f64) {
	h.plat.advance(ms)
	h.frame()
}

pub fn (mut h Harness) inject(e platform.Event) {
	h.plat.inject(h.win, e)
	h.frame()
}

pub fn (mut h Harness) move_to(x f32, y f32) {
	h.inject(platform.MouseEvent{
		action: .move
		x:      x
		y:      y
	})
}

pub fn (mut h Harness) click_at(x f32, y f32) {
	h.move_to(x, y)
	h.inject(platform.MouseEvent{
		action: .down
		button: .left
		x:      x
		y:      y
	})
	h.inject(platform.MouseEvent{
		action: .up
		button: .left
		x:      x
		y:      y
	})
}

// click clicks the center of the element with the given test id.
pub fn (mut h Harness) click(test_id string) ! {
	r := h.find(test_id) or { return error('testing: no element with test_id "${test_id}"') }
	h.click_at(r.x + r.width / 2, r.y + r.height / 2)
}

pub fn (mut h Harness) scroll_at(x f32, y f32, dy f32) {
	h.inject(platform.WheelEvent{
		dy:      dy
		precise: true
		x:       x
		y:       y
	})
}

pub fn (mut h Harness) resize(w f32, hh f32) {
	h.plat.resize(h.win, w, hh)
	h.frame()
}

pub fn (h &Harness) find(test_id string) ?prim.Rect {
	return h.window().test_ids[test_id] or { return none }
}

pub fn (h &Harness) texts() []string {
	return h.window().texts.map(it.text)
}

pub fn (h &Harness) has_text(s string) bool {
	return h.window().texts.any(it.text == s)
}

pub fn (h &Harness) text_bounds(s string) ?prim.Rect {
	for t in h.window().texts {
		if t.text == s {
			return t.bounds
		}
	}
	return none
}

// renders_of counts how many times a view has rendered.
pub fn (h &Harness) renders_of[T](v core.View[T]) int {
	return h.window().stats.renders[v.id]
}

pub fn (h &Harness) builds() int {
	return h.window().stats.builds
}

pub fn (h &Harness) scene() gpu.Scene {
	if h.dev == unsafe { nil } {
		return h.window().scene
	}
	return gpu.Scene{
		instances: h.dev.last.instances.clone()
		xforms:    h.dev.last.xforms.clone()
	} // a snapshot: the device reuses its arrays
}

pub fn (h &Harness) cursor() platform.Cursor {
	return h.plat.windows[0].cursor
}

// press sends key presses: "ctrl-s", "tab", or a chord "ctrl-k ctrl-s".
pub fn (mut h Harness) press(keys string) ! {
	for part in keys.fields() {
		ks := core.parse_keystroke(part)!
		h.inject(platform.KeyEvent{
			key:  ks.key
			mods: ks.mods
			down: true
		})
		h.inject(platform.KeyEvent{
			key:  ks.key
			mods: ks.mods
		})
	}
}

// type_text delivers committed text one grapheme at a time, like a keyboard would.
pub fn (mut h Harness) type_text(s string) {
	mut i := 0
	for i < s.len {
		j := text.next_grapheme(s, i)
		h.inject(platform.TextEvent{s[i..j]})
		i = j
	}
}

// ime_compose simulates an IME session: each preedit step, then the commit.
pub fn (mut h Harness) ime_compose(steps []string, commit string) {
	h.inject(platform.ImeEvent{
		kind: .start
	})
	for s in steps {
		h.inject(platform.ImeEvent{
			kind:   .preedit
			text:   s
			cursor: s.len
		})
	}
	h.inject(platform.ImeEvent{
		kind: .commit
		text: commit
	})
	h.inject(platform.ImeEvent{
		kind: .end
	})
}

// find_role finds a semantic node by role and (optionally) label.
pub fn (h &Harness) find_role(role core.Role, label string) ?core.SemanticNode {
	for n in h.window().semantics {
		if n.role == role && (label == '' || n.label == label) {
			return n
		}
	}
	return none
}

pub fn (h &Harness) focused() u64 {
	return h.window().focused
}

// wait_tasks waits (in real time) until every background task has delivered its result,
// then renders. Virtual time does not move.
pub fn (mut h Harness) wait_tasks() ! {
	for _ in 0 .. 1000 {
		h.plat.run_posted()
		if h.app.tasks_running() == 0 {
			h.frame()
			return
		}
		time.sleep(2 * time.millisecond)
	}
	return error('testing: tasks still running after 2 s')
}

// wait_images waits (in real time) for background image decodes to finish, then renders.
pub fn (mut h Harness) wait_images() ! {
	for _ in 0 .. 400 {
		if h.app.images.pending == 0 {
			h.frame()
			return
		}
		time.sleep(5 * time.millisecond)
		h.plat.run_posted()
		h.frame()
	}
	return error('testing: images still decoding after 2 s')
}

// drag presses at (x0, y0), moves to (x1, y1) in a few steps and releases there. With
// release false the button stays down (to check a drag in progress).
pub fn (mut h Harness) drag(x0 f32, y0 f32, x1 f32, y1 f32, release bool) {
	h.move_to(x0, y0)
	h.inject(platform.MouseEvent{
		action: .down
		button: .left
		x:      x0
		y:      y0
	})
	for k in 1 .. 5 {
		f := f32(k) / 4
		h.move_to(x0 + (x1 - x0) * f, y0 + (y1 - y0) * f)
	}
	if release {
		h.inject(platform.MouseEvent{
			action: .up
			button: .left
			x:      x1
			y:      y1
		})
	}
}
