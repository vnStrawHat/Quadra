// Save behaviour of the session store.
module store

import os

fn test_back_to_back_saves_keep_only_the_newest_snapshot_on_disk() {
	dir := test_dir('queue')
	mut s := open(dir, false)
	for i in 0 .. 25 {
		s.add(session_named('s${i}'))
	}
	s.flush()
	loaded := load_sessions(os.join_path(dir, document_name), false)!
	assert loaded.document.entries.len == 25
	assert loaded.document.next_id == 26
}

fn test_a_read_only_store_writes_nothing() {
	dir := test_dir('readonly')
	mut s := open(dir, true)
	s.add(session_named('a'))
	s.flush()
	assert !os.exists(os.join_path(dir, document_name))
}

fn test_vietnamese_labels_survive() {
	dir := test_dir('unicode')
	path := os.join_path(dir, document_name)
	mut v := session_named('Máy chủ Hà Nội')
	v.group = 'Nhóm 1 "đặc biệt"'
	save_snapshot(doc_of([entry_of(1, v)], 2), path)!
	back := load_sessions(path, false)!.document.entries[0].session
	assert back.label == 'Máy chủ Hà Nội'
	assert back.group or { '' } == 'Nhóm 1 "đặc biệt"'
}
