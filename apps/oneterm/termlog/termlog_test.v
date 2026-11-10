module termlog

import os
import rand

fn temp_dir(name string) string {
	dir := os.join_path(@VMODROOT, '.tmp', 'oneterm-tests', 'termlog', name,
		'${os.getpid()}-${rand.hex(8)}')
	os.rmdir_all(dir) or {} // rand repeats between runs: never inherit a leftover
	os.mkdir_all(dir) or { panic(err) }
	return dir
}

fn test_parser_keeps_printable_lines_and_strips_terminal_sequences() {
	mut c := LineCollector{}
	c.feed('hello \x1b[31mred\x1b[0m\nprogress\r'.bytes())
	c.feed('héllo\n'.bytes())
	assert c.take_completed() == ['hello red', 'progress', 'héllo']
}

fn test_parser_excludes_tabs_and_tracks_backspace_and_partial_chunks() {
	mut c := LineCollector{}
	c.feed('ab'.bytes())
	c.feed('c\x08\td\n'.bytes())
	assert c.take_completed() == ['abd']
}

fn test_osc_and_dcs_payloads_are_never_logged() {
	mut c := LineCollector{}
	c.feed('\x1b]0;window title\x07visible\x1bPdcs payload\x1b\\ text\x1b_apc\x1b\\!\n'.bytes())
	assert c.take_completed() == ['visible text!']
}

fn test_sequences_split_across_reads() {
	mut c := LineCollector{}
	c.feed('\x1b'.bytes())
	c.feed('['.bytes())
	c.feed('38;5;'.bytes())
	c.feed('1mcolored'.bytes())
	c.feed('\x1b]0;tit'.bytes())
	c.feed('le\x07 end\n'.bytes())
	assert c.take_completed() == ['colored end']
}

fn test_utf8_split_across_reads() {
	mut c := LineCollector{}
	bytes := 'chữ Việt 😀\n'.bytes()
	for b in bytes {
		c.feed([b])
	}
	assert c.take_completed() == ['chữ Việt 😀']
}

fn test_can_and_sub_abort_a_sequence() {
	mut c := LineCollector{}
	c.feed('a\x1b[1\x18b\n'.bytes())
	assert c.take_completed() == ['ab']
}

fn test_control_inside_csi_still_executes() {
	mut c := LineCollector{}
	c.feed('line\x1b[1\n2mtail\n'.bytes())
	assert c.take_completed() == ['line', 'tail']
}

fn test_empty_lines_are_not_records() {
	mut c := LineCollector{}
	c.feed('\n\r\nx\n\n'.bytes())
	assert c.take_completed() == ['x']
}

fn test_sanitizes_identity_without_path_escape() {
	assert sanitize_identity('../a/b:c') == '_a_b_c'
	assert sanitize_identity('...') == 'terminal'
	assert sanitize_identity('user_host_22') == 'user_host_22'
}

fn test_identities() {
	assert local_log_identity('C:/Windows/System32/cmd.exe', 42) == 'cmd.exe_42'
	assert local_log_identity('', 7) == 'shell_7'
	assert ssh_log_identity('root', 'db', 22) == 'root_db_22'
}

fn test_write_modes_preserve_or_truncate_an_existing_file() {
	dir := temp_dir('modes')
	path := os.join_path(dir, 'existing.log')
	os.write_file(path, 'old\n')!
	write_through_mode(path, .append, 'new\n')!
	assert os.read_file(path)! == 'old\nnew\n'
	write_through_mode(path, .overwrite, 'replacement\n')!
	assert os.read_file(path)! == 'replacement\n'
}

fn write_through_mode(path string, mode LogWriteMode, text string) ! {
	fd := open_log_fd(path, mode)!
	assert C.write(fd, text.str, usize(text.len)) == text.len
	C.close(fd)
}

fn test_stop_flushes_unterminated_message() {
	dir := temp_dir('partial')
	mut c := new_controller('shell_42')
	path := c.start(TerminalLogConfig{ directory: dir, write_mode: .overwrite })!
	c.process('partial'.bytes())
	c.stop()!
	assert c.state().kind == .stopped
	content := os.read_file(path)!
	assert content.ends_with('] partial\n')
	assert content.starts_with('[')
}

fn test_records_have_the_fixed_format() {
	dir := temp_dir('format')
	mut c := new_controller('shell_1')
	path := c.start(TerminalLogConfig{ directory: dir })!
	c.process('one\n'.bytes())
	c.stop()!
	line := os.read_file(path)!.trim_space()
	assert line.len == '[2026-01-02 03:04:05] one'.len
	assert line[5] == `-` && line[11] == ` ` && line[14] == `:`
	assert line.ends_with('] one')
	assert os.file_name(path).starts_with('shell_1_')
	assert os.file_name(path).ends_with('.log')
}

fn test_start_is_idempotent_and_identity_is_kept_while_active() {
	dir := temp_dir('idempotent')
	mut c := new_controller('first')
	config := TerminalLogConfig{
		directory: dir
	}
	path := c.start(config)!
	c.set_identity('ignored')
	assert c.start(config)! == path
	c.stop()!
	assert c.state().kind == .stopped
}

fn test_a_write_failure_closes_the_writer_and_surfaces_once() {
	dir := temp_dir('write-fail')
	mut c := new_controller('x')
	path := c.start(TerminalLogConfig{ directory: dir })!
	// A read-only descriptor fails like a full disk; a closed one would abort the UCRT.
	C.close(c.active.fd)
	c.active.fd = C._wopen(path.to_wide(), os.o_rdonly, 0)
	c.process('one\n'.bytes())
	state := c.state()
	assert state.kind == .failed
	assert state.message.contains('stopped')
	assert c.take_error().contains('stopped')
	assert c.take_error() == ''
	c.process('more\n'.bytes()) // no crash, nothing more happens
}

fn test_a_setup_failure_reports_and_changes_state() {
	dir := temp_dir('setup-fail')
	file := os.join_path(dir, 'not-a-dir')
	os.write_file(file, 'x')!
	mut c := new_controller('x')
	if _ := c.start(TerminalLogConfig{ directory: os.join_path(file, 'sub') }) {
		assert false
	}
	assert c.state().kind == .failed
	assert c.take_error().contains('Failed to start')
}

fn test_default_log_folder() {
	assert default_terminal_log_dir('C:/u') == os.join_path('C:/u', '.OneTerm', 'logs')
}
