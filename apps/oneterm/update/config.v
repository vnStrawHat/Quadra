// update_config.json: preferences and the check cache, each merged field by field so neither
// writer can clobber the other (specs/25).
module update

import os
import store
import x.json2

pub const document_name = 'update_config.json'
pub const repository_default = 'vnStrawHat/OneTerm'
pub const max_check_interval_hours = u64(24 * 365)
const current_schema = u32(1)
const preference_fields = ['auto_check', 'channel', 'check_interval_hours', 'proxy_url',
	'verify_certificates', 'skipped_version']
const cache_fields = ['last_checked_at', 'last_etag', 'last_checked_version', 'cached_candidate']

pub enum UpdateChannel {
	stable
	preview
}

// UpdateCandidate is a release artifact that can update this build; also the persisted cache.
pub struct UpdateCandidate {
pub mut:
	version           string
	tag_name          string
	release_name      ?string
	release_notes_url string
	body              ?string
	asset_name        string
	asset_url         string
	asset_digest      string
	asset_size        ?u64
	target_triple     string
	prerelease        bool
}

// UpdateCheckCache is what the release checker owns in the document.
pub struct UpdateCheckCache {
pub mut:
	last_checked_at      ?string
	last_etag            ?string
	last_checked_version ?string
	cached_candidate     ?UpdateCandidate
}

// save_to merges the cache fields into the document, leaving every preference untouched.
pub fn (c UpdateCheckCache) save_to(path string, read_only bool) ! {
	merge_owned_fields(path, cache_fields, cache_values(c), read_only)!
}

// UpdateConfig is the settings entity: preferences plus the cache fields.
pub struct UpdateConfig {
pub mut:
	auto_check           bool = true
	channel              UpdateChannel
	check_interval_hours u64 = 24
	proxy_url            ?string
	verify_certificates  bool = true
	skipped_version      ?string
	last_checked_at      ?string
	last_etag            ?string
	last_checked_version ?string
	cached_candidate     ?UpdateCandidate
}

// LoadedUpdateConfig: `needs_document_repair` means a background preference save must run.
pub struct LoadedUpdateConfig {
pub:
	config                UpdateConfig
	needs_document_repair bool
}

// read_from reads without writing, safe on the UI thread. A broken or missing document is
// repaired by the next save_preferences_to.
pub fn read_from(path string) LoadedUpdateConfig {
	if !os.exists(path) {
		return LoadedUpdateConfig{
			needs_document_repair: true
		}
	}
	raw := store.read_text(path) or {
		eprintln('${document_name}: ${err.msg()}; using defaults')
		return LoadedUpdateConfig{}
	}
	config := parse_config(raw) or {
		eprintln('${err.msg()}; using defaults and quarantining on the next save')
		return LoadedUpdateConfig{
			needs_document_repair: true
		}
	}
	return LoadedUpdateConfig{
		config: config
	}
}

fn parse_config(raw string) !UpdateConfig {
	doc := store.parse_versioned_object(raw, current_schema, document_name)!
	fields := store.object_of(doc) or { return error('${document_name} is not an object') }
	mut c := UpdateConfig{}
	c.auto_check = opt_bool(fields, 'auto_check', c.auto_check)!
	c.verify_certificates = opt_bool(fields, 'verify_certificates', c.verify_certificates)!
	if value := fields['check_interval_hours'] {
		c.check_interval_hours = store.u64_of(value) or { return error('bad check_interval_hours') }
	}
	if value := fields['channel'] {
		c.channel = match store.string_of(value) {
			'stable' { UpdateChannel.stable }
			'preview' { UpdateChannel.preview }
			else { return error('bad channel') }
		} or { return error('bad channel') }
	}
	check_text_fields(fields, ['proxy_url', 'skipped_version', 'last_checked_at', 'last_etag',
		'last_checked_version'])!
	c.proxy_url = opt_text(fields, 'proxy_url')
	c.skipped_version = opt_text(fields, 'skipped_version')
	c.last_checked_at = opt_text(fields, 'last_checked_at')
	c.last_etag = opt_text(fields, 'last_etag')
	c.last_checked_version = opt_text(fields, 'last_checked_version')
	if value := fields['cached_candidate'] {
		if !store.is_null(value) {
			c.cached_candidate = decode_candidate(value)!
		}
	}
	return c
}

