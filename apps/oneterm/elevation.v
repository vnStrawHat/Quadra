// The elevated window's policy (specs/22, Elevated window): pure, so it is tested without a
// consent prompt. The OS half is in quadra/native/sys.
module main

import os
import quadra.native.sys

pub enum Elevation {
	not_elevated
	elevated
	unknown
}

// elevation is what this process's token says; every gate reads it.
pub const elevation = process_elevation()

// process_elevation reads the process token. A failed query is .unknown: restricted, and the
// title does not claim an elevation the token did not confirm.
pub fn process_elevation() Elevation {
	elevated := sys.process_is_elevated() or { return .unknown }
	return if elevated { Elevation.elevated } else { Elevation.not_elevated }
}

// request_elevation asks for an elevated OneTerm. Blocks until the consent prompt answers, so
// it runs on its own thread. Only the + menu reaches it; tests never do.
fn request_elevation(shell string) (bool, u32) {
	return sys.shell_execute_runas(os.executable(), '${elevated_flag} ${shell}', os.getwd())
}

pub fn is_restricted() bool {
	return elevation != .not_elevated
}

const elevated_flag = '--elevated-shell'

// elevated_settings_notice is said once by an elevated window running on the defaults ('' = nothing).
pub fn elevated_settings_notice(restricted bool, blocked bool) string {
	return if restricted && blocked {
		'Settings could not be read for this account; this window is using the defaults.'
	} else {
		''
	}
}

pub fn window_title(e Elevation) string {
	return match e {
		.not_elevated { 'OneTerm' }
		.elevated { 'OneTerm (Administrator)' }
		.unknown { 'OneTerm (elevation unknown)' }
	}
}

// elevation_suffix is the part of the title the app bar draws in the warning color.
pub fn elevation_suffix(e Elevation) string {
	return match e {
		.not_elevated { '' }
		.elevated { ' (Administrator)' }
		.unknown { ' (elevation unknown)' }
	}
}

// parse_elevated_shell reads the command line: none without the flag, an error on a bad token.
pub fn parse_elevated_shell(args []string) !string {
	i := args.index(elevated_flag)
	if i < 0 {
		return ''
	}
	if i + 1 >= args.len || args[i + 1] !in ['cmd', 'powershell', 'pwsh'] {
		return error('${elevated_flag} takes cmd, powershell or pwsh')
	}
	return args[i + 1]
}

// trusted_shell is the fixed path of an elevated shell: never PATH, COMSPEC or terminal.json.
pub fn trusted_shell(kind string) !ShellConfig {
	root := os.getenv_opt('SystemRoot') or { r'C:\Windows' }
	files := os.getenv_opt('ProgramFiles') or { r'C:\Program Files' }
	path := match kind {
		'cmd' { root + r'\System32\cmd.exe' }
		'powershell' { root + r'\System32\WindowsPowerShell\v1.0\powershell.exe' }
		'pwsh' { files + r'\PowerShell\7\pwsh.exe' }
		else { return error('${kind} cannot be elevated') }
	}

	return ShellConfig{
		kind:    kind
		program: path
	}
}

// Action ids an elevated window must not run: everything that reaches SSH, SFTP or the store.
const denied_when_elevated = ['delete_session', 'new_ssh_session', 'open_session', 'session_property',
	'sftp_delete', 'sftp_download', 'sftp_new_folder', 'sftp_open', 'sftp_properties', 'sftp_refresh',
	'sftp_rename', 'sftp_upload_files', 'sftp_upload_folder', 'toggle_right_dock']

// Listed, not defaulted, so a new id fails the exhaustiveness test until it is classified.
const allowed_when_elevated = ['about', 'close_input_channel', 'close_panel', 'close_space',
	'duplicate_session', 'find', 'join_input_channel_a', 'join_input_channel_b',
	'join_input_channel_c', 'join_input_channel_d', 'join_input_channel_e', 'leave_input_channel',
	'lock_oneterm', 'new_terminal_tab', 'open_settings', 'quit', 'split_down', 'split_left',
	'split_right', 'split_up', 'terminal_clear', 'terminal_copy', 'terminal_paste',
	'terminal_select_all', 'toggle_fps', 'toggle_gutter']

// action_allowed_when_elevated: unelevated runs everything; elevated runs the table, and an
// id nobody classified is denied.
pub fn action_allowed_when_elevated(id string, elevated bool) bool {
	return !elevated || (id in allowed_when_elevated && id !in denied_when_elevated)
}

// should_start_launch is the debounce of `runas`: one consent prompt at a time.
pub fn should_start_launch(in_flight bool) bool {
	return !in_flight
}

// launch_outcome maps ShellExecuteExW's result to what the user is told ('' = nothing).
// A declined prompt (ERROR_CANCELLED, 1223) says nothing.
pub fn launch_outcome(started bool, last_error u32) string {
	return if started || last_error == 1223 {
		''
	} else {
		'Could not start an administrator window: Windows error ${last_error}'
	}
}
