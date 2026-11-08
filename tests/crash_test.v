// A native crash leaves a minidump and the extra text in the crash directory.
module main

import os

fn test_crash_writes_minidump() {
	$if !windows {
		return
	}
	dir := os.join_path(os.temp_dir(), 'quadra-tests', 'crash')
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	exe := os.join_path(os.temp_dir(), 'quadra-tests', 'crash_child.exe')
	src := os.join_path(os.dir(@FILE), 'crash_child')
	b := os.execute('${os.quoted_path(@VEXE)} -o ${os.quoted_path(exe)} ${os.quoted_path(src)}')
	assert b.exit_code == 0, b.output
	r := os.execute('${os.quoted_path(exe)} ${os.quoted_path(dir)}')
	assert r.exit_code != 0
	files := os.ls(dir)!
	dmp := files.filter(it.ends_with('.dmp'))
	assert dmp.len == 1, 'files: ${files}'
	assert os.file_size(os.join_path(dir, dmp[0])) > 1000
	txt := files.filter(it.ends_with('.txt'))
	assert os.read_file(os.join_path(dir, txt[0]))! == 'last log lines'
}
