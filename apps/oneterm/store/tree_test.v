module store

fn tree_entry(id u64, label string, group ?string) Entry {
	return entry_of(id, Session{
		label: label
		host:  '${label}.example'
		group: group
	})
}

fn test_rename_group_moves_only_its_members() {
	mut entries := [tree_entry(1, 'a', 'infra'), tree_entry(2, 'b', 'other'),
		tree_entry(3, 'c', none)]
	assert rename_group_in(mut entries, 'infra', '  platform  ')
	assert entries[0].session.group or { '' } == 'platform'
	assert entries[1].session.group or { '' } == 'other'
	assert entries[2].session.group == none
	assert rename_group_in(mut entries, 'platform', '   ')
	assert entries[0].session.group == none
	assert !rename_group_in(mut entries, 'missing', 'x')
}

fn labelled(labels []string) []Entry {
	return labels.map(tree_entry(1, it, none))
}

fn test_copy_label_numbers_only_the_candidates_already_in_use() {
	assert copy_label('prod', labelled(['prod'])) == 'prod (copy)'
	assert copy_label('prod', labelled(['prod', 'prod (copy)'])) == 'prod (copy 2)'
	assert copy_label('prod', labelled(['prod', 'prod (copy)', 'prod (copy 2)'])) == 'prod (copy 3)'
	assert copy_label('prod (copy)', labelled(['prod (copy)'])) == 'prod (copy) (copy)'
	assert copy_label('prod', labelled(['staging'])) == 'prod (copy)'
	assert copy_label('prod', []) == 'prod (copy)'
}

fn test_duplicate_copies_every_field_next_to_its_source() {
	mut s := tree_entry(3, 'prod', 'infra').session
	s.port = 2200
	s.username = 'deploy'
	s.sealed_password = 'otv1:x'
	s.agent_forwarding = true
	mut entries := [tree_entry(1, 'first', none), entry_of(3, s),
		tree_entry(4, 'last', none)]
	assert duplicate_in(mut entries, 3, 5)
	assert entries.map(it.id) == [u64(1), 3, 5, 4]
	copy := entries[2].session
	assert copy.label == 'prod (copy)'
	assert copy.port == 2200 && copy.username or { '' } == 'deploy'
	assert copy.sealed_password or { '' } == 'otv1:x' && copy.agent_forwarding
	assert copy.group or { '' } == 'infra'
	assert !duplicate_in(mut entries, 99, 6)
	assert entries.len == 4
}

fn test_set_group_moves_exactly_one_session() {
	mut entries := [tree_entry(1, 'a', 'infra'), tree_entry(2, 'b', none)]
	assert set_group_in(mut entries, 2, 'infra')
	assert entries[1].session.group or { '' } == 'infra'
	assert !set_group_in(mut entries, 2, 'infra')
	assert set_group_in(mut entries, 1, none)
	assert entries[0].session.group == none
	assert set_group_in(mut entries, 2, '   ')
	assert entries[1].session.group == none
	assert !set_group_in(mut entries, 99, 'infra')
}

fn test_normalized_group() {
	assert normalized_group(' infra ') or { '' } == 'infra'
	assert normalized_group('   ') == none
	assert normalized_group('') == none
	assert normalized_group(none) == none
}

fn ids(items []TreeItem) []string {
	mut out := []string{}
	for item in items {
		if item.children.len == 0 {
			out << item.id
		} else {
			out << item.children.map(it.id)
		}
	}
	return out
}

fn test_tree_groups_and_sorts() {
	entries := [tree_entry(10, 'zulu', none), tree_entry(11, 'Alpha', none),
		tree_entry(12, 'web-2', 'prod'), tree_entry(13, 'web-1', 'prod'),
		tree_entry(14, 'db', 'dev'), tree_entry(15, 'blank-group', '  ')]
	items := build_tree(entries, '')
	assert items.map(it.id) == ['session:11', 'session:15', 'session:10', 'group:dev', 'group:prod']
	assert items[4].children.map(it.id) == ['session:13', 'session:12']
	assert parse_session_item('session:13') or { 0 } == 13
	assert parse_session_item('group:prod') == none
	assert parse_group_item('group:prod') or { '' } == 'prod'
}

fn test_tree_filter_matches_label_host_user_and_group() {
	mut gamma := tree_entry(3, 'gamma', none)
	gamma.session.host = 'gamma.example'
	mut beta := tree_entry(2, 'beta', 'Infra')
	beta.session.host = 'b.host'
	beta.session.username = 'steven'
	entries := [tree_entry(1, 'Alpha', none), beta, gamma]
	assert ids(build_tree(entries, 'ALPHA')) == ['session:1']
	assert ids(build_tree(entries, 'gamma.example')) == ['session:3']
	assert ids(build_tree(entries, 'even')) == ['session:2']
	assert ids(build_tree(entries, 'infra')) == ['session:2']
	assert ids(build_tree(entries, 'nothing')).len == 0
}

fn test_subtitle_shows_user_only_when_present() {
	mut s := Session{
		host: 'h'
		port: 22
	}
	assert session_subtitle(s) == 'h:22'
	s.username = 'u'
	assert session_subtitle(s) == 'u@h:22'
}

fn test_menu_entries_order_and_fallbacks() {
	assert menu_entries([]).len == 0
	entries := [tree_entry(1, 'b', 'g2'), tree_entry(2, 'a', none),
		tree_entry(3, 'c', 'g1'), tree_entry(4, 'd', 'g2'), tree_entry(5, 'blank', '   '),
		tree_entry(6, '  padded  ', none)]
	sections := menu_entries(entries)
	assert sections.map(it.group) == ['', 'g2', 'g1']
	assert sections[0].rows.map(it.id) == [u64(2), 5, 6]
	assert sections[1].rows.map(it.id) == [u64(1), 4]
	assert sections[0].rows[2].title == 'padded'
	mut no_label := tree_entry(7, '  ', none)
	no_label.session.username = 'u'
	assert menu_entries([no_label])[0].rows[0].title == 'u@  .example:22'
}

fn test_every_surface_resolves_a_colour_alike() {
	cases := [['', default_color_hex], ['   ', default_color_hex],
		[' #E06C75 ', default_color_hex], ['#abc', default_color_hex],
		['#GGGGGG', default_color_hex], ['#56b6c2', '#56b6c2'],
		['98C379', '98C379'], ['#56B6C280', '#56B6C280']]
	for c in cases {
		mut e := tree_entry(1, 'probe', none)
		e.session.color = c[0]
		assert session_color_hex(e.session) == c[1]
		assert menu_entries([e])[0].rows[0].color == c[1]
	}
	assert session_color_hex(Session{}) == default_color_hex
}
