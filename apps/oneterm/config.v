// terminal.json and ui_config.json (specs/22, Config). Missing keys keep their defaults; a
// missing file is created; a file that does not parse is moved aside and the defaults are
// used; a file that exists but cannot be read is left alone and never written over.
module main

import json
import os
import time

pub struct FontConfig {
pub mut:
	family    string = 'Lilex'
	size      f32    = 15
	weight    string = 'normal'
	ligatures bool   = true
	features  []string
	fallbacks []string = ['Symbols Nerd Font Mono', 'Symbols Nerd Font']
}

pub struct CursorConfig {
pub mut:
	shape string = 'block'
	color string
	blink bool = true
}

pub struct PaddingConfig {
pub mut:
	top    f32
	right  f32 = 5
	bottom f32
	left   f32 = 10
}

pub struct LayoutConfig {
pub mut:
	line_height           f32 = 1.2
	cell_width            f32 @[omitempty] // 0: from the font
	padding               PaddingConfig
	show_gutter           bool
	tab_title             string = 'default'
	semantic_highlighting string = 'auto'
}

pub struct ShellConfig {
pub mut:
	kind    string = 'cmd'
	program string
	args    []string
	env     map[string]string
	cwd     string
	utf8    bool = true
}

pub struct ScrollConfig {
pub mut:
	multiplier         f32  = 1
	alternate_scroll   bool = true
	scrollback_history int  = 10000
}

pub struct MouseConfig {
pub mut:
	show_context_menu  bool = true
	copy_on_select     bool = true
	middle_click_paste bool = true
}

pub struct BellConfig {
pub mut:
	enabled bool = true
}

pub struct ColorsConfig {
pub mut:
	foreground     string
	background     string
	cursor         string
	selection      string
	gutter_fg      string
	gutter_bg      string
	clock_fg       string
	line_number_fg string
	min_contrast   f32
	ansi           []string
}

pub struct SecurityConfig {
pub mut:
	allow_clipboard_read bool
}

pub struct TerminalConfig {
pub mut:
	schema_version int = 1
	font           FontConfig
	cursor         CursorConfig
	layout         LayoutConfig
	shell          ShellConfig
	scroll         ScrollConfig
	mouse          MouseConfig
	bell           BellConfig
	colors         ColorsConfig
	security       SecurityConfig
	completion     CompletionConfig
	logging        LoggingConfig
	sftp           SftpConfig
	ssh            SshSettingsConfig
	// the file exists but could not be read: it may still be valid, so it is never written
	persist_blocked bool @[skip]
}

pub struct UiConfig {
pub mut:
	schema_version          int = 1
	ui_font_size            f32 = 16
	theme_name              string
	key_bindings            map[string]string
	right_dock              RightDockPrefs
	show_fps                bool
	lock_after_idle_minutes int @[omitempty] // 0: the default, 15
	vertical_tabs           VerticalTabs
	// could not be read at startup (or, elevated, is absent): never written
	persist_blocked bool @[skip]
}

// data_dir is the config folder: --data, or ~/.OneTerm.
pub fn data_dir(root string) string {
	return if root != '' { root } else { os.join_path(os.home_dir(), '.OneTerm') }
}

// strip_comments removes // and /* */ comments outside strings (terminal.json allows them).
pub fn strip_comments(s string) string {
	mut out := []u8{cap: s.len}
	mut i, mut in_string := 0, false
	for i < s.len {
		c := s[i]
		if in_string {
			out << c
			if c == `\\` && i + 1 < s.len {
				i++
				out << s[i]
			} else if c == `"` {
				in_string = false
			}
			i++
		} else if c == `"` {
			in_string = true
			out << c
			i++
		} else if c == `/` && i + 1 < s.len && s[i + 1] == `/` {
			for i < s.len && s[i] != `\n` {
				i++
			}
		} else if c == `/` && i + 1 < s.len && s[i + 1] == `*` {
			end := s.index_after('*/', i + 2) or { s.len - 2 }
			i = end + 2
		} else {
			out << c
			i++
		}
	}
	return out.bytestr()
}

fn quarantine(path string) {
	os.mv(path, '${path}.corrupt-${time.now().unix()}') or {}
}

pub fn load_terminal_config(dir string) TerminalConfig {
	return read_terminal_config(dir, is_restricted())
}

pub fn load_ui_config(dir string) UiConfig {
	return read_ui_config(dir, is_restricted())
}

// read_terminal_config takes `restricted` so the elevated rules can be tested.
fn read_terminal_config(dir string, restricted bool) TerminalConfig {
	path := os.join_path(dir, 'terminal.json')
	if !os.exists(path) {
		cfg := TerminalConfig{}
		if !restricted { // an elevated window leaves no trace, not even the first-run default
			write_config(path, json.encode_pretty(cfg)) or {}
		}
		return cfg
	}
	text := os.read_file(path) or {
		eprintln('terminal.json could not be read (${err}); using defaults and refusing to overwrite it')
		return TerminalConfig{
			persist_blocked: true
		}
	}
	return json.decode(TerminalConfig, strip_comments(text)) or {
		if !restricted {
			quarantine(path)
		}
		TerminalConfig{}
	}
}

fn read_ui_config(dir string, restricted bool) UiConfig {
	path := os.join_path(dir, 'ui_config.json')
	if !os.exists(path) {
		if restricted { // says once that the window runs on the defaults
			return UiConfig{
				persist_blocked: true
			}
		}
		cfg := UiConfig{}
		write_config(path, json.encode_pretty(cfg)) or {}
		return cfg
	}
	text := os.read_file(path) or {
		eprintln('ui_config.json could not be read (${err}); using defaults and refusing to overwrite it')
		return UiConfig{
			persist_blocked: true
		}
	}
	return json.decode(UiConfig, lenient_ui_document(text)) or {
		if !restricted {
			quarantine(path)
		}
		UiConfig{}
	}
}
