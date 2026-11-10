// The update UI state (specs/22, Services): what the Settings and About pages show.
module main

import update

enum UpdateStatus {
	idle
	checking
	up_to_date
	available
	downloading
	installing
	disabled
	manual_install
	skipped
	failed
	restart_scheduled
	restarted
}

// UpdateInstaller is the seam tests use: nothing else runs a real install.
pub type UpdateInstaller = fn (staged update.StagedUpdate) !update.InstallOutcome

pub struct UpdateUi {
pub mut:
	status    UpdateStatus
	detail    string // the version, the package folder, or the error
	candidate ?update.UpdateCandidate
	staged    ?update.StagedUpdate
}

pub fn (u UpdateUi) is_busy() bool {
	return u.status in [.checking, .downloading, .installing]
}

pub fn (u UpdateUi) has_offer() bool {
	return u.candidate != none || u.staged != none
}

pub fn (u UpdateUi) shows_install_button() bool {
	return u.has_offer() || u.status in [.available, .downloading, .installing]
}

pub fn (u UpdateUi) can_install() bool {
	return u.has_offer() && !u.is_busy()
}

pub fn (u UpdateUi) can_skip() bool {
	return u.candidate != none && u.staged == none && !u.is_busy()
}

pub fn (u UpdateUi) install_label() string {
	return match u.status {
		.downloading { 'Downloading...' }
		.installing { 'Installing...' }
		else { 'Install Update' }
	}
}

pub fn (u UpdateUi) status_text(current string) string {
	return match u.status {
		.idle { 'Current version: ${current}. Repository: ${update.repository_default}.' }
		.checking { 'Checking GitHub Releases...' }
		.up_to_date { 'OneTerm ${u.detail} is up to date.' }
		.available { 'OneTerm ${u.detail} is available.' }
		.downloading { 'Downloading OneTerm ${u.detail}...' }
		.installing { 'Installing update...' }
		.disabled { u.detail }
		.manual_install { 'Install location is not writable. Install manually from ${u.detail}.' }
		.skipped { 'OneTerm ${u.detail} was skipped. Clear it in Settings to be offered it again.' }
		.failed { 'Update failed: ${u.detail}' }
		.restart_scheduled { 'Update helper started. Quit OneTerm to complete installation.' }
		.restarted { 'Update installed. A new OneTerm process was started.' }
	}
}
