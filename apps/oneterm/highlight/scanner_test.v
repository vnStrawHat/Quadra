module highlight

fn scan(line string, role RowRole) []Class {
	return scan_line(line, .unix, role, -1).map(class_from_u8(it))
}

fn idx(line string, needle string) int {
	return line.index(needle) or { panic('no ${needle}') }
}

fn test_error_keyword_tagged() {
	c := scan('error: something failed', .unmarked)
	assert c[0] == .error && c[4] == .error
	assert c[idx('error: something failed', 'failed')] == .error
}

fn test_word_boundary_prevents_false_match() {
	c := scan('node server', .unmarked)
	assert c[0] == .plain && c[1] == .plain
}

fn test_ipv4_tagged() {
	c := scan('ping 192.168.1.1', .unmarked)
	p := idx('ping 192.168.1.1', '192')
	assert c[p] == .ip && c[p + 10] == .ip
}

fn test_ipv6_compressed_full_address_tagged() {
	line := 'Resolved example.com -> 2607:f8b0:4004:80a::200e'
	c := scan(line, .unmarked)
	p := idx(line, '2607')
	for i in 0 .. '2607:f8b0:4004:80a::200e'.len {
		assert c[p + i] == .ip, 'char ${i}'
	}
}

fn test_ipv6_zone_id_not_tagged() {
	line := 'IPv6 link-local: fe80::1c2d:3e4f:5a6b%eth0'
	c := scan(line, .unmarked)
	p := idx(line, 'fe80')
	addr := 'fe80::1c2d:3e4f:5a6b'
	for i in 0 .. addr.len {
		assert c[p + i] == .ip
	}
	assert c[p + addr.len] != .ip && c[p + addr.len + 1] != .ip
}

fn test_ipv6_leading_double_colon_tagged() {
	line := 'localhost ::1'
	c := scan(line, .unmarked)
	p := idx(line, '::1')
	assert c[p] == .ip && c[p + 1] == .ip && c[p + 2] == .ip
}

fn test_ipv4_not_partial() {
	assert scan('1.2.3.4.5', .unmarked).all(it != .ip)
}

fn test_path_tagged() {
	c := scan('cat /usr/bin/env', .unmarked)
	assert c[idx('cat /usr/bin/env', '/usr')] == .path
}

fn test_path_relative_multi_segment_tagged() {
	line := 'Found 3 matches in src/views/terminal/mod.rs:128'
	c := scan(line, .unmarked)
	p := idx(line, 'src')
	for i in p .. p + 'src/views/terminal/mod.rs'.len {
		assert c[i] == .path, i.str()
	}
}

fn test_path_relative_with_extension_tagged() {
	line := 'edit src/main.rs now'
	c := scan(line, .unmarked)
	p := idx(line, 'src')
	for i in p .. p + 'src/main.rs'.len {
		assert c[i] == .path
	}
}

fn test_slash_word_not_path() {
	for line in ['eth0: link/ether aa:bb:cc:dd:ee:ff',
		'Throughput: 1_048_576 bytes/sec, latency: 2.5ms', 'DEBUG: retrying connection (attempt 3/5)'] {
		assert scan(line, .unmarked).all(it != .path), line
	}
}

fn test_number_tagged() {
	assert scan('port 8080', .unmarked)[5] == .number
}

fn test_number_with_percent_tagged() {
	c := scan('usage 89% detected', .unmarked)
	assert c[6] == .number && c[7] == .number && c[8] == .number
}

fn test_hex_number_tagged() {
	assert scan('addr 0x1F', .unmarked)[5] == .number
}

fn test_string_tagged() {
	line := r'echo "hello world"'
	c := scan(line, .unmarked)
	p := idx(line, '"')
	assert c[p] == .quoted && c[p + 5] == .quoted
}

fn test_permission_block_tagged() {
	c := scan('-rw-r--r--  2 user group', .unmarked)
	assert c[0] == .perm_type
	assert c[1] == .perm_read && c[4] == .perm_read && c[7] == .perm_read
	assert c[2] == .perm_write
	for p in [3, 5, 6, 8, 9] {
		assert c[p] == .perm_none
	}
}

