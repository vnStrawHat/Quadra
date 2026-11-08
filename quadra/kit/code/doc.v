// Doc: a buffer with cursors, grouped undo and the editing commands (specs/18). No UI here;
// the editor view calls these and doc_test.v covers them.
module code

import time

pub struct Cursor {
pub mut:
	anchor int
	head   int
	goal   int = -1 // display column kept across vertical moves (-1 = none)
}

pub fn (c Cursor) start() int {
	return if c.anchor < c.head { c.anchor } else { c.head }
}

pub fn (c Cursor) end() int {
	return if c.anchor > c.head { c.anchor } else { c.head }
}

pub fn (c Cursor) empty() bool {
	return c.anchor == c.head
}

fn at(pos int) Cursor {
	return Cursor{
		anchor: pos
		head:   pos
	}
}

// Edit replaces a..e with text. The cursor that asked for it lands at a + caret (or keeps
// a selection from a + sel to a + caret when sel >= 0); caret -1 maps the old cursor.
pub struct Edit {
pub:
	a     int
	e     int
	text  string
	caret int = -1
	sel   int = -1
}

struct Change {
	off      int
	removed  string
	inserted string
}

struct Group {
mut:
	changes []Change
	before  []Cursor
	after   []Cursor
	kind    string
	at      i64
}

fn mono_ms() i64 {
	return time.sys_mono_now() / 1_000_000
}

@[heap]
pub struct Doc {
pub mut:
	buf      &Buffer  = new_buffer('')
	cursors  []Cursor = [Cursor{}]
	tab_size int      = 4
	use_tabs bool
	comment  string = '//'
	version  int // bumped by every change (views cache by it)
	dirty    int       = max_int // lowest line changed since the highlighter last looked
	clock    fn () i64 = mono_ms
mut:
	undo_stack []Group
	redo_stack []Group
}

pub const max_int = 0x7fffffff

pub fn new_doc(text string) &Doc {
	return &Doc{
		buf: new_buffer(text)
	}
}

pub fn (d &Doc) primary() Cursor {
	return d.cursors.last()
}

// normalize sorts the cursors and merges the ones that overlap or touch.
pub fn (mut d Doc) normalize() {
	mut cs := d.cursors.clone()
	cs.sort(a.start() < b.start())
	mut out := []Cursor{cap: cs.len}
	for c in cs {
		if out.len > 0 {
			last := out.last()
			if c.start() < last.end() || (c.start() == last.end() && (c.empty() || last.empty())) {
				lo := if last.start() < c.start() { last.start() } else { c.start() }
				hi := if last.end() > c.end() { last.end() } else { c.end() }
				merged := if last.head >= last.anchor {
					Cursor{
						anchor: lo
						head:   hi
					}
				} else {
					Cursor{
						anchor: hi
						head:   lo
					}
				}
				out[out.len - 1] = merged
				continue
			}
		}
		out << c
	}
	d.cursors = out
}

// apply makes edits (one per cursor, in cursor order, not overlapping) as one change.
pub fn (mut d Doc) apply(edits []Edit, kind string) {
	if edits.len == 0 {
		return
	}
	before := d.cursors.clone()
	mut changes := []Change{cap: edits.len}
	for j := edits.len - 1; j >= 0; j-- {
		ed := edits[j]
		line := d.buf.line_of(ed.a)
		if line < d.dirty {
			d.dirty = line
		}
		removed := d.buf.slice(ed.a, ed.e)
		d.buf.delete(ed.a, ed.e)
		d.buf.insert(ed.a, ed.text)
		changes << Change{ed.a, removed, ed.text}
	}
	// New cursors. With carets, edit i places cursor i; without, every cursor is mapped
	// through the edits (line operations make one edit per line, not per cursor).
	mut out := []Cursor{cap: before.len}
	if edits.any(it.caret >= 0) {
		mut shift := 0
		for ed in edits {
			base := ed.a + shift
			caret := if ed.caret >= 0 { ed.caret } else { ed.text.len }
			out << Cursor{
				anchor: if ed.sel >= 0 { base + ed.sel } else { base + caret }
				head:   base + caret
			}
			shift += ed.text.len - (ed.e - ed.a)
		}
	} else {
		for c in before {
			out << Cursor{
				anchor: map_through(c.anchor, edits)
				head:   map_through(c.head, edits)
			}
		}
	}
	d.cursors = out
	d.normalize()
	d.version++
	d.redo_stack.clear()
	now := d.clock()
	if d.undo_stack.len > 0 && kind == 'type' {
		mut last := &d.undo_stack[d.undo_stack.len - 1]
		if last.kind == 'type' && now - last.at < 1000 && cursors_equal(last.after, before)
			&& !edits.any(it.text.contains('\n')) {
			last.changes << changes
			last.after = d.cursors.clone()
			last.at = now
			return
		}
	}
	d.undo_stack << Group{changes, before, d.cursors.clone(), kind, now}
}

// map_through moves an offset across sorted, non-overlapping edits. An offset at an
// insertion point stays before it; one inside a replaced range lands after the new text.
fn map_through(p int, edits []Edit) int {
	mut shift := 0
	for ed in edits {
		if p < ed.e || (p == ed.e && ed.a == ed.e) {
			if p <= ed.a {
				return p + shift
			}
			return ed.a + shift + ed.text.len
		}
		shift += ed.text.len - (ed.e - ed.a)
	}
	return p + shift
}

fn cursors_equal(a []Cursor, b []Cursor) bool {
	if a.len != b.len {
		return false
	}
	for i in 0 .. a.len {
		if a[i].anchor != b[i].anchor || a[i].head != b[i].head {
			return false
		}
	}
	return true
}

// break_group ends the current typing group (cursor moves, focus changes).
pub fn (mut d Doc) break_group() {
	if d.undo_stack.len > 0 {
		d.undo_stack[d.undo_stack.len - 1].kind = 'closed'
	}
}

pub fn (mut d Doc) undo() bool {
	if d.undo_stack.len == 0 {
		return false
	}
	g := d.undo_stack.pop()
	for i := g.changes.len - 1; i >= 0; i-- {
		c := g.changes[i]
		d.touch(c.off)
		d.buf.delete(c.off, c.off + c.inserted.len)
		d.buf.insert(c.off, c.removed)
	}
	d.cursors = g.before.clone()
	d.version++
	d.redo_stack << g
	return true
}

pub fn (mut d Doc) redo() bool {
	if d.redo_stack.len == 0 {
		return false
	}
	mut g := d.redo_stack.pop()
	for c in g.changes {
		d.touch(c.off)
		d.buf.delete(c.off, c.off + c.removed.len)
		d.buf.insert(c.off, c.inserted)
	}
	d.cursors = g.after.clone()
	d.version++
	g.kind = 'closed'
	d.undo_stack << g
	return true
}

fn (mut d Doc) touch(off int) {
	line := d.buf.line_of(off)
	if line < d.dirty {
		d.dirty = line
	}
}

pub fn (d &Doc) text() string {
	return d.buf.text()
}

pub fn (d &Doc) selected_texts() []string {
	return d.cursors.map(d.buf.slice(it.start(), it.end()))
}
