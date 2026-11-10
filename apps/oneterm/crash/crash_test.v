module crash

import os
import quadra.native.sys
import rand

fn temporary_directory(name string) string {
	dir := os.join_path(@VMODROOT, '.tmp', 'oneterm-tests', 'crash', name,
		'${os.getpid()}-${rand.hex(8)}')
	os.rmdir_all(dir) or {} // rand repeats between runs: never inherit a leftover
	os.mkdir_all(dir) or { panic(err) }
	return dir
}

fn completed_path(dir string, identity string) string {
	return os.join_path(dir, identity + completed_suffix)
}

const fake_home = 'C:\\Users\\alice'

fn test_an_elevated_process_keeps_its_reports_in_its_own_store() {
	assert crashes_dir_in('config', false) == os.join_path('config', 'crashes')
	assert crashes_dir_in('config', true) == os.join_path('config', 'crashes', 'elevated')
}

fn test_deleting_a_report_outside_the_current_store_is_refused() {
	dir := temporary_directory('outside-store')
	outside := completed_path(dir, '20260921T000000000Z-p1-0000000a')
	os.write_file(outside, 'report')!
	if _ := delete_pending_report(dir, false, outside) {
		assert false
	}
	assert os.exists(outside)
	inside := completed_path(crashes_dir_in(dir, false), '20260921T000000000Z-p1-0000000a')
	os.mkdir_all(os.dir(inside))!
	os.write_file(inside, 'report')!
	delete_pending_report(dir, false, inside)!
	assert !os.exists(inside)
}

fn test_generated_identity_contains_time_pid_and_random_suffix() {
	dir := temporary_directory('identity')
	identity := unique_identity(dir)!
	assert pid_from_identity(identity) or { 0 } == u32(os.getpid())
	random := identity.all_after_last('-')
	assert random.len == 8
	assert random.contains_only('0123456789abcdef')
	assert identity.len == '20260811T023500000Z-p1-a7f3c912'.len + os.getpid().str().len - 1
	assert unique_identity(dir)! != identity
}

fn test_reports_load_newest_first_and_prune_after_twenty() {
	dir := temporary_directory('retention')
	for i in 0 .. 22 {
		name := '20260811T0235${i:02}000Z-p1-${i:08x}'
		os.write_file(completed_path(dir, name), 'report ${i}')!
	}
	reports := load_completed_reports(dir, none)!
	assert reports.len == max_reports
	assert reports[0].contents == 'report 21'
	assert reports[19].contents == 'report 2'
	assert report_names_with_suffix(dir, completed_suffix)!.len == 20
}

fn test_a_locked_report_does_not_hide_the_others() {
	dir := temporary_directory('unreadable')
	os.write_file(completed_path(dir, '20260811T023500000Z-p1-a7f3c912'), 'readable')!
	locked := completed_path(dir, '20260811T023501000Z-p1-a7f3c913')
	os.write_file(locked, 'locked')!
	handle := sys.open_exclusive(locked)!
	reports := load_completed_reports(dir, none)!
	sys.close_handle(handle)
	assert reports.len == 1
	assert reports[0].contents == 'readable'
}

fn test_inactive_native_staging_is_promoted_to_a_completed_report() {
	dir := temporary_directory('inactive-native')
	identity := '20260811T023500000Z-p${os.getpid()}-a7f3c912'
	native := os.join_path(dir, identity + native_suffix)
	os.write_file(native, 'native report')!
	promote_inactive_staging(dir, '')!
	assert !os.exists(native)
	assert os.read_file(completed_path(dir, identity))! == 'native report'
}

fn test_the_processs_own_staging_is_skipped() {
	dir := temporary_directory('own-staging')
	identity := '20260811T023500000Z-p${os.getpid()}-a7f3c912'
	native := os.join_path(dir, identity + native_suffix)
	os.write_file(native, 'native report')!
	promote_inactive_staging(dir, identity)!
	assert os.exists(native)
	assert !os.exists(completed_path(dir, identity))
}

fn test_native_staging_combines_with_matching_panic_report() {
	dir := temporary_directory('combine')
	identity := '20260811T023500000Z-p1-a7f3c912'
	completed := completed_path(dir, identity)
	claimed := os.join_path(dir, 'claimed.tmp')
	os.write_file(completed, 'panic report')!
	os.write_file(claimed, 'native report')!
	promote_claimed(identity, claimed, dir, false)!
	combined := os.read_file(completed)!
	assert combined.contains('panic report') && combined.contains('native report')
	assert combined.contains('Native crash context')
	assert !os.exists(claimed)
}

