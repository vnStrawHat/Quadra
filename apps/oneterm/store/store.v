// SessionStore: the saved SSH sessions in memory, written to ssh_session.json (specs/25).
module store

import os
import sync
import time

pub const document_name = sessions_doc_name

// PersistQueue is newest-wins and single-flight: a stale snapshot can never overwrite a newer one.
@[heap]
struct PersistQueue {
mut:
	mu      &sync.Mutex = sync.new_mutex()
	pending ?SessionDocument
	saving  bool
}

fn (mut q PersistQueue) enqueue(snapshot SessionDocument) bool {
	q.mu.@lock()
	defer {
		q.mu.unlock()
	}
	q.pending = snapshot
	if q.saving {
		return false
	}
	q.saving = true
	return true
}

fn (mut q PersistQueue) take() ?SessionDocument {
	q.mu.@lock()
	defer {
		q.mu.unlock()
	}
	if snapshot := q.pending {
		q.pending = none
		return snapshot
	}
	q.saving = false
	return none
}

fn (mut q PersistQueue) is_idle() bool {
	q.mu.@lock()
	defer {
		q.mu.unlock()
	}
	return !q.saving && q.pending == none
}

fn drain_queue(mut queue PersistQueue, path string) {
	for {
		snapshot := queue.take() or { return }
		save_snapshot(snapshot, path) or { eprintln('${document_name}: ${err.msg()}') }
	}
}

fn sealed_values(raw string) map[string]bool {
	mut found := map[string]bool{}
	value := parse_json(raw) or { return found }
	fields := object_of(value) or { return found }
	sessions := fields[sessions_field] or { return found }
	for item in array_of(sessions) or { return found } {
		obj := object_of(item) or { continue }
		sealed := obj[sealed_field] or { continue }
		found[string_of(sealed) or { continue }] = true
	}
	return found
}

// `.bak` is scrubbed only when this write drops or replaces a sealed value the replaced
// document held; a document that does not parse counts as holding some.
fn retires_secret(previous []u8, kept map[string]bool) bool {
	text := previous.bytestr()
	_ := parse_json(text) or { return true }
	for value, _ in sealed_values(text) {
		if value !in kept {
			return true
		}
	}
	return false
}

fn save_snapshot(document SessionDocument, path string) ! {
	json := encode_session_document(document)
	atomic_write_sealed_with(path, json.bytes(), sealed_values(json), fn (kept map[string]bool, previous []u8) bool {
		return retires_secret(previous, kept)
	})!
}

// load_sessions: a missing file means no sessions yet; a file that does not parse or migrate is
// quarantined and an empty store starts; any other read failure is an error.
fn load_sessions(path string, read_only bool) !LoadedSessions {
	if !os.exists(path) {
		return LoadedSessions{}
	}
	raw := read_text(path)!
	return parse_session_document(raw) or {
		eprintln('${document_name}: ${err.msg()}: starting empty')
		quarantine_file(path, read_only) or { eprintln('failed to quarantine: ${err.msg()}') }
		return LoadedSessions{}
	}
}

// SessionStore holds the list and persists every mutation through the queue.
@[heap]
pub struct SessionStore {
mut:
	entries         []Entry
	next_id         u64 = 1
	path            string
	read_only       bool
	persist_blocked bool
	queue           &PersistQueue = &PersistQueue{}
}

// open loads `<dir>/ssh_session.json` and writes it back when it was migrated or repaired.
pub fn open(dir string, read_only bool) &SessionStore {
	mut s := &SessionStore{
		path:      os.join_path(dir, document_name)
		read_only: read_only
	}
	loaded := load_sessions(s.path, read_only) or {
		eprintln('${err.msg()}; starting empty and refusing to overwrite the file')
		s.persist_blocked = true
		return s
	}
	s.entries = loaded.document.entries
	s.next_id = loaded.document.next_id
	if loaded.needs_resave {
		s.save()
	}
	return s
}

