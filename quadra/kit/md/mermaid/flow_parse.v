// Flowchart source: nodes with shapes, links with strokes, heads and labels, chains,
// `&` groups and subgraphs. Styling lines (classDef, style, click, linkStyle) are ignored.
module mermaid

pub enum Shape {
	rect       // id[text]
	round      // id(text)
	stadium    // id([text])
	circle     // id((text))
	diamond    // id{text}
	hexagon    // id{{text}}
	subroutine // id[[text]]
	cylinder   // id[(text)]
	flag       // id>text]
}

enum Stroke {
	solid
	dotted
	thick
}

struct FNode {
mut:
	id    string
	label string
	shape Shape
	sub   int = -1 // innermost subgraph, -1 = none
}

struct FEdge {
	from   int
	to     int
	label  string
	stroke Stroke
	head   Head // at `to`
	tail   Head // at `from` (`<-->`)
	minlen int = 1
}

struct Sub {
	title  string
	parent int = -1
}

struct Flow {
mut:
	dir   string = 'TD'
	nodes []FNode
	edges []FEdge
	subs  []Sub
	ids   map[string]int
}

struct Scan {
	s string
mut:
	i int
}

const ignored_flow = ['classDef', 'class', 'style', 'click', 'linkStyle', 'direction']

// parse_flow parses the lines after the `graph` / `flowchart` header (first is its index).
fn parse_flow(dir string, lines []string, first int) !Flow {
	mut f := Flow{
		dir: if dir in ['TD', 'TB', 'BT', 'LR', 'RL'] { dir } else { 'TD' }
	}
	mut stack := []int{}
	for n, raw in lines {
		for stmt in raw.split(';') {
			t := stmt.trim_space()
			word := t.all_before(' ')
			if t == '' || t.starts_with('%%') || word in ignored_flow {
				continue
			}
			if word == 'subgraph' {
				f.subs << sub_of(t[8..].trim_space(), if stack.len > 0 { stack.last() } else { -1 })
				stack << f.subs.len - 1
				continue
			}
			if t == 'end' {
				if stack.len > 0 {
					stack.pop()
				}
				continue
			}
			mut sc := Scan{
				s: t
			}
			f.statement(mut sc, if stack.len > 0 { stack.last() } else { -1 }) or {
				return error('line ${first + n + 1}: ${err.msg()}')
			}
		}
	}
	check_size(&f)!
	return f
}

fn sub_of(rest string, parent int) Sub {
	if rest.contains('[') && rest.ends_with(']') {
		return Sub{decode(rest.all_after('[').all_before_last(']')), parent}
	}
	return Sub{decode(rest), parent}
}

// statement reads `group (link group)*`, where a group is `node (& node)*`.
fn (mut f Flow) statement(mut sc Scan, sub int) ! {
	mut cur := f.group(mut sc, sub)!
	for {
		sc.ws()
		if sc.i >= sc.s.len {
			return
		}
		l := sc.link() or { return error('expected a link at `${sc.s[sc.i..]}`') }
		next := f.group(mut sc, sub)!
		f.add_edges(cur, next, l)!
		cur = next.clone()
	}
}

fn (mut f Flow) group(mut sc Scan, sub int) ![]int {
	mut out := [f.node(mut sc, sub)!]
	for {
		sc.ws()
		if sc.i < sc.s.len && sc.s[sc.i] == `&` {
			sc.i++
			v := f.node(mut sc, sub)!
			if v !in out { // `b & b` is one node, not two parallel links
				out << v
			}
			continue
		}
		return out
	}
	return out
}

fn is_id(c u8) bool {
	return c.is_letter() || c.is_digit() || c == `_` || c >= 0x80
}

fn (mut sc Scan) ws() {
	for sc.i < sc.s.len && (sc.s[sc.i] == ` ` || sc.s[sc.i] == `\t`) {
		sc.i++
	}
}

const openers = [['[[', ']]'], ['[(', ')]'], ['((', '))'], ['([', '])'],
	['{{', '}}'], ['[', ']'], ['(', ')'], ['{', '}'], ['>', ']']]

const opener_shapes = [Shape.subroutine, .cylinder, .circle, .stadium, .hexagon, .rect, .round,
	.diamond, .flag]

