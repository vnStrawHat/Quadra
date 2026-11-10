// The settings pages, in the original's sidebar order (specs/22, Settings window).
module main

import quadra.core
import quadra.kit

fn group(title string, rows []kit.SettingsRow) kit.SettingsGroup {
	return kit.SettingsGroup{
		title: title
		rows:  rows
	}
}

fn group_about(title string, description string, rows []kit.SettingsRow) kit.SettingsGroup {
	return kit.SettingsGroup{
		title:       title
		description: description
		rows:        rows
	}
}

fn (mut w SettingsWin) pages(t &kit.Theme, mut cx core.ViewCx[SettingsWin]) []kit.SettingsPage {
	return visible_pages([
		kit.SettingsPage{'General', w.general_groups(mut cx), icon_settings_2, ''},
		kit.SettingsPage{'Key Bindings', w.key_groups(t, mut cx), kit.icon_menu, ''},
		kit.SettingsPage{'Terminal', w.terminal_groups(mut cx), icon_square_terminal, ''},
		kit.SettingsPage{'Completion', w.completion_groups(mut cx), kit.icon_search, 'The suggestion overlay and the history it draws on.'},
		kit.SettingsPage{'SSH', w.ssh_groups(mut cx), icon_network, ''},
		kit.SettingsPage{'Security', w.security_groups(mut cx), kit.icon_lock, ''},
		kit.SettingsPage{'Network', w.network_groups(mut cx), icon_globe, ''},
		kit.SettingsPage{'About', w.about_groups(mut cx), kit.icon_info, ''},
	], is_restricted())
}

// visible_pages: an elevated window has no updater, so nothing the Network page sets would apply.
fn visible_pages(all []kit.SettingsPage, restricted bool) []kit.SettingsPage {
	return if restricted { all.filter(it.title != 'Network') } else { all }
}

fn (mut w SettingsWin) general_groups(mut cx core.ViewCx[SettingsWin]) []kit.SettingsGroup {
	o := cx.peek(w.one)
	current := o.current_theme()
	light := if i := theme_info(o.themes, current) { i.light } else { false }
	mode_at := if light { 0 } else { 1 }
	mode := kit.select_box('set-mode', ['Light', 'Dark'], mode_at).compact().size(.lg).on_change(cx.listener(fn (mut w SettingsWin, i &int, mut cx core.ViewCx[SettingsWin]) {
		mut o := cx.peek(w.one)
		o.ui.theme_name = if *i == 0 { o.last_light } else { o.last_dark }
		w.changed(mut cx)
	}))
	labels := settings_shell_kinds.map(display_name(it))
	picker := kit.menu_button_with('theme-picker', current, ThemePick{o, w.one}, fn (p ThemePick) []kit.MenuItem {
		return p.o.theme_items(p.o.current_theme(), p.one)
	}).outline().caret().size(.lg)
	return [
		group('Theme', [
			kit.SettingsRow{'Mode', 'Switching this also swaps the color theme to the last one used in that mode.', mode},
			kit.SettingsRow{'Color Theme', 'Built-in and loaded themes, grouped Light / Dark.', picker},
		]),
		group('Interface', [
			w.number_row('ui_font', 'UI Font Size',
				'Size of all non-terminal text, in px. Terminal text has its own size.', mut cx),
			w.switch_row('show_fps', 'Show FPS Monitor',
				"Frame times, dropped frames and GPU usage in the main window's top-right corner.", mut
				cx),
			w.switch_row('vertical_tabs', 'Vertical Tabs',
				'Show terminal tabs in a list on the left of the terminal area instead of a strip above it.', mut
				cx),
		]),
		group_about('Shell', 'Used by every new local terminal.', [
			w.choice_row('kind', 'Shell', 'For new terminals.', labels, settings_shell_kinds, mut
				cx),
			w.text_row('program', 'Custom Program', 'Path to the shell executable. Custom only.',
				'', mut cx),
			w.switch_row('utf8', 'UTF-8', 'Force the UTF-8 code page in Command Prompt.', mut cx),
		]),
	]
}

