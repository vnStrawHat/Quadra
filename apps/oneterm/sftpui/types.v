// Formatting, sorting and the column set of the browser.
module sftpui

import ssh
import time

pub enum SortCol {
	name
	modified
	size
	permissions
	owner
	group
}

// SortState: `on` false is the default order (Name ascending).
pub struct SortState {
pub mut:
	on   bool
	col  SortCol
	desc bool
}

pub struct Column {
pub:
	key   string
	label string
	right bool
	sort  SortCol
pub mut:
	visible bool = true
	width   f32
}

pub const name_min_width = f32(100)
pub const column_min_width = f32(40)
pub const column_max_width = f32(800)

// remote_columns is the canonical order, left to right. Name is always visible and fills
// what the others leave.
pub fn remote_columns() []Column {
	return [
		Column{
			key:   'name'
			label: 'Name'
			sort:  .name
			width: name_min_width
		},
		Column{
			key:     'size'
			label:   'Size'
			right:   true
			sort:    .size
			width:   72
			visible: false
		},
		Column{
			key:   'modified'
			label: 'Date Modified'
			sort:  .modified
			width: 140
		},
		Column{
			key:   'permissions'
			label: 'Permissions'
			sort:  .permissions
			width: 150
		},
		Column{
			key:     'owner'
			label:   'Owner'
			sort:    .owner
			width:   90
			visible: false
		},
		Column{
			key:     'group'
			label:   'Group'
			sort:    .group
			width:   90
			visible: false
		},
	]
}

// local_columns are the first three of the same order, so rows are built the same way.
pub fn local_columns() []Column {
	return [
		Column{
			key:   'name'
			label: 'Name'
			sort:  .name
			width: name_min_width
		},
		Column{
			key:   'size'
			label: 'Size'
			right: true
			sort:  .size
			width: 72
		},
		Column{
			key:   'modified'
			label: 'Date Modified'
			sort:  .modified
			width: 140
		},
	]
}

pub fn format_size(bytes u64) string {
	if bytes < 1024 {
		return '${bytes} B'
	}
	if bytes < 1024 * 1024 {
		return '${f64(bytes) / 1024.0:.1f} KB'
	}
	if bytes < 1024 * 1024 * 1024 {
		return '${f64(bytes) / (1024.0 * 1024.0):.1f} MB'
	}
	if bytes < u64(1024) * 1024 * 1024 * 1024 {
		return '${f64(bytes) / (1024.0 * 1024.0 * 1024.0):.1f} GB'
	}
	return '${f64(bytes) / (1024.0 * 1024.0 * 1024.0 * 1024.0):.1f} TB'
}

// format_date is local time, `YYYY-MM-DD HH:mm`; '' when unknown.
pub fn format_date(secs ?i64) string {
	t := secs or { return '' }
	return time.unix(t).local().custom_format('YYYY-MM-DD HH:mm')
}

fn flag(on bool, c string) string {
	return if on { c } else { '-' }
}

fn exec_bit(x bool, special bool, on string, off string) string {
	if x {
		return if special { on } else { 'x' }
	}
	return if special { off } else { '-' }
}

// format_permissions gives `drwxr-xr-x (0755)`.
pub fn format_permissions(perm u32) string {
	mode := perm & 0o7777
	typ := match perm & 0o170000 {
		0o040000 { 'd' }
		0o120000 { 'l' }
		0o020000 { 'c' }
		0o060000 { 'b' }
		0o010000 { 'p' }
		0o140000 { 's' }
		else { '-' }
	}

	mut out := typ
	out += flag(mode & 0o400 != 0, 'r') + flag(mode & 0o200 != 0, 'w')
	out += exec_bit(mode & 0o100 != 0, mode & 0o4000 != 0, 's', 'S')
	out += flag(mode & 0o040 != 0, 'r') + flag(mode & 0o020 != 0, 'w')
	out += exec_bit(mode & 0o010 != 0, mode & 0o2000 != 0, 's', 'S')
	out += flag(mode & 0o004 != 0, 'r') + flag(mode & 0o002 != 0, 'w')
	out += exec_bit(mode & 0o001 != 0, mode & 0o1000 != 0, 't', 'T')
	return '${out} (${mode:04o})'
}

