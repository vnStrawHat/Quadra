// The versioned session document: ids, migration, parse and encode (specs/25).
module store

import x.json2

// A row without a valid, unique id gets a fresh one; returns whether anything changed.
fn assign_missing_ids(mut fields map[string]json2.Any) !bool {
	sessions_any := fields[sessions_field] or {
		return InvalidData{
			reason: '${sessions_doc_name} sessions must be an array'
		}
	}
	mut sessions := array_of(sessions_any) or {
		return InvalidData{
			reason: '${sessions_doc_name} sessions must be an array'
		}
	}
	recorded_next := u64_of(fields[next_id_field] or { json2.Any(i64(1)) }) or { u64(1) }
	mut used := map[u64]bool{}
	mut max_id := u64(0)
	mut missing := []int{}
	for i, session in sessions {
		obj := object_of(session) or {
			missing << i
			continue
		}
		id := u64_of(obj['id'] or { json2.Any(i64(-1)) }) or { u64(0) }
		if id > 0 && id !in used {
			used[id] = true
			max_id = if id > max_id { id } else { max_id }
		} else {
			missing << i
		}
	}
	mut next_id := if recorded_next > max_id + 1 { recorded_next } else { max_id + 1 }
	if next_id < 1 {
		next_id = 1
	}
	mut changed := false
	for i in missing {
		mut obj := object_of(sessions[i]) or {
			return InvalidData{
				reason: '${sessions_doc_name} sessions must be objects'
			}
		}
		obj['id'] = json2.Any(i64(next_id))
		sessions[i] = json2.Any(obj)
		next_id++
		changed = true
	}
	fields[sessions_field] = json2.Any(sessions)
	current_next := if v := fields[next_id_field] { u64_of(v) or { u64(0) } } else { u64(0) }
	if current_next != next_id {
		fields[next_id_field] = json2.Any(i64(next_id))
		changed = true
	}
	return changed
}

fn migrate_session_step(version u32, value json2.Any) !json2.Any {
	if version == 0 {
		if list := array_of(value) {
			mut doc := map[string]json2.Any{}
			doc[sessions_field] = json2.Any(list)
			return json2.Any(doc)
		}
		if object_of(value) != none {
			return value
		}
	}
	if version == 1 {
		if mut fields := object_of(value) {
			assign_missing_ids(mut fields)!
			return json2.Any(fields)
		}
	}
	return InvalidData{
		reason: 'ssh_session.json schema must be an object or legacy array'
	}
}

struct SessionDocument {
mut:
	entries []Entry
	next_id u64 = 1
}

// LoadedSessions is a parsed document: `needs_resave` when the file was an older schema or repaired.
struct LoadedSessions {
mut:
	document     SessionDocument
	needs_resave bool
}

fn parse_session_document(raw string) !LoadedSessions {
	value := parse_json(raw) or { return InvalidData{
		reason: err.msg()
	} }
	source_version := schema_version(value) or { u32(0) }
	migrated := migrate_json_value(value, session_schema, sessions_doc_name, migrate_session_step)!
	mut fields := object_of(migrated) or { return InvalidData{
		reason: 'not an object'
	} }
	repaired := assign_missing_ids(mut fields)!
	sessions := array_of(fields[sessions_field] or {
		return InvalidData{
			reason: 'missing sessions'
		}
	}) or { return InvalidData{
		reason: 'sessions must be an array'
	} }
	mut entries := []Entry{cap: sessions.len}
	for item in sessions {
		obj := object_of(item) or { return bad_field('sessions') }
		entries << Entry{
			id:      u64_of(obj['id'] or { json2.Any(i64(0)) }) or { 0 }
			session: decode_session(obj)!
		}
	}
	next_id := u64_of(fields[next_id_field] or { return bad_field(next_id_field) }) or {
		return bad_field(next_id_field)
	}
	return LoadedSessions{
		document:     SessionDocument{
			entries: entries
			next_id: next_id
		}
		needs_resave: source_version < session_schema || repaired
	}
}

fn encode_session_document(document SessionDocument) string {
	mut fields := map[string]json2.Any{}
	fields[schema_field] = json2.Any(i64(session_schema))
	fields[sessions_field] = json2.Any(document.entries.map(encode_entry(it)))
	fields[next_id_field] = json2.Any(i64(document.next_id))
	return encode_pretty(json2.Any(fields))
}
