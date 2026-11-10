// Per-session browser state, and the channel from worker threads to the main thread.
module sftpui

import sync
import ssh
import quadra.core

pub enum Direction {
	upload
	download
}

pub enum TransferStatus {
	in_progress
	completed
	cancelled
	error
}

pub struct TransferItem {
pub mut:
	id         u64
	direction  Direction
	filename   string
	progress   f64
	discovered int = -1 // files found while a folder is listed; -1 once bytes move
	status     TransferStatus
	error      string
}

// Backend is everything shown for one SFTP session; a hidden session keeps its state and its
// transfers keep landing here.
@[heap]
struct Backend {
mut:
	key           u64
	client        Client
	cwd           string
	entries       []ssh.FileEntry
	sort          SortState
	selected      int = -1
	loading       bool
	error         string
	path_error    bool
	transfers     []TransferItem
	generation    u64
	follow        bool
	last_followed string
}

enum BusKind {
	begin
	progress
	discovering
	finished
}

// BusMsg is one transfer event, posted from a worker.
struct BusMsg {
	key       u64
	kind      BusKind
	id        u64
	direction Direction
	name      string
	fraction  f64
	files     int
	status    TransferStatus
	error     string
}

// Bus carries transfer events to the main thread: an Inbox byte wakes the drain, so a burst
// of chunks costs one delivery.
@[heap]
struct Bus {
mut:
	mu      &sync.Mutex = sync.new_mutex()
	queue   []BusMsg
	inbox   &core.Inbox = unsafe { nil }
	next_id u64
}

fn (mut b Bus) post(m BusMsg) {
	b.mu.@lock()
	b.queue << m
	b.mu.unlock()
	b.inbox.push([u8(1)])
}

fn (mut b Bus) take() []BusMsg {
	b.mu.@lock()
	q := b.queue
	b.queue = []BusMsg{}
	b.mu.unlock()
	return q
}

fn (mut b Bus) alloc_id() u64 {
	b.mu.@lock()
	defer {
		b.mu.unlock()
	}
	b.next_id++
	return b.next_id
}

fn is_cancelled_err(e IError) bool {
	return e is ssh.Cancelled
}

fn (st &Backend) find_transfer(id u64) int {
	for i, it in st.transfers {
		if it.id == id {
			return i
		}
	}
	return -1
}

fn (st &Backend) active_count() int {
	return st.transfers.filter(it.status == .in_progress).len
}

// apply updates the item an event refers to; a `begin` adds it.
fn (mut st Backend) apply(m BusMsg) {
	if m.kind == .begin {
		st.transfers << TransferItem{
			id:        m.id
			direction: m.direction
			filename:  m.name
		}
		return
	}
	i := st.find_transfer(m.id)
	if i < 0 {
		return
	}
	match m.kind {
		.progress {
			st.transfers[i].progress = m.fraction
			st.transfers[i].discovered = -1
		}
		.discovering {
			st.transfers[i].discovered = m.files
		}
		.finished {
			if m.status == .completed {
				st.transfers[i].progress = 1
			}
			if st.transfers[i].status == .in_progress {
				st.transfers[i].status = m.status
				st.transfers[i].error = m.error
			}
		}
		.begin {}
	}
}
