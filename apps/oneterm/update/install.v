// Installing a verified staged update: a helper on Windows, in-place with rollback elsewhere.
module update

import os
import time
import quadra.native.sys

pub enum InstallKind {
	restart_scheduled // a helper was launched; the app should quit so replacement can run
	restarted         // files were replaced and a new process was started
	manual_install    // the install location is not writable: the user installs by hand
}

pub struct InstallOutcome {
pub:
	kind        InstallKind
	package_dir string
}

// HelperSpawner launches the Windows install helper. Tests inject a stand-in: no real process
// starts and no real executable is ever replaced.
pub type HelperSpawner = fn (script string, env map[string]string) !

// install_staged_update installs for this platform. `current_exe` and `cache_dir` are parameters.
pub fn install_staged_update(staged StagedUpdate, current_exe string, cache_dir string, read_only bool) !InstallOutcome {
	if read_only {
		return error('a read-only window installs nothing')
	}
	$if windows {
		return schedule_windows_update(staged, current_exe, cache_dir, spawn_cmd_helper)
	} $else $if macos {
		// The bundle replacement of the Rust app is not ported; the user installs by hand.
		return InstallOutcome{
			kind:        .manual_install
			package_dir: staged.package_dir
		}
	} $else {
		return install_unix_update(staged, current_exe)
	}
}

fn is_writable_dir(path string) bool {
	probe := os.join_path(path, '.oneterm-write-test-${os.getpid()}')
	os.write_file(probe, 'test') or { return false }
	os.rm(probe) or {}
	return true
}

fn append_update_log(path string, message string) {
	os.mkdir_all(os.dir(path)) or {}
	mut f := os.open_append(path) or { return }
	defer {
		f.close()
	}
	f.write_string('[${time.utc().format_ss()}Z] ${message}\n') or {}
}

fn schedule_windows_update(staged StagedUpdate, current_exe string, cache_dir string, spawner HelperSpawner) !InstallOutcome {
	install_dir := os.dir(current_exe)
	if !is_writable_dir(install_dir) {
		return InstallOutcome{
			kind:        .manual_install
			package_dir: staged.package_dir
		}
	}
	log_path := os.join_path(cache_dir, 'update.log')
	append_update_log(log_path,
		'=== update to ${staged.version} scheduled; install_dir=${install_dir}, package_dir=${staged.package_dir} ===')
	pid := os.getpid()
	stamp := time.utc().unix_milli()
	script := os.join_path(cache_dir, 'oneterm-install-update-${pid}-${stamp}.cmd')
	backup_dir := os.join_path(cache_dir, 'backup-${pid}-${stamp}')
	os.mkdir_all(backup_dir)!
	if !is_writable_dir(backup_dir) {
		os.rmdir(backup_dir) or {}
		return error('update backup directory is not writable: ${backup_dir}')
	}
	os.write_file(script, windows_update_script)!
	// A bundled OpenConsole.exe that outlived the app would hold the files the helper replaces.
	for host in sys.terminate_console_hosts_in(install_dir) {
		append_update_log(log_path, 'terminated stale OpenConsole.exe at ${host}')
	}
	mut env := map[string]string{}
	env['ONETERM_UPDATER_PID'] = pid.str()
	env['ONETERM_INSTALL_DIR'] = install_dir
	env['ONETERM_PACKAGE_DIR'] = staged.package_dir
	env['ONETERM_BACKUP_DIR'] = backup_dir
	env['ONETERM_EXE_NAME'] = os.file_name(current_exe)
	env['ONETERM_STAGING_DIR'] = staged.staging_dir
	env['ONETERM_SCRIPT_PATH'] = script
	env['ONETERM_UPDATE_LOG'] = log_path
	spawner(script, env) or {
		append_update_log(log_path, 'ERROR: failed to launch update helper: ${err.msg()}')
		os.rm(script) or {}
		os.rmdir(backup_dir) or {}
		return err
	}
	append_update_log(log_path,
		'update helper launched; OneTerm is quitting so the install can proceed')
	return InstallOutcome{
		kind: .restart_scheduled
	}
}

