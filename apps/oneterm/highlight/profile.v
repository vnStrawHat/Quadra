// The shell profile: only the prompt pattern and path/option syntax differ per shell.
module highlight

pub enum ShellProfile {
	unix // bash/sh/zsh/fish, and SSH on Linux
	cmd
	powershell
	dumb // unknown, serial, router: the most permissive prompt
}

// profile_for maps a shell kind; unknown kinds are `dumb`, the permissive profile.
pub fn profile_for(kind string) ShellProfile {
	return match kind {
		'bash', 'zsh', 'sh', 'fish', 'unix' { .unix }
		'cmd' { .cmd }
		'powershell', 'pwsh' { .powershell }
		else { .dumb }
	}
}

fn (p ShellProfile) path_is_unix() bool {
	return p == .unix
}

fn (p ShellProfile) is_option_prefix(c rune) bool {
	return match p {
		.unix, .powershell { c == `-` }
		.cmd, .dumb { c == `-` || c == `/` }
	}
}

fn (p ShellProfile) prompt_regex() &Re {
	return match p {
		.unix { &global_rules.prompt_unix }
		.cmd { &global_rules.prompt_cmd }
		.powershell { &global_rules.prompt_pwsh }
		.dumb { &global_rules.prompt_dumb }
	}
}

// is_prompt_sign: the glyphs a prompt ends with, powerline separators included.
pub fn is_prompt_sign(c rune) bool {
	return match c {
		`$`, `#`, `%`, `>`, `<`, `❯`, `➜`, `λ`, `→`, `»`, 0xE0B0, 0xE0B1, 0xE0B2, 0xE0B3 { true }
		else { false }
	}
}
