// Helpers shared by this module's tests; nothing else calls them.
module store

import os
import rand

const test_root = os.join_path(@VMODROOT, '.tmp', 'oneterm-tests', 'store')

fn test_dir(name string) string {
	dir := os.join_path(test_root, name, '${os.getpid()}-${rand.hex(8)}')
	os.rmdir_all(dir) or {} // rand repeats between runs: never inherit a leftover
	os.mkdir_all(dir) or { panic(err) }
	return dir
}

fn write_text(path string, text string) {
	os.write_file(path, text) or { panic(err) }
}

fn session_named(label string) Session {
	return Session{
		label: label
		host:  '${label}.example.test'
		group: none
	}
}

fn doc_of(rows []Entry, next u64) SessionDocument {
	return SessionDocument{
		entries: rows
		next_id: next
	}
}

fn entry_of(id u64, s Session) Entry {
	return Entry{
		id:      id
		session: s
	}
}
