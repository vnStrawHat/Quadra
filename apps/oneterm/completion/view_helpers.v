// UI-free helpers ported from the terminal view.
module completion

// strip_prompt removes a shell prompt prefix from a row (best effort): the first `>`, or
// `$ # ❯ ➜ λ` followed by a space or the end. Returns the command and whether one was found.
pub fn strip_prompt(row string) (string, bool) {
	chars := row.runes()
	for i, ch in chars {
		is_sign := ch == `>`
			|| (ch in [`$`, `#`, `❯`, `➜`, `λ`] && (i + 1 >= chars.len || chars[i + 1] == ` `))
		if is_sign {
			mut j := i + 1
			for j < chars.len && chars[j] == ` ` {
				j++
			}
			return chars[j..].string(), true
		}
	}
	return row.trim_left(' \t\n\r\v\f'), false
}

// visible_window returns the scroll window (offset, count) of a list of `n` rows, keeping
// `selected` (-1 for none) in view.
pub fn visible_window(n int, selected int, max_visible int) (int, int) {
	max_v := if max_visible < 1 { 1 } else { max_visible }
	if n <= max_v {
		return 0, n
	}
	if selected < max_v {
		return 0, max_v
	}
	offset := selected + 1 - max_v
	return if offset > n - max_v { n - max_v } else { offset }, max_v
}
