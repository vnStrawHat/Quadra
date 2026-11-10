// Formatting, sorting and the overwrite question: no UI, no I/O.
module sftpui

import ssh

fn test_format_size_picks_the_largest_unit() {
	assert format_size(0) == '0 B'
	assert format_size(1536) == '1.5 KB'
	assert format_size(5 * 1024 * 1024) == '5.0 MB'
	assert format_size(u64(2) * 1024 * 1024 * 1024 * 1024) == '2.0 TB'
}

fn test_format_permissions_shows_type_special_bits_and_octal() {
	assert format_permissions(0o100644) == '-rw-r--r-- (0644)'
	assert format_permissions(0o040755) == 'drwxr-xr-x (0755)'
	assert format_permissions(0o104644) == '-rwSr--r-- (4644)'
	assert format_permissions(0o041777) == 'drwxrwxrwt (1777)'
}

fn test_format_owner_prefers_name_with_id() {
	assert format_owner('root', 0) == 'root (0)'
	assert format_owner(none, 1000) == '1000'
	assert format_owner(none, none) == '-'
}

fn test_default_sort_is_folders_first_case_insensitive() {
	mut es := [fake_entry('/d', 'zeta.txt', false, 1, 1), fake_entry('/d', 'Beta', true, 0, 1),
		fake_entry('/d', 'alpha.txt', false, 1, 1), fake_entry('/d', 'alpha', true, 0, 1)]
	sort_entries(mut es, SortState{})
	assert es.map(it.name) == ['alpha', 'Beta', 'alpha.txt', 'zeta.txt']
	sort_entries(mut es, SortState{ on: true, col: .name, desc: true })
	assert es.map(it.name) == ['Beta', 'alpha', 'zeta.txt', 'alpha.txt']
	sort_entries(mut es, SortState{ on: true, col: .size, desc: true })
	assert es[0].is_dir && es[1].is_dir // folders stay first whatever the column
}

fn test_entry_names_reject_slashes_and_dot_components() {
	assert validate_entry_name('  notes.txt ')! == 'notes.txt'
	assert validate_entry_name('.env')! == '.env'
	for bad in ['   ', 'a/b', 'a\\b', '.', '..'] {
		if _ := validate_entry_name(bad) {
			assert false, bad
		}
	}
}

fn test_sanitize_keeps_one_safe_component() {
	assert sanitize_file_name('a:b?.txt') == 'a_b_.txt'
	assert sanitize_file_name('../../evil.txt') == '.._.._evil.txt'
	assert sanitize_file_name('..') == 'file'
	assert sanitize_file_name('CON') == 'file'
}

fn test_collisions_ignore_case_and_repeats() {
	existing := [fake_entry('/d', 'Report.txt', false, 1, 1),
		fake_entry('/d', 'logs', true, 0, 1)]
	got := collisions(['C:/x/report.txt', 'C:/y/report.txt', 'C:/logs', 'C:/new.txt'], existing)
	assert got.len == 2
	assert got[0].existing == 'Report.txt' && got[0].differs_in_case()
	assert got[1].existing_is_dir
}

fn test_keep_after_drops_only_colliding_names() {
	c := [Collision{'a.txt', 'a.txt', false}]
	assert keep_after(['/l/a.txt', '/l/b.txt'], c, false) == ['/l/b.txt']
	assert keep_after(['/l/a.txt'], c, true) == ['/l/a.txt']
}

fn test_the_prompt_says_merge_for_a_folder() {
	folder := collision_prompt([Collision{'logs', 'logs', true}], false)
	assert folder.title == 'Replace Remote Folder'
	assert folder.replace == 'Merge' && folder.keep == 'Cancel'
	many := collision_prompt([Collision{'a', 'a', false}, Collision{'b', 'b', false}], true)
	assert many.keep == 'Skip'
	assert many.body.starts_with('2 files already exist')
}

fn test_is_conflict_needs_two_equal_times() {
	assert !is_conflict(i64(5), i64(5))
	assert is_conflict(i64(5), i64(6))
	assert is_conflict(none, i64(6))
	assert is_conflict(i64(5), none)
}

fn test_entry_row_formats_once() {
	r := row_of(ssh.FileEntry{
		name:        'x'
		size:        2048
		permissions: 0o100600
	})
	assert r.cells[1] == '2.0 KB'
	assert r.cells[3] == '-rw------- (0600)'
}
