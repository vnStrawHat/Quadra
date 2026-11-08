// Undo/redo with typing coalesced per word.
module input

struct Snapshot {
	text   string
	anchor int
	head   int
}

pub enum EditKind {
	typing
	deleting
	other // paste, cut, programmatic changes: always a separate step
}

pub struct History {
mut:
	undo      []Snapshot
	redo      []Snapshot
	last_kind EditKind = .other
	open      bool // a typing run is in progress
pub mut:
	limit int = 500
}

fn snap(b &Buffer) Snapshot {
	return Snapshot{b.text, b.anchor, b.head}
}

// record is called before an edit. Consecutive typing (or deleting) joins the previous
// step until a word boundary, so one undo removes one word.
pub fn (mut h History) record(b &Buffer, kind EditKind, boundary bool) {
	h.redo.clear()
	if h.open && kind == h.last_kind && kind != .other && !boundary {
		return
	}
	h.undo << snap(b)
	if h.undo.len > h.limit {
		h.undo.delete(0)
	}
	h.last_kind = kind
	h.open = kind != .other
}

// break_run ends the current typing run (caret moved, focus changed).
pub fn (mut h History) break_run() {
	h.open = false
}

pub fn (mut h History) undo(mut b Buffer) bool {
	if h.undo.len == 0 {
		return false
	}
	h.redo << snap(b)
	s := h.undo.pop()
	b.text, b.anchor, b.head = s.text, s.anchor, s.head
	h.open = false
	return true
}

pub fn (mut h History) redo(mut b Buffer) bool {
	if h.redo.len == 0 {
		return false
	}
	h.undo << snap(b)
	s := h.redo.pop()
	b.text, b.anchor, b.head = s.text, s.anchor, s.head
	h.open = false
	return true
}
