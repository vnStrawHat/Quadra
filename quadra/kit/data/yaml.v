// A YAML subset parser (specs/19, Value viewer): block mappings and sequences by
// indentation, flow collections on one line, quoted and plain scalars, block scalars,
// comments and the first document. Anchors, aliases and tags are kept as text or skipped.
module data

struct YLine {
mut:
	indent int
	text   string // without the indentation and the comment
	raw    string // the whole line (block scalars)
	blank  bool
}

struct YParser {
mut:
	lines []YLine
	i     int
}

pub fn parse_yaml(src string) !Value {
	mut p := YParser{}
	for raw in src.replace('\r\n', '\n').split('\n') {
		t := strip_comment(raw).trim_right(' \t')
		trimmed := t.trim_left(' ')
		if trimmed == '---' || trimmed.starts_with('--- ') || trimmed == '...' {
			if p.lines.any(!it.blank) {
				break // only the first document
			}
			continue
		}
		p.lines << YLine{t.len - trimmed.len, trimmed, raw, trimmed == ''}
	}
	p.skip_blank()
	if p.i >= p.lines.len {
		return scalar(.null, 'null')
	}
	return p.block(p.lines[p.i].indent)!
}

fn (mut p YParser) skip_blank() {
	for p.i < p.lines.len && p.lines[p.i].blank {
		p.i++
	}
}

fn (mut p YParser) peek() ?YLine {
	p.skip_blank()
	return if p.i < p.lines.len { p.lines[p.i] } else { none }
}

fn is_item(t string) bool {
	return t == '-' || t.starts_with('- ')
}

fn (mut p YParser) block(_ int) !Value {
	l := p.peek() or { return scalar(.null, 'null') }
	if is_item(l.text) {
		return p.sequence(l.indent)
	}
	if _, _ := split_key(l.text) {
		return p.mapping(l.indent)
	}
	p.i++
	return inline(l.text)
}

fn (mut p YParser) mapping(indent int) !Value {
	mut keys := []string{}
	mut items := []Value{}
	for {
		l := p.peek() or { break }
		if l.indent != indent || is_item(l.text) {
			if l.indent > indent {
				return error('line ${p.i + 1}: unexpected indentation')
			}
			break
		}
		key, rest0 := split_key(l.text) or { return error('line ${p.i + 1}: expected a key') }
		p.i++
		rest := skip_props(rest0)
		keys << key
		items << p.value_after(indent, rest)!
	}
	return Value{
		kind:  .map
		keys:  keys
		items: items
	}
}

// value_after reads what follows "key:" (or "- "): inline text, a block scalar, or a
// nested block on the next lines.
fn (mut p YParser) value_after(indent int, rest string) !Value {
	if rest.starts_with('|') || rest.starts_with('>') {
		return p.block_scalar(indent, rest[0] == `>`)
	}
	if rest != '' {
		return inline(rest)
	}
	next := p.peek() or { return scalar(.null, 'null') }
	if next.indent > indent || (is_item(next.text) && next.indent == indent) {
		return p.block(next.indent)
	}
	return scalar(.null, 'null')
}

fn (mut p YParser) sequence(indent int) !Value {
	mut items := []Value{}
	for {
		l := p.peek() or { break }
		if l.indent != indent || !is_item(l.text) {
			break
		}
		item := skip_props(l.text[1..].trim_left(' '))
		if item == '' {
			p.i++
			items << p.value_after(indent, '')!
		} else if _, _ := split_key(item) {
			// "- key: value" starts a mapping at the item's column.
			col := indent + l.text.len - l.text[1..].trim_left(' ').len
			p.lines[p.i] = YLine{col, item, l.raw, false}
			items << p.mapping(col)!
		} else {
			p.i++
			items << inline(item)
		}
	}
	return Value{
		kind:  .list
		items: items
	}
}

fn (mut p YParser) block_scalar(indent int, folded bool) Value {
	mut out := []string{}
	mut content := -1
	for p.i < p.lines.len {
		l := p.lines[p.i]
		if !l.blank && l.indent <= indent {
			break
		}
		if !l.blank && content < 0 {
			content = l.indent
		}
		out << if l.blank || content < 0 { '' } else { l.raw[content..] }
		p.i++
	}
	for out.len > 0 && out.last() == '' {
		out.pop()
	}
	return scalar(.str, out.join(if folded { ' ' } else { '\n' }))
}

// split_key splits "key: rest" (quoted keys too); none when the text is not a key.
fn split_key(t string) ?(string, string) {
	if t.len == 0 || t[0] == `{` || t[0] == `[` || is_item(t) {
		return none
	}
	if t[0] == `"` || t[0] == `'` {
		e := t.index_after(t[0..1], 1) or { return none }
		after := t[e + 1..].trim_left(' ')
		if !after.starts_with(':') {
			return none
		}
		return t[1..e], after[1..].trim_space()
	}
	mut k := t.index(': ') or { -1 }
	if k < 0 && t.ends_with(':') {
		k = t.len - 1
	}
	if k <= 0 {
		return none
	}
	return t[..k].trim_space(), t[k + 1..].trim_space()
}

// skip_props drops a leading anchor (&a) or tag (!t) before a value.
fn skip_props(s string) string {
	mut r := s
	for r.starts_with('&') || r.starts_with('!') {
		sp := r.index(' ') or { return '' }
		r = r[sp + 1..].trim_left(' ')
	}
	return r
}

// strip_comment removes a # comment that is not inside quotes.
fn strip_comment(s string) string {
	mut q := u8(0)
	for i, c in s {
		if q != 0 {
			if c == q {
				q = 0
			}
		} else if c == `'` || c == `"` {
			q = c
		} else if c == `#` && (i == 0 || s[i - 1] == ` ` || s[i - 1] == `\t`) {
			return s[..i]
		}
	}
	return s
}