// format_owner gives `name (id)`, the id alone without a name, `-` without either.
pub fn format_owner(name ?string, id ?u32) string {
	if n := name {
		if i := id {
			return '${n} (${i})'
		}
		return n
	}
	if i := id {
		return '${i}'
	}
	return '-'
}

// Row is one line of a file table: its cells in remote_columns order.
pub struct Row {
pub:
	name   string
	is_dir bool
	cells  []string
}

// row_of formats an entry once per listing, not per frame.
pub fn row_of(e ssh.FileEntry) Row {
	return Row{
		name:   e.name
		is_dir: e.is_dir
		cells:  [e.name, if e.is_dir { '' } else { format_size(e.size) }, format_date(e.modified),
			format_permissions(e.permissions), format_owner(e.owner, e.uid),
			format_owner(e.group, e.gid)]
	}
}

struct Keyed {
	e     ssh.FileEntry
	lower string
}

fn opt_i64(v ?i64) i64 {
	return v or { i64(-1) }
}

fn opt_u32(v ?u32) i64 {
	x := v or { return i64(-1) }
	return i64(x)
}

fn cmp_i64(a i64, b i64) int {
	return if a < b {
		-1
	} else if a > b {
		1
	} else {
		0
	}
}

// compare_col orders two entries on the sort column, before the direction is applied.
fn compare_col(a &Keyed, b &Keyed, col SortCol) int {
	return match col {
		.name { a.lower.compare(b.lower) }
		.modified { cmp_i64(opt_i64(a.e.modified), opt_i64(b.e.modified)) }
		.size { cmp_i64(i64(a.e.size), i64(b.e.size)) }
		.permissions { cmp_i64(a.e.permissions, b.e.permissions) }
		.owner { cmp_i64(opt_u32(a.e.uid), opt_u32(b.e.uid)) }
		.group { cmp_i64(opt_u32(a.e.gid), opt_u32(b.e.gid)) }
	}
}

// sort_entries puts folders first; the column orders each group, the name breaks ties.
pub fn sort_entries(mut es []ssh.FileEntry, s SortState) {
	col, desc := if s.on { s.col, s.desc } else { SortCol.name, false }
	mut keyed := es.map(Keyed{it, it.name.to_lower()})
	keyed.sort_with_compare(fn [col, desc] (a &Keyed, b &Keyed) int {
		if a.e.is_dir != b.e.is_dir {
			return if a.e.is_dir { -1 } else { 1 }
		}
		c := compare_col(a, b, col)
		if c == 0 {
			return a.lower.compare(b.lower)
		}
		return if desc { -c } else { c }
	})
	for i, k in keyed {
		es[i] = k.e
	}
}

// validate_entry_name returns the trimmed name typed in a rename or new-folder dialog, or the
// corrective message.
pub fn validate_entry_name(raw string) !string {
	name := raw.trim_space()
	if name == '' {
		return error('Name cannot be empty.')
	}
	if name.contains_any('/\\') {
		return error("Name cannot contain '/' or '\\'.")
	}
	if name == '.' || name == '..' {
		return error("Name cannot be '.' or '..'.")
	}
	return name
}

// sanitize_file_name makes a working-copy file name out of an untrusted remote name.
pub fn sanitize_file_name(name string) string {
	mut out := []u8{cap: name.len}
	for c in name {
		out << if c in '/\\:*?"<>|\0'.bytes() { `_` } else { c }
	}
	cleaned := out.bytestr()
	ssh.validate_remote_entry_name(cleaned) or { return 'file' }
	return cleaned
}

pub fn delete_confirmation(name string, is_dir bool) string {
	if is_dir {
		return 'Are you sure you want to delete folder "${name}" and all of its contents? This cannot be undone.'
	}
	return 'Are you sure you want to delete file "${name}"?'
}
