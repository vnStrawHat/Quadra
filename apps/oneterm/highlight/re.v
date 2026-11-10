// A minimal backtracking regular-expression matcher for the prompt and structural patterns.
// V's `regex` module cannot express them (specs/24): `|` acts on single tokens and there is
// no `\b`. Alternation is leftmost-first, quantifiers are greedy, as in the original engine.
module highlight

import encoding.utf8

// ponytail: a pathological attempt gives up after this many steps (the original's DFA is
// linear); a per-pattern DFA if a real prompt ever hits it.
const re_step_budget = 20000

enum ReOp as u8 {
	ch
	cls
	split
	jmp
	bol
	eol
	wb
	accept
}

// ReInst: `r` is the literal (.ch) or class index (.cls); x/y are relative jump offsets.
struct ReInst {
	op ReOp
	r  rune
	x  int
	y  int
}

struct ReClass {
mut:
	ascii   [128]bool
	negated bool
	space   bool
	digit   bool
	other   []rune
}

fn (c &ReClass) test(r rune) bool {
	if r < 128 {
		return c.ascii[r] != c.negated
	}
	found := (c.space && is_ws_rune(r)) || (c.digit && utf8.is_number(r)) || r in c.other
	return found != c.negated
}

// Re is a compiled pattern.
struct Re {
mut:
	prog        []ReInst
	classes     []ReClass
	first_ascii [128]bool // chars a match can start with
	first_other bool      // a non-ASCII char can start a match
}

// build_first_set walks the program without consuming: what can a match start with?
fn (mut re Re) build_first_set() {
	mut seen := []bool{len: re.prog.len}
	mut todo := [0]
	for todo.len > 0 {
		pc := todo.pop()
		if seen[pc] {
			continue
		}
		seen[pc] = true
		inst := re.prog[pc]
		match inst.op {
			.split {
				todo << pc + inst.x
				todo << pc + inst.y
			}
			.jmp {
				todo << pc + inst.x
			}
			.bol, .eol, .wb {
				todo << pc + 1
			}
			.ch {
				if inst.r < 128 {
					re.first_ascii[inst.r] = true
				} else {
					re.first_other = true
				}
			}
			.cls {
				cls := re.classes[inst.r]
				for r in 0 .. 128 {
					if cls.test(r) {
						re.first_ascii[r] = true
					}
				}
				if cls.negated || cls.space || cls.digit || cls.other.len > 0 {
					re.first_other = true
				}
			}
			.accept {
				panic('re: a pattern must not match the empty string')
			}
		}
	}
}

// match_at returns the end of a match starting at `start`, or -1.
fn (re &Re) match_at(chars []rune, start int, mut stack []int) int {
	stack.clear()
	n := chars.len
	mut pc := 0
	mut sp := start
	mut steps := 0
	for {
		steps++
		if steps > re_step_budget {
			return -1
		}
		inst := unsafe { &re.prog[pc] }
		match inst.op {
			.ch {
				if sp < n && chars[sp] == inst.r {
					pc++
					sp++
					continue
				}
			}
			.cls {
				if sp < n && re.classes[inst.r].test(chars[sp]) {
					pc++
					sp++
					continue
				}
			}
			.split {
				stack << pc + inst.y
				stack << sp
				pc += inst.x
				continue
			}
			.jmp {
				pc += inst.x
				continue
			}
			.bol {
				if sp == 0 {
					pc++
					continue
				}
			}
			.eol {
				if sp == n {
					pc++
					continue
				}
			}
			.wb {
				before := sp > 0 && is_re_word(chars[sp - 1])
				after := sp < n && is_re_word(chars[sp])
				if before != after {
					pc++
					continue
				}
			}
			.accept {
				return sp
			}
		}

		if stack.len == 0 {
			return -1
		}
		sp = stack.pop()
		pc = stack.pop()
	}
	return -1
}

// find returns the leftmost match at or after `from` as (start, end), (-1, -1) when none.
fn (re &Re) find(chars []rune, from int, mut stack []int) (int, int) {
	for s in from .. chars.len {
		c := chars[s]
		if (c < 128 && !re.first_ascii[c]) || (c >= 128 && !re.first_other) {
			continue
		}
		end := re.match_at(chars, s, mut stack)
		if end >= 0 {
			return s, end
		}
	}
	return -1, -1
}