fn test_permission_block_special_bits() {
	c := scan('drwsr-sr-t  2 user group', .unmarked)
	assert c[0] == .perm_type && c[1] == .perm_read && c[2] == .perm_write
	assert c[3] == .perm_special && c[6] == .perm_special && c[9] == .perm_special
}

fn test_operator_and_bracket_tagged() {
	c := scan('(a | b)', .unmarked)
	assert c[0] == .bracket && c[6] == .bracket
	assert c[3] == .operator
}

fn test_prompt_line_tagged() {
	c := scan(r'$ ls -la', .unmarked)
	assert c[0] == .prompt_sign
	assert c[2] == .command
	assert c[5] == .option
}

fn test_output_with_prompt_glyph_is_not_a_prompt_line() {
	mut c := scan('100% done', .unmarked)
	assert c[idx('100% done', '%')] != .prompt_sign
	assert c[idx('100% done', 'done')] != .command
	assert c[0] == .number
	c = scan('#include <stdio.h>', .unmarked)
	assert c[0] != .prompt_sign && c[1] != .command
	c = scan(r'$HOME=/root', .unmarked)
	assert c[0] != .prompt_sign && c[1] != .command
	line := r'user@host:~$ ls'
	c = scan(line, .unmarked)
	d := idx(line, r'$')
	assert c[d] == .prompt_sign && c[d + 2] == .command
}

fn test_scan_line_into_matches_scan_line_and_reuses_buffer() {
	mut out := []u8{len: 100, init: 7}
	mut scratch := ScanScratch{}
	for line in ['error: x', '日本語 error at /etc/hosts 42', r'$ ls -la',
		'ping 192.168.1.1 error', ''] {
		scan_line_into(line, .unix, .unmarked, -1, mut scratch, mut out)
		assert out == scan_line(line, .unix, .unmarked, -1)
	}
}

fn test_command_role_skips_prompt() {
	c := scan('git status', .command)
	assert c[0] == .command && c[2] == .command
}

fn test_command_separator_resets() {
	line := 'echo hi ; cat file'
	assert scan(line, .command)[idx(line, 'cat')] == .command
}

fn test_datetime_tagged() {
	assert scan('2026-06-23 log entry', .unmarked)[0] == .date_time
}

fn test_datetime_full_patterns_tagged() {
	for case_ in [['2024-01-15 09:30:45 INFO  Server started', '0', '19'],
		['12/25/2023 11:59 PM - session ended', '0', '19'],
		['Mon Jul 13 06:38:56 AM UTC 2026',
			'0', '31'],
		[
			'Last login: Wed Oct 25 10:15:30 UTC 2023',
			'12',
			'40',
		],
		[
			'Build finished at 2024-03-01T14:22:08.123Z',
			'18',
			'42',
		]] {
		line := case_[0]
		c := scan(line, .unmarked)
		for i in case_[1].int() .. case_[2].int() {
			assert c[i] == .date_time, line
		}
		end := case_[2].int()
		if end < c.len {
			assert c[end] != .date_time, line
		}
	}
}

fn test_clock_time_is_datetime_not_ip() {
	line := '2024-01-15 09:30:45'
	c := scan(line, .unmarked)
	p := idx(line, '09:30')
	assert c[p] == .date_time && c[p + 1] == .date_time
}

fn test_mac_tagged() {
	c := scan('mac aa:bb:cc:dd:ee:ff', .unmarked)
	assert c[4] == .mac
}

fn test_mac_with_digits_tagged_full() {
	line := 'mac aa:bb:cc:11:22:33 ok'
	c := scan(line, .unmarked)
	p := idx(line, 'aa:')
	for i in p .. p + 'aa:bb:cc:11:22:33'.len {
		assert c[i] == .mac
	}
}

fn test_ipv6_not_inside_path() {
	assert scan('warning: unused import: std::collections::HashMap', .unmarked).all(it != .ip)
}

fn test_empty_line() {
	assert scan('', .unmarked).len == 0
}

fn test_priority_string_over_keyword() {
	assert scan(r'"error"', .unmarked).all(it == .quoted)
}