fn (mut f Flow) node(mut sc Scan, sub int) !int {
	sc.ws()
	start := sc.i
	for sc.i < sc.s.len && is_id(sc.s[sc.i]) {
		sc.i++
	}
	if sc.i == start {
		return error('expected a node at `${sc.s[start..]}`')
	}
	id := sc.s[start..sc.i]
	mut idx := f.ids[id] or {
		f.nodes << FNode{
			id:    id
			label: id
		}
		f.ids[id] = f.nodes.len - 1
		f.nodes.len - 1
	}
	rest := sc.s[sc.i..]
	for k, o in openers {
		if !rest.starts_with(o[0]) {
			continue
		}
		body := rest[o[0].len..]
		// Quoted text may hold the closing characters.
		end := if body.starts_with('"') {
			q := body.index_after('"', 1) or { return error('unclosed quote') }
			body.index_after(o[1], q) or { -1 }
		} else {
			body.index(o[1]) or { -1 }
		}
		if end < 0 {
			return error('`${o[0]}` without `${o[1]}`')
		}
		mut label := decode(body[..end])
		if opener_shapes[k] == .rect && label.len >= 2 && label[0] in [`/`, `\\`] {
			label = label[1..label.len - 1] // parallelograms and trapezoids draw as rects
		}
		f.nodes[idx].label = label
		f.nodes[idx].shape = opener_shapes[k]
		sc.i += o[0].len + end + o[1].len
		break
	}
	if sc.s[sc.i..].starts_with(':::') { // a class: skip its name
		sc.i += 3
		for sc.i < sc.s.len && (is_id(sc.s[sc.i]) || sc.s[sc.i] == `-`) {
			sc.i++
		}
	}
	if sub >= 0 && f.nodes[idx].sub < 0 {
		f.nodes[idx].sub = sub
	}
	return idx
}

struct Link {
	label  string
	stroke Stroke
	head   Head
	tail   Head
	minlen int
}

// body reads a run of `-`, `=`, `.` and the head after it.
fn (mut sc Scan) body() (string, Head) {
	start := sc.i
	for sc.i < sc.s.len && sc.s[sc.i] in [`-`, `=`, `.`] {
		sc.i++
	}
	b := sc.s[start..sc.i]
	if b.len == 0 || sc.i >= sc.s.len {
		return b, Head.none
	}
	c := sc.s[sc.i]
	after := if sc.i + 1 < sc.s.len { sc.s[sc.i + 1] } else { ` ` }
	if c == `>` {
		sc.i++
		return b, Head.arrow
	}
	if c in [`o`, `x`] && !is_id(after) {
		sc.i++
		return b, if c == `o` {
			Head.circle
		} else {
			Head.cross
		}
	}
	return b, Head.none
}

fn (mut sc Scan) link() ?Link {
	mut tail := Head.none
	if sc.s[sc.i] == `<` {
		tail = .arrow
		sc.i++
	}
	mut b, mut head := sc.body()
	if b.len < 2 {
		return none
	}
	mut label := ''
	mut mb := b // the part that gives the length
	if head == .none && b in ['--', '==', '-.'] && sc.i < sc.s.len && sc.s[sc.i] == ` ` {
		// `-- text -->`: the label runs to the closing part of the link.
		rest := sc.s[sc.i..]
		mut j := -1
		for c in ['-->', '---', '.->', '.-', '==>', '===', '--o', '--x'] {
			if k := rest.index(c) {
				if j < 0 || k < j {
					j = k
				}
			}
		}
		if j < 0 {
			return none
		}
		label = decode(rest[..j])
		sc.i += j
		b2, h2 := sc.body()
		b += b2 // (one assignment: V 0.5.2 miscompiles `+` in a multiple assignment)
		head, mb = h2, b2
	}
	sc.ws()
	if sc.i < sc.s.len && sc.s[sc.i] == `|` {
		close := sc.s.index_after_('|', sc.i + 1)
		if close < 0 {
			return none
		}
		label = decode(sc.s[sc.i + 1..close])
		sc.i = close + 1
	}
	stroke := if b.contains('.') {
		Stroke.dotted
	} else if b.contains('=') {
		Stroke.thick
	} else {
		Stroke.solid
	}
	minlen := if stroke == .dotted {
		mb.count('.')
	} else {
		mb.len - if head == .none { 2 } else { 1 }
	}
	return Link{label, stroke, head, tail, link_len(minlen)}
}
