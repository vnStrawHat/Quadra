module sys

import os

fn test_only_paths_inside_the_directory_match() {
	assert under('C:\\Apps\\OneTerm\\x64\\OpenConsole.exe', 'c:/apps/oneterm/')
	assert !under('C:\\Apps\\OneTerm2\\x64\\OpenConsole.exe', 'C:\\Apps\\OneTerm')
	assert !under('C:\\Apps\\OneTerm', 'C:\\Apps\\OneTerm')
}

fn test_a_directory_without_hosts_ends_nothing() {
	dir := os.join_path(@VMODROOT, '.tmp', 'no-hosts-${os.getpid()}')
	os.mkdir_all(dir) or { panic(err) }
	defer {
		os.rmdir_all(dir) or {}
	}
	assert terminate_console_hosts_in(dir) == []
}