fn test_stderr_staging_is_a_report_only_when_a_panic_was_printed() {
	dir := temporary_directory('stderr')
	clean := '20260811T023500000Z-p${os.getpid()}-aaaaaaaa'
	os.write_file(os.join_path(dir, clean + stderr_suffix),
		'OneTerm Crash Report\nVersion: 1\n\nsome warning\n')!
	crashed := '20260811T023500000Z-p${os.getpid()}-bbbbbbbb'
	os.write_file(os.join_path(dir, crashed + stderr_suffix),
		'OneTerm Crash Report\nVersion: 1\n\nV panic: boom\nbacktrace\n')!
	promote_inactive_staging(dir, '')!
	assert !os.exists(completed_path(dir, clean))
	assert report_names_with_suffix(dir, stderr_suffix)!.len == 0
	report := os.read_file(completed_path(dir, crashed))!
	assert report.contains('V panic: boom')
	assert report.contains('Timestamp (Unix):')
}

fn test_empty_staging_and_empty_completed_reports_are_discarded() {
	dir := temporary_directory('empty')
	identity := '20260811T023500000Z-p${os.getpid()}-a7f3c912'
	os.write_file(os.join_path(dir, identity + native_suffix), '')!
	empty_completed := completed_path(dir, '20260811T023501000Z-p1-a7f3c913')
	os.write_file(empty_completed, '')!
	promote_inactive_staging(dir, '')!
	assert !os.exists(completed_path(dir, identity))
	assert load_and_sanitize_report(empty_completed, none)! == ''
	assert !os.exists(empty_completed)
}

fn test_redacts_native_and_alternate_home_separators() {
	report := r'C:\Users\alice\project\main.v' + '\n' + 'C:/Users/alice/project/main.v'
	assert redact_user_home(report, fake_home) == r'<USER_HOME>\project\main.v' + '\n' +
		'<USER_HOME>/project/main.v'
}

fn test_windows_home_redaction_is_ascii_case_insensitive() {
	$if windows {
		assert redact_user_home(r'c:\users\ALICE\project\main.v', fake_home) == r'<USER_HOME>\project\main.v'
	}
}

fn test_short_or_missing_homes_are_not_redacted() {
	assert redact_user_home('C:\\x', 'C:\\') == 'C:\\x'
	assert redact_user_home('abc', none) == 'abc'
}

fn test_loading_a_report_rewrites_redacted_content() {
	dir := temporary_directory('load-redaction')
	path := completed_path(dir, '20260811T023500000Z-p1-a7f3c912')
	os.write_file(path, r'panic at C:\Users\alice\project\main.v')!
	loaded := load_and_sanitize_report(path, fake_home)!
	assert loaded.contains(redacted_user_home)
	assert !loaded.contains('alice')
	assert os.read_file(path)! == loaded
}

fn test_prepare_capture_paths_creates_the_store() {
	dir := temporary_directory('prepare')
	paths := prepare_capture_paths(dir, true)!
	assert os.is_dir(os.join_path(dir, 'crashes', 'elevated'))
	assert paths.native.ends_with(native_suffix)
	assert paths.stderr.ends_with(stderr_suffix)
	assert paths.native.contains(paths.identity)
}

fn test_load_promotes_and_returns_reports() {
	dir := temporary_directory('load')
	store := crashes_dir_in(dir, false)
	os.mkdir_all(store)!
	identity := '20260811T023500000Z-p${os.getpid()}-a7f3c912'
	os.write_file(os.join_path(store, identity + native_suffix), r'crash in C:\Users\alice\x')!
	reports := load_pending_reports(dir, false, fake_home, 'not-me')!
	assert reports.len == 1
	assert reports[0].contents.contains('<USER_HOME>')
}

fn test_this_process_is_live_and_a_nonsense_pid_is_not() {
	assert sys.process_is_live(u32(os.getpid()))
	assert !sys.process_is_live(0xFFFFFFF0)
}

fn test_the_native_header_identifies_the_build() {
	header := native_header('1.2.3')
	assert header.starts_with('OneTerm Native Crash Report\n')
	assert header.contains('\nVersion: 1.2.3\nOS: windows\nArchitecture: ')
	assert header.ends_with('\nCapture: SetUnhandledExceptionFilter\n')
}

fn test_new_issue_url_never_carries_the_report() {
	url := new_issue_url('vnStrawHat/OneTerm')
	assert url == 'https://github.com/vnStrawHat/OneTerm/issues/new?title=Crash%20report'
}
