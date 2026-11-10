module update

import os
import quadra.native.sys

// Everything here runs in .tmp/: no real install and no real executable is touched.
fn fixture(name string) (string, string) {
	dir := test_dir(name)
	source := os.join_path(dir, 'pkg')
	install := os.join_path(dir, 'install')
	os.mkdir_all(os.join_path(source, 'x64')) or { panic(err) }
	os.mkdir_all(install) or { panic(err) }
	os.write_file(os.join_path(source, 'oneterm.exe'), 'new') or { panic(err) }
	os.write_file(os.join_path(source, 'x64', 'OpenConsole.exe'), 'newcon') or { panic(err) }
	os.write_file(os.join_path(install, 'oneterm.exe'), 'old') or { panic(err) }
	os.write_file(os.join_path(install, 'sibling.txt'), 'keep') or { panic(err) }
	return source, install
}

fn backups_of(install string) []string {
	return os.ls(os.dir(install)) or { []string{} }.filter(it.contains('.backup-'))
}

fn test_replace_replaces_package_files_and_backs_up_old_ones_leaving_siblings() {
	source, install := fixture('replace')
	backup := replace_directory_contents(source, install)!
	assert os.read_file(os.join_path(install, 'oneterm.exe'))! == 'new'
	assert os.read_file(os.join_path(install, 'x64', 'OpenConsole.exe'))! == 'newcon'
	assert os.read_file(os.join_path(install, 'sibling.txt'))! == 'keep'
	assert os.read_file(os.join_path(backup, 'oneterm.exe'))! == 'old'
}

fn test_replace_restores_old_install_when_package_is_missing() {
	source, install := fixture('missing')
	os.rmdir_all(source)!
	if _ := replace_directory_contents(source, install) {
		assert false
	}
	assert os.read_file(os.join_path(install, 'oneterm.exe'))! == 'old'
	assert backups_of(install).len == 0
}

fn test_replace_restores_old_install_when_copy_fails() {
	source, install := fixture('copy-fails')
	// An unreadable package file makes the copy fail after the old entries were moved away.
	locked := os.join_path(source, 'zz')
	os.write_file(locked, 'new-zz')!
	os.write_file(os.join_path(install, 'zz'), 'old-zz')!
	handle := sys.open_exclusive(locked)!
	mut failed := false
	replace_directory_contents(source, install) or { failed = true }
	sys.close_handle(handle)
	assert failed
	assert os.read_file(os.join_path(install, 'oneterm.exe'))! == 'old'
	assert os.read_file(os.join_path(install, 'zz'))! == 'old-zz'
	assert os.read_file(os.join_path(install, 'sibling.txt'))! == 'keep'
	assert !os.exists(os.join_path(install, 'x64'))
	assert backups_of(install).len == 0
}

fn test_launch_success_removes_backup_directory() {
	source, install := fixture('launch-ok')
	replace_directory_contents_and_launch(source, install, 0, fn (_ int) ! {})!
	assert os.read_file(os.join_path(install, 'oneterm.exe'))! == 'new'
	assert backups_of(install).len == 0
}

fn test_launch_failure_restores_old_install_and_removes_backup_directory() {
	source, install := fixture('launch-fail')
	if _ := replace_directory_contents_and_launch(source, install, 0, fn (_ int) ! {
		return error('cannot start')
	})
	{
		assert false
	}
	assert os.read_file(os.join_path(install, 'oneterm.exe'))! == 'old'
	assert !os.exists(os.join_path(install, 'x64'))
	assert backups_of(install).len == 0
}

fn test_windows_update_script_reads_paths_from_environment() {
	for name in ['ONETERM_INSTALL_DIR', 'ONETERM_PACKAGE_DIR', 'ONETERM_BACKUP_DIR',
		'ONETERM_STAGING_DIR', 'ONETERM_SCRIPT_PATH', 'ONETERM_EXE_NAME', 'ONETERM_UPDATE_LOG'] {
		assert windows_update_script.contains('%${name}%'), name
	}
	assert !windows_update_script.contains(r'C:\')
	assert !windows_update_script.to_lower().contains('powershell')
}

fn test_windows_update_script_sleeps_without_a_console_and_removes_backup_after_launch() {
	assert windows_update_script.contains('ping -n 2 127.0.0.1')
	assert !windows_update_script.contains('timeout')
	launch := windows_update_script.index('start ""') or { panic('start') }
	cleanup := windows_update_script.index('rmdir /S /Q "%ONETERM_BACKUP_DIR%"') or {
		panic('rmdir')
	}
	assert launch < cleanup
}

fn test_windows_update_script_overwrites_read_only_files_and_resets_errorlevel_before_launch() {
	assert windows_update_script.count('/R /Y') >= 3
	reset := windows_update_script.index('(call )') or { panic('call') }
	launch := windows_update_script.index('start ""') or { panic('start') }
	assert reset < launch
}

fn test_windows_update_script_logs_every_step_to_the_update_log() {
	assert windows_update_script.count('call :log') >= 8
	assert windows_update_script.contains('>>"%log%" echo')
}

fn test_scheduling_writes_the_helper_and_passes_paths_through_the_environment() {
	dir := test_dir('schedule')
	cache := os.join_path(dir, 'cache')
	os.mkdir_all(cache)!
	install := os.join_path(dir, 'install')
	os.mkdir_all(install)!
	exe := os.join_path(install, 'oneterm.exe')
	staged := StagedUpdate{
		version:       '0.2.0'
		staging_dir:   os.join_path(dir, 'staging')
		package_dir:   os.join_path(dir, 'pkg')
		target_triple: test_target
	}
	mut seen := &map[string]string{}
	spawner := fn [mut seen] (script string, env map[string]string) ! {
		assert os.read_file(script)! == windows_update_script
		for key, value in env {
			unsafe {
				seen[key] = value
			}
		}
	}
	outcome := schedule_windows_update(staged, exe, cache, spawner)!
	assert outcome.kind == .restart_scheduled
	assert unsafe { seen['ONETERM_INSTALL_DIR'] } == install
	assert unsafe { seen['ONETERM_PACKAGE_DIR'] } == staged.package_dir
	assert unsafe { seen['ONETERM_EXE_NAME'] } == 'oneterm.exe'
	assert os.is_dir(unsafe { seen['ONETERM_BACKUP_DIR'] })
	assert os.read_file(os.join_path(cache, 'update.log'))!.contains('scheduled')
}

fn test_a_helper_that_fails_to_launch_cleans_up() {
	dir := test_dir('schedule-fail')
	cache := os.join_path(dir, 'cache')
	os.mkdir_all(cache)!
	install := os.join_path(dir, 'install')
	os.mkdir_all(install)!
	staged := StagedUpdate{
		package_dir: os.join_path(dir, 'pkg')
	}
	spawner := fn (_ string, _ map[string]string) ! {
		return error('spawn failed')
	}
	if _ := schedule_windows_update(staged, os.join_path(install, 'oneterm.exe'), cache, spawner) {
		assert false
	}
	assert os.ls(cache)!.filter(it.ends_with('.cmd') || it.starts_with('backup-')).len == 0
}

fn test_is_writable_dir_probes_and_cleans_up() {
	dir := test_dir('writable')
	assert is_writable_dir(dir)
	assert os.ls(dir)!.len == 0
	assert !is_writable_dir(os.join_path(dir, 'missing'))
}

fn test_read_only_windows_install_nothing() {
	if _ := install_staged_update(StagedUpdate{}, 'x', 'y', true) {
		assert false
	}
}
