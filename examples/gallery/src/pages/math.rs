//! Math: formulas in the notation of TeX, laid out as boxes and painted with the text.

use quadra::{Div, column, div, rich_text, span};
use quadra_md::markdown;
use quadra_md::tex::formula;

use crate::common::{demo, page};
use quadra_design::token;

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
    let display = SAMPLES.iter().fold(column().gap(token::SPACE_XL).text_size(18.0), |all, source| {
        all.child(formula(source).display())
    });
    let line = rich_text([
        span("The sides of a right triangle have "),
        formula("a^2+b^2=c^2").span(),
        span(", and a half is "),
        formula(r"\frac{1}{2}").span(),
        span(". A sum in a line keeps its limits at its side: "),
        formula(r"\sum_{i=1}^{n} i = \frac{n(n+1)}{2}").span(),
        span(", whatever the line holds before and after it, and the words wrap around what is no word."),
    ]);
    let line = div().width(420.0).child(line);
    page(
        "Math",
        "Formulas as notes and technical documents write them: fractions, roots, sums, matrices.",
        [
            demo("Display", display, "formula(source).display()"),
            demo("In a line", line, "rich_text([span(..), formula(source).span()])"),
            demo("In a document", div().width(560.0).child(markdown(NOTE)), "markdown(source)"),
        ],
    )
}
