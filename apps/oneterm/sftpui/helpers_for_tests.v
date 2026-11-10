// A Client without a server, for the tests. Nothing else calls this.
module sftpui

import os
import sync
import time
import ssh

@[heap]
pub struct FakeClient {
pub mut:
	mu         &sync.Mutex = sync.new_mutex()
	tree       map[string][]ssh.FileEntry
	home       string = '/home/user'
	fail_read  map[string]string
	fail_op    string         // the next mutation fails with this message
	slow       map[string]int // read_dir of this path sleeps ms
	dead       bool
	log        []string
	uploaded   map[string]string // remote -> content
	content    map[string]string // remote -> what download writes
	mtime      map[string]i64    // remote -> stat override
	cancelled  []u64
	hang_chunk bool // the transfer stays at 50 % until cancelled
}

pub fn fake_entry(dir string, name string, is_dir bool, size u64, mtime i64) ssh.FileEntry {
	return ssh.FileEntry{
		name:        name
		path:        ssh.remote_join(dir, name)
		is_dir:      is_dir
		size:        size
		modified:    mtime
		permissions: if is_dir { u32(0o040755) } else { u32(0o100644) }
		uid:         0
		owner:       'root'
	}
}

pub fn (mut f FakeClient) add_dir(dir string, entries []ssh.FileEntry) {
	f.tree[dir] = entries
}

pub fn (mut f FakeClient) read_dir(path string) ![]ssh.FileEntry {
	if ms := f.slow[path] {
		time.sleep(ms * time.millisecond)
	}
	f.mu.@lock()
	defer {
		f.mu.unlock()
	}
	f.log << 'read_dir ${path}'
	if msg := f.fail_read[path] {
		return error(msg)
	}
	return f.tree[path] or { return error('no such file') }
}

pub fn (mut f FakeClient) find(path string) ?ssh.FileEntry {
	parent := ssh.remote_parent(path)
	for e in f.tree[parent] or { return none } {
		if e.path == path {
			return e
		}
	}
	return none
}

pub fn (mut f FakeClient) stat(path string) !ssh.FileEntry {
	f.mu.@lock()
	defer {
		f.mu.unlock()
	}
	if path in f.tree {
		return ssh.FileEntry{
			name:   ssh.remote_file_name(path)
			path:   path
			is_dir: true
		}
	}
	mut e := f.find(path) or { return error('no such file') }
	if t := f.mtime[path] {
		return ssh.FileEntry{
			...e
			modified: t
		}
	}
	return e
}

pub fn (mut f FakeClient) realpath(path string) !string {
	return if path == '.' { f.home } else { path }
}

fn (mut f FakeClient) mutation(what string) ! {
	f.mu.@lock()
	defer {
		f.mu.unlock()
	}
	f.log << what
	if f.fail_op != '' {
		msg := f.fail_op
		f.fail_op = ''
		return error(msg)
	}
}

pub fn (mut f FakeClient) rename(from string, to string) ! {
	f.mutation('rename ${from} ${to}')!
}

pub fn (mut f FakeClient) remove(path string) ! {
	f.mutation('remove ${path}')!
}

pub fn (mut f FakeClient) remove_dir_all(path string) ! {
	f.mutation('remove_dir_all ${path}')!
}

pub fn (mut f FakeClient) mkdir(path string) ! {
	f.mutation('mkdir ${path}')!
}

pub fn (mut f FakeClient) upload(id u64, local string, remote string, on_event ssh.OnTransfer) ! {
	on_event.f(on_event.ctx, ssh.TransferEvent{ kind: .progress, fraction: 0.5 })
	if f.hang_chunk {
		for id !in f.cancelled {
			time.sleep(2 * time.millisecond)
		}
		return ssh.Cancelled{}
	}
	data := os.read_file(local) or { return error('read: ${err}') }
	f.mu.@lock()
	f.uploaded[remote] = data
	f.log << 'upload ${remote}'
	f.mu.unlock()
	on_event.f(on_event.ctx, ssh.TransferEvent{ kind: .progress, fraction: 1 })
}

pub fn (mut f FakeClient) download(id u64, remote string, local string, on_event ssh.OnTransfer) ! {
	on_event.f(on_event.ctx, ssh.TransferEvent{ kind: .progress, fraction: 0.5 })
	if f.hang_chunk {
		for id !in f.cancelled {
			time.sleep(2 * time.millisecond)
		}
		return ssh.Cancelled{}
	}
	f.mu.@lock()
	data := f.content[remote] or { 'data of ${remote}' }
	f.log << 'download ${remote} -> ${local}'
	f.mu.unlock()
	os.write_file(local, data)!
	on_event.f(on_event.ctx, ssh.TransferEvent{ kind: .progress, fraction: 1 })
}

pub fn (mut f FakeClient) cancel_transfer(id u64) {
	f.mu.@lock()
	f.cancelled << id
	f.mu.unlock()
}

pub fn (mut f FakeClient) alive() bool {
	return !f.dead
}