const weights = ['thin', 'extra_light', 'light', 'normal', 'medium', 'semibold', 'bold', 'extra_bold',
	'black']
const weight_labels = ['Thin', 'Extra Light', 'Light', 'Normal', 'Medium', 'Semibold', 'Bold',
	'Extra Bold', 'Black']
const shapes = ['block', 'bar', 'underline']
const highlights = ['auto', 'on', 'off']
const title_modes = ['default', 'osc']
const log_modes = ['append', 'overwrite']

fn (mut w SettingsWin) terminal_groups(mut cx core.ViewCx[SettingsWin]) []kit.SettingsGroup {
	return [
		group_about('Font', 'Applies to terminal text only.', [
			w.family_row(mut cx),
			w.number_row('font_size', 'Size', 'In px (6-72).', mut cx),
			w.choice_row('weight', 'Weight', '', weight_labels, weights, mut cx),
			w.number_row('line_h', 'Line Height', 'Multiplier on the font size.', mut cx),
			w.text_row('fallbacks', 'Fallback Fonts',
				'Comma-separated families tried, in order, for glyphs the font lacks (Nerd Font icons).',
				'', mut cx),
			w.switch_row('ligatures', 'Ligatures',
				'Render ligatures (=>, ->, !=) when the font has them.', mut cx),
		]),
		group('Cursor', [
			w.choice_row('shape', 'Shape', '', ['Block', 'Bar', 'Underline'], shapes, mut cx),
			w.switch_row('blink', 'Blink', 'Blink when focused.', mut cx),
			w.text_row('cur_color', 'Color', 'Hex color; blank uses theme.', '#RRGGBB', mut cx),
		]),
		group('Layout', [
			w.switch_row('gutter', 'Show Gutter', '', mut cx),
			w.choice_row('highlight', 'Semantic Highlighting',
				'Highlight paths, commands, and URLs.', ['Auto', 'On', 'Off'], highlights, mut cx),
			w.choice_row('tab_title', 'Tab Title', 'A fixed label, or the title the shell sets.', [
				'Default (label)', 'OSC 0/2 (shell title)'], title_modes, mut cx),
			w.number_row('scrollback', 'Scrollback History', 'Lines kept above the screen.', mut cx),
		]),
		group('Scroll', [
			w.number_row('wheel', 'Scroll Multiplier', 'Lines per wheel notch.', mut cx),
			w.switch_row('alt_scroll', 'Alternate Scroll',
				'Send arrow keys to full-screen programs instead of scrolling.', mut cx),
		]),
		group('Bell', [
			w.switch_row('bell', 'Enabled', 'Show an indicator when a program rings the bell.', mut
				cx),
		]),
		group('Mouse', [
			w.switch_row('context_menu', 'Right-Click Context Menu',
				'Show OneTerm right-click menu.', mut cx),
			w.switch_row('copy_select', 'Copy on Select',
				'Copy the selection to the clipboard when the mouse button is released.', mut cx),
			w.switch_row('middle', 'Middle-Click Paste',
				'Paste the clipboard on a middle click. A program in mouse mode gets the click instead unless Shift is held.', mut
				cx),
		]),
		group('Security', [
			w.switch_row('clip_read', 'Allow Clipboard Read (OSC 52)',
				'A remote program can read your clipboard while this is on.', mut cx),
		]),
		group_about('Logging', 'Write printable terminal output to timestamped log files.', [
			w.switch_row('log_local', 'Automatic: Local Shell',
				'Start logging every new local shell.', mut cx),
			w.switch_row('log_ssh', 'Automatic: SSH',
				'Start logging new SSH terminals unless a saved session overrides it.', mut cx),
			w.log_folder_row(mut cx),
			w.choice_row('log_mode', 'Existing File',
				'Overwrite truncates once when logging starts; Append preserves content.', [
				'Append', 'Overwrite'], log_modes, mut cx),
			w.fixed_row('File Name Format',
				'Fixed for this release. %n = process or SSH endpoint.',
				'%n_%Y-%m-%d_%H-%M-%S.log', mut cx),
			w.fixed_row('Content Format',
				'Fixed for this release. %msg is one printable output line.',
				'[%Y-%m-%d %H:%M:%S] %msg', mut cx),
		]),
	]
}

