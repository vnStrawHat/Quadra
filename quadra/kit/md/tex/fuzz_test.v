// Fuzz the TeX parser and its box layout (specs/11, fuzz tests): formulas come from untrusted
// Markdown, so any input must lay out without a panic, a hang or a stack overflow.
module tex

import quadra.text
import quadra.text.fake
import quadra.testing.fuzz

const tex_corpus = [
	r'\frac{x}{y} + x^2_i - \sqrt[3]{x} \cdot \sum_{i=0}^{n} i',
	r'\left( \frac{\frac{a}{b}}{c} \right] \big| \Big\{ x \bigg\}',
	r'\begin{pmatrix} a & b \\ c & d \end{pmatrix} \begin{array}{cc} 1 & 2 \\[4pt] 3 & 4 \end{array}',
	r'\begin{cases} x & x > 0 \\ -x & \text{otherwise} \end{cases} \operatorname*{arg\,max}_x f',
	r"f'(x) = \lim\limits_{h \to 0} \frac{f(x+h) - f(x)}{h} \int_0^\infty e^{-x}\,dx f'''",
	r'\mathbb{R} \mathcal{F} \mathbf{v} \hat{a} \vec{b} \overline{xy} \underbrace{abc}_{n}',
	r'\alpha\beta\gamma \le \ge \ne \pm \infty \partial \nabla % comment
	\binom{n}{k} \tfrac12 \dfrac{1}{2} \color{red}{x} \mathrm{d}x',
	r'\begin{aligned} a &= b + c \\ &= d \end{aligned} \middle| \phantom{x} \quad \qquad',
	'αβγ 漢 😀 \\text{Xin chào} é',
]

const deep_openers = ['{', r'\frac{', 'x^{', r'\sqrt{', r'\left(', r'\begin{matrix}', r'\sqrt[',
	r'\text{', r'\overline{', 'a_{']

fn deep(mut r fuzz.Rng) string {
	n := 50 + r.intn(5000)
	if r.chance(30) { // a huge matrix
		cols := 1 + r.intn(60)
		row := 'a & '.repeat(cols) + r'\\ '
		return r'\begin{matrix}' + row.repeat(n / 50) + if r.chance(50) {
			r'\end{matrix}'
		} else {
			''
		}
	}
	open := r.pick(deep_openers)
	mut s := open.repeat(n) + 'x'
	if r.chance(50) {
		s += '}'.repeat(r.intn(n))
	}
	return s
}

fn check_formula(src string, mut l Lay) {
	nodes, err := parse(src)
	if err != '' {
		_ = l.error_box(src)
	}
	_ = l.list(nodes, 0)
	_ = l.list(nodes, 1)
	l.ts.end_frame() // what a frame does: unused layouts leave the cache
}

fn test_fuzz_tex() {
	mut l := Lay{
		ts:     text.new_system(fake.new())
		size:   20
		family: 'Fake Sans'
	}
	corpus := tex_corpus.map(it.bytes())
	for c in tex_corpus {
		check_formula(c, mut l)
	}
	// Regressions: nesting deeper than max_depth fails instead of overflowing the stack, and a
	// long 	ext argument is read in linear time (it was quadratic: seconds for 20 000).
	mut dw := fuzz.watch('tex deep', deep_openers.len)
	for k, open in deep_openers {
		dw.start()
		check_formula(open.repeat(5000), mut l)
		_, err := parse(open.repeat(20_000))
		assert err != '', open
		dw.done(k, open.len * 20_000, 2000)
	}
	dw.finish(64)
	n := fuzz.iters(600)
	mut r := fuzz.new_rng(0x746578)
	mut w := fuzz.watch('tex', n)
	for i in 0 .. n {
		mut src := ''
		if r.chance(10) {
			src = deep(mut r)
		} else {
			src = r.mutate(corpus[r.intn(corpus.len)], corpus).bytestr()
		}
		w.start()
		check_formula(src, mut l)
		w.done(i, src.len, 2000)
	}
	w.finish(64)
}
