// Markdown blocks: the CommonMark subset docs, READMEs and notes use (specs/21). Headings,
// paragraphs, bullet, numbered and task lists (nesting by indent), fenced code, quotes
// (nested), rules, pipe tables with alignment, display math and YAML front matter. Every
// block records its source line. Not handled: setext headings, indented code, HTML,
// reference links, footnotes.
module md

pub enum Kind {
	paragraph
	heading
	bullet
	ordered
	code
	quote
	rule
	table
	math  // $$...$$ or a ```math fence: text is the TeX source
	front // YAML front matter: text is its body
}

pub enum Align {
	none
	left
	center
	right
}

pub struct Block {
pub:
	kind  Kind
	level int        // heading level 1-6; list nesting depth from 0; quote depth from 1
	text  string     // inline source (paragraph, heading, item, quote) or the code body
	lang  string     // fenced code
	num   int        // the number of an ordered item
	rows  [][]string // table cells, the header row first
	align []Align    // table column alignment
	task  int        // list items: 0 not a task, 1 open `[ ]`, 2 done `[x]`
	line  int        // the source line the block starts on, from 0
}

struct Parser {
mut:
	out   []Block
	para  []string // lines of the open paragraph, quote or item
	kind  Kind
	lvl   int
	num   int
	task  int
	start int // the source line of the open block
}

fn (mut p Parser) flush() {
	if p.para.len == 0 {
		return
	}
	p.out << Block{
		kind:  p.kind
		level: p.lvl
		text:  join_lines(p.para)
		num:   p.num
		task:  p.task
		line:  p.start
	}
	p.para.clear()
}

// join_lines joins a block's lines with spaces; a line ending in two spaces or a backslash
// is a hard break (a newline in the text).
fn join_lines(lines []string) string {
	mut out := []u8{cap: 64}
	for k, l in lines {
		mut s := l
		hard := s.ends_with('  ') || (s.ends_with('\\') && !s.ends_with('\\\\'))
		s = if s.ends_with('\\') { s[..s.len - 1] } else { s }
		out << s.trim_space().bytes()
		if k < lines.len - 1 {
			out << if hard { `\n` } else { ` ` }
		}
	}
	return out.bytestr()
}

fn (mut p Parser) open(kind Kind, lvl int, num int, line string, at int) {
	p.flush()
	p.kind, p.lvl, p.num, p.task, p.start = kind, lvl, num, 0, at
	p.para << line
}

pub fn parse(src string) []Block {
	lines := src.replace('\r\n', '\n').split('\n')
	mut p := Parser{}
	mut i := front_matter(lines, mut p)
	for i < lines.len {
		at := i
		line := lines[i]
		t := line.trim_space()
		lead := line[..line.len - line.trim_left(' \t').len]
		indent := lead.len + lead.count('\t') * 3 // a tab counts as 4
		i++
		if t.starts_with('```') || t.starts_with('~~~') {
			p.flush()
			fence := t[..3]
			mut body := []string{}
			for i < lines.len && !lines[i].trim_space().starts_with(fence) {
				body << lines[i]
				i++
			}
			i++ // the closing fence
			lang := t[3..].trim_space().to_lower()
			p.out << Block{
				kind: if lang == 'math' { Kind.math } else { Kind.code }
				text: body.join('\n')
				lang: lang
				line: at
			}
			continue
		}
		if t.starts_with('$$') {
			p.flush()
			i = display_math(lines, at, mut p)
			continue
		}
		if t == '' {
			p.flush()
			continue
		}
		if t.starts_with('#') {
			n := t.len - t.trim_left('#').len
			if n <= 6 && (t.len == n || t[n] == ` `) {
				p.open(.heading, n, 0, t[n..].trim_right('#'), at)
				p.flush()
				continue
			}
		}
		if is_rule(t) {
			p.flush()
			p.out << Block{
				kind: .rule
				line: at
			}
			continue
		}
		if t.starts_with('|') && i < lines.len && is_table_rule(lines[i]) {
			p.flush()
			align := table_cells(lines[i]).map(cell_align(it))
			mut rows := [table_cells(t)]
			i++
			for i < lines.len && lines[i].trim_space().starts_with('|') {
				rows << table_cells(lines[i])
				i++
			}
			p.out << Block{
				kind:  .table
				rows:  rows
				align: align
				line:  at
			}
			continue
		}
		if t.starts_with('>') {
			depth, rest := quote_parts(t)
			if p.kind == .quote && p.lvl == depth && p.para.len > 0 {
				p.para << rest
			} else {
				p.open(.quote, depth, 0, rest, at)
			}
			continue
		}
		if kind, num, rest := list_item(t) {
			task, text := task_parts(rest)
			p.open(kind, indent / 2, num, text, at)
			p.task = task
			continue
		}
		if p.para.len > 0 && (p.kind == .paragraph || indent > 0) {
			p.para << line.trim_left(' \t') // a continuation line (trailing spaces kept)
		} else {
			p.open(.paragraph, 0, 0, line.trim_left(' \t'), at)
		}
	}
	p.flush()
	return p.out
}

// front_matter takes a leading `---` ... `---` block; it returns the line after it.
fn front_matter(lines []string, mut p Parser) int {
	if lines.len < 2 || lines[0].trim_space() != '---' {
		return 0
	}
	for k in 1 .. lines.len {
		if lines[k].trim_space() in ['---', '...'] {
			p.out << Block{
				kind: .front
				text: lines[1..k].join('\n')
			}
			return k + 1
		}
	}
	return 0
}

// display_math reads `$$ ... $$` starting at line at (one line or several); it returns the
// line after it.
fn display_math(lines []string, at int, mut p Parser) int {
	first := lines[at].trim_space()[2..]
	if first.trim_space().ends_with('$$') {
		p.out << Block{
			kind: .math
			text: first.trim_space().trim_string_right('$$').trim_space()
			line: at
		}
		return at + 1
	}
	mut body := [first]
	mut i := at + 1
	for i < lines.len {
		t := lines[i].trim_space()
		if t.ends_with('$$') {
			body << t.trim_string_right('$$')
			i++
			break
		}
		body << lines[i]
		i++
	}
	p.out << Block{
		kind: .math
		text: body.join('\n').trim_space()
		line: at
	}
	return i
}
