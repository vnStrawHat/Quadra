module completion

const raw = [
	CatalogFile{'dir', 'external', 'cmd', '{ "schema": 1, "name": "dir", "options": ["/A", "/B"] }'},
	CatalogFile{'ls', 'external', 'coreutils', '{ "schema": 1, "name": "ls", "options": ["-a", "--all"] }'},
	CatalogFile{'git', 'manual', 'common', '{ "schema": 1, "name": "git", "options": ["-C"],
		"subcommands": [ { "name": "commit", "options": ["-m", "--amend"] } ] }'},
	CatalogFile{'bad', 'external', 'cmd', '{ this is not json'},
	CatalogFile{'future', 'external', 'cmd', '{ "schema": 99, "name": "future" }'},
]

fn cat() Catalog {
	return catalog_from(raw)
}

fn test_parses_string_and_object_option_forms() {
	node :=
		parse_node('{ "schema": 1, "name": "grep", "options": ["-a", { "flag": "--all", "description": "x" }] }')!
	assert node.options[0].text == '-a'
	assert node.options[1].text == '--all'
	assert node.options[1].description == 'x'
}

fn test_rejects_unknown_major_schema() {
	if _ := parse_node('{ "schema": 99, "name": "x" }') {
		assert false
	}
}

fn test_lookup_resolves_by_category_path() {
	mut c := cat()
	cmd := ShellFamily.cmd.categories(false)
	assert c.lookup('dir', cmd, .cmd) != none
	assert c.lookup('ls', cmd, .cmd) == none
	unix := ShellFamily.unix.categories(false)
	assert c.lookup('ls', unix, .unix) != none
}

fn test_lookup_is_case_insensitive_for_cmd_only() {
	mut c := cat()
	assert c.lookup('DIR', ShellFamily.cmd.categories(false), .cmd) != none
	assert c.lookup('LS', ShellFamily.unix.categories(false), .unix) == none
}

fn test_subcommand_child_lookup() {
	mut c := cat()
	git := c.lookup('git', [Category.common], .unix) or { panic('git') }
	commit := git.child('commit', .unix) or { panic('commit') }
	assert commit.options.len == 2
}

fn test_malformed_file_skipped_without_breaking_siblings() {
	mut c := cat()
	cmd := ShellFamily.cmd.categories(false)
	assert c.lookup('bad', cmd, .cmd) == none
	assert c.lookup('future', cmd, .cmd) == none
	assert c.lookup('dir', cmd, .cmd) != none
}

fn test_command_names_deduped_across_categories() {
	c := cat()
	names := c.command_names(ShellFamily.unix.categories(false), .unix)
	assert 'ls' in names && 'git' in names
	assert 'dir' !in names
}

fn test_command_names_prefer_manual_spelling_over_external_case_variant() {
	c := catalog_from([
		CatalogFile{'COPY', 'external', 'cmd', '{ "schema": 1, "name": "COPY", "options": [] }'},
		CatalogFile{'dir', 'external', 'cmd', '{ "schema": 1, "name": "dir", "options": [] }'},
		CatalogFile{'copy', 'manual', 'cmd', '{ "schema": 1, "name": "copy", "options": [] }'},
	])
	assert c.command_names(ShellFamily.cmd.categories(false), .cmd) == ['copy', 'dir']
	mut unix := c.command_names([Category.cmd], .unix)
	unix.sort()
	assert unix == ['COPY', 'copy', 'dir']
}

fn test_embedded_catalogs_all_parse() {
	mut c := catalog_from(catalog_files)
	assert c.entries.len == 87
	for i in 0 .. c.entries.len {
		assert c.node_at(i) != none, c.entries[i].name
	}
}

fn test_kind_maps_to_family() {
	assert family_from_kind('cmd') == .cmd
	assert family_from_kind('powershell') == .powershell
	assert family_from_kind('pwsh') == .powershell
	assert family_from_kind('bash') == .unix
	assert family_from_kind('custom') == .unix
}

fn test_category_search_paths() {
	assert ShellFamily.cmd.categories(false) == [Category.cmd, .windows, .common]
	assert ShellFamily.powershell.categories(false) == [Category.powershell, .cmd, .windows, .common]
	assert ShellFamily.unix.categories(false) == [Category.coreutils, .linux, .common]
	assert ShellFamily.cmd.categories(true) == [Category.cmd, .windows, .common, .coreutils, .linux]
	assert ShellFamily.unix.categories(true) == [Category.coreutils, .linux, .common]
}

fn test_option_triggers_per_family() {
	assert ShellFamily.cmd.is_option_token('/x') && ShellFamily.cmd.is_option_token('-x')
	assert !ShellFamily.powershell.is_option_token('/x')
	assert !ShellFamily.unix.is_option_token('/x')
}

fn test_force_family_parsing() {
	assert family_from_config('cmd') or { panic('') } == .cmd
	assert family_from_config('POWERSHELL') or { panic('') } == .powershell
	assert family_from_config('unix') or { panic('') } == .unix
	assert family_from_config('nope') == none
}
