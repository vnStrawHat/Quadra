// Shell family: option triggers, case rules and the catalog search path (specs/24).
module completion

// Category is a folder-derived catalog category.
enum Category {
	cmd
	coreutils
	powershell
	windows
	linux
	common
}

fn category_from_str(s string) ?Category {
	return match s {
		'cmd' { Category.cmd }
		'coreutils' { Category.coreutils }
		'powershell' { Category.powershell }
		'windows' { Category.windows }
		'linux' { Category.linux }
		'common' { Category.common }
		else { none }
	}
}

// ShellFamily is the only thing the engine knows about the running shell.
pub enum ShellFamily {
	cmd
	powershell
	unix
}

// family_from_kind collapses a shell kind ("cmd", "powershell", "pwsh", "bash", ...); an
// unknown kind counts as Unix.
pub fn family_from_kind(kind string) ShellFamily {
	return match kind {
		'cmd' { .cmd }
		'powershell', 'pwsh' { .powershell }
		else { .unix }
	}
}

// family_from_config parses the `force_family` setting.
pub fn family_from_config(s string) ?ShellFamily {
	return match s.to_lower() {
		'cmd' { ShellFamily.cmd }
		'powershell', 'pwsh' { ShellFamily.powershell }
		'unix', 'bash', 'zsh', 'sh' { ShellFamily.unix }
		else { none }
	}
}

// categories returns the search path, high to low precedence.
fn (f ShellFamily) categories(allow_coreutils bool) []Category {
	mut c := match f {
		.cmd { [Category.cmd, .windows, .common] }
		.powershell { [Category.powershell, .cmd, .windows, .common] }
		.unix { [Category.coreutils, .linux, .common] }
	}

	if allow_coreutils && f != .unix {
		c << Category.coreutils
		c << Category.linux
	}
	return c
}

fn (f ShellFamily) option_triggers() []u8 {
	return if f == .cmd { [u8(`/`), `-`] } else { [u8(`-`)] }
}

fn (f ShellFamily) case_insensitive() bool {
	return f != .unix
}

fn (f ShellFamily) is_option_token(token string) bool {
	return token.len > 0 && token[0] in f.option_triggers()
}
