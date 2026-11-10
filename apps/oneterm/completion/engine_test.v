module completion

const fixture = [
	CatalogFile{'dir', 'external', 'cmd', '{ "schema": 1, "name": "dir", "options": ["/A", "/B", "/Q", "/S"] }'},
	CatalogFile{'date', 'external', 'cmd', '{ "schema": 1, "name": "date" }'},
	CatalogFile{'del', 'external', 'cmd', '{ "schema": 1, "name": "del" }'},
	CatalogFile{'ls', 'external', 'coreutils', '{ "schema": 1, "name": "ls", "options": ["-a", "-l", "--all", "--color"] }'},
	CatalogFile{'git', 'manual', 'common', '{ "schema": 1, "name": "git", "options": ["-C", "--version"],
		"subcommands": [
		  { "name": "commit", "options": ["-m", "--amend", "--no-verify"] },
		  { "name": "checkout", "options": ["-b"] },
		  { "name": "remote", "options": ["-v"],
		    "subcommands": [ { "name": "add", "options": ["-t", "-f", "--tags"] } ] }
		] }'},
]

fn engine() Engine {
	return Engine{
		catalog: catalog_from(fixture)
	}
}

fn ctx(family ShellFamily, line string) Context {
	return Context{family, line, line.len, 1_000_000}
}

fn texts(s []Suggestion) []string {
	return s.map(it.text)
}

fn test_command_context_lists_matching_commands() {
	mut e := engine()
	h := new_history(10)
	s := e.suggest(h, ctx(.cmd, 'd'), Params{})
	t := texts(s)
	assert 'dir' in t && 'date' in t && 'del' in t
	assert s.all(it.kind == .command)
}

fn test_bash_prompt_does_not_show_windows_commands() {
	mut e := engine()
	t := texts(e.suggest(new_history(10), ctx(.unix, 'd'), Params{}))
	assert 'dir' !in t && 'date' !in t
}

fn test_option_context_lists_options() {
	mut e := engine()
	s := e.suggest(new_history(10), ctx(.cmd, 'dir /'), Params{})
	t := texts(s)
	assert '/A' in t && '/Q' in t
	assert s.all(it.kind == .option)
}

fn test_option_context_narrows_on_prefix() {
	mut e := engine()
	assert texts(e.suggest(new_history(10), ctx(.cmd, 'dir /Q'), Params{})) == ['/Q']
}

fn test_unix_long_option_narrows() {
	mut e := engine()
	t := texts(e.suggest(new_history(10), ctx(.unix, 'ls --'), Params{}))
	assert '--all' in t && '--color' in t && '-a' !in t
}

fn test_subcommand_context_lists_children() {
	mut e := engine()
	t := texts(e.suggest(new_history(10), ctx(.unix, 'git '), Params{}))
	assert 'commit' in t && 'checkout' in t && 'remote' in t
}

fn test_nested_subcommand_context() {
	mut e := engine()
	assert 'add' in texts(e.suggest(new_history(10), ctx(.unix, 'git remote '), Params{}))
}

fn test_subcommand_option_context_uses_active_node() {
	mut e := engine()
	t := texts(e.suggest(new_history(10), ctx(.unix, 'git commit --'), Params{}))
	assert '--amend' in t && '--no-verify' in t
}

fn test_nested_option_context() {
	mut e := engine()
	t := texts(e.suggest(new_history(10), ctx(.unix, 'git remote add -'), Params{}))
	assert '-t' in t && '-f' in t
}

fn test_ancestor_options_inherited_but_ranked_lower() {
	mut e := engine()
	t := texts(e.suggest(new_history(10), ctx(.unix, 'git remote add -'), Params{}))
	own := t.index('-t')
	anc := t.index('-v')
	assert own >= 0
	assert anc < 0 || own < anc
}

fn test_inherit_off_hides_ancestor_options() {
	mut e := engine()
	t := texts(e.suggest(new_history(10), ctx(.unix, 'git remote add -'), Params{
		inherit_ancestor_options: false
	}))
	assert '-v' !in t
}

fn test_unknown_subcommand_falls_back_to_command_options() {
	mut e := engine()
	assert '-C' in texts(e.suggest(new_history(10), ctx(.unix, 'git frobnicate -'), Params{}))
}

