// Update preferences: saving, cache sync and the auto-check policy (specs/25).
module update

import store
import time
import x.json2

// save_preferences_to merges the preference fields, leaving the checker's cache untouched.
pub fn (c UpdateConfig) save_preferences_to(path string, read_only bool) ! {
	merge_owned_fields(path, preference_fields, preference_values(c), read_only)!
}

// check_cache snapshots the checker-owned fields.
pub fn (c UpdateConfig) check_cache() UpdateCheckCache {
	return UpdateCheckCache{
		last_checked_at:      c.last_checked_at
		last_etag:            c.last_etag
		last_checked_version: c.last_checked_version
		cached_candidate:     c.cached_candidate
	}
}

// apply_check_cache replaces only the checker-owned fields, keeping every preference.
pub fn (mut c UpdateConfig) apply_check_cache(cache UpdateCheckCache) {
	c.last_checked_at = cache.last_checked_at
	c.last_etag = cache.last_etag
	c.last_checked_version = cache.last_checked_version
	c.cached_candidate = cache.cached_candidate
}

// should_auto_check_at: a startup check runs when enabled and the interval has elapsed.
pub fn (c UpdateConfig) should_auto_check_at(now time.Time) bool {
	if !c.auto_check {
		return false
	}
	text := c.last_checked_at or { return true }
	last := time.parse_rfc3339(text) or { return true }
	interval := i64(c.effective_check_interval_hours()) * 3600
	return now.unix() - last.unix() >= interval
}

// effective_check_interval_hours is clamped to 1 ..= one year.
pub fn (c UpdateConfig) effective_check_interval_hours() u64 {
	return if c.check_interval_hours < 1 {
		u64(1)
	} else if c.check_interval_hours > max_check_interval_hours {
		max_check_interval_hours
	} else {
		c.check_interval_hours
	}
}

// record_success stores cache metadata after a successful GitHub response.
pub fn (mut c UpdateConfig) record_success(etag ?string, current_version string) {
	c.last_checked_at = time.utc().format_rfc3339()
	c.last_checked_version = current_version
	if e := etag {
		c.last_etag = e
	}
}

// should_reuse_cached_etag: a new build must re-evaluate release availability.
pub fn (c UpdateConfig) should_reuse_cached_etag(current_version string) bool {
	return c.last_etag != none && (c.last_checked_version or { '' }) == current_version
}

// Keys absent from `values` are removed, so a cleared field does not survive; a document that
// no longer parses is quarantined once and the merge restarts from an empty document.
fn merge_owned_fields(path string, keys []string, values map[string]json2.Any, read_only bool) ! {
	if read_only {
		return error('a read-only window writes no configuration')
	}
	owned := OwnedFields{keys, values}
	store.update_json_file_with(path, owned, merge_fields) or {
		if err is store.InvalidData {
			eprintln('${document_name} is unreadable; quarantining before rewrite')
			store.quarantine_file(path, false)!
			store.update_json_file_with(path, owned, merge_fields)!
			return
		}
		return err
	}
}

// OwnedFields is what merge_fields writes: the owned keys and their new values.
struct OwnedFields {
	keys   []string
	values map[string]json2.Any
}

fn merge_fields(o OwnedFields, mut doc json2.Any) ! {
	mut fields := store.object_of(doc) or {
		map[string]json2.Any{}
	}
	for key in o.keys {
		if value := o.values[key] {
			fields[key] = value
		} else {
			fields.delete(key)
		}
	}
	fields[store.schema_field] = json2.Any(i64(current_schema))
	doc = json2.Any(fields)
}
