// Finding and validating the extracted release package (specs/25).
module update

import os

struct RequiredEntry {
	relative string
	is_dir   bool
}

fn required_entry(target string) RequiredEntry {
	if target.contains('windows') {
		return RequiredEntry{'oneterm.exe', false}
	}
	if target.contains('apple-darwin') {
		return RequiredEntry{'OneTerm.app', true}
	}
	return RequiredEntry{'oneterm', false}
}

fn (r RequiredEntry) exists(dir string) bool {
	path := os.join_path(dir, r.relative)
	return if r.is_dir { os.is_dir(path) } else { os.is_file(path) }
}

fn is_release_package_name(name string, target string) bool {
	return name == 'oneterm-${target}'
		|| (name.starts_with('oneterm-') && name.ends_with('-${target}'))
}

fn collect_package_dirs(dir string, target string, depth int, mut out []string) ! {
	if depth > 2 {
		return
	}
	mut children := os.ls(dir)!
	children.sort()
	for child in children {
		path := os.join_path(dir, child)
		if !os.is_dir(path) {
			continue
		}
		if is_release_package_name(child, target) && path !in out {
			out << path
		}
		if depth < 2 {
			collect_package_dirs(path, target, depth + 1, mut out)!
		}
	}
}

// validate_staged_package finds the package directory in an extraction.
pub fn validate_staged_package(extract_dir string, target string) !string {
	required := required_entry(target)
	mut candidates := []string{}
	collect_package_dirs(extract_dir, target, 0, mut candidates)!
	if extract_dir !in candidates {
		candidates << extract_dir
	}
	for c in candidates {
		if required.exists(c) {
			return c
		}
	}
	kind := if required.is_dir { 'directory' } else { 'file' }
	return error('staged update is missing required ${kind}: ${os.join_path(candidates[0],
		required.relative)}')
}
