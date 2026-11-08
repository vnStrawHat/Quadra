// style_spans: ANSI-colored text outside a terminal (log views). The escape sequences are
// stripped and SGR runs through the terminal's own SGR code, so colors match the terminal.
module term

pub struct Span {
pub:
	start int // byte range in the stripped text
	end   int
	pen   Cell // fg, bg and flags; ch is unused
}

// style_spans returns the text without escape sequences and its styled spans (spans with
// the default pen are left out). Non-SGR CSI, OSC and other escapes are dropped.
pub fn style_spans(s string) (string, []Span) {
	if !s.contains_u8(0x1b) {
		return s, []Span{}
	}
	mut out := []u8{cap: s.len}
	mut spans := []Span{}
	mut pen := Cell{}
	mut from := 0
	mut i := 0
	for i < s.len {
		c := s[i]
		if c != 0x1b {
			out << c
			i++
			continue
		}
		next := if i + 1 < s.len { s[i + 1] } else { u8(0) }
		if next == `[` {
			mut p := Parser{}
			mut j := i + 2
			mut final := u8(0)
			for j < s.len {
				b := s[j]
				if b >= `0` && b <= `9` {
					if p.np == 0 {
						p.np = 1
					}
					if p.np <= p.params.len {
						p.params[p.np - 1] = p.params[p.np - 1] * 10 + int(b - `0`)
					}
				} else if b == `;` || b == `:` {
					if p.np == 0 {
						p.np = 1
					}
					if p.np < p.params.len {
						if b == `:` {
							p.sub |= u32(1) << p.np
						}
						p.np++
					}
				} else if b >= 0x40 && b <= 0x7e {
					final = b
					j++
					break
				}
				j++
			}
			if final == `m` {
				if out.len > from && pen != Cell{} {
					spans << Span{from, out.len, pen}
				}
				p.sgr(mut pen)
				from = out.len
			}
			i = j
		} else if next == `]` { // OSC: up to BEL or ST
			mut j := i + 2
			for j < s.len && s[j] != 7 && !(s[j] == 0x1b && j + 1 < s.len && s[j + 1] == `\\`) {
				j++
			}
			i = if j < s.len && s[j] == 7 { j + 1 } else { j + 2 }
		} else {
			i += 2
		}
	}
	if out.len > from && pen != Cell{} {
		spans << Span{from, out.len, pen}
	}
	return out.bytestr(), spans
}
