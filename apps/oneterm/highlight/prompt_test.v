module highlight

fn scan(line string, role RowRole) []Class {
	return scan_line(line, .unix, role, -1).map(class_from_u8(it))
}

fn idx(line string, needle string) int {
	return line.index(needle) or { panic('no ${needle}') }
}

fn scan_p(line string, role RowRole, profile ShellProfile) []Class {
	return scan_line(line, profile, role, -1).map(class_from_u8(it))
}

fn scan_marked(line string, role RowRole, input_at int, profile ShellProfile) []Class {
	return scan_line(line, profile, role, input_at).map(class_from_u8(it))
}

// class_at is the class of the char that starts `needle` in `line`.
fn class_at(line string, profile ShellProfile, needle string) Class {
	c := scan_p(line, .unmarked, profile)
	return c[line[..idx(line, needle)].runes().len]
}

fn test_prompt_sign_only_is_tagged() {
	line := r'user@host:~/dir$ ls'
	c := scan(line, .unmarked)
	d := idx(line, r'$')
	for i in 0 .. d {
		assert c[i] != .prompt_sign
	}
	assert c[d] == .prompt_sign
}

fn test_unix_prompt_path_before_sign_is_highlighted() {
	line := r'user@host:~/dir$ ls'
	c := scan(line, .unmarked)
	assert c[idx(line, '~')] == .path
	assert c[idx(line, r'$') - 1] == .path
}

fn test_windows_prompt_sign_only_is_tagged() {
	line := r'C:\Users\foo> dir'
	c := scan_p(line, .unmarked, .cmd)
	gt := idx(line, '>')
	for i in 0 .. gt {
		assert c[i] != .prompt_sign
	}
	assert c[gt] == .prompt_sign
}

fn test_windows_prompt_path_before_sign_is_highlighted() {
	for line in [r'C:\Users\foo> dir', r'D:\Rust\myTerm2>dir', r'D:\Rust\myTerm2>',
		r'D:\Rust\myTerm2> ', r'D:\Rust\myTerm2> x'] {
		c := scan_p(line, .unmarked, .cmd)
		gt := idx(line, '>')
		assert c[0] == .path, line
		assert c[gt - 1] == .path, line
		assert c[gt] == .prompt_sign, line
	}
}

fn test_windows_prompt_path_highlighted_without_trailing_space() {
	line := r'D:\Rust\myTerm2>dir'
	c := scan_p(line, .unmarked, .cmd)
	assert c[idx(line, 'dir')] == .command
	line2 := r'D:\Rust\myTerm2> x'
	assert scan_p(line2, .unmarked, .cmd)[idx(line2, 'x')] == .command
}

fn test_output_option_long_and_short() {
	line := ' -d, --data <data>  HTTP POST data'
	c := scan(line, .unmarked)
	d := idx(line, ' -d') + 1
	data := idx(line, '--data')
	assert c[d] == .option && c[d + 1] == .option
	assert c[data] == .option && c[data + 5] == .option
}

fn test_option_not_tagged_inside_filename() {
	line := 'drwxr-xr-x 2 root root 4.0K May 21 2025 container-diff-linux-amd64'
	assert scan(line, .unmarked).all(it != .option)
}

fn test_number_not_tagged_inside_identifier() {
	for line in ['result: math-2 ok', 'read file_v2 now', 'stage step-1 done'] {
		assert scan(line, .unmarked).all(it != .number), line
	}
}

fn test_standalone_number_still_tagged() {
	line := 'error -1 happened at 89%'
	c := scan(line, .unmarked)
	m := idx(line, '-1')
	p := idx(line, '89')
	assert c[m] == .number && c[m + 1] == .number
	assert c[p] == .number && c[p + 1] == .number && c[p + 2] == .number
}

fn test_cross_shell_prompt_unix_inside_cmd() {
	line := r'user@host:~$ echo hi'
	c := scan_p(line, .unmarked, .cmd)
	assert c[idx(line, r'$')] == .prompt_sign
	assert c[idx(line, 'echo')] == .command
}

fn test_cmd_prompt_with_spaces_in_the_cwd_is_a_prompt() {
	line := r'C:\Users\John Doe\oneterm workspace>echo hi'
	c := scan_p(line, .unmarked, .cmd)
	gt := idx(line, '>')
	assert c[gt] == .prompt_sign && c[0] == .path && c[gt - 1] == .path
	assert class_at(line, .cmd, 'echo') == .command
}

fn test_powershell_prompt_is_a_prompt() {
	line := r'PS C:\Users\John Doe\ws> echo hi -Force'
	c := scan_p(line, .unmarked, .powershell)
	gt := idx(line, '>')
	assert c[gt] == .prompt_sign && c[gt - 1] == .path
	assert class_at(line, .powershell, 'echo') == .command
	assert class_at(line, .powershell, '-Force') == .option
}

fn test_unc_prompt_is_a_prompt() {
	for case_ in [[r'\\server\share\My Files>dir', 'cmd'],
		[r'PS \\server\share\My Files> dir', 'powershell']] {
		line := case_[0]
		p := profile_for(case_[1])
		c := scan_p(line, .unmarked, p)
		assert c[idx(line, '>')] == .prompt_sign, line
		assert class_at(line, p, 'dir') == .command, line
	}
}

