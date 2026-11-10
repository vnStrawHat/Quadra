// The rebindable actions (specs/22, Key bindings): order is the Key Bindings page's order.
module main

import quadra.core

pub struct Bindable {
pub:
	id    string
	label string
	group string
	def   string
	cmd   Cmd
	arg   string
}

const app_menu = 'App Menu'
const edit_menu = 'Edit Menu'
const term_menu = 'Terminal Context Menu'
const channel_group = 'Input Channel'
const session_menu = 'Session Tabs Context Menu'
const sftp_menu = 'SFTP Context Menu'

pub const key_groups = [app_menu, edit_menu, term_menu, channel_group, session_menu, sftp_menu]

pub const bindables = [
	Bindable{'close_panel', 'Close Panel', app_menu, 'ctrl-w', .close_panel, ''},
	Bindable{'new_terminal_tab', 'New Terminal Tab', app_menu, 'ctrl-t', .new_default, ''},
	Bindable{'new_ssh_session', 'New SSH Session', app_menu, 'ctrl-shift-n', .quick_connect, ''},
	Bindable{'toggle_right_dock', 'Toggle Right Dock', app_menu, 'ctrl-shift-b', .toggle_dock, ''},
	Bindable{'toggle_gutter', 'Toggle Gutter', app_menu, '', .toggle_gutter, ''},
	Bindable{'toggle_fps', 'Toggle FPS Monitor', app_menu, '', .toggle_fps, ''},
	Bindable{'about', 'About OneTerm', app_menu, '', .about, ''},
	Bindable{'quit', 'Quit', app_menu, 'ctrl-shift-q', .quit, ''},
	Bindable{'lock_oneterm', 'Lock OneTerm', app_menu, 'ctrl-shift-l', .lock, ''},
	Bindable{'open_settings', 'Open Settings', app_menu, 'ctrl-,', .open_settings, ''},
	Bindable{'terminal_copy', 'Copy', edit_menu, 'ctrl-shift-c', .copy, ''},
	Bindable{'terminal_paste', 'Paste', edit_menu, 'ctrl-shift-v', .paste, ''},
	Bindable{'find', 'Find', edit_menu, 'ctrl-f', .find, ''},
	Bindable{'terminal_select_all', 'Select All', edit_menu, 'ctrl-shift-a', .select_all, ''},
	Bindable{'terminal_clear', 'Clear', edit_menu, '', .clear, ''},
	Bindable{'duplicate_session', 'Duplicate Session', term_menu, '', .duplicate_space, ''},
	Bindable{'split_right', 'Split Right', term_menu, 'ctrl-shift-right', .split_right, ''},
	Bindable{'split_left', 'Split Left', term_menu, 'ctrl-shift-left', .split_left, ''},
	Bindable{'split_up', 'Split Up', term_menu, 'ctrl-shift-up', .split_up, ''},
	Bindable{'split_down', 'Split Down', term_menu, 'ctrl-shift-down', .split_down, ''},
	Bindable{'close_space', 'Close Space', term_menu, 'ctrl-shift-x', .close_space, ''},
	Bindable{'join_input_channel_a', 'Join Input Channel A', channel_group, '', .join_channel, '|A'},
	Bindable{'join_input_channel_b', 'Join Input Channel B', channel_group, '', .join_channel, '|B'},
	Bindable{'join_input_channel_c', 'Join Input Channel C', channel_group, '', .join_channel, '|C'},
	Bindable{'join_input_channel_d', 'Join Input Channel D', channel_group, '', .join_channel, '|D'},
	Bindable{'join_input_channel_e', 'Join Input Channel E', channel_group, '', .join_channel, '|E'},
	Bindable{'leave_input_channel', 'Leave Input Channel', channel_group, '', .leave_channel, ''},
	Bindable{'close_input_channel', 'Close Input Channel', channel_group, '', .close_channel, ''},
	Bindable{'open_session', 'Open Session', session_menu, '', .open_selected, ''},
	Bindable{'delete_session', 'Delete Session', session_menu, '', .delete_selected, ''},
	Bindable{'session_property', 'Session Properties', session_menu, '', .properties_selected, ''},
	Bindable{'sftp_open', 'SFTP Open', sftp_menu, '', .sftp, 'open'},
	Bindable{'sftp_download', 'SFTP Download', sftp_menu, '', .sftp, 'download'},
	Bindable{'sftp_rename', 'SFTP Rename', sftp_menu, '', .sftp, 'rename'},
	Bindable{'sftp_delete', 'SFTP Delete', sftp_menu, '', .sftp, 'delete'},
	Bindable{'sftp_properties', 'SFTP Properties', sftp_menu, '', .sftp, 'properties'},
	Bindable{'sftp_upload_files', 'SFTP Upload Files', sftp_menu, '', .sftp, 'upload_files'},
	Bindable{'sftp_upload_folder', 'SFTP Upload Folder', sftp_menu, '', .sftp, 'upload_folder'},
	Bindable{'sftp_new_folder', 'SFTP New Folder', sftp_menu, '', .sftp, 'new_folder'},
	Bindable{'sftp_refresh', 'SFTP Refresh', sftp_menu, '', .sftp, 'refresh'},
]

