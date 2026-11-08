// quadra.app glue: init, dirs under the root, logging, router, window state, close flow,
// dropped files.
module main

import os
import quadra.app
import quadra.core
import quadra.platform
import quadra.testing
import quadra.ui

fn fresh(name string) string {
	d := os.join_path(os.temp_dir(), 'quadra-tests', name)
	os.rmdir_all(d) or {}
	return d
}

fn test_init_creates_dirs_and_logs() {
	mut h := testing.new_harness()!
	root := fresh('services')
	s := app.init(mut h.app, app.AppInfo{
		id:      'quadra-test'
		root:    root
		locales: {
			'en': '{"hello": "Hello"}'
		}
	})!
	assert s.dirs.config.starts_with(root)
	assert os.is_dir(s.dirs.logs) && os.is_dir(s.dirs.crashes) && os.is_dir(s.dirs.data)
	mut log := s.log
	log.warn('test', 'something odd')
	log.close()
	logs := os.ls(s.dirs.logs)!
	assert logs.len == 1 && logs[0].starts_with('quadra-test-')
	text := os.read_file(os.join_path(s.dirs.logs, logs[0]))!
	assert text.contains('WARN  [test] something odd')
	cx := core.Cx{
		app: h.app
	}
	assert app.services(cx).info.id == 'quadra-test'
	assert app.t(cx, 'hello') == 'Hello'
}

fn test_router_history() {
	mut r := app.new_router[string]('home')
	r.push('note/1')
	r.push('note/2')
	assert r.back() && r.current() == 'note/1'
	r.push('settings') // drops note/2 from the forward history
	assert !r.can_forward()
	assert r.back() && r.back() && r.current() == 'home'
	assert !r.back()
	assert r.forward() && r.current() == 'note/1'
}

struct Editor {
mut:
	dirty   bool = true
	asked   int
	dropped []string
}

@[markused]
pub fn (mut e Editor) render(mut cx core.ViewCx[Editor]) core.Element {
	cx.on_close_requested(cx.listener(fn (mut e Editor, _ &core.CloseRequest, mut cx core.ViewCx[Editor]) {
		if e.dirty {
			e.asked++
			cx.prevent_close()
		}
	}))
	cx.on_files_dropped(cx.listener(fn (mut e Editor, d &platform.FileDropEvent, mut _ core.ViewCx[Editor]) {
		e.dropped << d.paths
	}))
	return ui.text('editor')
}

fn test_close_flow_and_file_drop() {
	mut h := testing.new_harness()!
	v := h.open(Editor{})!
	h.inject(platform.CloseRequested{})
	mut cx := core.Cx{
		app: h.app
	}
	e := cx.peek(v.model())
	assert e.asked == 1
	assert !h.window().pw.is_closed() // vetoed: unsaved changes
	h.inject(platform.FileDropEvent{
		paths: ['C:/a.md', 'C:/b.txt']
	})
	assert e.dropped == ['C:/a.md', 'C:/b.txt']
	cx.update_view(v, fn (mut e Editor, mut _ core.ViewCx[Editor]) {
		e.dirty = false
	})
	h.frame()
	h.inject(platform.CloseRequested{})
	assert h.window().pw.is_closed()
}

fn test_window_state_round_trip() {
	mut h := testing.new_harness()!
	root := fresh('winstate')
	os.mkdir_all(root)!
	h.open(Editor{
		dirty: false
	},
		width:  700
		height: 500
	)!
	mut cx := core.Cx{
		app: h.app
		win: h.window()
	}
	st := app.load_window_states(mut cx, root)
	app.remember_placement(mut cx, st, 'main')
	st.flush(mut cx)!
	mut cx2 := core.Cx{
		app: h.app
	}
	again := app.load_window_states(mut cx2, root)
	opts := app.with_placement(cx2, again, 'main', platform.WindowOptions{})
	assert opts.placement.valid && opts.placement.width == 700 && opts.placement.height == 500
}
