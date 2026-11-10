module update

import os
import store
import time

fn v(text string) Version {
	return parse_release_version(text) or { panic(err) }
}

fn test_parses_v_prefixed_semver() {
	assert v('v1.2.3').str() == '1.2.3'
	assert v(' 1.2.3 ').major == 1
	assert v('1.2.3-rc.1+build.5').str() == '1.2.3-rc.1+build.5'
}

fn test_rejects_invalid_semver() {
	for bad in ['release-1', '1.2', '1.2.3.4', '01.2.3', '1.2.3-', '1.2.3-01', '1.2.3+', 'v'] {
		if _ := parse_release_version(bad) {
			assert false, bad
		}
	}
}

fn test_comparison_follows_semver_precedence() {
	chain := ['1.0.0-alpha', '1.0.0-alpha.1', '1.0.0-alpha.beta', '1.0.0-beta', '1.0.0-beta.2',
		'1.0.0-beta.11', '1.0.0-rc.1', '1.0.0', '1.0.1', '1.1.0', '2.0.0']
	for i in 0 .. chain.len - 1 {
		assert v(chain[i]).compare(v(chain[i + 1])) < 0, chain[i]
		assert v(chain[i + 1]).compare(v(chain[i])) > 0, chain[i]
	}
	assert v('1.0.0+build').compare(v('1.0.0')) == 0
}

fn test_builds_versioned_asset_name() {
	assert expected_asset_name('0.3.0', 'x86_64-pc-windows-msvc') == 'oneterm-0.3.0-x86_64-pc-windows-msvc.zip'
	assert expected_asset_name('v0.3.0', 'x86_64-unknown-linux-gnu') == 'oneterm-0.3.0-x86_64-unknown-linux-gnu.tar.gz'
}

fn test_asset_digest_rules() {
	verify_asset_digest('a', 'sha256:${'a'.repeat(64)}', 'a'.repeat(64))!
	verify_asset_digest('a', 'A'.repeat(64), 'a'.repeat(64))!
	if _ := verify_asset_digest('a', 'sha256:${'a'.repeat(64)}', 'b'.repeat(64)) {
		assert false
	}
	if _ := verify_asset_digest('a', 'sha256:00', 'b'.repeat(64)) {
		assert false
	}
}

fn test_proxy_userinfo_is_redacted_but_host_and_path_survive() {
	assert redact_url_userinfo('http://alice:s3cret@proxy.example:8080/path?x=1') == 'http://***@proxy.example:8080/path?x=1'
	assert redact_url_userinfo('socks5://user@proxy.example') == 'socks5://***@proxy.example'
	assert redact_url_userinfo('http://proxy.example:8080') == 'http://proxy.example:8080'
	assert redact_url_userinfo('not a url') == 'not a url'
	assert redact_url_userinfo('http://proxy.example/a@b') == 'http://proxy.example/a@b'
}

fn test_only_https_asset_urls_are_accepted() {
	require_https('https://github.com/o/r/releases/download/v1/a.zip')!
	require_https('HTTPS://github.com/a.zip')!
	for bad in ['http://github.com/a.zip', 'file:///tmp/a.zip', ''] {
		if _ := require_https(bad) {
			assert false, bad
		}
	}
}

fn test_release_json_parsing_with_missing_optional_fields() {
	body := '[{"tag_name":"v1.0.0","draft":false,"prerelease":false,"html_url":"https://x","assets":[{"name":"a","browser_download_url":"https://x/a"}]}]'
	releases := parse_releases(body)!
	assert releases[0].tag_name == 'v1.0.0'
	assert releases[0].name == none && releases[0].body == none
	assert releases[0].assets[0].size == none && releases[0].assets[0].digest == none
	if _ := parse_releases('[{"tag_name":1}]') {
		assert false
	}
}

fn test_missing_file_uses_defaults_and_read_never_writes() {
	dir := test_dir('missing')
	path := os.join_path(dir, document_name)
	loaded := read_from(path)
	assert loaded.needs_document_repair
	assert !os.exists(path)
	config := loaded.config
	assert config.check_interval_hours == 24
	assert config.auto_check && config.verify_certificates
	assert config.proxy_url == none
	config.save_preferences_to(path, false)!
	assert os.exists(path)
}

