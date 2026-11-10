// Shells: the command line, environment and shell integration of each kind (specs/22, Local
// shell). Integration needs no script files: only arguments and environment variables.
module main

import os

const version = '0.6.11'

// The terminal identities a child must not inherit from a parent terminal.
const dropped_env = ['WT_SESSION', 'WT_PROFILE_ID', 'ConEmuPID', 'ConEmuANSI', 'ITERM_SESSION_ID',
	'VSCODE_INJECTION', 'KONSOLE_VERSION', 'TERMINATOR_UUID', 'ALACRITTY_LOG', 'KITTY_WINDOW_ID',
	'WEZTERM_PANE']

const cmd_prompt = r'$E]7;$P$E\$E]133;A$E\$P$G$E]133;B$E\'

const powershell_init = $embed_file('integration/powershell_init.txt').to_string().trim_space()

const bash_prompt_command = $embed_file('integration/bash_prompt.txt').to_string().trim_space()

const bash_ps0 = r'\e]133;C\e\\'

// zsh_ps1 carries D, A and B; C needs preexec, a function no environment variable can set.
const zsh_ps1 = '%{\x1b]133;D;%?\x1b\\\x1b]133;A\x1b\\%}%n@%m:%~ %# %{\x1b]133;B\x1b\\%}'

// ShellSpec is everything needed to start a shell.
pub struct ShellSpec {
pub:
	kind    string
	name    string // for the tab title
	command string
	env     map[string]string
	cwd     string
}

// shell_choices are the rows of the + menu.
pub const shell_choices = [['cmd', 'Command Prompt'], ['powershell', 'PowerShell'],
	['pwsh', 'PowerShell 7']]

// settings_shell_kinds are the Shell dropdown's kinds, in the original's order.
pub const settings_shell_kinds = ['cmd', 'powershell', 'pwsh', 'bash', 'zsh', 'sh', 'custom']

// display_name is the + menu's row and the tab's label for a shell kind.
pub fn display_name(kind string) string {
	return match kind {
		'cmd' { 'Command Prompt' }
		'powershell' { 'PowerShell' }
		'pwsh' { 'PowerShell 7' }
		'bash' { 'Bash' }
		'zsh' { 'Zsh' }
		'sh' { 'Sh' }
		else { 'Custom Shell' }
	}
}

fn quoted(arg string) string {
	return if arg.contains(' ') || arg.contains('"') {
		'"' + arg.replace('"', r'\"') + '"'
	} else {
		arg
	}
}

fn base_env(lang string) map[string]string {
	mut env := {
		'TERM':                 'xterm-256color'
		'COLORTERM':            'truecolor'
		'TERM_PROGRAM':         'OneTerm'
		'TERM_PROGRAM_VERSION': version
		'LANG':                 lang
	}
	for name in dropped_env {
		env[name] = ''
	}
	return env
}

// resolve_unix_shell finds Bash, Zsh or Sh: PATH, then /bin/<name>, then $SHELL when it names
// the same program. $SHELL alone would make a Zsh tab start the user's Bash.
pub fn resolve_unix_shell(name string, shell_env string, lookup fn (string) ?string, exists fn (string) bool) string {
	if found := lookup(name) {
		return found
	}
	fallback := '/bin/${name}'
	if exists(fallback) {
		return fallback
	}
	if shell_env.replace('\\', '/').all_after_last('/') == name {
		return shell_env
	}
	return fallback
}

fn find_on_path(name string) ?string {
	return os.find_abs_path_of_executable(name) or { return none }
}

// add_bash_integration leaves a PROMPT_COMMAND that already emits OSC 133 alone (that is the
// opt-out) and runs after ours any other, which then still sees its own command's status.
fn add_bash_integration(mut env map[string]string) {
	user := env['PROMPT_COMMAND'] or { '' }
	if user.contains('133;') {
		return
	}
	env['PROMPT_COMMAND'] = if user.trim_space() != '' {
		'${bash_prompt_command}; ${user}'
	} else {
		bash_prompt_command
	}
	if 'PS0' !in env {
		env['PS0'] = bash_ps0
	}
}

// resolve_shell builds the spec of `kind` ('' = the configured default). `cwd`, when set,
// beats the configured directory (Duplicate).
pub fn resolve_shell(c ShellConfig, kind string, cwd string) ShellSpec {
	k := if kind != '' { kind } else { c.kind }
	mut env := base_env(os.getenv_opt('LANG') or { 'en_US.UTF-8' })
	mut program, mut name := '', display_name(k)
	mut args := []string{}
	for key, value in c.env { // first: the integrations below yield to what the user set
		env[key] = value
	}
	match k {
		'cmd' {
			program = os.getenv_opt('COMSPEC') or { 'cmd.exe' }
			if 'PROMPT' !in env {
				env['PROMPT'] = cmd_prompt
			}
			if c.utf8 {
				args = ['/K', 'chcp', '65001', '>nul']
			}
		}
		'powershell', 'pwsh' {
			program = '${k}.exe'
			utf := if c.utf8 { '[Console]::OutputEncoding=[Text.UTF8Encoding]::new();' } else { '' }
			args = ['-NoLogo', '-NoExit', '-Command', utf + powershell_init]
		}
		'bash', 'zsh', 'sh' {
			program = resolve_unix_shell(k, os.getenv('SHELL'), find_on_path, os.is_file)
			if k != 'sh' {
				args = ['-l'] // a login shell, so the profile is loaded
			}
			if k == 'bash' {
				add_bash_integration(mut env)
			} else if k == 'zsh' && 'PS1' !in env {
				env['PS1'] = zsh_ps1
			}
		}
		else {
			program = if c.program != '' { c.program } else { k }
			name = os.file_name(program).trim_string_right('.exe')
		}
	}

	if c.program != '' && k == c.kind {
		program = c.program
		name = os.file_name(program).trim_string_right('.exe')
	}
	if k == c.kind {
		args << c.args // after the kind's own arguments
	}
	mut parts := [quoted(program)]
	parts << args.map(if it.contains('>') { it } else { quoted(it) })
	return ShellSpec{
		kind:    k
		name:    name
		command: parts.join(' ')
		env:     env
		cwd:     if cwd != '' { cwd } else { c.cwd }
	}
}
