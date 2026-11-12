//! Math: formulas in the notation of TeX, laid out as boxes and painted with the text.

use quadra::{Align, Div, column, div, row, text};
use quadra_md::markdown;
use quadra_md::tex::formula;

use crate::common::{demo, page};

const SAMPLES: [&str; 6] = [
    r"\int_0^1 \frac{x^2}{\sqrt{1+x}}\,dx = \sum_{n=0}^{\infty} \binom{n}{k}",
    r"A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}, \quad |x| = \begin{cases} x & x \ge 0 \\ -x & x < 0 \end{cases}",
    r"e^{i\pi} + 1 = 0, \quad \lim_{n \to \infty} \left(1 + \frac{1}{n}\right)^n = e",
    r"\mathbb{R}^n, \mathcal{L}(f), \hat{x}, \vec{v}, \overline{AB}, \sqrt[3]{x^3+y^3}, x_i^2, f'(x)",
    r"\begin{aligned} f(x) &= (a+b)^2 \\ &= a^2 + 2ab + b^2 \end{aligned}",
    r"\left\{ \frac{\frac{a}{b}}{\frac{c}{d}} \right\} \le \Bigl\lfloor \prod_{i=1}^{n} x_i \Bigr\rfloor \cdot \left\| v \right\|",
];

const NOTE: &str = "The roots of a quadratic:

$$
x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}
$$

A formula with a slip in it shows its source:

$$
\\frac{a}
$$
";

pub fn math_page() -> Div {
    let display = SAMPLES
        .iter()
        .fold(column().gap(18.0).text_size(20.0), |all, source| all.child(formula(source).display()));
    let line = row().gap(6.0).align_items(Align::Center);
    let line = line
        .child(text("In a line:"))
        .child(formula("a^2+b^2=c^2"))
        .child(text("and"))
        .child(formula(r"\frac{1}{2}"));
    page(
        "Math",
        "Formulas as notes and technical documents write them: fractions, roots, sums, matrices.",
        [
            demo("Display", display, "formula(source).display()"),
            demo("In a line", line, "formula(source)"),
            demo("In a document", div().width(560.0).child(markdown(NOTE)), "markdown(source)"),
        ],
    )
}
