// GitHub Releases: the release model, the injectable transport, digest and URL checks (specs/25).
module update

import crypto.sha256
import net.http
import os
import store

// Hard cap for a release archive when GitHub reports no asset size.
pub const max_download_bytes = u64(512) * 1024 * 1024

pub struct GitHubAsset {
pub:
	name                 string
	browser_download_url string
	size                 ?u64
	digest               ?string
}

pub struct GitHubRelease {
pub:
	tag_name   string
	name       ?string
	draft      bool
	prerelease bool
	body       ?string
	html_url   string
	assets     []GitHubAsset
}

// ReleaseResponse: `not_modified` for a 304, then `releases` is empty.
pub struct ReleaseResponse {
pub:
	etag         ?string
	not_modified bool
	releases     []GitHubRelease
}

// ReleaseClient is the transport. GitHubClient is the production one; tests pass canned
// responses so nothing ever touches the network.
pub interface ReleaseClient {
mut:
	fetch_releases(repository string, etag ?string) !ReleaseResponse
	download_to_file(url string, path string, max_bytes u64) !
}

fn parse_releases(body string) ![]GitHubRelease {
	doc := store.parse_json(body)!
	mut out := []GitHubRelease{}
	for item in store.array_of(doc) or { return error('expected a release array') } {
		f := store.object_of(item) or { return error('expected a release object') }
		check_text_fields(f, ['name', 'body'])!
		mut assets := []GitHubAsset{}
		for a in store.array_of(f['assets'] or { return error('missing assets') }) or {
			return error('bad assets')
		} {
			af := store.object_of(a) or { return error('bad asset') }
			check_text_fields(af, ['digest'])!
			mut size := ?u64(none)
			if s := af['size'] {
				if !store.is_null(s) {
					size = store.u64_of(s) or { return error('bad size') }
				}
			}
			assets << GitHubAsset{
				name:                 req_text(af, 'name')!
				browser_download_url: req_text(af, 'browser_download_url')!
				size:                 size
				digest:               opt_text(af, 'digest')
			}
		}
		out << GitHubRelease{
			tag_name:   req_text(f, 'tag_name')!
			name:       opt_text(f, 'name')
			draft:      store.bool_of(f['draft'] or { return error('missing draft') }) or {
				return error('bad draft')
			}
			prerelease: store.bool_of(f['prerelease'] or { return error('missing prerelease') }) or {
				return error('bad prerelease')
			}
			body:       opt_text(f, 'body')
			html_url:   req_text(f, 'html_url')!
			assets:     assets
		}
	}
	return out
}

// GitHubClient is the blocking production transport. Run it off the UI thread.
pub struct GitHubClient {
pub:
	user_agent          string
	proxy_url           ?string
	verify_certificates bool = true
}

// base_fetch has what every request shares: user agent, proxy and certificate settings.
fn (c &GitHubClient) base_fetch(url string, read_timeout i64) !http.FetchConfig {
	mut config := http.FetchConfig{
		url:          url
		method:       .get
		user_agent:   c.user_agent
		read_timeout: read_timeout
		validate:     c.verify_certificates
		max_retries:  1
	}
	proxy_url := (c.proxy_url or { '' }).trim_space()
	if proxy_url != '' {
		config.proxy = http.new_http_proxy(proxy_url) or {
			return error("invalid update proxy URL '${redact_url_userinfo(proxy_url)}': ${err.msg()}")
		}
	}
	return config
}

fn (c &GitHubClient) warn_if_insecure() {
	if !c.verify_certificates {
		eprintln('TLS certificate verification for update requests is DISABLED in Settings; anyone on the network path can serve a forged release list, archive and matching SHA-256 digest.')
	}
}

// fetch_releases requests the release list, honouring If-None-Match when `etag` is given.
pub fn (mut c GitHubClient) fetch_releases(repository string, etag ?string) !ReleaseResponse {
	c.warn_if_insecure()
	mut header := http.new_header(key: .accept, value: 'application/vnd.github+json')
	if e := etag {
		header.add(.if_none_match, e)
	}
	mut config := c.base_fetch('https://api.github.com/repos/${repository}/releases',
		5 * time_second)!
	config.header = header
	resp := http.fetch(config) or { return error('HTTP error: ${err.msg()}') }
	if resp.status_code == 304 {
		return ReleaseResponse{
			not_modified: true
		}
	}
	if resp.status_code < 200 || resp.status_code >= 300 {
		return error('GitHub update check failed with status ${resp.status_code}')
	}
	new_etag := resp.header.get(.etag) or { '' }
	return ReleaseResponse{
		etag:     if new_etag == '' { none } else { new_etag }
		releases: parse_releases(resp.body)!
	}
}

// download_to_file writes `url` to `path`, failing once more than `max_bytes` arrive.
pub fn (mut c GitHubClient) download_to_file(url string, path string, max_bytes u64) ! {
	require_https(url)!
	c.warn_if_insecure()
	mut config := c.base_fetch(url, 30 * time_second)!
	config.stop_receiving_limit = i64(max_bytes) + 1
	resp := http.fetch(config) or { return error('HTTP error: ${err.msg()}') }
	if resp.status_code < 200 || resp.status_code >= 300 {
		return error('download failed with status ${resp.status_code}')
	}
	if u64(resp.body.len) > max_bytes {
		return download_too_large(u64(resp.body.len), max_bytes)
	}
	os.write_file(path, resp.body)!
}

// Only HTTPS release assets are ever fetched.
pub fn require_https(url string) ! {
	if !is_https_url(url) {
		return error('refusing to download update asset over a non-HTTPS URL: ${url}')
	}
}

pub fn is_https_url(url string) bool {
	return url.len >= 8 && url[..8].to_lower() == 'https://'
}

fn download_too_large(received u64, max_bytes u64) IError {
	return error('update download exceeds the expected size (${received} bytes received, limit ${max_bytes} bytes)')
}

// redact_url_userinfo replaces `user:password@` with `***@` for log and error text.
pub fn redact_url_userinfo(url string) string {
	scheme_end := url.index('://') or { return url }
	start := scheme_end + 3
	mut end := url.len
	for i in start .. url.len {
		if url[i] in [`/`, `?`, `#`] {
			end = i
			break
		}
	}
	at := url[start..end].last_index('@') or { return url }
	return '${url[..start]}***@${url[start + at + 1..]}'
}

pub fn sha256_file(path string) !string {
	return sha256.sum256(os.read_bytes(path)!).hex()
}

// verify_asset_digest compares `sha256:<hex>` or bare hex with the computed digest.
pub fn verify_asset_digest(asset_name string, expected_digest string, actual string) ! {
	trimmed := expected_digest.trim_space()
	expected := trimmed.trim_string_left('sha256:')
	if expected.len != 64 || !expected.contains_only('0123456789abcdefABCDEF') {
		return error('GitHub Release asset ${asset_name} has an invalid SHA-256 digest: ${expected_digest}')
	}
	if expected.to_lower() != actual.to_lower() {
		return error('checksum mismatch for ${asset_name}: expected ${expected}, got ${actual}')
	}
}
