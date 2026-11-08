module code

import rand

fn check_lines(b &Buffer, model string) {
	assert b.text() == model
	assert b.len() == model.len
	lines := model.split('\n')
	assert b.line_count() == lines.len
	mut off := 0
	for n, l in lines {
		assert b.line_start(n) == off
		assert b.line(n) == l
		assert b.line_of(off) == n
		if l.len > 0 {
			assert b.line_of(off + l.len - 1) == n
		}
		off += l.len + 1
	}
}

fn test_random_edits_match_a_string_model() {
	rand.seed([u32(7), 11])
	alphabet := ['a', 'b', ' ', '\n', 'xy\nz', 'ế', '\n\n', 'tiếng việt']
	mut model := 'hello\nworld\n'
	mut b := new_buffer(model)
	for step in 0 .. 3000 {
		if rand.intn(3) or { 0 } == 0 && model.len > 0 {
			a := rand.intn(model.len) or { 0 }
			e := a + (rand.intn(model.len - a + 1) or { 0 })
			b.delete(a, e)
			model = model[..a] + model[e..]
		} else {
			off := rand.intn(model.len + 1) or { 0 }
			s := alphabet[rand.intn(alphabet.len) or { 0 }]
			b.insert(off, s)
			model = model[..off] + s + model[off..]
		}
		if step % 50 == 0 {
			check_lines(b, model)
			a := rand.intn(model.len + 1) or { 0 }
			e := a + (rand.intn(model.len - a + 1) or { 0 })
			assert b.slice(a, e) == model[a..e]
		}
	}
	check_lines(b, model)
}

fn test_typing_stays_one_piece_and_crlf_round_trips() {
	mut b := new_buffer('ab')
	for i, c in 'hello'.split('') {
		b.insert(1 + i, c)
	}
	assert b.text() == 'ahellob'
	assert b.pieces.len == 3 // a, hello, b
	mut w := new_buffer('one\r\ntwo\r\n')
	assert w.crlf && w.line_count() == 3 && w.line(1) == 'two'
	w.insert(w.line_end(0), '!')
	assert w.file_text() == 'one!\r\ntwo\r\n'
	assert new_buffer('').line_count() == 1
	assert w.byte_at(3) == `!` && w.byte_at(100) == 0
}
