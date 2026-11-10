module highlight

import quadra.prim

fn re_find(re Re, line string) ?string {
	chars := line.runes()
	mut stack := []int{}
	start, end := re.find(chars, 0, mut stack)
	if start < 0 {
		return none
	}
	return chars[start..end].string()
}

fn re_match(re Re, line string) bool {
	if _ := re_find(re, line) {
		return true
	}
	return false
}

fn test_keyword_longest_wins() {
	chars := 'unknown'.runes()
	end, class := global_rules.longest_keyword(chars, 0)
	assert end == 7 && class == u8(Class.error)
	a, _ := global_rules.longest_keyword('known'.runes(), 0)
	assert a == 5
	x, y := global_rules.longest_keyword('Not Supported!'.runes(), 0)
	assert x == 13 && y == u8(Class.error)
}

fn test_ipv6_matches() {
	r := global_rules
	for s in ['2001:db8::1', 'fe80::1', 'fe80::1c2d:3e4f:5a6b', '2607:f8b0:4004:80a::200e',
		'fe80::1c2d:3e4f:5a6b%eth0', '::1', '::1c2d:3e4f:5a6b', '2001:db8:0:0:0:0:0:1'] {
		assert re_match(r.ipv6, s), s
	}
}

fn test_ipv6_full_match_length() {
	r := global_rules
	assert re_find(r.ipv6, 'fe80::1c2d:3e4f:5a6b') or { '' } == 'fe80::1c2d:3e4f:5a6b'
	assert re_find(r.ipv6, '2607:f8b0:4004:80a::200e') or { '' } == '2607:f8b0:4004:80a::200e'
	assert re_find(r.ipv6, 'fe80::1c2d:3e4f:5a6b%eth0') or { '' } == 'fe80::1c2d:3e4f:5a6b'
}

fn test_ipv6_does_not_match_clock_time() {
	assert !re_match(global_rules.ipv6, '09:30:45')
	assert !re_match(global_rules.ipv6, '11:59:00')
}

fn test_mac_matches() {
	assert re_match(global_rules.mac, 'aa:bb:cc:dd:ee:ff')
}

fn test_datetime_matches() {
	r := global_rules
	assert re_match(r.datetime, '2026-06-23')
	assert re_match(r.datetime, '14:30')
	assert re_match(r.datetime, 'Jan')
	for s in ['2024-01-15 09:30:45', '12/25/2023 11:59 PM', 'Wed Oct 25 10:15:30 UTC 2023',
		'Mon Jul 13 06:38:56 AM UTC 2026', '2024-03-01T14:22:08.123Z'] {
		assert re_find(r.datetime, s) or { '' } == s
	}
}

fn prompt_str(re Re, line string) ?string {
	chars := line.runes()
	mut stack := []int{}
	end := re.match_at(chars, 0, mut stack)
	if end < 0 {
		return none
	}
	return chars[..end].string()
}

fn test_unix_prompt_matches() {
	for line in ['user@host:~\$ ', 'user@host:~\$ ls', '[user@host ~]\$ ', '# ', '\$ echo hi',
		'~/src\$ ', '(venv) user@host:~/p\$ ', 'root@box:/srv# ', 'user@host:~%'] {
		assert re_match(global_rules.prompt_unix, line), line
	}
}

fn test_unix_prompt_no_match_on_output() {
	for line in ['hello world', 'error: something failed', '100% done', '#include <stdio.h>',
		r'$HOME=/root', 'total 100%', '50%'] {
		assert !re_match(global_rules.prompt_unix, line), line
	}
}

fn test_dumb_prompt_requires_space_or_eol_after_sign() {
	p := global_rules.prompt_dumb
	assert re_match(p, 'Router#') && re_match(p, 'Router> show ip')
	assert !re_match(p, '#include <stdio.h>') && !re_match(p, r'$HOME=/root')
}

