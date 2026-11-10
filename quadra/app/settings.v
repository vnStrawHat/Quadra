// Settings: a model persisted as JSON, saved debounced and atomically (specs/10).
module app

import json
import os
import time
import x.json2
import quadra.core
import quadra.platform

pub const save_delay_ms = 500

pub struct SettingsOptions[T] {
pub:
	version int // stored with the data; older files go through migrate
	// migrate turns the data of an older version (raw JSON) into the current type.
	migrate fn (from int, data string) !T = unsafe { nil }
	log     &Logger                       = unsafe { nil }
}

// Settings wraps a model: views that read it update when it changes.
pub struct Settings[T] {
pub:
	model core.Model[T]
	path  string
	state &SaveState
}

@[heap]
pub struct SaveState {
pub:
	path    string
	version int
	encode  fn (value voidptr) string = unsafe { nil }
mut:
	timer   u64
	pending string
	log     &Logger = unsafe { nil }
pub mut:
	saves int // statistics: files written
}

// load_settings reads <dir>/<name>.json. A missing file gives the default; a corrupt one is
// moved aside (<name>.json.corrupt-<unix time>) and logged, and the default is used.
pub fn load_settings[T](mut cx core.Cx, dir string, name string, default T, opts SettingsOptions[T]) Settings[T] {
	path := os.join_path(dir, '${name}.json')
	mut value := default
	if os.exists(path) {
		value = decode_settings[T](path, default, opts) or {
			aside := '${path}.corrupt-${time.now().unix()}'
			os.mv(path, aside) or {}
			if opts.log != unsafe { nil } {
				mut l := opts.log
				l.warn('settings', '${path}: ${err.msg()}; using defaults, kept ${aside}')
			}
			default
		}
	}
	return Settings[T]{
		model: cx.new_model(value)
		path:  path
		state: &SaveState{
			path:    path
			version: opts.version
			encode:  encode_value[T]
			log:     opts.log
		}
	}
}

// encode_value is stored as a function pointer: json.encode inside a method of the generic
// Settings[T] gets instantiated with the wrong T when several settings types exist.
fn encode_value[T](value voidptr) string {
	return json.encode(*unsafe { &T(value) })
}

fn decode_settings[T](path string, _ T, opts SettingsOptions[T]) !T {
	raw := json2.decode[json2.Any](os.read_file(path)!)!
	obj := raw.as_map()
	version := (obj['version'] or { json2.Any(0) }).int()
	data := (obj['data'] or { return error('no data field') }).json_str()
	if version < opts.version {
		if opts.migrate == unsafe { nil } {
			return error('version ${version} is older than ${opts.version} and there is no migration')
		}
		return opts.migrate(version, data)!
	}
	return json.decode(T, data)!
}

// get reads the current value (tracked: a view reading it re-renders on change).
pub fn (s Settings[T]) get(cx &core.Cx) &T {
	return cx.read(s.model)
}

// update changes the settings and schedules a save.
pub fn (s Settings[T]) update(mut cx core.Cx, f fn (mut v T, mut cx core.ViewCx[T])) {
	cx.update(s.model, f)
	s.saved(mut cx)
}

// update_with is Settings.update for a callback that needs data, without a closure
// (specs/14). A free function: a generic method on Settings[T] trips V (docs/v-notes.md).
pub fn update_with[T, D](mut cx core.Cx, s Settings[T], data D, f fn (mut v T, d D, mut cx core.ViewCx[T])) {
	cx.update_with(s.model, data, f)
	s.saved(mut cx)
}

fn (s Settings[T]) saved(mut cx core.Cx) {
	mut st := s.state
	text := st.encode(voidptr(cx.peek(s.model)))
	schedule_save(mut cx.app, mut st, text)
}

// flush writes a pending save now (call on quit).
pub fn (s Settings[T]) flush(mut cx core.Cx) ! {
	mut st := s.state
	if st.timer != 0 {
		cx.app.plat.cancel_timer(st.timer)
		st.timer = 0
	}
	st.write()!
}

struct SaveRef {
	st &SaveState
}

fn schedule_save(mut a core.App, mut st SaveState, text string) {
	st.pending = text
	if st.timer != 0 {
		mut cx := core.Cx{
			app: a
		}
		cx.cancel_timer(st.timer)
	}
	st.timer = a.after_with(save_delay_ms, SaveRef{st}, save_now)
}

fn save_now(r SaveRef, mut _ core.App) {
	mut st := r.st
	st.timer = 0
	st.write() or {
		if st.log != unsafe { nil } {
			mut l := st.log
			l.error('settings', err.msg())
		}
	}
}

// write saves atomically: a temp file next to the target, flushed, then moved over it.
fn (mut st SaveState) write() ! {
	if st.pending == '' {
		return
	}
	tmp := st.path + '.tmp'
	os.write_file(tmp, '{"version":${st.version},"data":${st.pending}}')!
	platform.replace_file(tmp, st.path)!
	st.pending = ''
	st.saves++
}
