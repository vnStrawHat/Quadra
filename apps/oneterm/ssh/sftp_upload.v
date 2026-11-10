// Local-to-remote transfers: one file, or a folder listed in full first.
module ssh

import os

// upload copies a local file or folder to remote. It blocks: call it on a worker thread.
pub fn (mut s Sftp) upload(id u64, local string, remote_ string, on_event OnTransfer) ! {
	mut cancel := s.register(id)!
	defer {
		s.unregister(id)
	}
	remote := remote_path(remote_)
	s.upload_any(local, remote, on_event, mut cancel) or {
		if err is Cancelled {
			emit(on_event, .cancelled, 0, 0)
		}
		return err
	}
}

fn (mut s Sftp) upload_any(local string, remote string, on_event OnTransfer, mut cancel Cancel) ! {
	if !os.exists(local) {
		return error('stat local: ${local} does not exist')
	}
	if os.is_dir(local) {
		s.upload_dir(local, remote, on_event, mut cancel)!
		return
	}
	total := os.file_size(local)
	mut rep := Reporter{
		on_event: on_event
		total:    total
	}
	s.upload_file(local, remote, mut cancel, mut rep)!
	emit(on_event, .progress, 1.0, 0)
}

// upload_file writes a temporary remote sibling and moves it into place only when complete.
fn (mut s Sftp) upload_file(local string, remote string, mut cancel Cancel, mut rep Reporter) ! {
	mut input := os.open(local) or { return error('open local: ${err}') }
	defer {
		input.close()
	}
	stat := os.stat(local) or { return error('stat local: ${err}') }
	temporary := s.temp_remote(remote, 'part')!
	handle := s.open_handle(temporary, fxf_write | fxf_creat | fxf_trunc, Attrs{})!
	mut failed := false
	s.push(mut input, handle, mut cancel, mut rep) or {
		failed = true
		s.close_handle(handle) or {}
		s.remove(temporary) or {}
		return err
	}
	if !failed {
		s.close_handle(handle) or {
			s.remove(temporary) or {}
			return error('close remote: ${err}')
		}
	}
	s.finalize_remote(temporary, remote)!
	// Windows has no Unix mode: only the times are copied.
	s.set_attrs(remote, Attrs{
		flags: attr_times
		atime: u32(stat.atime)
		mtime: u32(stat.mtime)
	}) or {}
}

struct LocalEntry {
	local  string
	remote string
	size   u64
	is_dir bool
}

// collect_local walks the tree breadth-first, each folder before its contents.
fn collect_local(root string, remote string, mut cancel Cancel) ![]LocalEntry {
	mut pending := [PendingCopy{remote, root, 0}]
	mut entries := []LocalEntry{}
	mut visited := 0
	mut head := 0
	for head < pending.len {
		cur := pending[head]
		head++
		if cancel.is_cancelled() {
			return Cancelled{}
		}
		if cur.depth > max_traversal_depth {
			return error('local upload exceeded traversal depth limit')
		}
		entries << LocalEntry{cur.local, cur.remote, 0, true}
		names := os.ls(cur.local) or { return error('walk local dir: ${err}') }
		for name in names {
			visited++
			if visited > max_traversal_entries {
				return error('local upload exceeded traversal entry limit')
			}
			path := os.join_path(cur.local, name)
			child := remote_join(cur.remote, name)
			if os.is_link(path) {
				return error('refusing to upload local symlink: ${path}')
			}
			if os.is_dir(path) {
				pending << PendingCopy{child, path, cur.depth + 1}
			} else if os.is_file(path) {
				entries << LocalEntry{path, child, os.file_size(path), false}
			}
		}
	}
	return entries
}

fn (mut s Sftp) upload_dir(local string, remote string, on_event OnTransfer, mut cancel Cancel) ! {
	entries := collect_local(local, remote, mut cancel)!
	mut total := u64(0)
	for e in entries {
		total += e.size
	}
	mut tree := &TreeProgress{}
	for e in entries {
		if cancel.is_cancelled() {
			return Cancelled{}
		}
		if e.is_dir {
			s.mkdir(e.remote) or {
				attrs := s.attrs_of(e.remote, true) or {
					return error('create remote directory ${e.remote}: ${err}')
				}
				if !attrs.is_dir() || attrs.is_symlink() {
					return error('remote upload path exists but is not a directory: ${e.remote}')
				}
			}
			continue
		}
		start := tree.done
		mut rep := Reporter{
			on_event: on_event
			total:    total
			start:    start
			tree:     tree
		}
		s.upload_file(e.local, e.remote, mut cancel, mut rep)!
		tree.done = start + e.size
	}
	emit(on_event, .progress, 1.0, 0)
}
