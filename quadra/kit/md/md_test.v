module md

fn test_blocks() {
	src := '# Title\n\nSome *text*\ncontinued.\n\n- one\n- two\n  - nested\n1. first\n\n```v\nfn main() {}\n```\n> quoted\n> more\n\n---\n| A | B |\n|---|:-:|\n| 1 | 2 |\n'
	b := parse(src)
	assert b.map(it.kind) == [Kind.heading, .paragraph, .bullet, .bullet, .bullet, .ordered, .code,
		.quote, .rule, .table]
	assert b[0].level == 1
	assert b[0].text == 'Title'
	assert b[1].text == 'Some *text* continued.'
	assert b[4].level == 1
	assert b[4].text == 'nested'
	assert b[5].num == 1
	assert b[6].lang == 'v'
	assert b[6].text == 'fn main() {}'
	assert b[7].text == 'quoted more'
	assert b[9].rows == [['A', 'B'], ['1', '2']]
}

fn test_inlines() {
	s := parse_inline('a **b *c*** `d*e` [f **g**](http://x) snake_case_name <https://y>')
	assert s.map(it.text) == ['a ', 'b ', 'c', ' ', 'd*e', ' ', 'f ', 'g', ' snake_case_name ',
		'https://y']
	assert s[1].bold && !s[1].italic
	assert s[2].bold && s[2].italic
	assert s[4].code
	assert s[6].link == 'http://x'
	assert s[7].bold
	assert s[7].link == 'http://x'
	assert s[8].link == ''
	assert s[9].link == 'https://y'
	assert parse_inline('2 * 3 * 4').map(it.text) == ['2 * 3 * 4']
	assert parse_inline('\\*not\\*').map(it.text) == ['*not*']
}
