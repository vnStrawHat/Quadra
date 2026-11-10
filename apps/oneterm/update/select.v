// Choosing the newest viable release for this target (specs/25).
module update

import os
import time

struct Selection {
	candidate  ?UpdateCandidate
	no_package bool
}

struct Viable {
	version Version
	release GitHubRelease
	asset   GitHubAsset
	digest  string
}

fn (m &UpdateManager) select_candidate(releases []GitHubRelease, current Version) Selection {
	mut best := ?Viable(none)
	mut missing := false
	for release in releases {
		if release.draft || (release.prerelease && m.config.channel != .preview) {
			continue
		}
		version := parse_release_version(release.tag_name) or {
			eprintln('ignoring GitHub Release with non-SemVer tag: ${release.tag_name}')
			continue
		}
		if version.compare(current) <= 0 {
			continue
		}
		version_text := version.str()
		if (m.config.skipped_version or { '' }) == version_text {
			continue
		}
		expected := expected_asset_name(version_text, m.target)
		mut found := ?GitHubAsset(none)
		for a in release.assets {
			if a.name == expected {
				found = a
				break
			}
		}
		matched := found or {
			missing = true
			continue
		}

		digest := (matched.digest or { '' }).trim_space()
		if digest == '' || !is_https_url(matched.browser_download_url) {
			missing = true
			continue
		}
		if b := best {
			if version.compare(b.version) <= 0 {
				continue
			}
		}
		best = Viable{version, release, matched, digest}
	}
	viable := best or { return Selection{
		no_package: missing
	} }

	return Selection{
		candidate: UpdateCandidate{
			version:           viable.version.str()
			tag_name:          viable.release.tag_name
			release_name:      viable.release.name
			release_notes_url: viable.release.html_url
			body:              viable.release.body
			asset_name:        viable.asset.name
			asset_url:         viable.asset.browser_download_url
			asset_digest:      viable.digest
			asset_size:        viable.asset.size
			target_triple:     m.target
			prerelease:        viable.release.prerelease
		}
	}
}

// download_and_stage downloads, verifies, extracts and validates a candidate. On any failure
// the staging directory is removed and the current app is left as it was.
pub fn (mut m UpdateManager) download_and_stage(candidate UpdateCandidate) !StagedUpdate {
	require_https(candidate.asset_url)!
	if m.read_only {
		return error('a read-only window installs nothing')
	}
	stage_root := os.join_path(m.storage.cache_dir,
		'${candidate.version}-${candidate.target_triple}-${time.utc().unix_milli()}')
	os.mkdir_all(stage_root)!
	return m.stage_in(candidate, stage_root) or {
		os.rmdir_all(stage_root) or {
			eprintln('failed to remove update staging directory ${stage_root}: ${err.msg()}')
		}
		return err
	}
}

fn (mut m UpdateManager) stage_in(candidate UpdateCandidate, stage_root string) !StagedUpdate {
	artifact := os.join_path(stage_root, candidate.asset_name)
	mut max_bytes := max_download_bytes
	if size := candidate.asset_size {
		if size > 0 {
			max_bytes = size
		}
	}
	m.client.download_to_file(candidate.asset_url, artifact, max_bytes)!
	verify_asset_digest(candidate.asset_name, candidate.asset_digest, sha256_file(artifact)!)!
	extract_dir := os.join_path(stage_root, 'extracted')
	os.mkdir_all(extract_dir)!
	extract_archive(artifact, extract_dir)!
	package_dir := validate_staged_package(extract_dir, candidate.target_triple)!
	return StagedUpdate{
		version:       candidate.version
		staging_dir:   stage_root
		package_dir:   package_dir
		target_triple: candidate.target_triple
	}
}
