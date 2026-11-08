module code

fn kinds(line string, state int) ([]TokenKind, int) {
	toks, next := MarkdownTokenizer{}.tokenize(line, state)
	return toks.map(it.kind), next
}

fn test_markdown_lines() {
	mut k, mut st := kinds('## Title', 0)
	assert k == [TokenKind.tag] && st == 0
	k, st = kinds('```v', 0)
	assert k == [TokenKind.punctuation, .typ] && st == 1
	k, st = kinds('fn main() {}', 1)
	assert k == [TokenKind.str] && st == 1
	k, st = kinds('```', 1)
	assert k == [TokenKind.punctuation] && st == 0
	k, st = kinds('$$', 0)
	assert st == 3
	k, st = kinds('x^2 $$', 3)
	assert k == [TokenKind.escape] && st == 0
	k, _ = kinds('- [x] **done** and *maybe* `code` [a](http://b) \$y\$', 0)
	assert k == [TokenKind.operator, .operator, .number, .keyword, .str, .function, .comment, .escape]
	k, _ = kinds('> quoted snake_case', 0)
	assert k == [TokenKind.comment]
	k, _ = kinds('---', 0)
	assert k == [TokenKind.punctuation]
}