// sessions returns the entries in storage order.
pub fn (s &SessionStore) sessions() []Entry {
	return s.entries
}

// get returns the session with `id`.
pub fn (s &SessionStore) get(id u64) ?Session {
	for e in s.entries {
		if e.id == id {
			return e.session
		}
	}
	return none
}

fn (s &SessionStore) may_persist() bool {
	return !s.read_only && !s.persist_blocked
}

fn (s &SessionStore) snapshot() SessionDocument {
	return SessionDocument{
		entries: s.entries.clone()
		next_id: s.next_id
	}
}

fn (mut s SessionStore) save() {
	if !s.may_persist() {
		return
	}
	mut queue := s.queue
	if queue.enqueue(s.snapshot()) {
		spawn drain_queue(mut queue, s.path)
	}
}

// flush blocks until every queued write has reached the disk.
pub fn (mut s SessionStore) flush() {
	for !s.queue.is_idle() {
		time.sleep(2 * time.millisecond)
	}
}

// add stores a session under a fresh id and returns it.
pub fn (mut s SessionStore) add(session Session) u64 {
	id := s.next_id
	s.next_id++
	s.entries << Entry{
		id:      id
		session: session
	}
	s.save()
	return id
}

// update replaces the session stored under `id`; returns whether it still exists.
pub fn (mut s SessionStore) update(id u64, session Session) bool {
	for i, e in s.entries {
		if e.id == id {
			s.entries[i].session = session
			s.save()
			return true
		}
	}
	return false
}

// remove deletes the session stored under `id`.
pub fn (mut s SessionStore) remove(id u64) {
	before := s.entries.len
	s.entries = s.entries.filter(it.id != id)
	if s.entries.len != before {
		s.save()
	}
}

// duplicate copies every field of `id` right after it, under a fresh id and a free label.
pub fn (mut s SessionStore) duplicate(id u64) ?u64 {
	new_id := s.next_id
	if !duplicate_in(mut s.entries, id, new_id) {
		return none
	}
	s.next_id++
	s.save()
	return new_id
}

// set_group moves one session into `group` (none or blank means ungrouped).
pub fn (mut s SessionStore) set_group(id u64, group ?string) {
	if set_group_in(mut s.entries, id, group) {
		s.save()
	}
}

// rename_group moves every member of `old_name`; a blank new name ungroups them.
pub fn (mut s SessionStore) rename_group(old_name string, new_name string) {
	if rename_group_in(mut s.entries, old_name, new_name) {
		s.save()
	}
}

// set_sealed_password stores an envelope, refused when the session moved to another address.
pub fn (mut s SessionStore) set_sealed_password(id u64, host string, port int, username string, sealed string) ! {
	set_sealed_in(mut s.entries, id, host, port, username, sealed)!
	s.save()
}

// clear_sealed_passwords drops every saved password. Idempotent: a vault reset calls it first.
pub fn (mut s SessionStore) clear_sealed_passwords() {
	if clear_sealed_in(mut s.entries) {
		s.save()
	}
}

// jump_chain follows jump_host links from `first`, outermost hop first. `target` is the
// session being connected or edited: a chain reaching it is a cycle.
pub fn (s &SessionStore) jump_chain(first ?u64, target ?u64) ![]Entry {
	mut chain := []Entry{}
	mut next := first
	for {
		id := next or { break }
		if target or { 0 } == id && target != none {
			return JumpError{
				kind: .cycle
				id:   id
			}
		}
		if chain.any(it.id == id) {
			return JumpError{
				kind: .cycle
				id:   id
			}
		}
		if chain.len == max_jump_hops {
			return JumpError{
				kind:  .too_long
				count: chain.len + 1
			}
		}
		mut entry := Entry{}
		mut found := false
		for e in s.entries {
			if e.id == id {
				entry = e
				found = true
				break
			}
		}
		if !found {
			return JumpError{
				kind: .missing
				id:   id
			}
		}
		next = entry.session.jump_host
		chain << entry
	}
	chain.reverse_in_place()
	return chain
}
