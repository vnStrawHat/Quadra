// Row roles from OSC 133 shell-integration marks; `.unmarked` means the shell said nothing,
// and only such a row may run the prompt detector.
module highlight

pub enum RowRole {
	unmarked
	output
	prompt
	command
}

// RowRoles holds the role of every display row of one viewport.
pub struct RowRoles {
pub mut:
	role []RowRole
}

// reset clears to `rows` unmarked entries, keeping the allocation.
pub fn (mut r RowRoles) reset(rows int) {
	r.role.clear()
	for _ in 0 .. rows {
		r.role << .unmarked
	}
}

// role_at returns a row's role; `.unmarked` when out of range.
pub fn (r &RowRoles) role_at(row int) RowRole {
	return if row >= 0 && row < r.role.len { r.role[row] } else { RowRole.unmarked }
}

// is_empty tells whether no row carries a mark.
pub fn (r &RowRoles) is_empty() bool {
	for x in r.role {
		if x != .unmarked {
			return false
		}
	}
	return true
}

// PromptRun is a run of rows of one prompt: start inclusive, end exclusive.
pub struct PromptRun {
pub:
	start int
	end   int
}

// last_completed_prompt returns the rows of the prompt whose command has finished: the last
// prompt run that has another prompt run below it. `wraps` holds each row's soft-wrap flag; it
// separates two adjacent prompts from one wrapped prompt.
pub fn (r &RowRoles) last_completed_prompt(wraps []bool) ?PromptRun {
	mut has_last := false
	mut last := PromptRun{}
	mut has_before_last := false
	mut before_last := PromptRun{}
	mut row := 0
	for row < r.role.len {
		if r.role_at(row) != .prompt {
			row++
			continue
		}
		start := row
		for row < wraps.len && wraps[row] && r.role_at(row + 1) == .prompt {
			row++
		}
		row++
		if has_last {
			before_last = last
			has_before_last = true
		}
		last = PromptRun{start, row}
		has_last = true
	}
	if !has_before_last {
		return none
	}
	return before_last
}
