//! Motion: the pose of a box, the changes that ease, and timelines.

use quadra::{Align, Cursor, Div, Ease, Justify, Point, Refine, Timeline, div, row, text, timeline};
use quadra_design::{size, token};

use crate::common::{demo, line, page};

/// A card whose changes of style take 160 ms.
fn card(title: &str) -> Div {
    let card = div().size(150.0, 90.0).padding(token::SPACE_LG).radius(token::RADIUS_MD).bg(token::RAISED);
    let card = card.border(1.0).border_color(token::LINE).cursor(Cursor::Pointer).transition(160);
    card.child(text(title.to_string()).weight(size::WEIGHT_STRONG))
}

/// A dot that plays a timeline, in the room a card takes.
fn dot(plays: Timeline) -> Div {
    let dot = div().size(22.0, 22.0).radius(token::RADIUS_SM).bg(token::ACCENT).animate(plays);
    row().size(150.0, 90.0).align_items(Align::Center).justify(Justify::Center).child(dot)
}

pub fn motion_page() -> Div {
    let lift = |s: Refine| s.translate(Point::new(0.0, -3.0)).border_color(token::ACCENT);
    let cards = line()
        .child(card("Lift").hover(lift).active(|s| s.scale(0.97)))
        .child(card("Grow").hover(|s| s.scale(1.06)).active(|s| s.scale(1.0)))
        .child(card("Fade").hover(|s| s.opacity(0.55).bg(token::ACCENT.alpha(0.12))))
        .child(card("Turn").hover(|s| s.rotate(-6.0)).active(|s| s.rotate(6.0)));
    let hop =
        timeline()
            .to(380, Ease::InOut, |s| s.translate(Point::new(0.0, -18.0)))
            .to(380, Ease::In, |s| s.translate(Point::ZERO));
    let turn = timeline().to(1200, Ease::Linear, |s| s.rotate(360.0));
    let pulse = timeline().to(600, Ease::InOut, |s| s.scale(1.6).opacity(0.3)).alternate();
    let dots = line().child(dot(hop.forever())).child(dot(turn.forever())).child(dot(pulse.forever()));
    let about = "Transforms move, scale and turn what a div paints without changing layout; transition \
                 eases a change of style, and animate plays a timeline of tweens in sequence or together.";
    page(
        "Motion",
        about,
        [
            demo(
                "Hover and press",
                cards,
                "div().transition(160).hover(|s| s.translate(Point::new(0.0, -3.0)))",
            ),
            demo(
                "Timelines",
                dots,
                "div().animate(timeline().to(380, Ease::InOut, up).to(380, Ease::In, down).forever())",
            ),
        ],
    )
}
