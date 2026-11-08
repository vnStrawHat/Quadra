// Parsing of operator limits, delimiters and environments.
module tex

// limits reads \limits or \nolimits after an operator.
fn (mut p Parser) limits(default int) int {
	t := p.peek()
	if t.kind == .cmd && t.text == 'limits' {
		p.pos++
		return 1
	}
	if t.kind == .cmd && t.text == 'nolimits' {
		p.pos++
		return -1
	}
	return default
}

// delimiter reads the token after \left, \right or \big.
fn (mut p Parser) delimiter() string {
	t := p.next()
	key := if t.kind == .cmd { '\\' + t.text } else { t.text }
	if d := delimiters[key] {
		return d
	}
	p.fail('bad delimiter ${key}')
	return ''
}

fn (mut p Parser) left_right() Node {
	open := p.delimiter()
	body := p.list(.eof)
	t := p.next()
	if t.kind != .cmd || t.text != 'right' {
		p.fail('\\left without \\right')
		return Node{
			kind: .delim
			text: open
			body: body
			cls:  .inner
		}
	}
	return Node{
		kind:  .delim
		text:  open
		text2: p.delimiter()
		body:  body
		cls:   .inner
	}
}

// environment parses \begin{env} cells & ... \\ rows \end{env}.
fn (mut p Parser) environment() Node {
	env := p.raw()
	if env !in environments {
		p.fail('unknown environment ${env}')
	}
	if env == 'array' {
		p.raw() // the column spec: every column is centered
	}
	mut rows := [][][]Node{}
	mut row := [][]Node{}
	for {
		row << p.list(.eof)
		t := p.next()
		if t.kind == .amp {
			continue
		}
		rows << row
		row = [][]Node{}
		if t.kind == .cmd && t.text == '\\' {
			p.opt_arg() or {} // \\[4pt]
			continue
		}
		if t.kind == .cmd && t.text == 'end' {
			if p.raw() != env {
				p.fail('\\end does not match \\begin{${env}}')
			}
		} else {
			p.fail('missing \\end{${env}}')
		}
		break
	}
	// A trailing \\ leaves an empty last row.
	if rows.len > 1 && rows.last().len == 1 && rows.last()[0].len == 0 {
		rows.pop()
	}
	return Node{
		kind: .matrix
		env:  env
		rows: rows
		cls:  .inner
	}
}
