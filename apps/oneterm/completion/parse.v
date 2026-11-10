// Input-line parsing: the command head, the token under the cursor and the tokens left of it.
module completion

import strings

struct Token {
	text  string
	start int
}

// tokenize honors single and double quotes; quote chars are stripped from the text, and
// `start` is the byte offset of the token's first char (the opening quote if quoted).
fn tokenize(input string) []Token {
	mut tokens := []Token{}
	mut cur := strings.new_builder(32)
	mut start := 0
	mut in_token := false
	mut quote := rune(0)
	mut i := 0
	for i < input.len {
		ch, w := decode_rune(input, i)
		if quote != 0 {
			if ch == quote {
				quote = 0
			} else {
				cur.write_rune(ch)
			}
		} else if ch == `"` || ch == `'` {
			if !in_token {
				in_token = true
				start = i
			}
			quote = ch
		} else if is_ws(ch) {
			if in_token {
				tokens << Token{cur.str(), start}
				in_token = false
			}
		} else {
			if !in_token {
				in_token = true
				start = i
			}
			cur.write_rune(ch)
		}
		i += w
	}
	if in_token {
		tokens << Token{cur.str(), start}
	}
	return tokens
}

// ParsedLine is the prompt line relative to the cursor.
pub struct ParsedLine {
pub:
	head           ?string  // the command name (first token), if any
	token          string   // the token under the cursor (empty after trailing whitespace)
	token_start    int      // byte offset of `token`
	is_first_token bool     // the cursor is editing the command name itself
	prior_tokens   []string // the tokens left of the cursor token, including head
}

// parse_line parses `line` given the cursor byte offset.
pub fn parse_line(line string, cursor_col int) ParsedLine {
	cursor := if cursor_col > line.len { line.len } else { cursor_col }
	prefix := line[..cursor]
	tokens := tokenize(prefix)
	mut trailing_ws := prefix.len == 0
	if !trailing_ws {
		// The last rune decides: walk back to its first byte.
		mut i := prefix.len - 1
		for i > 0 && (prefix[i] & 0xC0) == 0x80 {
			i--
		}
		r, _ := decode_rune(prefix, i)
		trailing_ws = is_ws(r)
	}
	if trailing_ws {
		prior := tokens.map(it.text)
		return ParsedLine{
			head:           if prior.len > 0 { prior[0] } else { none }
			token:          ''
			token_start:    cursor
			is_first_token: prior.len == 0
			prior_tokens:   prior
		}
	}
	mut last := Token{'', cursor}
	if tokens.len > 0 {
		last = tokens.last()
	}
	mut prior := []string{}
	for k in 0 .. tokens.len - 1 {
		prior << tokens[k].text
	}
	return ParsedLine{
		head:           if prior.len > 0 { prior[0] } else { none }
		token:          last.text
		token_start:    last.start
		is_first_token: tokens.len <= 1
		prior_tokens:   prior
	}
}
