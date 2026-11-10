// The services behind Settings and the + menu: vault dialogs, updates with a fake client, crash
// reports, saved sessions (specs/22, Services). Nothing touches the network or the home folder.
module main

import os
import store
import update
import quadra.core
import quadra.kit.term
import quadra.testing

struct FakeReleases {
mut:
	fetches int
}

fn (mut f FakeReleases) fetch_releases(_ string, _ ?string) !update.ReleaseResponse {
	f.fetches++
	return update.ReleaseResponse{
		releases: [
			update.GitHubRelease{
				tag_name: 'v9.9.9'
				html_url: 'https://github.test/release'
				assets:   [
					update.GitHubAsset{
						name:                 update.expected_asset_name('9.9.9',
							update.current_target_triple())
						browser_download_url: 'https://github.test/release.zip'
						size:                 u64(10)
						digest:               'sha256:' + 'a'.repeat(64)
					},
				]
			},
		]
	}
}

fn (mut f FakeReleases) download_to_file(_ string, _ string, _ u64) ! {
	return error('no download in this test')
}

fn svc_rig(name string, prep fn (dir string)) !(&testing.Harness, &OneTerm, core.View[OneTerm]) {
	dir := '.tmp/wire_svc_${name}'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	prep(dir)
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	if name in ['update', 'install'] {
		one.update_client = update.ReleaseClient(&FakeReleases{})
	}
	one.update_install = fn (s update.StagedUpdate) !update.InstallOutcome {
		return update.InstallOutcome{
			kind:        .manual_install
			package_dir: s.package_dir
		}
	}
	v := h.open(one, width: 1000, height: 700)!
	h.frame()
	cx := core.Cx{
		app: unsafe { h.app }
	}
	return h, cx.peek(v.model()), v
}

fn settings_of(mut h testing.Harness, v core.View[OneTerm]) ! {
	_ := h.open(SettingsWin{ one: v.model() },
		width:  920
		height: 900
	)!
	h.frame()
}

fn test_saved_sessions_reach_the_new_tab_menu() {
	mut h, o, _ := svc_rig('sessions', fn (dir string) {
		mut st := store.open(dir, false)
		st.add(store.Session{
			label: 'prod'
			host:  'example.test'
			group: 'servers'
		})
		st.flush()
	})!
	rows := o.saved_sessions()
	assert rows.len == 1 && rows[0].title == 'prod' && rows[0].group == 'servers'
	assert o.new_tab_items().any(it.label == 'prod')
	h.frame()
}

fn test_a_check_finds_the_release_and_skip_persists() {
	mut h, o, v := svc_rig('update', fn (_ string) {})!
	mut one := unsafe { o }
	cx := core.Cx{
		app: unsafe { h.app }
	}
	// Check Now, as the settings button does, on the OneTerm view.
	mut c := cx
	c.update_view(v, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.check_for_updates(mut cx)
	})
	h.wait_tasks()!
	h.frame()
	assert one.upd.status == .available && one.upd.detail == '9.9.9'
	assert one.upd.status_text(version).contains('9.9.9')
	assert one.upd.can_install() && one.upd.can_skip()
	c.update_view(v, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.skip_offered(mut cx)
	})
	assert one.upd.status == .skipped
	assert load_update_config(one.dir).skipped_version == '9.9.9'
}

fn test_the_install_seam_reports_manual_install() {
	mut h, o, v := svc_rig('install', fn (_ string) {})!
	h.wait_tasks()! // the auto-check at start must land before the state is staged
	mut one := unsafe { o }
	one.upd.staged = update.StagedUpdate{
		package_dir: '.tmp/wire_svc_install/pkg'
	}
	one.upd.status = .available
	cx := core.Cx{
		app: unsafe { h.app }
	}
	mut c := cx
	c.update_view(v, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.download_and_install(mut cx)
	})
	h.wait_tasks()!
	h.frame()
	assert one.upd.status == .manual_install
}

fn test_a_crash_report_shows_once_and_dismiss_deletes() {
	file := '20260101T000000000Z-p999999-deadbeef.crash.txt'
	name := os.join_path('.tmp/wire_svc_crash', 'crashes', file)
	mut h, _, _ := svc_rig('crash', fn [file] (dir string) {
		crashes := os.join_path(dir, 'crashes')
		os.mkdir_all(crashes) or {}
		os.write_file(os.join_path(crashes, file), 'V panic: boom') or {}
	})!
	h.frame()
	h.frame()
	assert h.has_text('OneTerm closed unexpectedly')
	h.click('crash-dismiss')!
	h.frame()
	assert !h.has_text('OneTerm closed unexpectedly')
	assert !os.exists(name)
}

fn test_set_up_change_and_reset_a_master_password() {
	mut h, o, v := svc_rig('vault', fn (_ string) {})!
	mut one := unsafe { o }
	assert !one.vault.can_lock_screen()
	settings_of(mut h, v)!
	h.click('settings-page-5')!
	h.frame()
	h.click('sec-setup')!
	h.frame()
	h.type_text('correct-horse')
	h.click('vault-field-1')!
	h.frame()
	h.type_text('correct-horse')
	h.click('vault-confirm')!
	h.wait_tasks()!
	h.frame()
	assert one.vault.phase() == .unlocked
	assert one.vault.can_lock_screen()
	h.click('sec-reset')!
	h.frame()
	h.type_text('RESET')
	h.click('vault-confirm')!
	h.wait_tasks()!
	h.frame()
	assert one.vault.phase() == .uninitialised
}
