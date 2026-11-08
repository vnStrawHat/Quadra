module code

// spans renders the tokens of one line as 'text:kind' items.
fn spans(t Tokenizer, line string, state int) (string, int) {
	toks, st := t.tokenize(line, state)
	return toks.map('${line[it.start..it.end]}:${it.kind}').join(' '), st
}

fn test_v_tokens() {
	v := VTokenizer{}
	mut s, mut st := spans(v, 'pub fn (mut d Doc) move(m Move) int {', 0)
	assert s == 'pub:keyword fn:keyword (:punctuation mut:keyword Doc:typ ):punctuation move:function (:punctuation Move:typ ):punctuation int:typ {:punctuation'
	assert st == 0
	s, _ = spans(v, "x := 'a\\n\${b.c}' // done", 0)
	assert s == ":=:operator 'a:str \\n\${:escape b.c:variable }:escape ':str // done:comment"
	s, _ = spans(v, 'if a.len > 0x1F { return none }', 0)
	assert s == 'if:control .:punctuation len:property >:operator 0x1F:number {:punctuation return:control none:constant }:punctuation'
	s, _ = spans(v, "@[heap] r'\\d' `a`", 0)
	assert s == "@[heap]:attribute r'\\d':str `a`:str"
	// Nested block comments and strings span lines.
	s, st = spans(v, 'a /* x /* y */', 0)
	assert s == '/* x /* y */:comment' && st == 1
	s, st = spans(v, 'still */ b()', st)
	assert s == 'still */:comment b:function ():punctuation' && st == 0
	s, st = spans(v, "s := 'one", 0)
	assert st > 0
	s, st = spans(v, "two' + 1", st)
	assert s == "two':str +:operator 1:number" && st == 0
}

fn test_yaml_tokens() {
	y := YamlTokenizer{}
	mut s, mut st := spans(y, 'apiVersion: apps/v1 # api', 0)
	assert s == 'apiVersion:property ::punctuation apps/v1:str # api:comment'
	s, _ = spans(y, '  - name: "web:1"', 0)
	assert s == '-:punctuation name:property ::punctuation "web:1":str'
	s, _ = spans(y, 'replicas: 3', 0)
	assert s == 'replicas:property ::punctuation 3:number'
	s, _ = spans(y, 'on: [true, null, 1.5e3]', 0)
	assert s == 'on:property ::punctuation [:punctuation true:constant ,:punctuation null:constant ,:punctuation 1.5e3:number ]:punctuation'
	s, _ = spans(y, 'base: &b !!map {a: *x}', 0)
	assert s == 'base:property ::punctuation &b:attribute !!map:tag {:punctuation a:property ::punctuation *x:attribute }:punctuation'
	s, _ = spans(y, '--- # doc', 0)
	assert s == '---:keyword # doc:comment'
	s, _ = spans(y, 'url: http://x:80', 0)
	assert s == 'url:property ::punctuation http://x:80:str'
	// A block scalar lasts while lines are blank or deeper than its key.
	s, st = spans(y, '  - run: |', 0)
	assert st > 0
	s, st = spans(y, '      echo # not a comment', st)
	assert s == 'echo # not a comment:str' && st > 0
	_, st = spans(y, '', st)
	assert st > 0
	s, st = spans(y, '    next: 1', st)
	assert s == 'next:property ::punctuation 1:number' && st == 0
}

fn test_highlighter_is_lazy() {
	mut lines := []string{len: 10000, init: 'x := ${index}'}
	mut d := new_doc(lines.join('\n'))
	mut h := new_highlighter(VTokenizer{})
	assert h.tokens(mut d, 50).len == 2
	assert h.known() == 51 // only up to the asked line
	// Opening a comment at line 10 changes line 50's start state.
	d.set_cursor(d.buf.line_start(10), d.buf.line_start(10), false)
	d.insert_text('/* ', 'type')
	toks := h.tokens(mut d, 50)
	assert toks.len == 1 && toks[0].kind == .comment
	assert h.known() == 51
	// An edit trims the cached states to the edited line.
	d.set_cursor(d.buf.line_start(20), d.buf.line_start(20), false)
	d.insert_text('*/', 'type')
	h.state_before(mut d, 0)
	assert h.known() == 20
	assert h.tokens(mut d, 50).len == 2
	// Undo marks the lines it touches too.
	d.undo()
	assert h.tokens(mut d, 50)[0].kind == .comment
}

fn test_tokenizer_for() {
	assert tokenizer_for('a/b.V') is VTokenizer
	assert tokenizer_for('deploy.yml') is YamlTokenizer
	assert tokenizer_for('README') is PlainTokenizer
}
