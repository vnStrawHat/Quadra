// OneTerm: a terminal for local shells (specs/22).
//
//   oneterm [--data <folder>] [--stats] [--mem] [--tabs <n>]
//
// --data keeps every config file in one folder; development and tests always pass it.
module main

import os
import completion
import store
import vault
import quadra.core
import quadra.kit
import quadra.native
import quadra.platform

fn main() {
	root := arg_value('--data')
	shell := parse_elevated_shell(os.args) or {
		eprintln(err.msg())
		return
	}
	mut a := core.new_app(native.backends(force_warp: '--warp' in os.args)!, core.AppOptions{})!
	eprintln(a.plat.pty_host()) // which ConPTY host local shells get (specs/22)
	opts, mut one := prepare(mut a, root)!
	one.first_shell = shell
	one.crash_identity = install_crash_capture(mut a, data_dir(root))
	a.open_window(opts, one)!
	a.run()
}

fn arg_value(flag string) string {
	i := os.args.index(flag)
	return if i >= 0 && i + 1 < os.args.len { os.args[i + 1] } else { '' }
}

// prepare installs the kit, loads the config and binds the keys; the tests open the returned
// view in a harness.
pub fn prepare(mut a core.App, root string) !(platform.WindowOptions, OneTerm) {
	kit.install(mut a)!
	dir := data_dir(root)
	mut cfg := load_terminal_config(dir)
	cfg.normalize(dir)
	mut ui_cfg := load_ui_config(dir)
	ui_cfg.normalize()
	ui_cfg.key_bindings = without_defaults(resolve_default_collisions(ui_cfg.key_bindings))
	kit.set_theme(mut a, theme_for(dir, ui_cfg, cfg.colors))
	bind_keys(mut a, ui_cfg.key_bindings)!
	return platform.WindowOptions{
		title:           window_title(elevation)
		custom_titlebar: true
		width:           1600
		height:          1000
		min_width:       640
		min_height:      480
	}, OneTerm{
		cfg:         cfg
		ui:          ui_cfg
		dir:         dir
		themes:      theme_list(dir)
		update:      load_update_config(dir)
		vault:       vault.load_at_start(dir, is_restricted())
		history:     completion.new_history(cfg.completion.max_history)
		sessions:    store.open(dir, is_restricted())
		known_hosts: if root != '' { os.join_path(dir, 'known_hosts') } else { '' }
		local_home:  if root != '' { dir } else { '' }
		stats_on:    '--stats' in os.args
		mem:         MemProbe{
			on:   '--mem' in os.args
			tabs: arg_value('--tabs').int()
		}
		res:         &ResourceBox{}
	}
}
