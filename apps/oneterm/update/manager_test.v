module update

import os

fn check_result(dir string, config UpdateConfig, client &FakeClient) UpdateCheckResult {
	mut m := new_test_manager(dir, config, client)
	return m.check_now() or { panic(err) }
}

fn test_disabled_when_repository_is_empty() {
	dir := test_dir('empty-repo')
	mut client := &FakeClient{}
	mut m := new_manager('  ', UpdateConfig{}, client, '0.1.0', test_target, storage_in(dir), false)
	assert m.check_now()!.kind == .disabled
	assert client.fetches == 0
}

fn test_invalid_build_version_disables_updates() {
	dir := test_dir('bad-version')
	mut client := &FakeClient{}
	mut m := new_manager('o/r', UpdateConfig{}, client, 'not-semver', test_target, storage_in(dir),
		false)
	assert m.check_now()!.kind == .disabled
	assert client.fetches == 0
}

fn test_a_read_only_window_neither_checks_nor_downloads() {
	dir := test_dir('read-only')
	mut client := &FakeClient{}
	mut m := new_manager('o/r', UpdateConfig{}, client, '0.1.0', test_target, storage_in(dir), true)
	assert m.check_now()!.kind == .disabled
	assert client.fetches == 0
	if _ := m.download_and_stage(cached('0.2.0', false)) {
		assert false
	}
	assert client.downloads == 0
}

fn test_newer_release_with_current_target_asset_is_offered_and_cached() {
	dir := test_dir('offered')
	mut client := &FakeClient{
		response: ReleaseResponse{
			etag:     'etag-1'
			releases: [release('v0.2.0', false, [asset('0.2.0')]),
				release('v0.3.0', false, [asset('0.3.0')])]
		}
	}
	result := check_result(dir, UpdateConfig{}, client)
	assert result.kind == .available
	candidate := result.candidate or { panic('candidate') }
	assert candidate.version == '0.3.0'
	persisted := read_from(storage_in(dir).config_path).config
	assert persisted.last_etag or { '' } == 'etag-1'
	assert persisted.last_checked_version or { '' } == '0.1.0'
	assert (persisted.cached_candidate or { panic('cache') }).version == '0.3.0'
}

fn test_drafts_and_older_releases_are_never_offered() {
	dir := test_dir('filtered')
	mut draft := release('v9.0.0', false, [asset('9.0.0')])
	draft = GitHubRelease{
		...draft
		draft: true
	}
	mut client := &FakeClient{
		response: ReleaseResponse{
			releases: [draft, release('v0.1.0', false, [asset('0.1.0')]),
				release('v0.0.9', false, [asset('0.0.9')]), release('not-a-version', false, [])]
		}
	}
	assert check_result(dir, UpdateConfig{}, client).kind == .up_to_date
}

fn test_prerelease_is_offered_only_on_preview_channel() {
	dir := test_dir('channel')
	mut client := &FakeClient{
		response: ReleaseResponse{
			releases: [release('v0.2.0-rc.1', true, [asset('0.2.0-rc.1')])]
		}
	}
	assert check_result(dir, UpdateConfig{}, client).kind == .up_to_date
	preview := UpdateConfig{
		channel: .preview
	}
	assert check_result(dir, preview, client).kind == .available
}

fn test_skipped_version_is_not_offered_but_a_newer_one_is() {
	dir := test_dir('skipped')
	mut client := &FakeClient{
		response: ReleaseResponse{
			releases: [release('v0.2.0', false, [asset('0.2.0')])]
		}
	}
	mut config := UpdateConfig{
		skipped_version: '0.2.0'
	}
	assert check_result(dir, config, client).kind == .up_to_date
	client.response = ReleaseResponse{
		releases: [release('v0.2.0', false, [asset('0.2.0')]),
			release('v0.2.1', false, [asset('0.2.1')])]
	}
	assert (check_result(dir, config, client).candidate or { panic('c') }).version == '0.2.1'
}

fn test_newer_release_without_current_target_is_not_up_to_date() {
	dir := test_dir('no-target')
	mut client := &FakeClient{
		response: ReleaseResponse{
			releases: [release('v0.2.0', false, [])]
		}
	}
	result := check_result(dir, UpdateConfig{}, client)
	assert result.kind == .disabled
	assert result.reason.contains('no compatible update package')
}

fn test_asset_without_https_url_or_digest_is_treated_as_missing_package() {
	dir := test_dir('no-https')
	mut plain := asset('0.2.0')
	plain = GitHubAsset{
		...plain
		browser_download_url: 'http://example.invalid/a.zip'
	}
	mut undigested := asset('0.2.0')
	undigested = GitHubAsset{
		...undigested
		digest: none
	}
	mut client := &FakeClient{
		response: ReleaseResponse{
			releases: [release('v0.2.0', false, [plain])]
		}
	}
	assert check_result(dir, UpdateConfig{}, client).kind == .disabled
	client.response = ReleaseResponse{
		releases: [release('v0.2.0', false, [undigested])]
	}
	assert check_result(dir, UpdateConfig{}, client).kind == .disabled
}

fn config_with_cache(channel UpdateChannel, candidate UpdateCandidate) UpdateConfig {
	return UpdateConfig{
		channel:              channel
		last_etag:            'etag-old'
		last_checked_version: '0.1.0'
		cached_candidate:     candidate
	}
}

