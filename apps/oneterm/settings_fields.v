// Every field the settings pages reach, by key: one place maps keys to config storage.
module main

import json
import os
import quadra.core
import quadra.kit
import x.json2

// number_ranges: min, max, step, decimals.
const number_ranges = {
	'ui_font':    [f64(8), 32, 1, 0]
	'font_size':  [f64(6), 72, 1, 0]
	'line_h':     [f64(1), 3, 0.1, 1]
	'scrollback': [f64(0), 1000000, 1000, 0]
	'wheel':      [f64(0), 20, 0.5, 1]
	'max_hist':   [f64(0), 100000, 10, 0]
	'min_prefix': [f64(0), 16, 1, 0]
	'visible':    [f64(1), 32, 1, 0]
	'ka_secs':    [f64(5), 3600, 1, 0]
	'ka_max':     [f64(1), 20, 1, 0]
	'edit_mb':    [f64(0), 4096, 1, 0]
	'lock_mins':  [f64(1), 480, 1, 0]
	'check_h':    [f64(1), 8760, 1, 0]
}

fn (mut o OneTerm) bool_ref(key string) &bool {
	return match key {
		'show_fps' { &o.ui.show_fps }
		'vertical_tabs' { &o.ui.vertical_tabs.enabled }
		'utf8' { &o.cfg.shell.utf8 }
		'ligatures' { &o.cfg.font.ligatures }
		'blink' { &o.cfg.cursor.blink }
		'gutter' { &o.cfg.layout.show_gutter }
		'alt_scroll' { &o.cfg.scroll.alternate_scroll }
		'bell' { &o.cfg.bell.enabled }
		'context_menu' { &o.cfg.mouse.show_context_menu }
		'copy_select' { &o.cfg.mouse.copy_on_select }
		'middle' { &o.cfg.mouse.middle_click_paste }
		'clip_read' { &o.cfg.security.allow_clipboard_read }
		'log_local' { &o.cfg.logging.local }
		'log_ssh' { &o.cfg.logging.ssh }
		'cmp_on' { &o.cfg.completion.enabled }
		'cmp_tab' { &o.cfg.completion.accept_tab }
		'cmp_mem' { &o.cfg.completion.sources.memory }
		'cmp_man' { &o.cfg.completion.sources.manual }
		'cmp_ext' { &o.cfg.completion.sources.external }
		'cmp_fuzzy' { &o.cfg.completion.fuzzy }
		'cmp_alt' { &o.cfg.completion.disable_in_alt_screen }
		'cmp_core' { &o.cfg.completion.windows_allow_coreutils }
		'ka_on' { &o.cfg.ssh.keepalive_enabled }
		'auto_check' { &o.update.auto_check }
		'verify_tls' { &o.update.verify_certificates }
		else { unsafe { nil } }
	}
}

fn (mut o OneTerm) text_ref(key string) &string {
	return match key {
		'mode' { &o.ui.theme_name } // not read: the Mode row is built by hand
		'kind' { &o.cfg.shell.kind }
		'program' { &o.cfg.shell.program }
		'weight' { &o.cfg.font.weight }
		'shape' { &o.cfg.cursor.shape }
		'cur_color' { &o.cfg.cursor.color }
		'highlight' { &o.cfg.layout.semantic_highlighting }
		'tab_title' { &o.cfg.layout.tab_title }
		'log_dir' { &o.cfg.logging.directory }
		'log_mode' { &o.cfg.logging.write_mode }
		'force_family' { &o.cfg.completion.force_family }
		'editor_mode' { &o.cfg.sftp.editor.mode }
		'editor_prog' { &o.cfg.sftp.editor.program }
		'channel' { &o.update.channel }
		'proxy' { &o.update.proxy_url }
		'skipped' { &o.update.skipped_version }
		else { &o.ui.theme_name }
	}
}

// text_of and set_text also handle the comma- and space-separated list fields.
fn (mut o OneTerm) text_of(key string) string {
	return match key {
		'fallbacks' { o.cfg.font.fallbacks.join(', ') }
		'editor_args' { o.cfg.sftp.editor.args.join(' ') }
		else { *o.text_ref(key) }
	}
}