fn test_breadcrumb_is_the_resolved_path() {
	mut e := engine()
	assert e.breadcrumb(ctx(.unix, 'git remote add -'), Params{}) == ['git', 'remote', 'add']
	assert e.breadcrumb(ctx(.unix, 'git '), Params{}) == ['git']
	assert e.breadcrumb(ctx(.unix, 'nope -'), Params{}) == []
}

fn test_history_beats_catalog_and_dedups_with_h_tag() {
	mut e := engine()
	mut h := new_history(10)
	for t in [u64(1000), 2000, 3000] {
		h.record(.cmd, 'dir', t)
	}
	s := e.suggest(h, ctx(.cmd, 'd'), Params{})
	dirs := s.filter(it.text == 'dir')
	assert dirs.len == 1
	assert dirs[0].kind == .history
	assert dirs[0].kind.tag() == 'H'
	assert s[0].text == 'dir'
}

fn test_prefix_beats_fuzzy() {
	mut e := engine()
	assert e.suggest(new_history(10), ctx(.cmd, 'da'), Params{})[0].text == 'date'
}

fn test_case_sensitivity_per_family() {
	mut e := engine()
	h := new_history(10)
	assert texts(e.suggest(h, ctx(.cmd, 'DI'), Params{})).contains('dir')
	assert !texts(e.suggest(h, ctx(.unix, 'LS'), Params{})).contains('ls')
}

fn test_is_prefix_of_typed_handles_multibyte_candidate() {
	s := Suggestion{
		text: '日x'
		kind: .history
	}
	assert !s.is_prefix_of_typed('x')
	assert s.is_prefix_of_typed('日')
	assert s.is_prefix_of_typed('日x')
	assert !s.is_prefix_of_typed('日xy')
}

fn test_match_option_handles_multibyte_flag() {
	assert match_option('-日x', '-x', Params{}) or { Match{true, 99} } == Match{false, 0}
	assert match_option('-日x', '-x', Params{ fuzzy: false }) == none
	assert match_option('-日x', '-日', Params{}) or { Match{false, 0} } == Match{true, '-日'.len}
}

fn test_fuzzy_history_match_with_multibyte_head() {
	mut e := engine()
	mut h := new_history(10)
	h.record(.cmd, '日x', 1000)
	s := e.suggest(h, ctx(.cmd, 'x'), Params{})
	hit := s.filter(it.text == '日x')
	assert hit.len == 1
	assert !hit[0].is_prefix_of_typed('x')
}

fn test_history_option_with_multibyte_flag() {
	mut e := engine()
	mut h := new_history(10)
	h.record(.cmd, 'foo -日x', 1000)
	assert texts(e.suggest(h, ctx(.cmd, 'foo -x'), Params{})).contains('-日x')
}

fn test_min_prefix_len_suppresses_short_tokens() {
	mut e := engine()
	assert e.suggest(new_history(10), ctx(.cmd, 'd'), Params{ min_prefix_len: 2 }).len == 0
}

fn test_recorded_secret_never_suggested() {
	mut e := engine()
	mut h := new_history(10)
	h.record(.unix, 'deploy --token ghp_0123456789abcdefghij', 1000)
	s := e.suggest(h, ctx(.unix, 'deploy '), Params{})
	assert s.all(!it.text.contains('ghp_'))
}

fn test_sources_toggle_off_history() {
	mut e := engine()
	mut h := new_history(10)
	h.record(.cmd, 'docker ps', 1000)
	s := e.suggest(h, ctx(.cmd, 'd'), Params{
		sources: SourceToggles{
			memory: false
		}
	})
	assert !texts(s).contains('docker')
}

fn test_embedded_catalog_suggests_cmd_commands_for_d() {
	mut e := new_engine()
	s := e.suggest(new_history(10), ctx(.cmd, 'd'), Params{})
	assert s.len > 0
	t := texts(s)
	assert 'dir' in t || 'date' in t || 'del' in t
}

fn test_embedded_catalog_suggests_unix_commands_for_l() {
	mut e := new_engine()
	assert texts(e.suggest(new_history(10), ctx(.unix, 'l'), Params{})).contains('ls')
}

fn test_option_description_flows_from_manual_catalog() {
	mut e := new_engine()
	s := e.suggest(new_history(10), ctx(.unix, 'git checkout -b'), Params{})
	b := s.filter(it.text == '-b')
	assert b.len == 1
	assert b[0].description == 'new-branch'
}