fn test_not_modified_restores_cached_candidate() {
	dir := test_dir('304')
	mut client := &FakeClient{
		response: ReleaseResponse{
			not_modified: true
		}
	}
	result := check_result(dir, config_with_cache(.stable, cached('0.2.0', false)), client)
	assert client.last_etag or { '' } == 'etag-old'
	assert result.kind == .available
}

fn test_not_modified_drops_cached_prerelease_when_channel_is_stable() {
	dir := test_dir('304-stable')
	mut client := &FakeClient{
		response: ReleaseResponse{
			not_modified: true
		}
	}
	result := check_result(dir, config_with_cache(.stable, cached('0.2.0-rc.1', true)), client)
	assert result.kind == .up_to_date
	assert read_from(storage_in(dir).config_path).config.cached_candidate == none
}

fn test_not_modified_drops_cached_candidate_that_was_skipped_since() {
	dir := test_dir('304-skipped')
	mut config := config_with_cache(.stable, cached('0.2.0', false))
	config.skipped_version = '0.2.0'
	mut client := &FakeClient{
		response: ReleaseResponse{
			not_modified: true
		}
	}
	assert check_result(dir, config, client).kind == .up_to_date
}

fn test_cached_etag_is_ignored_after_an_upgrade() {
	dir := test_dir('upgrade')
	mut config := config_with_cache(.stable, cached('0.2.0', false))
	config.last_checked_version = '0.0.9'
	mut client := &FakeClient{}
	check_result(dir, config, client)
	assert client.last_etag == none
}

fn test_manual_refresh_bypasses_cached_etag() {
	dir := test_dir('refresh')
	mut client := &FakeClient{}
	mut m := new_test_manager(dir, config_with_cache(.stable, cached('0.2.0', false)), client)
	m.refresh_now()!
	assert client.last_etag == none
}

fn package_archive() []u8 {
	return build_zip([
		ZipEntry{
			name: 'dist/oneterm-0.2.0-${test_target}/oneterm.exe'
			data: 'exe'.bytes()
		},
	])
}

fn candidate_for(bytes []u8) UpdateCandidate {
	return UpdateCandidate{
		...cached('0.2.0', false)
		asset_digest: 'sha256:${sha256_of(bytes)}'
		asset_size:   u64(bytes.len)
	}
}

fn sha256_of(bytes []u8) string {
	path := os.join_path(test_dir('hash'), 'x')
	os.write_file_array(path, bytes) or { panic(err) }
	return sha256_file(path) or { panic(err) }
}

fn test_download_and_stage_verifies_digest_extracts_and_validates_package() {
	dir := test_dir('stage')
	bytes := package_archive()
	mut client := &FakeClient{
		bytes: bytes
	}
	mut m := new_test_manager(dir, UpdateConfig{}, client)
	staged := m.download_and_stage(candidate_for(bytes))!
	assert staged.version == '0.2.0'
	assert os.is_file(os.join_path(staged.package_dir, 'oneterm.exe'))
	assert staged.package_dir.ends_with('oneterm-0.2.0-${test_target}')
}

fn assert_stage_fails(mut m UpdateManager, candidate UpdateCandidate, needle string, dir string) {
	m.download_and_stage(candidate) or {
		assert err.msg().contains(needle), err.msg()
		updates := storage_in(dir).cache_dir
		entries := os.ls(updates) or { []string{} }
		assert entries.len == 0, 'staging left behind: ${entries}'
		return
	}
	assert false, 'expected a failure'
}

fn test_download_and_stage_rejects_checksum_mismatch_and_cleans_staging() {
	dir := test_dir('stage-sum')
	mut client := &FakeClient{
		bytes: package_archive()
	}
	mut m := new_test_manager(dir, UpdateConfig{}, client)
	assert_stage_fails(mut m, candidate_for('other'.bytes()), 'exceeds the expected size', dir)
	mut wrong := candidate_for(package_archive())
	wrong.asset_digest = valid_digest
	assert_stage_fails(mut m, wrong, 'checksum mismatch', dir)
}

fn test_download_and_stage_refuses_non_https_asset_before_touching_disk() {
	dir := test_dir('stage-http')
	mut client := &FakeClient{
		bytes: package_archive()
	}
	mut m := new_test_manager(dir, UpdateConfig{}, client)
	mut plain := candidate_for(package_archive())
	plain.asset_url = 'http://example.invalid/a.zip'
	assert_stage_fails(mut m, plain, 'non-HTTPS', dir)
	assert client.downloads == 0
	assert !os.exists(storage_in(dir).cache_dir)
}

fn test_a_download_failure_leaves_no_staging_directory() {
	dir := test_dir('stage-fail')
	mut client := &FakeClient{
		fail_download: true
	}
	mut m := new_test_manager(dir, UpdateConfig{}, client)
	assert_stage_fails(mut m, candidate_for('x'.bytes()), 'network down', dir)
}

fn test_github_client_builds_requests_without_network() {
	mut client := GitHubClient{
		user_agent: 'OneTerm/0.1.0'
		proxy_url:  'http://user:pw@proxy.example:3128'
	}
	config := client.base_fetch('https://example.invalid/x', 1 * time_second)!
	assert config.validate
	assert config.proxy != unsafe { nil }
	mut insecure := GitHubClient{
		user_agent:          'OneTerm/0.1.0'
		verify_certificates: false
	}
	assert !insecure.base_fetch('https://example.invalid/x', 1 * time_second)!.validate
}
