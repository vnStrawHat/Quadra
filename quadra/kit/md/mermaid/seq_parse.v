// Sequence diagram source: participants and actors, messages, notes, frames (loop, alt /
// else, opt, par / and, critical / option, break) and autonumber. Activation, boxes and
// titles are accepted and not drawn.
module mermaid

struct Actor {
	id    string
	label string
	actor bool // drawn as a figure
}

enum SeqKind {
	message
	note
	open   // a frame starts
	branch // else / and / option inside a frame
	close
}

struct SeqItem {
	kind   SeqKind
	from   int // message: sender; note: first participant
	to     int // message: receiver; note: last participant
	text   string
	dashed bool
	head   Head
	side   string // note: 'left', 'right' or 'over'; frame: its keyword
}

struct Seq {
mut:
	actors     []Actor
	ids        map[string]int
	items      []SeqItem
	autonumber bool
}

const seq_arrows = ['-->>', '->>', '--x', '-x', '--)', '-)', '-->', '->']

const frame_words = ['loop', 'alt', 'opt', 'par', 'critical', 'break']

const branch_words = ['else', 'and', 'option']

fn parse_seq(lines []string, first int) !Seq {
	mut s := Seq{}
	mut stack := []bool{} // per open block: true when it is a drawn frame
	for n, raw in lines {
		t := raw.trim_space()
		word := t.all_before(' ').to_lower()
		rest := if t.contains(' ') { t.all_after(' ').trim_space() } else { '' }
		if t == '' || t.starts_with('%%') || word in ['activate', 'deactivate', 'title'] {
			continue
		}
		if word == 'autonumber' {
			s.autonumber = true
		} else if word in ['participant', 'actor'] {
			id := rest.all_before(' as ').trim_space()
			label := if rest.contains(' as ') { rest.all_after(' as ').trim_space() } else { id }
			if id !in s.ids {
				s.ids[id] = s.actors.len
				s.actors << Actor{id, decode(label), word == 'actor'}
			}
		} else if word == 'note' {
			s.note(rest) or { return error('line ${first + n + 1}: ${err.msg()}') }
		} else if word in frame_words {
			s.items << SeqItem{
				kind: .open
				side: word
				text: decode(rest)
			}
			stack << true
		} else if word in ['box', 'rect'] {
			stack << false
		} else if word in branch_words {
			s.items << SeqItem{
				kind: .branch
				side: word
				text: decode(rest)
			}
		} else if word == 'end' {
			if stack.len > 0 && stack.pop() {
				s.items << SeqItem{
					kind: .close
				}
			}
		} else {
			s.message(t) or { return error('line ${first + n + 1}: ${err.msg()}') }
		}
	}
	for stack.len > 0 {
		if stack.pop() {
			s.items << SeqItem{
				kind: .close
			}
		}
	}
	return s
}

fn (mut s Seq) actor(id string) int {
	return s.ids[id] or {
		s.ids[id] = s.actors.len
		s.actors << Actor{id, id, false}
		s.actors.len - 1
	}
}

// message parses `A->>B: text` (activation marks `+` / `-` before B are dropped).
fn (mut s Seq) message(t string) ! {
	mut at, mut arrow := -1, ''
	for a in seq_arrows {
		if k := t.index(a) {
			if at < 0 || k < at || (k == at && a.len > arrow.len) {
				at, arrow = k, a
			}
		}
	}
	if at <= 0 {
		return error('cannot read `${t}`')
	}
	from := t[..at].trim_space()
	rest := t[at + arrow.len..].trim_left('+- ')
	to := rest.all_before(':').trim_space()
	if to == '' {
		return error('a message needs a receiver: `${t}`')
	}
	head := match arrow.trim_left('-') {
		'>>' { Head.arrow }
		'x' { Head.cross }
		')' { Head.open }
		else { Head.none }
	}

	s.items << SeqItem{
		kind:   .message
		from:   s.actor(from)
		to:     s.actor(to)
		text:   if rest.contains(':') { decode(rest.all_after(':')) } else { '' }
		dashed: arrow.starts_with('--')
		head:   head
	}
}

// note parses `left of A: text`, `right of A: text`, `over A: text`, `over A,B: text`.
fn (mut s Seq) note(rest string) ! {
	where := rest.all_before(':').trim_space()
	text := decode(rest.all_after(':'))
	lw := where.to_lower()
	side := if lw.starts_with('left of') {
		'left'
	} else if lw.starts_with('right of') {
		'right'
	} else if lw.starts_with('over') {
		'over'
	} else {
		return error('a note is left of, right of or over participants')
	}
	who := where[if side == 'over' { 4 } else { side.len + 3 }..].split(',').map(it.trim_space())
	a := s.actor(who[0])
	b := if who.len > 1 { s.actor(who[1]) } else { a }
	s.items << SeqItem{
		kind: .note
		from: if a < b { a } else { b }
		to:   if a < b { b } else { a }
		text: text
		side: side
	}
}
