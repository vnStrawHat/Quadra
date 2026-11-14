//! Images: files decoded by the system's codecs, off the thread of the window, at the size
//! they show at.

use quadra::{
    Align, AnyView, BorderStyle, Div, DroppedFiles, Fit, Justify, column, div, dynamic, image, row, signal,
    text,
};
use quadra_kit::{size, token};
use quadra_md::markdown;

use crate::common::{demo, page};

const HILLS: &[u8] = include_bytes!("../../assets/hills.png");
const MARK: &[u8] = include_bytes!("../../assets/mark.png");

const NOTE: &str = "A picture beside the document, no wider than the page:

![Hills at dusk](hills.png)

One that is not there is its words:

![The missing chart](charts/missing.png)
";

/// A box of 150 by 96 with the hills fitted one way, and how under it.
fn fitted(fit: Fit, how: &'static str) -> Div {
    let picture = image(HILLS).width(150.0).height(96.0).fit(fit).radius(size::RADIUS_LG).label(how);
    let frame = div().bg(token::PANEL).radius(size::RADIUS_LG).child(picture);
    let said = text(how).size(size::TEXT_SM.size).color(token::TEXT_2);
    column().gap(6.0).align_items(Align::Center).child(frame).child(said)
}

/// A box that shows the first file another program drops on it, as a picture.
fn dropped() -> Div {
    let file = signal(None);
    let shown = dynamic(move || match file.get() {
        Some(path) => AnyView::new(image(path).width(280.0).height(140.0).fit(Fit::Contain)),
        None => AnyView::new(text("Drop a picture here from the file manager").color(token::TEXT_3)),
    });
    let frame = div().width(320.0).height(160.0).align_items(Align::Center).justify(Justify::Center);
    let frame = frame.border(1.0).border_color(token::LINE_STRONG).border_style(BorderStyle::Dashed);
    let frame = frame.radius(size::RADIUS_LG).drop_target(|s| s.bg(token::ACCENT_TINT));
    frame.on_drop(move |files: &DroppedFiles, _| file.set(files.0.first().cloned())).child(shown)
}

pub fn image_page() -> Div {
    let sizes = [96.0, 64.0, 40.0, 24.0, 16.0].map(|side| image(MARK).width(side).label("The mark"));
    let marks = row().gap(16.0).align_items(Align::End).padding(12.0).radius(size::RADIUS_LG);
    let fits = [(Fit::Contain, "Contain"), (Fit::Cover, "Cover"), (Fit::Fill, "Fill")];
    page(
        "Image",
        "A file's picture: read and decoded in the background, shown when it arrives.",
        [
            demo(
                "Its own size, no wider than there is room",
                column().width(480.0).align_items(Align::Start).child(image(HILLS).label("Hills at dusk")),
                "image(include_bytes!(\"hills.png\").as_slice())",
            ),
            demo(
                "A box of another shape",
                row().gap(20.0).children(fits.map(|(fit, how)| fitted(fit, how))),
                "image(file).width(150.0).height(96.0).fit(Fit::Cover).radius(6.0)",
            ),
            demo(
                "Decoded at the size it shows at",
                marks.bg(token::PANEL).children(sizes),
                "image(file).width(40.0)",
            ),
            demo(
                "Dropped from another program",
                dropped(),
                ".on_drop(|files: &DroppedFiles, at| ..).drop_target(|s| s.bg(token::ACCENT_TINT))",
            ),
            demo(
                "In a document",
                div().width(480.0).child(markdown(NOTE).base(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))),
                "markdown(source).base(folder)",
            ),
        ],
    )
}
