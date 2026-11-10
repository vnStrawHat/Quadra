// The config blocks whose engines are other modules, and ui_config's shell blocks (specs/22,
// Config). Every key and default matches the original; a value of 0 or '' means "not set".
module main

import json
import os

pub struct CompletionSources {
pub mut:
	memory   bool = true
	manual   bool = true
	external bool = true
}

pub struct CompletionConfig {
pub mut:
	enabled                  bool = true
	accept_tab               bool = true
	max_history              int  = 500
	min_prefix_len           int  = 1
	max_visible_items        int  = 8
	sources                  CompletionSources
	fuzzy                    bool = true
	inherit_ancestor_options bool = true
	disable_in_alt_screen    bool = true
	require_prompt_region    bool = true
	windows_allow_coreutils  bool
	force_family             string
	redact_sensitive         bool = true
}

pub struct LoggingConfig {
pub mut:
	local      bool
	ssh        bool
	directory  string
	write_mode string = 'append'
}

pub struct EditorConfig {
pub mut:
	mode    string = 'os_default'
	program string
	args    []string
}

pub struct SftpConfig {
pub mut:
	editor             EditorConfig
	edit_max_file_size i64 = 1048576
}

pub struct SshSettingsConfig {
pub mut:
	keepalive_enabled       bool = true
	keepalive_interval_secs int  = 30
	keepalive_max           int  = 3
}

pub struct RightDockPrefs {
pub mut:
	local bool
	ssh   bool = true
}

// open_for is the remembered open state for a tab of this kind.
pub fn (p RightDockPrefs) open_for(local bool) bool {
	return if local { p.local } else { p.ssh }
}

pub struct VerticalTabs {
pub mut:
	enabled   bool
	width     int = 220
	collapsed bool
}

pub const rail_width_min = 140
pub const rail_width_max = 400

// lock_timeout_minutes is the idle lock's timeout, clamped to 1-480.
pub fn (c UiConfig) lock_timeout_minutes() int {
	m := if c.lock_after_idle_minutes == 0 { 15 } else { c.lock_after_idle_minutes }
	return if m < 1 {
		1
	} else if m > 480 {
		480
	} else {
		m
	}
}

// normalize fills in what the file may leave empty; `dir` is the config folder.
pub fn (mut c TerminalConfig) normalize(dir string) {
	if c.logging.directory == '' {
		c.logging.directory = os.join_path(dir, 'logs')
	}
}

// normalize clamps the rail width of a hand-edited file.
pub fn (mut c UiConfig) normalize() {
	w := c.vertical_tabs.width
	c.vertical_tabs.width = if w < rail_width_min {
		rail_width_min
	} else if w > rail_width_max {
		rail_width_max
	} else {
		w
	}
}

// UpdateConfig is the part of update_config.json the settings window edits. The file is
// merged on write, so the updater's cache fields survive.
pub struct UpdateConfig {
pub mut:
	auto_check           bool   = true
	channel              string = 'stable'
	check_interval_hours int    = 24
	proxy_url            string
	verify_certificates  bool = true
	skipped_version      string
}

pub fn load_update_config(dir string) UpdateConfig {
	text := os.read_file(os.join_path(dir, 'update_config.json')) or { return UpdateConfig{} }
	return json.decode(UpdateConfig, text) or { UpdateConfig{} }
}
