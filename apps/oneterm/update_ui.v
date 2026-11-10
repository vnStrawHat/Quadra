// The update UI state and actions (specs/22, Services): check, download, stage, install on
// restart. A check runs on a worker; the result applies on the OneTerm view.
module main

import os
import time
import update
import quadra.core
import quadra.kit

fn (o &OneTerm) update_path() string {
	return os.join_path(o.dir, update.document_name)
}

// update_engine_config is the file with the app's current preferences over it.
fn (o &OneTerm) update_engine_config() update.UpdateConfig {
	mut c := update.read_from(o.update_path()).config
	c.auto_check = o.update.auto_check
	c.channel = if o.update.channel == 'preview' {
		update.UpdateChannel.preview
	} else {
		update.UpdateChannel.stable
	}
	c.check_interval_hours = u64(if o.update.check_interval_hours > 0 {
		o.update.check_interval_hours
	} else {
		24
	})
	c.proxy_url = if o.update.proxy_url != '' { ?string(o.update.proxy_url) } else { ?string(none) }
	c.verify_certificates = o.update.verify_certificates
	c.skipped_version = if o.update.skipped_version != '' {
		?string(o.update.skipped_version)
	} else {
		?string(none)
	}
	return c
}

fn (o &OneTerm) update_storage() update.UpdateStorage {
	return update.UpdateStorage{
		cache_dir:   os.join_path(o.dir, 'update-cache')
		config_path: o.update_path()
	}
}

fn new_update_manager(config update.UpdateConfig, storage update.UpdateStorage, client ?update.ReleaseClient) update.UpdateManager {
	if c := client {
		return update.new_manager(update.repository_default, config, c, version,
			update.current_target_triple(), storage, is_restricted())
	}
	return update.new_github_manager(update.repository_default, config, version, storage,
		is_restricted())
}

struct CheckOutcome {
	result update.UpdateCheckResult
	error  string
	cache  update.UpdateCheckCache
}

// auto_check_due: release builds only unless a client is injected, as the original skips debug
// builds; an elevated window never checks.
fn (o &OneTerm) auto_check_allowed() bool {
	if is_restricted() {
		return false
	}
	if o.update_client != none {
		return true
	}
	$if test {
		return false
	}
	$if prod {
		return true
	}
	return os.getenv('ONETERM_UPDATE_AUTO_CHECK_DEBUG') != ''
}

// start_auto_check runs once at start, when the interval has elapsed.
fn (mut o OneTerm) start_auto_check(mut cx core.ViewCx[OneTerm]) {
	if !o.auto_check_allowed() || !o.update.auto_check {
		return
	}
	if !o.update_engine_config().should_auto_check_at(time.now()) {
		return
	}
	o.run_check(false, mut cx)
}

// check_for_updates is the Check Now of the settings page and the About dialog.
fn (mut o OneTerm) check_for_updates(mut cx core.ViewCx[OneTerm]) {
	if is_restricted() || o.upd.is_busy() {
		return
	}
	o.run_check(true, mut cx)
}

fn (mut o OneTerm) run_check(manual bool, mut cx core.ViewCx[OneTerm]) {
	o.upd.status = .checking
	config := o.update_engine_config()
	storage := o.update_storage()
	client := o.update_client
	core.spawn_task[OneTerm, CheckOutcome](mut cx, fn [manual, config, storage, client] (_ &core.TaskCx) !CheckOutcome {
		mut m := new_update_manager(config, storage, client)
		mut result := update.UpdateCheckResult{}
		mut message := ''
		if manual {
			result = m.refresh_now() or {
				message = err.msg()
				update.UpdateCheckResult{}
			}
		} else {
			result = m.check_now() or {
				message = err.msg()
				update.UpdateCheckResult{}
			}
		}
		return CheckOutcome{result, message, m.check_cache()}
	}, fn (mut o OneTerm, r &core.TaskResult[CheckOutcome], mut cx core.ViewCx[OneTerm]) {
		o.finish_check(r, mut cx)
	})
	cx.notify()
}

fn (mut o OneTerm) finish_check(r &core.TaskResult[CheckOutcome], mut cx core.ViewCx[OneTerm]) {
	if r.ok {
		// Only the checker-owned cache fields: preferences edited meanwhile survive.
		r.value.cache.save_to(o.update_path(), is_restricted()) or {
			eprintln('update cache not saved: ${err.msg()}')
		}
		if r.value.error != '' {
			o.upd.status, o.upd.detail = .failed, r.value.error
		} else {
			o.apply_check(r.value.result)
		}
	} else {
		o.upd.status, o.upd.detail = .failed, r.error
	}
	o.notify_update(mut cx)
	cx.notify()
}