fn test_cmd_prompt_matches_and_stops_at_the_sign() {
	p := global_rules.prompt_cmd
	assert re_match(p, r'C:\Users\me> ') && re_match(p, '> dir')
	for case_ in [
		[r'C:\Users\John Doe\Documents>echo hi', r'C:\Users\John Doe\Documents>'],
		[r'C:\Users\John Doe\oneterm workspace\deep> ', r'C:\Users\John Doe\oneterm workspace\deep> '],
		[r'C:\>', r'C:\>'],
		[r'\\server\share\My Files>dir', r'\\server\share\My Files>'],
		[r'C:\work>dir > out.txt', r'C:\work>'],
		['> dir', '> '],
		[r'C:\log size > 3', ''],
		['total 100%', ''],
		[r'see C:\x> not a prompt', ''],
	] {
		assert prompt_str(p, case_[0]) or { '' } == case_[1], case_[0]
	}
}

fn test_a_drive_anchored_line_is_a_prompt_only_when_its_head_is_a_path() {
	cmd := global_rules.prompt_cmd
	pwsh := global_rules.prompt_pwsh
	// (line, cmd match, PowerShell match); '' = no match
	for c in [
		[r'C:\src -> C:\dst', '', ''],
		[r'PS C:\src -> C:\dst', '', ''],
		[r"c:\proj\x.cpp(5): error C2059: syntax error: '>'", '', ''],
		[r'C:\Users\John Doe\projects>', r'C:\Users\John Doe\projects>', ''],
		[r'C:\work>dir > out.txt', r'C:\work>', ''],
		[r'\\server\share>', r'\\server\share>', ''],
		[r'PS C:\Users\me> ', '', r'PS C:\Users\me> '],
		[r'PS Env:\> dir', '', r'PS Env:\> '],
		[r'PS HKLM:\Software>', '', r'PS HKLM:\Software>'],
		[r'PS Microsoft.PowerShell.Core\FileSystem::\\server\share>', '',
			r'PS Microsoft.PowerShell.Core\FileSystem::\\server\share>'],
		[r'PS Microsoft.PowerShell.Core\FileSystem::C:\x> dir', '',
			r'PS Microsoft.PowerShell.Core\FileSystem::C:\x> '],
		['PS>', '', 'PS>'],
		['>> ', '>', '>> '],
		[r'C:\Program Files (x86)>cd ..', r'C:\Program Files (x86)>', ''],
	] {
		assert prompt_str(cmd, c[0]) or { '' } == c[1], 'cmd: ${c[0]}'
		assert prompt_str(pwsh, c[0]) or { '' } == c[2], 'pwsh: ${c[0]}'
	}
}

fn test_powershell_prompt_matches() {
	p := global_rules.prompt_pwsh
	for c in [
		[r'PS C:\Users\me> ', r'PS C:\Users\me> '],
		[r'PS C:\Users\John Doe\Documents> echo hi', r'PS C:\Users\John Doe\Documents> '],
		[r'PS \\server\share\My Files> dir', r'PS \\server\share\My Files> '],
		['PS>', 'PS>'],
		['>> ', '>> '],
		[r'PS C:\work> dir > out.txt', r'PS C:\work> '],
		['PS is > 3', ''],
		['PSReadLine loaded', ''],
		['hello world', ''],
	] {
		assert prompt_str(p, c[0]) or { '' } == c[1], c[0]
	}
}

fn test_prompt_sign_chars_and_option_prefix() {
	assert is_prompt_sign(`$`) && is_prompt_sign(`#`) && is_prompt_sign(`❯`)
	assert !is_prompt_sign(`a`)
	assert ShellProfile.unix.is_option_prefix(`-`) && !ShellProfile.unix.is_option_prefix(`/`)
	assert ShellProfile.cmd.is_option_prefix(`/`) && ShellProfile.cmd.is_option_prefix(`-`)
}

fn test_profile_for() {
	assert profile_for('cmd') == .cmd
	assert profile_for('pwsh') == .powershell
	assert profile_for('zsh') == .unix
	assert profile_for('whatever') == .dumb
}

const rp = RowRole.prompt
const ro = RowRole.output
const rn = RowRole.unmarked

fn roles(rows []RowRole) RowRoles {
	return RowRoles{
		role: rows
	}
}

fn test_empty_is_absent() {
	assert RowRoles{}.is_empty()
}

