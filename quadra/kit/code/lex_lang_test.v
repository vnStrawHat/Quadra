module code

fn C.GC_get_total_bytes() usize

// lang_spans tokenizes lines in order and renders the tokens as 'text:kind' items, one
// string per line.
fn lang_spans(lang string, lines ...string) []string {
	t := tokenizer_for(lang)
	mut st := 0
	mut out := []string{}
	for line in lines {
		toks, next := t.tokenize(line, st)
		st = next
		out << toks.map('${line[it.start..it.end]}:${it.kind}').join(' ')
	}
	return out
}

fn has(s string, items ...string) bool {
	for it in items {
		if !s.contains(it) {
			eprintln('missing ${it} in: ${s}')
			return false
		}
	}
	return true
}

fn test_c_family() {
	s := lang_spans('c', '#include <stdio.h>', 'static int f(void) { return 0x1F; } // x', '/* a',
		'b */ char *s = "a\\n";')
	assert s[0] == '#include:keyword <stdio.h>:str'
	assert has(s[1], 'static:keyword', 'int:typ', 'f:function', 'return:control', '0x1F:number',
		'// x:comment')
	assert s[2] == '/* a:comment'
	assert has(s[3], 'b */:comment', '"a:str', '\\n:escape')
	j := lang_spans('java', '@Override public String name() { return null; }')
	assert has(j[0], '@Override:attribute', 'public:keyword', 'String:typ', 'null:constant')
	k := lang_spans('kotlin', 'val s = "hi \$name \${a.b}"')
	assert has(k[0], 'val:keyword', '\$name:variable', '\${:escape', 'a.b:variable')
	sw := lang_spans('swift', 'let s = "v \\(x)" /* a /* b */ c */ + 1')
	assert has(sw[0], 'let:keyword', '\\(:escape', 'x:variable', '/* a /* b */ c */:comment',
		'1:number')
	g := lang_spans('go', 'func main() { s := `raw', 'still` }')
	assert has(g[0], 'func:keyword', 'main:function', '`raw:str')
	assert has(g[1], 'still`:str', '}:punctuation')
}

fn test_rust() {
	s := lang_spans('rs',
		'fn f<\'a>(x: &\'a str) -> char { let r = r#"q"#; println!("{}"); \'x\' }',
		'#[derive(Debug)] let n = 1_000.5e-3;')
	assert has(s[0], 'fn:keyword', "'a:attribute", 'str:typ', 'r#"q"#:str', 'println!:function',
		"'x':str")
	assert has(s[1], '#[derive(Debug)]:attribute', 'let:keyword', '1_000.5e-3:number')
}

fn test_js_ts() {
	s := lang_spans('javascript', 'const a = `x \${b}', 'y`; if (a) { return f(1) } // c')
	assert has(s[0], 'const:keyword', '`x :str', '\${:escape', 'b:variable')
	assert has(s[1], 'y`:str', 'if:control', 'return:control', 'f:function', '// c:comment')
	t := lang_spans('ts', 'interface A { n: number; s: string }')
	assert has(t[0], 'interface:keyword', 'A:typ', 'number:typ')
}

fn test_scripts() {
	p := lang_spans('python', '@dataclass', 'def f(x: int) -> None:  # c', 's = f"a" + """doc',
		'end""" + 1.5')
	assert p[0] == '@dataclass:attribute'
	assert has(p[1], 'def:keyword', 'f:function', 'int:typ', 'None:constant', '# c:comment')
	assert has(p[2], 'f"a":str', '"""doc:str')
	assert has(p[3], 'end""":str', '1.5:number')
	r := lang_spans('ruby', 'def hi; puts "a #{@x}" end')
	assert has(r[0], 'def:keyword', '#{:escape', '@x:variable', 'end:keyword')
	l := lang_spans('lua', 'local x = [[a', 'b]] --[[ c', 'd ]] -- e')
	assert has(l[0], 'local:keyword', '[[a:str')
	assert has(l[1], 'b]]:str', '--[[ c:comment')
	assert l[2] == 'd ]]:comment -- e:comment'
	php := lang_spans('php', '<?php echo "v \$x"; // c')
	assert has(php[0], 'php:keyword', 'echo:keyword', '\$x:variable', '// c:comment')
}

fn test_shells() {
	s := lang_spans('bash', 'if [ -f "\$HOME/x" ]; then echo \'a\' # c', 'a#b X=1 \${y}')
	assert has(s[0], 'if:control', '\$HOME:variable', 'then:control', 'echo:function', "'a':str",
		'# c:comment')
	assert !s[1].contains('comment')
	ps := lang_spans('pwsh', 'Get-Item -Path \$p -eq 1 <# a', 'b #>')
	assert has(ps[0], 'Get-Item:function', '-Path:attribute', '\$p:variable', '-eq:keyword',
		'<# a:comment')
	assert ps[1] == 'b #>:comment'
	b := lang_spans('bat', '@echo off', 'REM note', ':loop', 'set X=%PATH%')
	assert has(b[0], 'echo:keyword', 'off:keyword')
	assert b[1] == 'REM note:comment'
	assert b[2] == ':loop:function'
	assert has(b[3], 'set:keyword', '%PATH%:variable')
}

