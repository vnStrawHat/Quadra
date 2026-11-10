// The ConPTY host choice both ways (specs/17). The bundled case needs the pair staged by
// `tools/conpty.sh .tmp`; it is skipped otherwise.
module win

import os

fn scratch(name string) string {
	dir := os.join_path(@VMODROOT, '.tmp', 'conpty-host-${name}-${os.getpid()}')
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir) or { panic(err) }
	return dir
}

fn test_bundled_host_is_preferred() {
	dll := os.join_path(@VMODROOT, '.tmp', 'conpty.dll')
	if !os.is_file(dll) {
		eprintln('skipped: run tools/conpty.sh .tmp to stage the bundled host')
		return
	}
	assert resolve_conpty(os.join_path(@VMODROOT, '.tmp')) == .bundled
}

fn test_system_host_without_the_dll() {
	dir := scratch('none')
	defer {
		os.rmdir_all(dir) or {}
	}
	assert resolve_conpty(dir) == .system
}

fn test_a_file_that_is_no_dll_falls_back() {
	dir := scratch('bad')
	defer {
		os.rmdir_all(dir) or {}
	}
	os.write_file(os.join_path(dir, 'conpty.dll'), 'not a module') or { panic(err) }
	assert resolve_conpty(dir) == .system
}

fn test_host_names_for_the_log() {
	assert ConptyHost.bundled.name() == 'conpty: bundled'
	assert ConptyHost.system.name() == 'conpty: system'
}
