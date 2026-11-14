//! Rich text: mixed styles and links in one wrapped paragraph.

use quadra::{Div, div, rich_text, signal, span};
use quadra_kit::{fonts, token};

use crate::common::{demo, hint, page, stack};

pub fn rich_text_page() -> Div {
    let clicked = signal(String::new());
    let (code, link) = (
        |words: &str| span(words).family(fonts::MONO),
        |words: &str, to: &str| span(words).link(to).color(token::ACCENT_TEXT),
    );
    let event = rich_text([
        span("Pod "),
        code("api-7d9f8c").color(token::ACCENT_TEXT),
        span(" was "),
        span("restarted").bold(),
        span(" 3 times in the last hour. The probe on "),
        code("/healthz"),
        span(" timed out; see the "),
        link("troubleshooting guide", "https://example.com/probes"),
        span(" or the "),
        link("events", "events"),
        span("."),
    ]);
    let event = event.on_link(move |target| clicked.set(target.to_string()));
    let said = hint(move || format!("Clicked: {}", clicked.get()));
    page(
        "Rich text",
        "Mixed styles and links in one wrapped paragraph: words glued across spans never break apart.",
        [demo(
            "Event detail",
            stack().child(div().width(420.0).child(event)).child(said),
            "rich_text([span(\"was \"), span(\"restarted\").bold(), span(\"guide\").link(url)]).on_link(f)",
        )],
    )
}
