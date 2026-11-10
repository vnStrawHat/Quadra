// Shell integration, OSC 133 (specs/17): marks on rows, zones on cells, the program-status
// protocol (OSC 7501) that shares the prompt lifetime.
module term

pub const mark_a = u8(1) // prompt start
pub const mark_b = u8(2) // prompt end, input starts
pub const mark_c = u8(4) // output starts
pub const mark_d = u8(8) // command done

const mark_exit = u8(16) // RowExtra.exit is set

// RowExtra is the rare data of a row (specs/17): OSC 133 marks and Sixel images.
@[heap]
pub struct RowExtra {
pub mut:
	marks  u8  // mark_a | ...
	exit   int // the D exit code, with mark_exit
	images []ImageRef
}

// ShellMarks is what the marks on one row say.
pub struct ShellMarks {
pub:
	marks u8
	exit  ?int
}

fn (mut r Row) extra_mut() &RowExtra {
	if r.extra == unsafe { nil } {
		r.extra = &RowExtra{}
	}
	return r.extra
}

fn (mut s Screen) osc_shell(parts []string) {
	if parts.len < 2 {
		return
	}
	z, bit := match parts[1] {
		'A' { Zone.prompt, mark_a }
		'B' { Zone.input, mark_b }
		'C' { Zone.output, mark_c }
		'D' { Zone.none, mark_d }
		else { return }
	}

	s.pen.flags = (s.pen.flags & ~f_zone) | zone_bits(z)
	mut e := s.lines[s.y].extra_mut()
	e.marks |= bit
	s.touch(s.y)
	mut ev_exit := ?int(none)
	if bit == mark_a {
		s.prompt_count++
		s.status_prompt()
	} else if bit == mark_d && parts.len >= 3 && parts[2].len > 0 && parts[2].bytes().all(it >= `0`
		&& it <= `9`) {
		e.exit = parts[2].int()
		e.marks |= mark_exit
		s.last_exit = e.exit
		ev_exit = e.exit
	}
	s.emit(TerminalInfo{ kind: .shell_mark, mark: bit, exit: ev_exit })
}

// zone_at is the zone of the cell at p (line counts from the oldest history line).
pub fn (t &Terminal) zone_at(p Pos) Zone {
	row := t.row_at(p.line)
	return if p.col >= 0 && p.col < row.cells.len { row.cells[p.col].zone() } else { Zone.none }
}

// row_zone is the zone of a row: input if any of its cells is, else the first written cell's.
pub fn (t &Terminal) row_zone(line int) Zone {
	mut first := Zone.none
	for c in t.row_at(line).cells {
		z := c.zone()
		if z == .input {
			return .input
		}
		if first == .none && z != .none {
			first = z
		}
	}
	return first
}

// input_start is the first input column of a row, -1 when it has none.
pub fn (t &Terminal) input_start(line int) int {
	for x, c in t.row_at(line).cells {
		if c.zone() == .input {
			return x
		}
	}
	return -1
}

// in_command_input says whether the cursor is in the shell's input region.
pub fn (t &Terminal) in_command_input() bool {
	return !t.screen.alt_active && t.screen.pen.zone() == .input
}

// shell_marks_at returns the marks on a row, with the exit code of a D mark.
pub fn (t &Terminal) shell_marks_at(line int) ShellMarks {
	e := t.row_at(line).extra
	if e == unsafe { nil } {
		return ShellMarks{}
	}
	return ShellMarks{
		marks: e.marks & (mark_a | mark_b | mark_c | mark_d)
		exit:  if e.marks & mark_exit != 0 { e.exit } else { none }
	}
}
