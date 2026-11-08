module md

fn test_lines_tasks_math_front_matter() {
	src := '---\ntitle: x\n---\n# A\n\n- [ ] open\n- [x] done\n\n$$\n\\frac{a}{b}\n$$\n\n$$ x^2 $$\n```math\ny\n```\n> > deep\n> top\n'
	b := parse(src)
	assert b.map(it.kind) == [Kind.front, .heading, .bullet, .bullet, .math, .math, .math, .quote,
		.quote]
	assert b[0].text == 'title: x'
	assert b[1].line == 3
	assert b[2].task == 1 && b[2].text == 'open' && b[2].line == 5
	assert b[3].task == 2 && b[3].text == 'done'
	assert b[4].text == '\\frac{a}{b}' && b[4].line == 8
	assert b[5].text == 'x^2' && b[5].line == 12
	assert b[6].text == 'y'
	assert b[7].level == 2 && b[7].text == 'deep'
	assert b[8].level == 1
}

fn test_tables_align_and_hard_breaks() {
	b := parse('| a | b | c |\n|:--|:-:|--:|\n| 1 | 2 | 3 |\n\nline one  \nline two\\\nthree\nfour')
	assert b[0].align == [Align.left, .center, .right]
	assert b[1].text == 'line one\nline two\nthree four'
	assert b[1].line == 4
}

fn test_tab_indented_lists_nest() {
	b := parse('- a\n\t- b\n\t\t- c')
	assert b.map(it.level) == [0, 2, 4]
}

fn test_new_inlines() {
	s :=
		parse_inline('~~old~~ new \$x^2\$ and \$5 or \$10 ![logo](img/a.png "t") see https://x.io/a, ok')
	assert s[0].strike && s[0].text == 'old'
	assert !s[1].strike
	math := s.filter(it.math)
	assert math.len == 1 && math[0].text == 'x^2'
	assert s.any(it.text.contains('\$5 or \$10'))
	img := s.filter(it.image != '')
	assert img.len == 1 && img[0].image == 'img/a.png' && img[0].text == 'logo'
	url := s.filter(it.link != '')
	assert url.len == 1 && url[0].link == 'https://x.io/a'
	nested := parse_inline('[a [b] c](u)')
	assert nested.len == 1 && nested[0].text == 'a [b] c' && nested[0].link == 'u'
}
