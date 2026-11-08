// Line classifiers for the block parser.
module md

// list_item recognizes "- x", "* x", "+ x", "1. x" and "1) x"; it returns the kind, the
// number and the text after the marker.
fn list_item(t string) ?(Kind, int, string) {
	if t.len >= 2 && t[0] in [`-`, `*`, `+`] && t[1] == ` ` {
		return Kind.bullet, 0, t[2..]
	}
	mut i := 0
	for i < t.len && t[i].is_digit() {
		i++
	}
	if i > 0 && i < 10 && i + 1 < t.len && t[i] in [`.`, `)`] && t[i + 1] == ` ` {
		return Kind.ordered, t[..i].int(), t[i + 2..]
	}
	return none
}

// task_parts splits "[ ] text" or "[x] text" into the task state (1 open, 2 done) and the
// text; other items are 0.
fn task_parts(s string) (int, string) {
	if s.len >= 3 && s[0] == `[` && s[2] == `]` && (s.len == 3 || s[3] == ` `) {
		match s[1] {
			` ` { return 1, s[3..].trim_left(' ') }
			`x`, `X` { return 2, s[3..].trim_left(' ') }
			else {}
		}
	}
	return 0, s
}

fn is_rule(t string) bool {
	s := t.replace(' ', '')
	return s.len >= 3 && (s.count('-') == s.len || s.count('*') == s.len || s.count('_') == s.len)
}

fn table_cells(t string) []string {
	mut s := t.trim_space()
	s = s.trim_left('|').trim_right('|')
	return s.split('|').map(it.trim_space())
}

fn is_table_rule(t string) bool {
	s := t.trim_space()
	return s.len > 2 && s.contains('-') && s.trim('|:- ').len == 0
}

fn cell_align(c string) Align {
	l, r := c.starts_with(':'), c.ends_with(':')
	return if l && r {
		.center
	} else if r {
		.right
	} else if l {
		.left
	} else {
		.none
	}
}

// quote_parts counts the `>` markers of a quote line and returns the depth and the text.
fn quote_parts(t string) (int, string) {
	mut depth := 0
	mut i := 0
	for i < t.len && (t[i] == `>` || t[i] == ` `) {
		if t[i] == `>` {
			depth++
		}
		i++
	}
	return depth, t[i..]
}