fn opt_bool(fields map[string]json2.Any, key string, fallback bool) !bool {
	value := fields[key] or { return fallback }
	return store.bool_of(value) or { return error('bad ${key}') }
}

fn check_text_fields(fields map[string]json2.Any, keys []string) ! {
	for key in keys {
		value := fields[key] or { continue }
		if !store.is_null(value) && store.string_of(value) == none {
			return error('bad ${key}')
		}
	}
}

fn opt_text(fields map[string]json2.Any, key string) ?string {
	value := fields[key] or { return none }
	return store.string_of(value)
}

fn req_text(fields map[string]json2.Any, key string) !string {
	value := fields[key] or { return error('missing ${key}') }
	return store.string_of(value) or { return error('bad ${key}') }
}

fn decode_candidate(value json2.Any) !UpdateCandidate {
	f := store.object_of(value) or { return error('bad cached_candidate') }
	check_text_fields(f, ['release_name', 'body'])!
	mut size := ?u64(none)
	if s := f['asset_size'] {
		if !store.is_null(s) {
			size = store.u64_of(s) or { return error('bad asset_size') }
		}
	}
	mut prerelease := false
	if p := f['prerelease'] {
		prerelease = store.bool_of(p) or { return error('bad prerelease') }
	}
	return UpdateCandidate{
		version:           req_text(f, 'version')!
		tag_name:          req_text(f, 'tag_name')!
		release_name:      opt_text(f, 'release_name')
		release_notes_url: req_text(f, 'release_notes_url')!
		body:              opt_text(f, 'body')
		asset_name:        req_text(f, 'asset_name')!
		asset_url:         req_text(f, 'asset_url')!
		asset_digest:      req_text(f, 'asset_digest')!
		asset_size:        size
		target_triple:     req_text(f, 'target_triple')!
		prerelease:        prerelease
	}
}

fn encode_candidate(c UpdateCandidate) json2.Any {
	mut out := map[string]json2.Any{}
	out['version'] = json2.Any(c.version)
	out['tag_name'] = json2.Any(c.tag_name)
	out['release_name'] = text_or_null(c.release_name)
	out['release_notes_url'] = json2.Any(c.release_notes_url)
	out['body'] = text_or_null(c.body)
	out['asset_name'] = json2.Any(c.asset_name)
	out['asset_url'] = json2.Any(c.asset_url)
	out['asset_digest'] = json2.Any(c.asset_digest)
	if size := c.asset_size {
		out['asset_size'] = json2.Any(i64(size))
	} else {
		out['asset_size'] = store.null_value()
	}
	out['target_triple'] = json2.Any(c.target_triple)
	out['prerelease'] = json2.Any(c.prerelease)
	return json2.Any(out)
}

fn text_or_null(value ?string) json2.Any {
	return if text := value { json2.Any(text) } else { store.null_value() }
}

fn cache_values(c UpdateCheckCache) map[string]json2.Any {
	mut out := map[string]json2.Any{}
	if v := c.last_checked_at {
		out['last_checked_at'] = json2.Any(v)
	}
	if v := c.last_etag {
		out['last_etag'] = json2.Any(v)
	}
	if v := c.last_checked_version {
		out['last_checked_version'] = json2.Any(v)
	}
	if v := c.cached_candidate {
		out['cached_candidate'] = encode_candidate(v)
	}
	return out
}

fn preference_values(c UpdateConfig) map[string]json2.Any {
	mut out := map[string]json2.Any{}
	out['auto_check'] = json2.Any(c.auto_check)
	out['channel'] = json2.Any(c.channel.str())
	out['check_interval_hours'] = json2.Any(i64(c.check_interval_hours))
	out['verify_certificates'] = json2.Any(c.verify_certificates)
	if v := c.proxy_url {
		out['proxy_url'] = json2.Any(v)
	}
	if v := c.skipped_version {
		out['skipped_version'] = json2.Any(v)
	}
	return out
}
