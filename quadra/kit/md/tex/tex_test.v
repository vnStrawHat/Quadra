module tex

import quadra.text
import quadra.text.fake
import quadra.ui

fn lay() Lay {
	return Lay{
		ts:     text.new_system(fake.new())
		size:   20
		family: 'Fake Sans'
	}
}

fn boxed(src string, st int) Box {
	nodes, err := parse(src)
	assert err == '', err
	mut l := lay()
	return l.list(nodes, st)
}

fn test_metrics() {
	x := boxed('x', 1)
	frac := boxed(r'\frac{x}{y}', 1)
	assert frac.h > x.h && frac.d > 0
	sup := boxed('x^2', 1)
	assert sup.h > x.h && sup.w > x.w
	sub := boxed('x_i', 1)
	assert sub.d > x.d && sub.h == x.h
	root := boxed(r'\sqrt{x}', 1)
	assert root.h > x.h && root.w > x.w
	plain := boxed('(x)', 1)
	grown := boxed(r'\left( \frac{\frac{a}{b}}{c} \right)', 0)
	assert grown.h + grown.d > 2 * (plain.h + plain.d)
	// Display limits sit above and below; inline ones at the side.
	d := boxed(r'\sum_{i=0}^{n} i', 0)
	t := boxed(r'\sum_{i=0}^{n} i', 1)
	assert d.h > t.h && d.w < t.w
	m := boxed(r'\begin{pmatrix} a & b \\ c & d \end{pmatrix}', 1)
	assert m.h + m.d > 2 * (x.h + x.d)
}

fn test_errors() {
	for src, want in {
		r'\frac{a}':                'missing argument'
		r'\foo x':                  r'unknown command \foo'
		'x^2^3':                    'double superscript'
		'{x':                       'missing }'
		'x}':                       'unexpected }'
		r'\left( x':                r'\left without \right'
		r'\begin{foo} x \end{foo}': 'unknown environment foo'
	} {
		_, err := parse(src)
		assert err == want, src
	}
	f := formula(r'\oops', false)
	assert f.error()? == r'unknown command \oops'
}

fn test_every_command_parses() {
	mut srcs := []string{}
	for name, _ in symbols {
		if name.len > 1 || name[0].is_letter() {
			srcs << '\\${name}'
		} else {
			srcs << '\\${name} '
		}
	}
	for name, _ in big_ops {
		srcs << '\\${name}_a^b x'
	}
	for name, _ in functions {
		srcs << '\\${name} x'
	}
	for name, _ in accents {
		srcs << '\\${name}{x}'
	}
	for name, _ in font_commands {
		srcs << '\\${name}{Ab1}'
	}
	for name, _ in spaces {
		srcs << 'a\\${name} b'
	}
	for env in environments {
		spec := if env == 'array' { '{cc}' } else { '' }
		srcs << '\\begin{${env}}${spec} a & b \\\\ c & d \\end{${env}}'
	}
	srcs << [r'\frac{a}{b}', r'\dfrac12', r'\tfrac{1}{2}', r'\binom{n}{k}', r'\sqrt[3]{x}',
		r'\left\{ x \right\}', r'\left\langle x \right.', r'\bigl( x \bigr)', r'\Bigg[ x \Bigg]',
		r'\text{if } x', r'\operatorname{rank} A', r'\overset{!}{=}', r'\underset{x}{\arg\max}',
		r'\pmod{n}', r'\not= \not\in', "f''(x)", r'\displaystyle x \scriptstyle y',
		r'\lim\limits_{x} \int\nolimits_0^1', r'\mathrm{d}x', r'\{ \} \| \% \$ \# \& \_']
	for src in srcs {
		nodes, err := parse(src)
		assert err == '', '${src}: ${err}'
		mut l := lay()
		l.list(nodes, 0)
	}
}

const layout_samples = [
	'\\int_0^1 \\frac{x^2}{\\sqrt{1+x}}\\,dx = \\sum_{n=0}^{\\infty} \\binom{n}{k}',
	'A = \\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}, \\quad |x| = \\begin{cases} x & x \\ge 0 \\\\ -x & x < 0 \\end{cases}',
	'e^{i\\pi} + 1 = 0, \\quad \\lim_{n \\to \\infty} \\left(1 + \\frac{1}{n}\\right)^n = e',
	"\\mathbb{R}^n, \\mathcal{L}(f), \\hat{x}, \\vec{v}, \\overline{AB}, \\sqrt[3]{x^3+y^3}, x_i^2, f'(x)",
	'\\begin{aligned} f(x) &= (a+b)^2 \\\\ &= a^2 + 2ab + b^2 \\end{aligned}',
]

fn test_samples_lay_out() {
	for s in layout_samples {
		b := boxed(s, 0)
		assert b.w > 0
	}
}

fn test_is_an_inline_box() {
	b := ui.InlineBox(formula('x^2', false))
	_ = b
}
