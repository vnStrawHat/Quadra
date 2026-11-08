// Line formatting: headings, lists and quotes on every selected line, and blocks (fences,
// snippets) inserted on lines of their own.
module main

import quadra.kit.code

// LinePrefix splits a line into its indentation, its block marker and the rest.
struct LinePrefix {
	indent string
	kind   Cmd = .para // .h1 .. .h3 (deeper headings count as .para with a marker), lists
	marker string
	rest   string
}

fn split_line(line string) LinePrefix {
	t := line.trim_left(' \t')
	indent := line[..line.len - t.len]
	mut n := 0
	for n < t.len && t[n] == `#` {
		n++
	}
	if n in [1, 2, 3, 4, 5, 6] && (n == t.len || t[n] == ` `) {
		kind := match n {
			1 { Cmd.h1 }
			2 { Cmd.h2 }
			3 { Cmd.h3 }
			else { Cmd.para }
		}

		return LinePrefix{indent, kind, t[..n], t[n..].trim_left(' ')}
	}
	for m in ['- [ ] ', '- [x] ', '- [X] ', '* [ ] ', '* [x] '] {
		if t.starts_with(m) {
			return LinePrefix{indent, .task, m, t[m.len..]}
		}
	}
	if t.len >= 2 && t[0] in [`-`, `*`, `+`] && t[1] == ` ` {
		return LinePrefix{indent, .bullet, t[..2], t[2..]}
	}
	mut d := 0
	for d < t.len && t[d].is_digit() {
		d++
	}
	if d > 0 && d + 1 < t.len && t[d] in [`.`, `)`] && t[d + 1] == ` ` {
		return LinePrefix{indent, .numbered, t[..d + 2], t[d + 2..]}
	}
	return LinePrefix{indent, .para, '', t}
}

// set_lines gives the selected lines a heading, list or quote marker, or takes it away
// when they all have it already. Blank lines inside a selection stay blank.
fn set_lines(mut d code.Doc, cmd Cmd) {
	l0, l1 := line_range(d)
	lines := []string{len: l1 - l0 + 1, init: d.buf.line(l0 + index)}
	parts := lines.map(split_line(it))
	used := lines.filter(it.trim_space() != '')
	all := if cmd == .quote {
		used.len > 0 && used.all(it.trim_left(' \t').starts_with('>'))
	} else {
		used.len > 0 && parts.filter(it.rest != '' || it.marker != '').all(it.kind == cmd)
	}
	mut out := []string{cap: lines.len}
	mut num := 0
	for k, line in lines {
		p := parts[k]
		if line.trim_space() == '' && lines.len > 1 {
			out << if cmd == .quote && !all { '>' } else { line }
			continue
		}
		num++
		out << styled(cmd, all, line, p, num)
	}
	a, e := d.buf.line_start(l0), d.buf.line_end(l1)
	t := out.join('\n')
	c := d.primary()
	if c.empty() && lines.len == 1 {
		// The caret keeps its place in the text; it moves past the new marker at least.
		mut rest := parts[0].rest.len
		if cmd == .quote {
			rest = if all { t.len } else { lines[0].len }
		}
		col := c.head - a + t.len - lines[0].len
		d.apply([
			code.Edit{
				a:     a
				e:     e
				text:  t
				caret: clamp(col, t.len - rest, t.len)
			},
		], 'format')
		return
	}
	d.apply([code.Edit{
		a:     a
		e:     e
		text:  t
		sel:   0
		caret: t.len
	}], 'format')
}

// styled is one line under cmd; all: every line has it already, so it comes off.
fn styled(cmd Cmd, all bool, line string, p LinePrefix, num int) string {
	match cmd {
		.quote {
			if all {
				return line.trim_left(' \t').trim_string_left('>').trim_left(' ')
			}
			return '> ' + line
		}
		.h1, .h2, .h3 {
			if all {
				return p.rest
			}
			level := match cmd {
				.h1 { 1 }
				.h2 { 2 }
				else { 3 }
			}

			return '#'.repeat(level) + ' ' + p.rest
		}
		.bullet, .numbered, .task {
			if all {
				return p.indent + p.rest
			}
			return p.indent + list_marker(cmd, num) + p.rest
		}
		else {
			if p.marker.starts_with('#') {
				return p.rest
			}
			return p.indent + p.rest
		}
	}
}

fn list_marker(cmd Cmd, n int) string {
	return match cmd {
		.numbered { '${n}. ' }
		.task { '- [ ] ' }
		else { '- ' }
	}
}

fn clamp(v int, lo int, hi int) int {
	return if v < lo {
		lo
	} else if v > hi {
		hi
	} else {
		v
	}
}

// fence puts the selected lines between two fence lines, the caret after the opening one
// (to type a language); with no selection it inserts an empty block, the caret inside.
fn fence(mut d code.Doc, mark string) {
	if d.primary().empty() {
		insert_block(mut d, mark + '\n\n' + mark, mark.len + 1, -1)
		return
	}
	l0, l1 := line_range(d)
	a, e := d.buf.line_start(l0), d.buf.line_end(l1)
	d.apply([
		code.Edit{
			a:     a
			e:     e
			text:  mark + '\n' + d.buf.slice(a, e) + '\n' + mark
			caret: mark.len
		},
	], 'format')
}

// insert_block puts text on lines of its own: in place of a blank line, else after the
// caret's line, with blank lines around it. caret and sel are offsets in text.
fn insert_block(mut d code.Doc, text string, caret int, sel int) {
	n := d.buf.line_of(d.primary().head)
	a, e := d.buf.line_start(n), d.buf.line_end(n)
	blank := d.buf.line(n).trim_space() == ''
	before := if !blank {
		'\n\n'
	} else if n > 0 && d.buf.line(n - 1).trim_space() != '' {
		'\n'
	} else {
		''
	}
	after := if n + 1 < d.buf.line_count() && d.buf.line(n + 1).trim_space() != '' {
		'\n'
	} else {
		''
	}
	d.apply([
		code.Edit{
			a:     if blank { a } else { e }
			e:     e
			text:  before + text + after
			caret: before.len + caret
			sel:   if sel >= 0 { before.len + sel } else { -1 }
		},
	], 'format')
}
