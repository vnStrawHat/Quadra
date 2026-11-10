// Fuzz the Markdown block and inline parsers, the outline helpers and fence highlighting
// (specs/11, fuzz tests): untrusted documents must never panic, hang or grow the heap.
module md

import strings
import quadra.kit
import quadra.testing.fuzz

const md_corpus = [
	'# Title\n\nSome *text*\ncontinued.\n\n- one\n- two\n  - nested\n1. first\n',
	'```v\nfn main() {}\n```\n> quoted\n> more\n\n---\n| A | B |\n|---|:-:|\n| 1 | 2 |\n',
	'---\ntitle: x\n---\n## H2 ##\n- [ ] open\n- [x] done\n3) third\n',
	'a **b *c*** `d*e` [f **g**](http://x) snake_case_name <https://y> ~~s~~ ![img](a.png "t")',
	'$$\n\\frac{a}{b}\n$$\ninline $x^2$ and \\$5 and $$ y $$\n```math\n\\sqrt{2}\n```\n',
	'```mermaid\ngraph TD\nA-->B\n```\n<!-- comment\nmore -->\n<b>html</b> &amp; text\n',
	'>>> deep\n> > spaced\n\t- tab item\n    code?\n***\n___\n| x |\n|:--|\n',
	'```python\ndef f(x):\n    return "s" # c\n```\n~~~js\nlet a = `t${1}`; /* c */\n~~~\n',
	'[link](<http://a b>) [ref][1] [^note] http://bare.example/x?y=1. *a _b* c_ **unclosed',
	'Xin chào **thế giới** 漢字 😀 é\r\n\r\n1. a\r\n2. b\r\n',
]

const langs = ['v', 'c', 'cpp', 'rust', 'go', 'python', 'js', 'ts', 'json', 'html', 'css', 'sh',
	'sql', 'lua', 'ruby', 'java', 'yaml', 'md', 'toml', 'xml', 'diff', 'php', 'zig', 'mermaid',
	'math', '', 'x{y}', 'a/b.rs']

// deep builds generated worst cases: nesting, unclosed constructs and huge tables.
fn deep(mut r fuzz.Rng) string {
	n := 200 + r.intn(3000)
	match r.intn(8) {
		0 {
			return strings.repeat(`>`, n) + ' q'
		}
		1 {
			mut s := []string{}
			for k in 0 .. n / 10 {
				s << ' '.repeat(k * 2) + '- item'
			}
			return s.join('\n')
		}
		2 {
			return strings.repeat(`[`, n) + 'x' + strings.repeat(`]`, r.intn(n)) + '(y)'
		}
		3 {
			return strings.repeat(`*`, n) + 'a' + strings.repeat(`_`, n)
		}
		4 {
			return '```\n' + 'unclosed\n'.repeat(n / 10)
		}
		5 {
			return '$$\n' + 'x+'.repeat(n)
		}
		6 {
			cols := 1 + r.intn(200)
			head := '|' + 'h|'.repeat(cols)
			rule := '|' + '-|'.repeat(cols)
			return head + '\n' + rule + '\n' + ('|' + 'c|'.repeat(cols) + '\n').repeat(n / 20)
		}
		else {
			return '`'.repeat(n) + '$'.repeat(n) + '<!--'.repeat(n / 4)
		}
	}
}

fn check_doc(src string, t &kit.Theme) {
	blocks := parse(src)
	for b in blocks {
		match b.kind {
			.code {
				_ = code_spans(b.text, b.lang, t)
			}
			.paragraph, .heading, .bullet, .ordered, .quote {
				_ = parse_inline(b.text)
			}
			.table {
				for row in b.rows {
					for c in row {
						_ = parse_inline(c)
					}
				}
			}
			else {}
		}
	}
	_ = outline(blocks)
	_ = block_at(blocks, src.count('\n') / 2)
	_ = words(src)
	_ = anchor(plain(src))
}

fn test_fuzz_markdown() {
	t := kit.dark()
	corpus := md_corpus.map(it.bytes())
	for c in md_corpus {
		check_doc(c, &t)
	}
	n := fuzz.iters(3000)
	mut r := fuzz.new_rng(0x6d64)
	mut w := fuzz.watch('markdown', n)
	for i in 0 .. n {
		mut src := ''
		if r.chance(10) {
			src = deep(mut r)
		} else {
			src = r.mutate(corpus[r.intn(corpus.len)], corpus).bytestr()
		}
		w.start()
		check_doc(src, &t)
		w.done(i, src.len, 2000)
	}
	w.finish(64)
}

fn test_fuzz_fence_languages() {
	t := kit.dark()
	corpus := md_corpus.map(it.bytes())
	n := fuzz.iters(1500)
	mut r := fuzz.new_rng(0x6c616e67)
	mut w := fuzz.watch('fences', n)
	for i in 0 .. n {
		body := r.mutate(corpus[r.intn(corpus.len)], corpus).bytestr()
		w.start()
		_ = code_spans(body, r.pick(langs), &t)
		w.done(i, body.len, 2000)
	}
	w.finish(64)
}
