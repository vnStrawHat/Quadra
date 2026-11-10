// Remote path normalization, and the local destination policy for untrusted remote names.
module ssh

import os

// remote_path normalizes a POSIX path: every backslash becomes /, runs of / collapse and a
// trailing / is dropped except for the root.
pub fn remote_path(raw string) string {
	mut out := []u8{cap: raw.len}
	mut prev_sep := false
	for c in raw {
		ch := if c == `\\` { `/` } else { c }
		if ch == `/` {
			if prev_sep {
				continue
			}
			prev_sep = true
		} else {
			prev_sep = false
		}
		out << ch
	}
	if out.len > 1 && out.last() == `/` {
		out.pop()
	}
	return out.bytestr()
}

// remote_join appends a component; an absolute name replaces the base.
pub fn remote_join(base string, name string) string {
	n := name.replace('\\', '/')
	if n.starts_with('/') || base == '' {
		return remote_path(n)
	}
	return remote_path('${base}/${n}')
}

// remote_parent returns the containing folder, '' for the root, empty or a bare name.
pub fn remote_parent(path string) string {
	if path == '/' || path == '' {
		return ''
	}
	i := path.last_index_u8(`/`)
	if i < 0 {
		return ''
	}
	return if i == 0 { '/' } else { path[..i] }
}

// remote_file_name returns the last component, '' for the root, '.' and '..'.
pub fn remote_file_name(path string) string {
	name := path.all_after_last('/')
	if name == '' || name == '.' || name == '..' || path == '/' {
		return ''
	}
	return name
}

const reserved_names = ['CON', 'PRN', 'AUX', 'NUL', 'COM1', 'COM2', 'COM3', 'COM4', 'COM5', 'COM6',
	'COM7', 'COM8', 'COM9', 'LPT1', 'LPT2', 'LPT3', 'LPT4', 'LPT5', 'LPT6', 'LPT7', 'LPT8', 'LPT9']

// validate_remote_entry_name refuses a name that could not be one safe local path component.
pub fn validate_remote_entry_name(name string) ! {
	if name == '' || name == '.' || name == '..' {
		return error('unsafe remote filename')
	}
	if name.contains_any('/\\:') || name.contains('\0') || name.ends_with('.')
		|| name.ends_with(' ') {
		return error('unsafe remote filename: ${name}')
	}
	if name.all_before('.').to_upper() in reserved_names {
		return error('unsafe remote filename: ${name}')
	}
}

// safe_local_child joins a validated remote name to a local folder.
fn safe_local_child(root string, name string) !string {
	validate_remote_entry_name(name)!
	return os.join_path(root, name)
}

fn below(root string, path string) bool {
	r := root.trim_right('/\\').to_lower()
	p := path.to_lower()
	return p == r || p.starts_with(r + os.path_separator) || p.starts_with(r + '/')
}

// create_safe_parent_dirs creates the missing folders of a file below root, one at a time,
// refusing pre-existing symlinks and non-folders, re-checking that the path stays below root.
fn create_safe_parent_dirs(root string, candidate string) ! {
	if !below(root, candidate) {
		return error('local download path escaped destination')
	}
	rel := candidate[root.trim_right('/\\').len..].trim_left('/\\')
	parts := rel.split_any('/\\')
	mut current := root
	for i in 0 .. parts.len - 1 {
		current = os.join_path(current, parts[i])
		if os.is_link(current) {
			return error('refusing to traverse local symlink: ${current}')
		}
		if os.exists(current) {
			if !os.is_dir(current) {
				return error('local download parent is not a directory: ${current}')
			}
		} else {
			os.mkdir(current) or { return error('create local directory: ${err}') }
		}
		if !below(root, os.real_path(current)) {
			return error('local download path escaped destination')
		}
	}
	if os.is_link(candidate) {
		return error('refusing to traverse local symlink: ${candidate}')
	}
	if !below(root, os.real_path(os.dir(candidate))) {
		return error('local download path escaped destination')
	}
}