fn (mut w SettingsWin) completion_groups(mut cx core.ViewCx[SettingsWin]) []kit.SettingsGroup {
	return [
		group('', [// the page carries the title and the description
			w.switch_row('cmp_on', 'Enabled',
				'Master switch for the overlay and the history capture.', mut cx),
			w.switch_row('cmp_tab', 'Accept With Tab', 'When off, Tab is forwarded to the shell.', mut
				cx),
			w.number_row('max_hist', 'Max Command History',
				'Per-family in-session history capacity (0 disables history).', mut cx),
			w.number_row('min_prefix', 'Min Characters Before Suggesting',
				'Command suggestions appear after this many typed characters.', mut cx),
			w.number_row('visible', 'Visible Suggestions',
				'Rows shown in the overlay before scrolling.', mut cx),
			w.switch_row('cmp_mem', 'Source: History', 'Suggest commands you ran this session.', mut
				cx),
			w.switch_row('cmp_man', 'Source: Manual',
				'Hand-authored bundled catalogs (git, cargo, ...).', mut cx),
			w.switch_row('cmp_ext', 'Source: External',
				'Generated catalogs (Windows commands, coreutils).', mut cx),
			w.switch_row('cmp_fuzzy', 'Fuzzy Matching', 'Secondary subsequence matching.', mut cx),
			w.switch_row('cmp_alt', 'Disable Inside Full-Screen Apps',
				'Suppress suggestions in vim/less/htop (alternate screen).', mut cx),
			w.switch_row('cmp_core', 'Allow Coreutils On Windows',
				'Also suggest coreutils/linux commands in cmd/PowerShell (Git-Bash users).', mut cx),
			w.choice_row('force_family', 'Force Shell Family',
				'Override the detected shell family for suggestions.', ['Auto-detect', 'cmd',
				'PowerShell', 'Unix'], ['', 'cmd', 'powershell', 'unix'], mut cx),
		]),
	]
}

fn (mut w SettingsWin) ssh_groups(mut cx core.ViewCx[SettingsWin]) []kit.SettingsGroup {
	return [
		group_about('Connection', 'Keepalive settings applied to newly opened SSH sessions.', [
			w.switch_row('ka_on', 'Enable Keepalive',
				'Detect peers or network paths that stop responding.', mut cx),
			w.number_row('ka_secs', 'Keepalive Interval (seconds)',
				'Seconds between keepalive requests.', mut cx),
			w.number_row('ka_max', 'Keepalive Max',
				'Unanswered requests tolerated before the connection is closed.', mut cx),
		]),
		group_about('SFTP Editor',
			"Which editor the SFTP browser's Edit action opens a remote file with.", [
			w.choice_row('editor_mode', 'Editor',
				'OS default opens the associated application; Custom runs your command.', [
				'OS default application',
				'Custom command',
			], [
				'os_default',
				'custom',
			], mut cx),
			w.text_row('editor_prog', 'Custom Program',
				'Executable (e.g. code, notepad). Custom mode only.', '', mut cx),
			w.text_row('editor_args', 'Custom Arguments',
				'Space-separated, before the file path. Custom mode only.', '', mut cx),
		]),
		group('SFTP Edit Limit', [
			w.number_row('edit_mb', 'Max Edit File Size (MB)',
				'Files larger than this prompt before opening. 0 = no limit.', mut cx),
		]),
	]
}

// ThemePick builds the theme picker's items when it opens.
struct ThemePick {
	o   &OneTerm = unsafe { nil }
	one core.Model[OneTerm]
}