fn (mut o OneTerm) apply_check(r update.UpdateCheckResult) {
	match r.kind {
		.available {
			c := r.candidate or { return }
			o.upd.status, o.upd.detail = .available, c.version
			o.upd.candidate = c
			o.upd.staged = none
		}
		.up_to_date {
			o.upd.status, o.upd.detail = .up_to_date, r.current_version
			o.upd.candidate, o.upd.staged = none, none
		}
		.disabled {
			o.upd.status, o.upd.detail = .disabled, r.reason
			o.upd.candidate, o.upd.staged = none, none
		}
	}
}

// skip_offered persists the skip and withdraws the offer.
fn (mut o OneTerm) skip_offered(mut cx core.ViewCx[OneTerm]) {
	if o.upd.is_busy() {
		return
	}
	c := o.upd.candidate or { return }
	o.update.skipped_version = c.version
	o.save_update()
	o.upd.candidate, o.upd.staged = none, none
	o.upd.status, o.upd.detail = .skipped, c.version
	cx.notify()
}

// ask_install is the confirmation in front of download-and-install.
fn (mut o OneTerm) ask_install(mut cx core.Cx, m core.Model[OneTerm]) {
	if is_restricted() || o.upd.is_busy() {
		return
	}
	kit.confirm(mut cx, 'Install Update', 'OneTerm restarts and closes active work.',
		'Install and Restart', core.listener_with[OneTerm, core.Model[OneTerm], bool](m.id, m, fn (mut o OneTerm, m core.Model[OneTerm], yes &bool, mut cx core.ViewCx[OneTerm]) {
		if *yes {
			o.download_and_install(mut cx)
		}
	}))
}

fn (mut o OneTerm) download_and_install(mut cx core.ViewCx[OneTerm]) {
	if staged := o.upd.staged {
		o.run_install(staged, mut cx)
		return
	}
	candidate := o.upd.candidate or { return }
	o.upd.status, o.upd.detail = .downloading, candidate.version
	config := o.update_engine_config()
	storage := o.update_storage()
	client := o.update_client
	core.spawn_task[OneTerm, update.StagedUpdate](mut cx, fn [config, storage, client, candidate] (_ &core.TaskCx) !update.StagedUpdate {
		mut m := new_update_manager(config, storage, client)
		return m.download_and_stage(candidate)!
	}, fn (mut o OneTerm, r &core.TaskResult[update.StagedUpdate], mut cx core.ViewCx[OneTerm]) {
		if r.ok {
			o.upd.staged = r.value
			o.run_install(r.value, mut cx)
		} else {
			o.upd.status, o.upd.detail = .failed, r.error
			o.notify_update(mut cx)
		}
		cx.notify()
	})
}

fn (mut o OneTerm) run_install(staged update.StagedUpdate, mut cx core.ViewCx[OneTerm]) {
	o.upd.status = .installing
	dir := os.join_path(o.dir, 'update-cache')
	installer := o.update_install
	core.spawn_task[OneTerm, update.InstallOutcome](mut cx, fn [staged, dir, installer] (_ &core.TaskCx) !update.InstallOutcome {
		if run := installer {
			return run(staged)!
		}
		return update.install_staged_update(staged, os.executable(), dir, is_restricted())!
	}, fn (mut o OneTerm, r &core.TaskResult[update.InstallOutcome], mut cx core.ViewCx[OneTerm]) {
		o.finish_install(r, mut cx)
	})
	cx.notify()
}

fn (mut o OneTerm) finish_install(r &core.TaskResult[update.InstallOutcome], mut cx core.ViewCx[OneTerm]) {
	if !r.ok {
		o.upd.status, o.upd.detail = .failed, r.error
	} else {
		match r.value.kind {
			.restart_scheduled {
				o.upd.status = .restart_scheduled
			}
			.restarted {
				o.upd.status = .restarted
			}
			.manual_install {
				o.upd.status, o.upd.detail = .manual_install, r.value.package_dir
			}
		}
	}
	o.notify_update(mut cx)
	cx.notify()
	if o.upd.status in [.restart_scheduled, .restarted] {
		cx.app.quit()
	}
}

// notify_update is the toast of a status, per the original.
fn (o &OneTerm) notify_update(mut cx core.ViewCx[OneTerm]) {
	match o.upd.status {
		.available {
			toast(mut cx, .info, 'Update available', 'OneTerm ${o.upd.detail} is available.')
		}
		.failed {
			toast(mut cx, .error, 'Update failed', o.upd.detail)
		}
		.restart_scheduled {
			toast(mut cx, .info, 'Update',
				'Quit OneTerm to let the update helper complete installation.')
		}
		.manual_install {
			toast(mut cx, .warning, 'Manual install required',
				'The install location is not writable. Install the update manually from ${o.upd.detail}.')
		}
		.restarted {
			toast(mut cx, .success, 'Update', 'Update installed. Restarting OneTerm.')
		}
		else {}
	}
}
