// LineCollector: the escape stripper. Printable runs in, sequences dropped, one record per line.
// Our own small state machine: quadra.kit.term.Parser feeds a Screen.
module termlog

enum ScanState {
	ground
	escape
	csi
	string_seq
}

struct LineCollector {
mut:
	current   []rune
	completed []string
	state     ScanState
	st_escape bool // inside a string sequence, the previous byte was ESC
	utf8_buf  []u8
	utf8_need int
}

fn (mut c LineCollector) commit() {
	if c.current.len > 0 {
		c.completed << c.current.string()
		c.current.clear()
	}
}

fn (mut c LineCollector) take_completed() []string {
	out := c.completed
	c.completed = []
	return out
}

fn (mut c LineCollector) execute(b u8) {
	match b {
		`\n`, `\r` {
			c.commit()
		}
		0x08 {
			if c.current.len > 0 {
				c.current.delete_last()
			}
		}
		else {}
	}
}

fn is_control_rune(r rune) bool {
	return r < 0x20 || (r >= 0x7f && r <= 0x9f)
}

fn (mut c LineCollector) print_rune(r rune) {
	if !is_control_rune(r) {
		c.current << r
	}
}

fn (mut c LineCollector) flush_invalid_utf8() {
	if c.utf8_need > 0 {
		c.utf8_buf.clear()
		c.utf8_need = 0
		c.print_rune(0xFFFD)
	}
}

// feed runs one read through the stripper. A UTF-8 sequence or an escape sequence may be split
// across reads.
fn (mut c LineCollector) feed(bytes []u8) {
	for b in bytes {
		match c.state {
			.ground { c.ground_byte(b) }
			.escape { c.escape_byte(b) }
			.csi { c.csi_byte(b) }
			.string_seq { c.string_byte(b) }
		}
	}
}

fn (mut c LineCollector) ground_byte(b u8) {
	if b == 0x1b {
		c.flush_invalid_utf8()
		c.state = .escape
		return
	}
	if b < 0x20 || b == 0x7f {
		c.flush_invalid_utf8()
		c.execute(b)
		return
	}
	if b < 0x80 {
		c.flush_invalid_utf8()
		c.print_rune(rune(b))
		return
	}
	if c.utf8_need > 0 {
		if b & 0xC0 != 0x80 {
			c.flush_invalid_utf8()
		} else {
			c.utf8_buf << b
			c.utf8_need--
			if c.utf8_need == 0 {
				c.print_utf8_sequence()
			}
			return
		}
	}
	need := if b & 0xE0 == 0xC0 {
		1
	} else if b & 0xF0 == 0xE0 {
		2
	} else if b & 0xF8 == 0xF0 {
		3
	} else {
		0
	}
	if need == 0 {
		c.print_rune(0xFFFD)
		return
	}
	c.utf8_buf << b
	c.utf8_need = need
}

fn (mut c LineCollector) print_utf8_sequence() {
	text := c.utf8_buf.bytestr()
	c.utf8_buf.clear()
	runes := text.runes()
	if runes.len == 1 && runes[0] != 0 {
		c.print_rune(runes[0])
	} else {
		c.print_rune(0xFFFD)
	}
}

fn (mut c LineCollector) escape_byte(b u8) {
	if b == 0x18 || b == 0x1a {
		c.state = .ground
	} else if b == 0x1b {
		// A new ESC restarts the sequence.
	} else if b < 0x20 {
		c.execute(b)
	} else if b == `[` {
		c.state = .csi
	} else if b in [`]`, `P`, `_`, `^`, `X`] {
		c.state = .string_seq
		c.st_escape = false
	} else if b >= 0x30 {
		c.state = .ground
	}
	// 0x20..=0x2F: intermediates; stay until the final byte.
}

fn (mut c LineCollector) csi_byte(b u8) {
	if b == 0x18 || b == 0x1a {
		c.state = .ground
	} else if b == 0x1b {
		c.state = .escape
	} else if b < 0x20 {
		c.execute(b)
	} else if b >= 0x40 && b <= 0x7e {
		c.state = .ground
	}
}

fn (mut c LineCollector) string_byte(b u8) {
	if c.st_escape {
		c.st_escape = false
		if b == `\\` {
			c.state = .ground
			return
		}
		c.state = .escape
		c.escape_byte(b)
		return
	}
	if b == 0x18 || b == 0x1a || b == 0x07 {
		c.state = .ground
	} else if b == 0x1b {
		c.st_escape = true
	}
}