fn test_reset_clears_to_unmarked() {
	mut r := roles([rp, ro])
	r.reset(5)
	assert r.is_empty() && r.role.len == 5 && r.role_at(0) == .unmarked
}

fn test_role_at_out_of_range_is_unmarked() {
	assert roles([rp]).role_at(100) == .unmarked
}

fn test_absence_is_per_row_not_per_viewport() {
	r := roles([rn, rn, rp, ro])
	assert !r.is_empty()
	assert r.role_at(1) == .unmarked && r.role_at(2) == .prompt
}

fn run(start int, end int) PromptRun {
	return PromptRun{start, end}
}

fn test_last_completed_prompt_is_the_one_above_the_newest() {
	assert roles([rp, ro, ro, rp]).last_completed_prompt([]bool{len: 4}) or { run(9, 9) } == run(0,
		1)
}

fn test_a_wrapped_prompt_is_tinted_as_a_whole_run() {
	wraps := [true, false, false, true, false]
	assert roles([rp, rp, ro, rp, rp]).last_completed_prompt(wraps) or { run(9, 9) } == run(0, 2)
}

fn test_a_running_command_tints_the_previous_prompt_not_its_own() {
	assert roles([rp, ro, rp, ro, ro]).last_completed_prompt([]bool{len: 5}) or { run(9, 9) } == run(0,
		1)
}

fn test_two_prompts_on_adjacent_rows_are_two_runs() {
	assert roles([rp, rp]).last_completed_prompt([false, false]) or { run(9, 9) } == run(0, 1)
	assert roles([rp, rp]).last_completed_prompt([true, false]) == none
}

fn test_a_single_prompt_on_screen_is_not_tinted() {
	assert roles([rp, ro, ro]).last_completed_prompt([]bool{len: 3}) == none
	assert roles([rn, rn]).last_completed_prompt([]bool{len: 2}) == none
}

fn test_parse_hex_forms() {
	a := parse_color('#f00') or { panic('') }
	assert a == prim.parse_hex('#ff0000') or { panic('') }
	assert (parse_color('#F92672') or { panic('') }).a == 1.0
	assert parse_color('xyz') == none
	assert parse_color('#12') == none
}

fn test_default_styles_loaded() {
	s := load_default_styles()
	assert s.is_active()
	assert s.style(u8(Class.error)).has_fg
	assert s.style(u8(Class.url)).has_fg && s.style(u8(Class.url)).underline
}

fn test_unknown_key_is_ignored() {
	assert !parse_semantic_json(r'{"promptLineBg": "#262626"}').is_active()
}

fn test_empty_is_noop() {
	s := ClassStyles{}
	assert !s.is_active()
	assert !s.style(u8(Class.error)).has_fg
	assert !s.style(255).has_fg
}

fn test_spans_merge_runs_and_skip_plain() {
	styles := load_default_styles()
	sp := highlight_row('error: x', .unix, .output, -1, &styles)
	assert sp[0].start == 0 && sp[0].end == 5 && sp[0].class == .error && sp[0].style.has_fg
	assert sp.all(it.class != .plain)
}

fn test_overlay_disabled_returns_all_default() {
	o := new_overlay(.unix, false)
	mut scratch := ScanScratch{}
	mut out := []u8{}
	o.scan_into('error: failed', .unmarked, -1, mut scratch, mut out)
	assert out.len == 'error: failed'.len && out.all(it == 0)
}

fn test_overlay_enabled_tags_keywords() {
	o := new_overlay(.unix, true)
	mut scratch := ScanScratch{}
	mut out := []u8{}
	o.scan_into('error: failed', .unmarked, -1, mut scratch, mut out)
	assert out[0] == u8(Class.error)
}

fn test_overlay_marked_output_skips_the_prompt_regex() {
	o := new_overlay(.unix, true)
	mut scratch := ScanScratch{}
	mut marked := []u8{}
	o.scan_into(r'$ ls', .output, -1, mut scratch, mut marked)
	assert marked[0] != u8(Class.prompt_sign)
	mut plain_ := []u8{}
	o.scan_into(r'$ ls', .unmarked, -1, mut scratch, mut plain_)
	assert plain_[0] == u8(Class.prompt_sign)
}