fn test_a_redirection_is_not_the_prompt_sign() {
	line := r'C:\work>dir > out.txt'
	c := scan_p(line, .unmarked, .cmd)
	assert c[idx(line, '>')] == .prompt_sign
	assert c[line.last_index_u8(`>`)] != .prompt_sign
}

fn test_windows_output_is_not_a_prompt() {
	assert scan_p(r'C:\log size > 3', .unmarked, .cmd).all(it != .prompt_sign)
	assert scan_p('PS is > 3', .unmarked, .powershell).all(it != .prompt_sign)
	assert scan_p(r'see C:\x> not a prompt', .unmarked, .cmd).all(it != .prompt_sign)
}

fn test_drive_anchored_output_with_an_unspaced_angle_bracket_is_output() {
	for line in [r'C:\src -> C:\dst', r"c:\proj\x.cpp(5): error C2059: syntax error: '>'"] {
		for profile in [ShellProfile.cmd, .powershell, .unix, .dumb] {
			assert scan_p(line, .unmarked, profile).all(it != .prompt_sign), line
		}
	}
	mut line := r"c:\proj\x.cpp(5): error C2059: syntax error: '>'"
	mut c := scan_p(line, .unmarked, .cmd)
	assert c[idx(line, 'error')] == .error
	line = r'C:\src -> C:\dst'
	c = scan_p(line, .unmarked, .cmd)
	assert c[idx(line, '->') + 1] != .path
	assert c[0] == .path
}

fn test_a_bare_angle_bracket_is_a_prompt_only_on_the_windows_profiles() {
	for line in ['> quoted text from a mail reply', '> 3 files changed'] {
		assert scan_p(line, .unmarked, .unix).all(it != .prompt_sign), line
		for profile in [ShellProfile.cmd, .powershell] {
			assert scan_p(line, .unmarked, profile)[0] == .prompt_sign, line
		}
	}
	assert scan_p('>> Get-Date', .unmarked, .powershell).contains(.prompt_sign)
}

fn test_multibyte_text_does_not_shift_keyword_classes() {
	line := '\u65e5\u672c\u8a9e error here'
	c := scan(line, .unmarked)
	e := line.runes().index(`e`)
	for i in e .. e + 5 {
		assert c[i] == .error
	}
	for i in 0 .. e {
		assert c[i] != .error
	}
	assert c[line.runes().len - 4] == .plain
}

fn test_a_structural_match_after_an_emoji_lands_on_the_right_chars() {
	line := '🚀 2026-09-21 10:00:00 done'
	c := scan(line, .unmarked)
	stamp := line.runes().index(`2`)
	assert stamp == 2
	for i in stamp .. stamp + 19 {
		assert c[i] == .date_time
	}
	assert c[0] == .plain
	assert c[line.runes().len - 1] == .plain
}

fn test_a_marked_prompt_with_a_space_in_it_still_finds_its_sign() {
	line := r'PS C:\Program Files> dir'
	sign := idx(line, '>')
	c := scan_marked(line, .prompt, sign + 2, .powershell)
	assert c[sign] == .prompt_sign
	assert c[3] == .path
	assert c[sign + 2] == .command
}

fn test_a_marked_prompt_without_a_boundary_uses_the_whole_line() {
	line := r'C:\work>'
	c := scan_marked(line, .prompt, -1, .cmd)
	assert c[line.len - 1] == .prompt_sign
	assert c[0] == .path
}

fn test_a_marked_prompt_with_an_unknown_glyph_uses_its_last_character() {
	c := scan_marked('~/src| ls', .prompt, 7, .unix)
	assert c[5] == .prompt_sign
	assert c[7] == .command
}

fn test_a_marked_command_line_is_command_mode() {
	c := scan_marked('cargo test --lib', .command, -1, .unix)
	assert c[0] == .command
	assert c[12] == .option
}

fn test_marked_output_never_runs_the_prompt_regex() {
	assert scan_marked(r'C:\src -> C:\dst', .output, -1, .cmd).all(it != .prompt_sign)
	redirect := r'C:\work>dir > out.txt'
	marked := scan_marked(redirect, .output, -1, .cmd)
	assert marked.all(it != .prompt_sign)
	assert marked != scan_p(redirect, .unmarked, .cmd)
}

fn test_the_exit_code_tints_the_sign() {
	line := r'user@host:~$ ls'
	sign := idx(line, r'$')
	for code in [0, 101, -1] {
		mut classes := scan_line(line, .unix, .unmarked, -1)
		tint_prompt_sign(mut classes, code)
		assert classes[sign] == u8(if code == 0 {
			Class.success
		} else {
			Class.error
		})
		assert classes[sign + 2] == u8(Class.command)
	}
}

fn test_an_untinted_sign_keeps_its_class() {
	line := r'user@host:~$ ls'
	assert scan_line(line, .unix, .unmarked, -1)[idx(line, r'$')] == u8(Class.prompt_sign)
}
