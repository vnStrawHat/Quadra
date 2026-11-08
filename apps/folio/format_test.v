// Formatting commands on a bare document: marks, links, line prefixes, blocks, undo.
module main

import quadra.kit.code

// fmt runs cmd on text with the selection a..e ("|" marks in the result show it).
fn fmt(text string, a int, e int, cmd Cmd) string {
	mut d := code.new_doc(text)
	d.set_cursor(a, e, false)
	format_doc(mut d, cmd)
	c := d.primary()
	s := d.text()
	if c.empty() {
		return s[..c.head] + '|' + s[c.head..]
	}
	return s[..c.start()] + '[' + s[c.start()..c.end()] + ']' + s[c.end()..]
}

fn test_marks_wrap_and_unwrap() {
	assert fmt('say hello now', 4, 9, .bold) == 'say **[hello]** now'
	assert fmt('say **hello** now', 6, 11, .bold) == 'say [hello] now' // marks outside
	assert fmt('say **hello** now', 4, 13, .bold) == 'say [hello] now' // marks inside
	assert fmt('say hello now', 6, 6, .italic) == 'say *[hello]* now' // the word at the caret
	assert fmt('a  ', 3, 3, .code) == 'a  `|`' // nowhere to wrap: an empty pair
	assert fmt('a ``', 3, 3, .code) == 'a |' // and again takes it away
	assert fmt('say hello ', 4, 10, .strike) == 'say ~~[hello]~~ ' // trailing space left out
	// Italic and bold tell their stars apart.
	assert fmt('**b**', 2, 3, .italic) == '***[b]***'
	assert fmt('***b***', 3, 4, .italic) == '**[b]**'
	assert fmt('***b***', 3, 4, .bold) == '*[b]*'
	assert fmt('**a** and **b**', 0, 15, .bold) == '**[**a** and **b**]**'
}

fn test_links_and_images() {
	assert fmt('see docs', 4, 8, .link) == 'see [docs]([https://])' // the URL to type over
	assert fmt('x', 1, 1, .link) == 'x[[text]](https://)'
	assert fmt('https://a.io', 0, 12, .link) == '[|](https://a.io)'
	assert fmt('', 0, 0, .image) == '![[alt text]](image.png)'
}

fn test_headings_lists_quotes() {
	assert fmt('Title', 2, 2, .h2) == '## Ti|tle'
	assert fmt('## Title', 5, 5, .h2) == 'Ti|tle' // toggles off
	assert fmt('## Title', 5, 5, .h1) == '# Ti|tle'
	assert fmt('- item', 3, 3, .h3) == '### i|tem' // the list marker goes
	assert fmt('# T', 3, 3, .para) == 'T|'
	assert fmt('a\nb\n\nc', 0, 6, .numbered) == '[1. a\n2. b\n\n3. c]'
	assert fmt('1. a\n2. b', 0, 9, .numbered) == '[a\nb]'
	assert fmt('- a\n  - b', 0, 9, .task) == '[- [ ] a\n  - [ ] b]' // indentation kept
	assert fmt('- [ ] a', 7, 7, .bullet) == '- a|'
	assert fmt('a\n\nb', 0, 4, .quote) == '[> a\n>\n> b]'
	assert fmt('> a\n>\n> b', 0, 9, .quote) == '[a\n\nb]'
	assert fmt('', 0, 0, .bullet) == '- |'
	assert fmt('a\nb\n', 0, 4, .bullet) == '[- a\n- b]\n' // the line after the selection stays
}

fn test_blocks() {
	assert fmt('text', 4, 4, .rule) == 'text\n\n---|'
	assert fmt('', 0, 0, .code_block) == '```\n|\n```'
	assert fmt('a\nb', 0, 3, .code_block) == '```|\na\nb\n```'
	assert fmt('p\n\nq', 2, 2, .table) == 'p\n\n| [Column 1] | Column 2 |\n| --- | --- |\n|  |  |\n\nq'
	assert fmt('', 0, 0, .diagram) == '```mermaid\ngraph TD\n    A[[Start]] --> B[End]\n```'
	assert fmt('', 0, 0, .math_block) == '$$\n|\n$$'
}

fn test_each_command_is_one_undo_step() {
	mut d := code.new_doc('word')
	d.set_cursor(0, 4, false)
	format_doc(mut d, .bold)
	format_doc(mut d, .h1)
	assert d.text() == '# **word**'
	d.undo()
	assert d.text() == '**word**'
	d.undo()
	assert d.text() == 'word'
}
