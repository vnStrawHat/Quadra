module mermaid

import quadra.text
import quadra.text.fake

fn seq(src string) Seq {
	d := diagram(src)
	assert d.error() == none, d.error() or { '' }
	assert d.kind() == 'sequence'
	return d.seq
}

fn test_participants_and_messages() {
	s :=
		seq('sequenceDiagram\nparticipant A as Alice\nactor B as Bob\nA->>B: hi\nB-->>A: back\nA->B: plain\nA-->B: dashed\nA-xB: lost\nA-)C: async\nA->>+B: activate\nactivate B')
	assert s.actors.map(it.label) == ['Alice', 'Bob', 'C']
	assert s.actors[1].actor
	m := s.items
	assert m.len == 7
	assert m[0].head == .arrow && !m[0].dashed && m[0].text == 'hi'
	assert m[1].head == .arrow && m[1].dashed && m[1].from == 1 && m[1].to == 0
	assert m[2].head == .none && !m[2].dashed
	assert m[3].head == .none && m[3].dashed
	assert m[4].head == .cross
	assert m[5].head == .open && m[5].to == 2
	assert m[6].to == 1 && m[6].text == 'activate'
}

fn test_notes_and_frames() {
	s :=
		seq('sequenceDiagram\nautonumber\nNote left of A: l\nNote right of A: r\nNote over B,A: both\nloop every s\nA->>B: x\nalt ok\nB->>A: y\nelse bad\nB->>A: z\nend\nend\nopt maybe\nA->>A: self\nend')
	assert s.autonumber
	notes := s.items.filter(it.kind == .note)
	assert notes.map(it.side) == ['left', 'right', 'over']
	assert notes[2].from == 0 && notes[2].to == 1 // ordered left to right
	kinds := s.items.map(it.kind)
	assert kinds.filter(it == .open).len == 3
	assert kinds.filter(it == .close).len == 3
	assert kinds.filter(it == .branch).len == 1
	assert s.items.filter(it.kind == .open).map(it.side) == ['loop', 'alt', 'opt']
}

fn test_unclosed_frames_close_and_bad_lines_report() {
	s := seq('sequenceDiagram\nloop forever\nA->>B: x')
	assert s.items.last().kind == .close
	assert diagram('sequenceDiagram\nA->>B: ok\nthis is not a message').error()?.contains('line 3')
}

fn test_sequence_layout() {
	s :=
		seq('sequenceDiagram\nparticipant A\nparticipant B\nparticipant C\nA->>C: a rather long message label that needs room\nloop again\nB->>C: inner\nend\nNote over A,B: n')
	mut m := Measure{
		ts:   text.new_system(fake.new())
		base: text.TextStyle{
			size: 14
		}
	}
	d := layout_seq(&s, mut m)
	// Participant boxes: three on top, three at the bottom, in column order.
	boxes := d.ops.filter(it.kind == .box && it.bg == .panel && it.radius == 4)
	assert boxes.len == 6
	heads := boxes.filter(it.rect.y == boxes[0].rect.y)
	assert heads.len == 3
	assert heads[0].rect.x < heads[1].rect.x && heads[1].rect.x < heads[2].rect.x
	// The long label fits between A and C.
	label := d.ops.filter(it.kind == .text && it.text.starts_with('a rather'))[0]
	ca := heads[0].rect.x + heads[0].rect.width / 2
	cc := heads[2].rect.x + heads[2].rect.width / 2
	assert cc - ca > label.rect.width
	// The loop frame wraps its message line.
	frame := d.ops.filter(it.kind == .box && it.bg == .frame)[0]
	line := d.ops.filter(it.kind == .poly && it.ink == .accent)[1]
	for q in line.pts {
		assert q.x >= frame.rect.x && q.x <= frame.rect.right()
		assert q.y >= frame.rect.y && q.y <= frame.rect.bottom()
	}
}
