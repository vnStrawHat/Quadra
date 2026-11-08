// Editing commands: typing, newline with auto-indent, deletion, tab, clipboard.
module code

fn (d &Doc) indent_unit() string {
	return if d.use_tabs { '\t' } else { ' '.repeat(d.tab_size) }
}

// insert_text types s at every cursor (replacing selections). kind 'type' merges into the
// typing group; use 'paste' or another kind for one group per call.
pub fn (mut d Doc) insert_text(s string, kind string) {
	d.apply(d.cursors.map(Edit{
		a:     it.start()
		e:     it.end()
		text:  s
		caret: s.len
	}), kind)
}

// leading returns the whitespace at the start of line n.
fn (d &Doc) leading(n int) string {
	return d.buf.slice(d.buf.line_start(n), d.first_text(n))
}

// newline breaks the line, copying its indentation and adding a level after an opener.
pub fn (mut d Doc) newline() {
	mut edits := []Edit{}
	for c in d.cursors {
		line := d.buf.line_of(c.start())
		mut indent := d.leading(line)
		if indent.len > c.start() - d.buf.line_start(line) {
			indent = indent[..c.start() - d.buf.line_start(line)]
		}
		before := d.buf.slice(d.buf.line_start(line), c.start()).trim_right(' \t')
		if before.len > 0 && before[before.len - 1] in [u8(`{`), `(`, `[`, `:`] {
			indent += d.indent_unit()
		}
		s := '\n' + indent
		edits << Edit{
			a:     c.start()
			e:     c.end()
			text:  s
			caret: s.len
		}
	}
	d.apply(edits, 'newline')
}

// delete_back deletes the selection, or the grapheme (word) before each cursor.
pub fn (mut d Doc) delete_back(word bool) {
	d.delete_dir(false, word)
}

pub fn (mut d Doc) delete_forward(word bool) {
	d.delete_dir(true, word)
}

fn (mut d Doc) delete_dir(forward bool, word bool) {
	mut edits := []Edit{}
	for c in d.cursors {
		mut a, mut e := c.start(), c.end()
		if c.empty() {
			if forward {
				e = if word { d.word_right(a) } else { d.grapheme_right(a) }
			} else {
				a = if word { d.word_left(e) } else { d.grapheme_left(e) }
			}
		}
		if a < e {
			edits << Edit{
				a:     a
				e:     e
				text:  ''
				caret: 0
			}
		}
	}
	if edits.len == d.cursors.len {
		d.apply(edits, 'type')
	} else if edits.len > 0 { // some cursors had nothing to delete: map them instead
		d.apply(edits.map(Edit{ ...it, caret: -1 }), 'delete')
	}
}

// tab indents the selected lines, or inserts an indent at each empty cursor.
pub fn (mut d Doc) tab() {
	if d.cursors.any(!it.empty() && d.buf.line_of(it.start()) != d.buf.line_of(it.end())) {
		d.indent_lines(true)
		return
	}
	mut edits := []Edit{}
	for c in d.cursors {
		s := if d.use_tabs {
			'\t'
		} else {
			' '.repeat(d.tab_size - d.col_of(c.start()) % d.tab_size)
		}
		edits << Edit{
			a:     c.start()
			e:     c.end()
			text:  s
			caret: s.len
		}
	}
	d.apply(edits, 'tab')
}

// copy_text is what copy puts on the clipboard: the selections, or the cursor lines when
// nothing is selected (one per cursor, joined by newlines).
pub fn (d &Doc) copy_text() string {
	if d.cursors.all(it.empty()) {
		mut out := []string{}
		for c in d.cursors {
			out << d.buf.line(d.buf.line_of(c.head)) + '\n'
		}
		return out.join('')
	}
	return d.selected_texts().join('\n')
}

// cut copies, then removes the selections (or the cursor lines).
pub fn (mut d Doc) cut() string {
	s := d.copy_text()
	if d.cursors.all(it.empty()) {
		d.delete_lines()
	} else {
		d.apply(d.cursors.map(Edit{ a: it.start(), e: it.end(), text: '', caret: 0 }), 'cut')
	}
	return s
}

// paste inserts s at every cursor, or one line per cursor when the counts match.
pub fn (mut d Doc) paste(s string) {
	text := s.replace('\r\n', '\n')
	lines := text.trim_right('\n').split('\n')
	if d.cursors.len > 1 && lines.len == d.cursors.len {
		mut edits := []Edit{}
		for i, c in d.cursors {
			edits << Edit{
				a:     c.start()
				e:     c.end()
				text:  lines[i]
				caret: lines[i].len
			}
		}
		d.apply(edits, 'paste')
		return
	}
	d.insert_text(text, 'paste')
}