fn (mut o OneTerm) set_text(key string, v string) {
	match key {
		'fallbacks' {
			o.cfg.font.fallbacks = v.split(',').map(it.trim_space()).filter(it != '')
		}
		'editor_args' {
			o.cfg.sftp.editor.args = v.fields()
		}
		'proxy', 'skipped' {
			mut r := o.text_ref(key)
			unsafe {
				*r = v.trim_space()
			}
		}
		'program' {
			o.cfg.shell.program = if v.trim_space() == '' { '' } else { v }
		}
		else {
			mut r := o.text_ref(key)
			unsafe {
				*r = v
			}
		}
	}
}

fn (mut o OneTerm) number_of(key string) f64 {
	return match key {
		'ui_font' { f64(o.ui.ui_font_size) }
		'font_size' { f64(o.cfg.font.size) }
		'line_h' { f64(o.cfg.layout.line_height) }
		'scrollback' { f64(o.cfg.scroll.scrollback_history) }
		'wheel' { f64(o.cfg.scroll.multiplier) }
		'max_hist' { f64(o.cfg.completion.max_history) }
		'min_prefix' { f64(o.cfg.completion.min_prefix_len) }
		'visible' { f64(o.cfg.completion.max_visible_items) }
		'ka_secs' { f64(o.cfg.ssh.keepalive_interval_secs) }
		'ka_max' { f64(o.cfg.ssh.keepalive_max) }
		'edit_mb' { f64(o.cfg.sftp.edit_max_file_size) / 1048576 }
		'lock_mins' { f64(o.ui.lock_timeout_minutes()) }
		'check_h' { f64(o.update.check_interval_hours) }
		else { 0 }
	}
}

fn (mut o OneTerm) set_number(key string, v f64) {
	match key {
		'ui_font' {
			o.ui.ui_font_size = f32(v)
		}
		'font_size' {
			o.cfg.font.size = f32(v)
		}
		'line_h' {
			o.cfg.layout.line_height = f32(v)
		}
		'scrollback' {
			o.cfg.scroll.scrollback_history = int(v)
		}
		'wheel' {
			o.cfg.scroll.multiplier = f32(v)
		}
		'max_hist' {
			o.cfg.completion.max_history = int(v)
		}
		'min_prefix' {
			o.cfg.completion.min_prefix_len = int(v)
		}
		'visible' {
			o.cfg.completion.max_visible_items = int(v)
		}
		'ka_secs' {
			o.cfg.ssh.keepalive_interval_secs = int(v)
		}
		'ka_max' {
			o.cfg.ssh.keepalive_max = int(v)
		}
		'edit_mb' {
			o.cfg.sftp.edit_max_file_size = i64(v * 1048576)
		}
		'lock_mins' {
			o.ui.lock_after_idle_minutes = int(v)
		}
		'check_h' {
			o.update.check_interval_hours = int(v)
		}
		else {}
	}
}

// save_update merges into update_config.json, keeping the updater's cache fields.
fn (o &OneTerm) save_update() {
	if is_restricted() {
		return
	}
	path := os.join_path(o.dir, 'update_config.json')
	mut root := map[string]json2.Any{}
	if text := os.read_file(path) {
		if raw := json2.decode[json2.Any](text) {
			root = raw.as_map().clone()
		}
	} else if os.exists(path) {
		return
	}
	u := o.update
	root['auto_check'] = json2.Any(u.auto_check)
	root['channel'] = json2.Any(u.channel)
	root['check_interval_hours'] = json2.Any(u.check_interval_hours)
	root['proxy_url'] = json2.Any(u.proxy_url)
	root['verify_certificates'] = json2.Any(u.verify_certificates)
	root['skipped_version'] = json2.Any(u.skipped_version)
	write_config(path, json2.encode(root)) or {}
}

// persist writes each config file unless reading it failed at startup or this window is
// elevated. Every setting reaches the disk through here or persist_ui.
fn (o &OneTerm) persist() ! {
	if is_restricted() {
		return
	}
	if !o.cfg.persist_blocked {
		write_config(os.join_path(o.dir, 'terminal.json'), json.encode_pretty(o.cfg))!
	}
	o.persist_ui()!
}

// persist_ui writes ui_config.json only (the shell chrome changes it often).
fn (o &OneTerm) persist_ui() ! {
	if !is_restricted() && !o.ui.persist_blocked {
		write_config(os.join_path(o.dir, 'ui_config.json'), json.encode_pretty(o.ui))!
	}
}

// apply_and_persist puts the settings into effect and saves them, reporting a failed save.
fn (mut o OneTerm) apply_and_persist(mut cx core.Cx) {
	o.apply(mut cx)
	o.persist() or { kit.notify(mut cx, .error, 'Settings not saved', err.msg()) }
}