fn test_sql_and_build_files() {
	s := lang_spans('SQL', "SELECT name, COUNT(*) FROM t WHERE x = 'a' -- c")
	assert has(s[0], 'SELECT:keyword', 'COUNT:function', 'FROM:keyword', "'a':str", '-- c:comment')
	m := lang_spans('Makefile', 'CC := gcc', 'all: \$(OBJ)', '\t\$(CC) -o \$@')
	assert has(m[0], 'CC:property')
	assert has(m[1], 'all:function', '\$(OBJ):variable')
	assert has(m[2], '\$(CC):variable', '\$@:variable')
	d := lang_spans('dockerfile', 'FROM alpine AS base', 'RUN echo \$HOME # c')
	assert has(d[0], 'FROM:keyword', 'AS:keyword')
	assert has(d[1], 'RUN:keyword', '\$HOME:variable', '# c:comment')
}

fn test_data_formats() {
	j := lang_spans('json', '{"a": [1, true, null], "b": "x\\"y"}')
	assert has(j[0], '"a":property', '1:number', 'true:constant', 'null:constant', '"b":property',
		'\\":escape')
	t := lang_spans('toml', '[tool.x]', 'name = "q" # c', 'n = 0x1f')
	assert t[0] == '[tool.x]:tag'
	assert has(t[1], 'name:property', '"q":str', '# c:comment')
	assert has(t[2], 'n:property', '0x1f:number')
	i := lang_spans('ini', '; c', '[core]', 'editor = vim')
	assert i[0] == '; c:comment' && i[1] == '[core]:tag'
	assert has(i[2], 'editor:property')
	c := lang_spans('scss', '.btn:hover, #id {', '  max-width: 10px; color: #fff; // c', '}',
		'@media (x) {')
	assert has(c[0], 'btn:typ', ':hover:keyword', '#id:constant')
	assert has(c[1], 'max-width:property', '10px:number', '#fff:number', '// c:comment')
	assert has(c[3], '@media:keyword')
}

fn test_markup_and_diff() {
	h := lang_spans('html', '<!DOCTYPE html>', '<a href="x" data-id=1>T &amp; U</a> <!-- c',
		'd --> <img', '  src="y" />')
	assert has(h[0], '<!:punctuation', 'DOCTYPE:keyword')
	assert has(h[1], '<:punctuation', 'a:tag', 'href:attribute', '"x":str', 'data-id:attribute',
		'&amp;:escape', '</:punctuation', '<!-- c:comment')
	assert has(h[2], 'd -->:comment', 'img:tag')
	assert has(h[3], 'src:attribute', '"y":str', '/>:punctuation')
	d := lang_spans('patch', '--- a/x', '@@ -1 +1 @@', '-old', '+new', ' same')
	assert d == ['--- a/x:keyword', '@@ -1 +1 @@:escape', '-old:variable', '+new:str', '']
}

fn test_aliases() {
	for name in ['x.py', 'Python', 'python title=x', 'py {.cls}', 'src/A.RS', 'rust', 'C++', 'cpp',
		'h', 'cs', 'csharp', 'x.jsx', 'js {.x}', 'mjs', 'TSX', 'sh', 'zsh', 'ps1', 'cmd',
		'Dockerfile', 'GNUmakefile', 'make', 'jsonc', 'conf', 'svg', 'xml', 'diff', 'kt', 'swift',
		'dart', 'rb', 'lua', 'sql', 'toml', 'css', 'go', 'java', 'php'] {
		assert tokenizer_for(name) is LangTokenizer, name
	}
	assert tokenizer_for('my file.py') is LangTokenizer
	assert tokenizer_for('yaml') is YamlTokenizer
	assert tokenizer_for('md') is MarkdownTokenizer
	assert tokenizer_for('v') is VTokenizer
	assert tokenizer_for('brainfuck') is PlainTokenizer
	assert tokenizer_for('') is PlainTokenizer
}

// Tokenizing allocates the token array per line and nothing per word (specs/18).
fn test_tokenize_allocates_per_line_only() {
	t := tokenizer_for('js')
	line := 'export async function load(url, opts) { return fetch(url, { method: "GET", retries: 3 }); }'
	t.tokenize(line, 0)
	b0 := C.GC_get_total_bytes()
	for _ in 0 .. 1000 {
		t.tokenize(line, 0)
	}
	per := f64(C.GC_get_total_bytes() - b0) / 1000
	assert per < 512, 'bytes per line: ${per}' // one token array of line.len / 3 + 2
}
