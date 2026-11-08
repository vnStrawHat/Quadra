// Outline: a document's headings, with their source lines and anchors (specs/21).
module md

pub struct Heading {
pub:
	level  int
	text   string // without inline markup
	line   int    // source line
	block  int    // index in the parsed blocks
	anchor string // as hosts make them: `## Hello, World` -> `hello-world`
}

// outline lists the headings of blocks in order.
pub fn outline(blocks []Block) []Heading {
	mut out := []Heading{}
	for k, b in blocks {
		if b.kind == .heading {
			text := plain(b.text)
			out << Heading{b.level, text, b.line, k, anchor(text)}
		}
	}
	return out
}

// plain is inline Markdown without its markup (math keeps its source).
pub fn plain(s string) string {
	return parse_inline(s).map(it.text).join('')
}

// anchor lowercases text, turns spaces into dashes and drops punctuation.
pub fn anchor(text string) string {
	mut out := []u8{cap: text.len}
	for c in text.trim_space().to_lower().bytes() {
		if c == ` ` || c == `-` {
			out << `-`
		} else if c.is_letter() || c.is_digit() || c == `_` || c >= 0x80 {
			out << c
		}
	}
	return out.bytestr()
}

// block_at is the index of the last block starting at or before source line (the block
// the line is in, or the one before a blank line); 0 for an empty list.
pub fn block_at(blocks []Block, line int) int {
	mut lo, mut hi := 0, blocks.len - 1
	mut found := 0
	for lo <= hi {
		m := (lo + hi) / 2
		if blocks[m].line <= line {
			found = m
			lo = m + 1
		} else {
			hi = m - 1
		}
	}
	return found
}

// words counts words in src (runs of letters and digits), for the reading time.
pub fn words(src string) int {
	mut n := 0
	mut in_word := false
	for c in src.bytes() {
		w := c.is_letter() || c.is_digit() || c >= 0x80
		if w && !in_word {
			n++
		}
		in_word = w
	}
	return n
}
