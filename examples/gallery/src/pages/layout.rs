//! What lays a page out: tabs, the accordion, the split, a scroll area and the grid.

use quadra::{Align, Color, Div, Justify, column, div, dynamic, row, scroll, signal, text};
use quadra_design::{accordion, split, tabs, token};

use crate::common::{demo, framed, hint, mono, page};
use crate::shell::Gallery;

pub fn tabs_page() -> Div {
    let (tab, pill) = (signal(0usize), signal(0usize));
    let panels = [
        "Pod api-7d9f8c6b5-x2k4p is running on node pool-a-3.",
        "12:04:51  GET /healthz 200 0.4ms",
        "apiVersion: v1\nkind: Pod",
    ];
    let panel = dynamic(move || div().padding(token::SPACE_LG).child(mono(panels[tab.get().min(2)])));
    let underline = column().child(tabs(["Overview", "Logs", "YAML"], tab)).child(panel);
    page(
        "Tabs",
        "Switches between panels in the same place. The active tab has an accent label.",
        [
            demo("Underline", underline, "tabs([\"Overview\", \"Logs\", \"YAML\"], tab)"),
            demo("Pill", row().child(tabs(["Day", "Week", "Month"], pill).pill()), ".pill()"),
        ],
    )
}

pub fn accordion_page() -> Div {
    let faq = accordion()
        .item("What is Quadra?", signal(true), || text("A GPU UI framework in Rust."))
        .item("Does it support Vietnamese?", signal(false), || text("Có, cả hiển thị lẫn bộ gõ."))
        .item("Which platforms?", signal(false), || text("Windows first; Linux and macOS next."));
    page(
        "Accordion",
        "Sections that expand in place, for long settings and details.",
        [demo("Basic", faq, "accordion().item(\"What is Quadra?\", open, || body)")],
    )
}

pub fn split_page() -> Div {
    let (share, upper) = (signal(0.3f32), signal(0.6f32));
    let left = div().grow(1.0).padding(token::SPACE_LG).bg(token::PANEL).child(hint("Explorer"));
    let top = div().grow(1.0).padding(token::SPACE_LG).child(text("Editor: drag the dividers."));
    let bottom = div().grow(1.0).padding(token::SPACE_LG).bg(token::PANEL).child(mono("$ cargo run"));
    let inner = split(top, bottom, upper).stacked();
    page(
        "Split pane",
        "Resizable panes with a 1 px divider and a 6 px grab area.",
        [demo("Workbench", framed(split(left, inner, share)).height(260.0), "split(left, right, share)")],
    )
}

pub fn scroll_page() -> Div {
    let mut list = column();
    for i in 0..60 {
        let item =
            row().gap(token::SPACE_LG).padding_x(token::SPACE_LG).height(26.0).align_items(Align::Center);
        let item = item.border_bottom(1.0).border_color(token::LINE).child(mono(format!("{:3}", i + 1)));
        list = list.child(item.child(text(format!("12:04:{:02}  GET /api/v1/pods 200", i % 60))));
    }
    let area = framed(scroll(signal(0.0), list)).size(460.0, 222.0);
    page(
        "Scroll area",
        "A fixed-height region with a thin overlay scrollbar.",
        [demo("Log", row().child(area), "scroll(scroll_y, content)")],
    )
}

pub fn grid_page(gallery: Gallery) -> Div {
    let light = if gallery.dark.get() { 0.3 } else { 0.8 };
    let mut cards = div().grid_fill(140.0).gap(token::SPACE_MD);
    for i in 0..12 {
        let card = row().height(64.0).align_items(Align::Center).justify(Justify::Center);
        let card = card.bg(Color::hsla(190.0 + i as f32 * 6.0, 0.5, light, 1.0));
        cards = cards.child(card.child(mono(format!("node-{}", i + 1))));
    }
    page(
        "Grid",
        "Responsive columns: as many as fit at the minimum width.",
        [demo("Fill", cards, "div().grid_fill(140.0).gap(token::SPACE_MD)")],
    )
}
