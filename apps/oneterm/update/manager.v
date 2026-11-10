// UpdateManager: the check, candidate selection, download and staging flow (specs/25).
module update

const time_second = 1000 * 1000 * 1000 // time.Duration nanoseconds, as http expects

// CheckKind is the result shape of a release check.
pub enum CheckKind {
	disabled   // updating is disabled or impossible; `reason` explains why
	up_to_date // the current build is already the newest compatible release
	available  // a newer compatible release is available to download
}

pub struct UpdateCheckResult {
pub:
	kind            CheckKind
	reason          string
	current_version string
	candidate       ?UpdateCandidate
}

// StagedUpdate is a verified update on disk, ready for the platform installer.
pub struct StagedUpdate {
pub:
	version       string
	staging_dir   string
	package_dir   string
	target_triple string
}

// UpdateStorage says where the manager writes.
pub struct UpdateStorage {
pub:
	cache_dir   string // downloads, staged packages, the install backup
	config_path string // update_config.json, receives the check cache
}

// UpdateManager drives one check-or-download; run it off the UI thread.
pub struct UpdateManager {
mut:
	repository string
	client     ReleaseClient
	config     UpdateConfig
	storage    UpdateStorage
	read_only  bool
	target     string
	// none when the build's own version is not valid SemVer; every check then reports disabled.
	current_version ?Version
	current_text    string
}

// new_manager builds a manager over any transport.
pub fn new_manager(repository string, config UpdateConfig, client ReleaseClient, current_version string, target string, storage UpdateStorage, read_only bool) UpdateManager {
	mut version := ?Version(none)
	if parsed := parse_release_version(current_version) {
		version = parsed
	} else {
		eprintln('the running build version is not valid SemVer; automatic updates are disabled')
	}
	return UpdateManager{
		repository:      repository
		client:          client
		config:          config
		storage:         storage
		read_only:       read_only
		target:          target
		current_version: version
		current_text:    current_version
	}
}

// new_github_manager is the production manager: GitHub, proxy and certificate settings from `config`.
pub fn new_github_manager(repository string, config UpdateConfig, current_version string, storage UpdateStorage, read_only bool) UpdateManager {
	mut client := GitHubClient{
		user_agent:          'OneTerm/${current_version}'
		proxy_url:           config.proxy_url
		verify_certificates: config.verify_certificates
	}
	return new_manager(repository, config, client, current_version, current_target_triple(),
		storage, read_only)
}

// check_cache is what a check recorded; callers holding live preferences merge only this.
pub fn (m &UpdateManager) check_cache() UpdateCheckCache {
	return m.config.check_cache()
}

fn disabled(reason string) UpdateCheckResult {
	return UpdateCheckResult{
		kind:   .disabled
		reason: reason
	}
}

// check_now checks GitHub Releases, reusing the cached ETag when it matches this app version.
pub fn (mut m UpdateManager) check_now() !UpdateCheckResult {
	return m.check_with_cache(true)
}

// refresh_now forces a fresh request without the cached ETag.
pub fn (mut m UpdateManager) refresh_now() !UpdateCheckResult {
	return m.check_with_cache(false)
}

fn (mut m UpdateManager) check_with_cache(reuse bool) !UpdateCheckResult {
	if m.read_only {
		return disabled('Updates are checked and installed from the normal OneTerm window.')
	}
	if m.repository.trim_space() == '' {
		return disabled('No GitHub release repository is configured.')
	}
	current := m.current_version or {
		return disabled("This build reports version '${m.current_text}', which is not valid SemVer; updates are disabled.")
	}

	current_text := current.str()
	mut etag := ?string(none)
	if reuse && m.config.should_reuse_cached_etag(current_text) {
		etag = m.config.last_etag
	}
	response := m.client.fetch_releases(m.repository, etag)!
	m.config.record_success(response.etag, current_text)
	if response.not_modified {
		reused := if reuse { m.cached_candidate() } else { none }
		if reused == none {
			m.config.cached_candidate = none
		}
		m.persist_config()
		if c := reused {
			return UpdateCheckResult{
				kind:      .available
				candidate: c
			}
		}
		return UpdateCheckResult{
			kind:            .up_to_date
			current_version: current_text
		}
	}
	selection := m.select_candidate(response.releases, current)
	mut result := UpdateCheckResult{}
	if c := selection.candidate {
		m.config.cached_candidate = c
		result = UpdateCheckResult{
			kind:      .available
			candidate: c
		}
	} else if selection.no_package {
		m.config.cached_candidate = none
		result =
			disabled('GitHub has a newer release, but no compatible update package is available for this platform.')
	} else {
		m.config.cached_candidate = none
		result = UpdateCheckResult{
			kind:            .up_to_date
			current_version: current_text
		}
	}
	m.persist_config()
	return result
}

fn (m &UpdateManager) cached_candidate() ?UpdateCandidate {
	hit := m.config.cached_candidate or { return none }
	if hit.target_triple != m.target {
		return none
	}
	// The cache was filled under the channel active at that time.
	if hit.prerelease && m.config.channel != .preview {
		return none
	}
	version := parse_release_version(hit.version) or { return none }
	current := m.current_version or { return none }
	if version.compare(current) <= 0 {
		return none
	}
	if (m.config.skipped_version or { '' }) == hit.version {
		return none
	}
	return hit
}

// Only the checker-owned fields: the settings UI owns preferences and may have changed them.
fn (m &UpdateManager) persist_config() {
	m.config.check_cache().save_to(m.storage.config_path, m.read_only) or {
		eprintln('failed to persist update_config.json cache after check: ${err.msg()}')
	}
}
