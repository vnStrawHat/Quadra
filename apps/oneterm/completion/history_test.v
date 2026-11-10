module completion

fn test_record_and_match_whole_line() {
	mut h := new_history(10)
	h.record(.unix, 'git commit -m msg', 1000)
	assert h.entries(.unix).len == 1
	assert h.entries(.unix)[0].line == 'git commit -m msg'
}

fn test_prefix_match_rejects_non_character_boundary() {
	assert !prefix_match('❯ prompt', 'x', false)
}

fn test_family_partitioned() {
	mut h := new_history(10)
	h.record(.cmd, 'dir /Q', 1000)
	assert h.entries(.unix).len == 0
	assert h.entries(.cmd).len == 1
}

fn test_repeat_updates_frecency_not_duplicates() {
	mut h := new_history(10)
	h.record(.unix, 'docker ps', 1000)
	h.record(.unix, 'docker ps', 2000)
	assert h.entries(.unix).len == 1
	assert h.entries(.unix)[0].use_count == 2
}

fn test_ring_evicts_oldest() {
	mut h := new_history(2)
	h.record(.unix, 'a', 1000)
	h.record(.unix, 'b', 2000)
	h.record(.unix, 'c', 3000)
	lines := h.entries(.unix).map(it.line)
	assert lines.len == 2
	assert 'a' !in lines && 'c' in lines
}

fn test_set_capacity_zero_clears() {
	mut h := new_history(10)
	h.record(.unix, 'ls', 1000)
	h.set_capacity(0)
	assert h.is_empty()
	h.record(.unix, 'ls', 2000)
	assert h.is_empty()
}

fn test_frecency_recent_beats_old() {
	recent := FrecencyRecord{'x', 1, 3_600_000}
	old := FrecencyRecord{'y', 1, 0}
	now := u64(7_200_000)
	assert recent.frecency(now) > old.frecency(now)
}

fn test_tokenize_simple() {
	t := tokenize('dir /Q')
	assert t.len == 2
	assert t[0].text == 'dir' && t[0].start == 0
	assert t[1].text == '/Q' && t[1].start == 4
}

fn test_tokenize_respects_quotes() {
	t := tokenize(r'cd "C:\Program Files"')
	assert t.len == 2
	assert t[1].text == r'C:\Program Files'
	assert t[1].start == 3
}

fn test_tokenize_single_quotes() {
	t := tokenize("echo 'a b c'")
	assert t.len == 2 && t[1].text == 'a b c'
}

fn test_command_context_first_token() {
	p := parse_line('di', 2)
	assert p.token == 'di' && p.token_start == 0 && p.is_first_token
	assert p.head == none
	assert p.prior_tokens.len == 0
}

fn test_trailing_space_after_command_is_subcommand_context() {
	p := parse_line('git ', 4)
	assert p.token == '' && p.token_start == 4 && !p.is_first_token
	assert p.head or { '' } == 'git'
	assert p.prior_tokens == ['git']
}

fn test_option_context_head_and_token() {
	p := parse_line('dir /', 5)
	assert p.token == '/' && !p.is_first_token
	assert p.head or { '' } == 'dir'
	assert p.prior_tokens == ['dir']
}

fn test_nested_prior_tokens() {
	p := parse_line('git remote ', 11)
	assert p.token == ''
	assert p.prior_tokens == ['git', 'remote']
	assert p.head or { '' } == 'git'
}

fn test_cursor_mid_line_ignores_text_after_cursor() {
	p := parse_line('git commit', 7)
	assert p.token == 'com'
	assert p.prior_tokens == ['git']
}