// bind_keys applies the defaults and the overrides of ui_config.json ('' unbinds). An
// elevated window binds only what its policy allows.
pub fn bind_keys(mut a core.App, overrides map[string]string) ! {
	for b in bindables {
		if !action_allowed_when_elevated(b.id, is_restricted()) {
			continue
		}
		keys := overrides[b.id] or { b.def }
		if keys != '' {
			a.bind(keys, Do{ cmd: b.cmd, arg: b.arg },
				context: key_context_of(b)
			)!
		}
	}
}

fn context_of_id(id string) string {
	for b in bindables {
		if b.id == id {
			return key_context_of(b)
		}
	}
	return ''
}

// key_context_of: the original handles these actions on the panel, so they fire only while the
// focus is inside it. An empty context is global.
fn key_context_of(b Bindable) string {
	return match b.group {
		session_menu { 'SessionPanel' }
		sftp_menu { 'SftpPanel' }
		else { '' }
	}
}

pub fn key_default(id string) string {
	for b in bindables {
		if b.id == id {
			return b.def
		}
	}
	return ''
}

// effective_key is the binding of `id` now.
pub fn effective_key(overrides map[string]string, id string) string {
	return overrides[id] or { key_default(id) }
}

// is_at_default is the one test both the "Default:" line and the stored overrides use.
pub fn is_at_default(overrides map[string]string, id string) bool {
	return effective_key(overrides, id) == key_default(id)
}

// show_default is the line under a row's label, or '' at the default.
pub fn show_default(overrides map[string]string, id string) string {
	if is_at_default(overrides, id) {
		return ''
	}
	d := key_default(id)
	return if d == '' { 'Default: (unbound)' } else { 'Default: ${d}' }
}

// conflicting names the action already holding `keys` in `id`'s key context, other than `id`.
pub fn conflicting(overrides map[string]string, id string, keys string) ?string {
	if keys == '' {
		return none
	}
	context := context_of_id(id)
	for b in bindables {
		if b.id != id && key_context_of(b) == context && effective_key(overrides, b.id) == keys {
			return b.label
		}
	}
	return none
}

// resolve_default_collisions leaves unbound every action still on its shipped default whose key
// a different action in the same context holds as a user override. Run at start, and after every
// rebind and reset, so the keymap and the Key Bindings page tell the same story.
pub fn resolve_default_collisions(overrides map[string]string) map[string]string {
	mut out := overrides.clone()
	for b in bindables {
		key := effective_key(overrides, b.id)
		if key == '' || !is_at_default(overrides, b.id) {
			continue
		}
		for other in bindables {
			if other.id != b.id && key_context_of(other) == key_context_of(b)
				&& effective_key(overrides, other.id) == key && !is_at_default(overrides, other.id) {
				out[b.id] = ''
				break
			}
		}
	}
	return out
}

// displacing_action names the action holding `id`'s default when `id` was left unbound by
// resolve_default_collisions.
pub fn displacing_action(overrides map[string]string, id string) ?string {
	if effective_key(overrides, id) != '' {
		return none
	}
	return conflicting(overrides, id, key_default(id))
}

// without_defaults keeps only the overrides that differ from their default.
pub fn without_defaults(overrides map[string]string) map[string]string {
	mut out := map[string]string{}
	for b in bindables {
		if !is_at_default(overrides, b.id) {
			out[b.id] = effective_key(overrides, b.id)
		}
	}
	return out
}
