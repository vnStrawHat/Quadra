// Renders formulas on the GPU (WARP) and saves .tmp/out/tex.png for a look.
module tex

import os
import quadra.core
import quadra.prim
import quadra.kit
import quadra.testing
import quadra.ui

const samples = [
	'\\int_0^1 \\frac{x^2}{\\sqrt{1+x}}\\,dx = \\sum_{n=0}^{\\infty} \\binom{n}{k}',
	'A = \\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}, \\quad |x| = \\begin{cases} x & x \\ge 0 \\\\ -x & x < 0 \\end{cases}',
	'e^{i\\pi} + 1 = 0, \\quad \\lim_{n \\to \\infty} \\left(1 + \\frac{1}{n}\\right)^n = e',
	"\\mathbb{R}^n, \\mathcal{L}(f), \\hat{x}, \\vec{v}, \\overline{AB}, \\sqrt[3]{x^3+y^3}, x_i^2, f'(x)",
	'\\begin{aligned} f(x) &= (a+b)^2 \\\\ &= a^2 + 2ab + b^2 \\end{aligned}',
]

pub struct Sheet {}

@[markused]
pub fn (mut v Sheet) render(mut cx core.ViewCx[Sheet]) core.Element {
	mut col :=
		ui.div().flex_col().gap(18).p(16).size_full().bg(prim.white).text_color(prim.black).text_size(20)
	for i, s in samples {
		col.child(formula(s, i != 3))
	}
	col.child(formula(r'\frac{a}', true)) // an error: the source in the danger color
	col.child(ui.div().flex_row().items_center().gap(6).child(ui.text('inline:')).child(formula('a^2+b^2=c^2',
		false)).child(ui.text('and')).child(formula('\\frac{1}{2}', false)))
	return col
}

fn test_formulas_render() {
	mut h := testing.new_pixel_harness()!
	kit.install(mut h.app)!
	h.open(Sheet{}, width: 760, height: 620)!
	img := h.rgba()!
	os.mkdir_all(os.join_path(os.dir(@FILE), '..', '..', '..', '..', '.tmp', 'out'))!
	h.save_png(os.join_path(os.dir(@FILE), '..', '..', '..', '..', '.tmp', 'out', 'tex.png'))!
	mut dark, mut red := 0, 0
	for i := 0; i < img.len; i += 4 {
		if img[i] < 100 {
			dark++
		}
		if img[i] > 150 && img[i + 1] < 100 && img[i + 2] < 100 {
			red++
		}
	}
	assert red > 20
	assert dark > 3000
}