fn spawn_cmd_helper(script string, env map[string]string) ! {
	mut full := os.environ()
	for key, value in env {
		full[key] = value
	}
	mut p := os.new_process(os.join_path(os.getenv('SystemRoot'), 'System32', 'cmd.exe'))
	p.set_args(['/D', '/S', '/C', os.file_name(script)])
	p.set_work_folder(os.dir(script))
	p.set_environment(full)
	p.create_no_window = true
	p.run()
	if p.err != '' || p.pid == 0 {
		return error('failed to launch update helper: ${p.err}')
	}
}

fn install_unix_update(staged StagedUpdate, current_exe string) !InstallOutcome {
	install_dir := os.dir(current_exe)
	if !is_writable_dir(install_dir) {
		return InstallOutcome{
			kind:        .manual_install
			package_dir: staged.package_dir
		}
	}
	replace_directory_contents_and_launch(staged.package_dir, install_dir, fn [current_exe] () ! {
		mut p := os.new_process(current_exe)
		p.run()
		if p.err != '' {
			return error(p.err)
		}
	})!
	os.rmdir_all(staged.staging_dir) or {}
	return InstallOutcome{
		kind: .restarted
	}
}

// replace_directory_contents_and_launch replaces the package files in `install_dir`, then runs
// `launch`. The previous files stay in a backup until launch succeeds; a failure restores them.
fn replace_directory_contents_and_launch(source_dir string, install_dir string, launch fn () !) ! {
	backup := replace_directory_contents(source_dir, install_dir)!
	launch() or {
		restore_directory_contents(source_dir, install_dir, backup) or {
			eprintln('failed to restore previous install after launch error: ${err.msg()}')
		}
		return err
	}
	remove_backup_dir(backup)
}

fn replace_directory_contents(source_dir string, install_dir string) !string {
	backup := backup_path(install_dir)
	os.mkdir_all(backup)!
	// Nothing has been copied yet, so only the already-moved entries need to come back.
	move_package_entries_to_backup(source_dir, install_dir, backup) or {
		failure := err
		restore_backup_entries(install_dir, backup) or {
			return rollback_failed(install_dir, failure, err)
		}
		return failure
	}
	copy_dir_contents(source_dir, install_dir) or {
		failure := err
		restore_directory_contents(source_dir, install_dir, backup) or {
			return rollback_failed(install_dir, failure, err)
		}
		return failure
	}
	return backup
}

fn rollback_failed(install_dir string, failure IError, restore IError) IError {
	return error('failed to replace ${install_dir}: ${failure.msg()}; rollback failed: ${restore.msg()}')
}

// Only entries shipped in the package move: the directory may be shared.
fn move_package_entries_to_backup(source_dir string, install_dir string, backup string) ! {
	for name in os.ls(source_dir)! {
		existing := os.join_path(install_dir, name)
		if os.exists(existing) || os.is_link(existing) {
			os.rename(existing, os.join_path(backup, name))!
		}
	}
}

fn restore_directory_contents(source_dir string, install_dir string, backup string) ! {
	// Entries copied from the package exist only if the package was readable.
	if entries := os.ls(source_dir) {
		for name in entries {
			remove_path(os.join_path(install_dir, name))!
		}
	}
	restore_backup_entries(install_dir, backup)!
}

fn restore_backup_entries(install_dir string, backup string) ! {
	for name in os.ls(backup)! {
		os.rename(os.join_path(backup, name), os.join_path(install_dir, name))!
	}
	remove_backup_dir(backup)
}

fn remove_path(path string) ! {
	if os.is_dir(path) && !os.is_link(path) {
		os.rmdir_all(path)!
	} else if os.exists(path) || os.is_link(path) {
		os.rm(path)!
	}
}

fn remove_backup_dir(backup string) {
	os.rmdir_all(backup) or {
		eprintln('failed to remove update backup directory ${backup}: ${err.msg()}')
	}
}

fn backup_path(path string) string {
	name := os.file_name(path)
	return os.join_path(os.dir(path), '.${name}.backup-${os.getpid()}-${time.utc().unix_milli()}')
}

fn copy_dir_contents(source string, destination string) ! {
	os.mkdir_all(destination)!
	for name in os.ls(source)! {
		from := os.join_path(source, name)
		to := os.join_path(destination, name)
		if os.is_dir(from) {
			copy_dir_contents(from, to)!
		} else {
			os.cp(from, to)!
		}
	}
}