fn test_invalid_document_is_repaired_and_quarantined_on_the_next_save() {
	dir := test_dir('invalid')
	path := os.join_path(dir, document_name)
	os.write_file(path, 'not-json')!
	loaded := read_from(path)
	assert loaded.needs_document_repair
	assert os.read_file(path)! == 'not-json'
	loaded.config.save_preferences_to(path, false)!
	assert !read_from(path).needs_document_repair
	assert os.ls(dir)!.any(it.contains('.invalid-'))
}

fn test_preference_edit_during_check_survives_check_completion() {
	dir := test_dir('merge')
	path := os.join_path(dir, document_name)
	mut entity := read_from(path).config
	mut checker_copy := entity
	entity.proxy_url = 'https://proxy.example'
	entity.auto_check = false
	entity.save_preferences_to(path, false)!
	checker_copy.record_success('etag-1', '0.3.0')
	checker_copy.check_cache().save_to(path, false)!
	entity.apply_check_cache(checker_copy.check_cache())
	assert entity.proxy_url or { '' } == 'https://proxy.example'
	assert entity.last_etag or { '' } == 'etag-1'
	on_disk := read_from(path).config
	assert on_disk.proxy_url or { '' } == 'https://proxy.example'
	assert !on_disk.auto_check
	assert on_disk.last_etag or { '' } == 'etag-1'
}

fn test_cleared_fields_are_removed_and_corrupt_documents_are_quarantined() {
	dir := test_dir('clear')
	path := os.join_path(dir, document_name)
	mut config := UpdateConfig{
		proxy_url: 'https://proxy.example'
	}
	config.save_preferences_to(path, false)!
	config.proxy_url = none
	config.save_preferences_to(path, false)!
	assert !os.read_file(path)!.contains('proxy_url')
	os.write_file(path, 'not-json')!

	UpdateCheckCache{
		last_etag: 'etag'
	}.save_to(path, false)!
	assert read_from(path).config.last_etag or { '' } == 'etag'
	assert os.ls(dir)!.any(it.contains('.invalid-'))
}

fn test_a_read_only_window_writes_no_update_config() {
	dir := test_dir('read-only')
	path := os.join_path(dir, document_name)
	if _ := UpdateConfig{}.save_preferences_to(path, true) {
		assert false
	}
	if _ := UpdateCheckCache{}.save_to(path, true) {
		assert false
	}
	assert !os.exists(path)
}

fn hours_ago(now time.Time, hours int) string {
	return now.add(-hours * time.hour).format_rfc3339()
}

fn test_auto_check_runs_when_never_checked_or_interval_elapsed() {
	now := time.utc()
	mut c := UpdateConfig{}
	assert c.should_auto_check_at(now)
	c.last_checked_at = hours_ago(now, 25)
	assert c.should_auto_check_at(now)
	c.last_checked_at = hours_ago(now, 23)
	assert !c.should_auto_check_at(now)
	c.check_interval_hours = 12
	assert c.should_auto_check_at(now)
	c.last_checked_at = 'garbage'
	assert c.should_auto_check_at(now)
	c.auto_check = false
	assert !c.should_auto_check_at(now)
}

fn test_check_interval_is_clamped_and_never_overflows() {
	now := time.utc()
	mut c := UpdateConfig{
		check_interval_hours: 0
		last_checked_at:      now.add(-59 * time.minute).format_rfc3339()
	}
	assert c.effective_check_interval_hours() == 1
	assert !c.should_auto_check_at(now)
	c.check_interval_hours = max_u64
	assert c.effective_check_interval_hours() == max_check_interval_hours
	assert !c.should_auto_check_at(now)
}

fn test_cached_candidate_without_prerelease_field_defaults_to_stable() {
	raw := '{"schema_version":1,"cached_candidate":{"version":"9.9.9","tag_name":"v9.9.9","release_name":null,"release_notes_url":"https://example.invalid","body":null,"asset_name":"a.zip","asset_url":"https://example.invalid/a.zip","asset_digest":"sha256:00","asset_size":null,"target_triple":"t"}}'
	path := os.join_path(test_dir('cached'), document_name)
	os.write_file(path, raw)!
	c := read_from(path).config.cached_candidate or { panic('candidate') }
	assert !c.prerelease
	_ = store.schema_field
}

fn test_cached_etag_is_only_reused_for_same_version() {
	mut c := UpdateConfig{}
	c.record_success('etag-123', '0.3.0')
	assert c.should_reuse_cached_etag('0.3.0')
	assert !c.should_reuse_cached_etag('0.2.1')
}
