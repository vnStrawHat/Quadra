//! The first pages: what Quadra is, and what its design language is made of.

use quadra::{Align, Div, Justify, Token, column, div, row, text};
use quadra_design::{KitTheme, fonts, icon, icons, kbd, size, token};

use crate::common::{demo, hint, line, mono, page};

pub fn start_page() -> Div {
    let principles = [
        ("Quiet chrome, loud content", "Panels and toolbars recede; code, logs and data get full contrast."),
        (
            "Planes, not boxes",
            "Regions differ by one step of tone and a hairline. Only overlays cast shadows.",
        ),
        ("Dense with rhythm", "Compact by default, on a 4 px grid and four control heights: 18, 22, 28, 32."),
        ("Mono is first-class", "Identifiers, numbers, paths and durations use the mono face."),
        ("Status by color and shape", "Every status has a shape too, so it reads in grayscale."),
        (
            "One accent",
            "The accent colors text and icons: toggled, matched, linked. Selection is a neutral step.",
        ),
    ];
    let mut list = column();
    for (i, (title, about)) in principles.into_iter().enumerate() {
        let words = column().gap(2.0).child(text(title).weight(size::WEIGHT_STRONG)).child(hint(about));
        let item = row().gap(16.0).padding_y(8.0).border_bottom(1.0).border_color(token::LINE);
        list = list.child(item.child(div().width(24.0).child(mono(format!("0{}", i + 1)))).child(words));
    }
    let keys = [
        ("ctrl-k", "commands"),
        ("ctrl-shift-p", "profiler"),
        ("ctrl-shift-f", "paint flashing"),
        ("ctrl-shift-i", "inspector"),
    ];
    let mut shortcuts = line();
    for (keys, does) in keys {
        shortcuts = shortcuts.child(kbd(keys)).child(hint(does));
    }
    let about = "A native, GPU-rendered UI framework in Rust. Everything here is drawn by Quadra itself, \
                 in the One design language: built for terminals, editors and cluster tools that people \
                 keep open all day.";
    page(
        "Quadra",
        about,
        [
            demo(
                "Hello, world",
                text("Xin ch\u{E0}o! Pick a component on the left."),
                "div().padding(16.0).child(text(\"Xin ch\u{E0}o\"))",
            ),
            demo("Design principles", list, ""),
            demo("Developer shortcuts", shortcuts, ""),
        ],
    )
}

pub fn typography_page() -> Div {
    let roles = [
        ("Display \u{B7} 23 / 600", size::TEXT_2XL.size, size::WEIGHT_STRONG),
        ("Headline \u{B7} 18 / 400", size::TEXT_XL.size, size::WEIGHT_NORMAL),
        ("Large \u{B7} 16 / 400", size::TEXT_LG.size, size::WEIGHT_NORMAL),
        ("Body \u{B7} 14 / 400", size::TEXT_MD.size, size::WEIGHT_NORMAL),
        ("Small \u{B7} 12 / 400", size::TEXT_SM.size, size::WEIGHT_NORMAL),
        ("XSmall \u{B7} 10 / 400", size::TEXT_XS.size, size::WEIGHT_NORMAL),
    ];
    let mut scale = column();
    for (name, px, weight) in roles {
        let sample = text("Tiếng Việt có dấu, rõ ràng").size(px).weight(weight);
        let step = row().align_items(Align::Center).gap(16.0).padding_y(8.0);
        let step = step.border_bottom(1.0).border_color(token::LINE);
        scale = scale.child(step.child(div().width(120.0).child(hint(name))).child(sample));
    }
    let code = "fn main() {\n    let pods = cluster.list(\"default\")?;\n    println!(\"{} pods, 0O 1lI\", pods.len());\n}";
    let code = div().padding(12.0).bg(token::PANEL).font_family(fonts::MONO).child(text(code).selectable());
    let words = "Quadra shapes text with the platform engine and caches every word, so long paragraphs wrap \
                 in microseconds. Chữ tiếng Việt như “Đường Trường Sơn”, “nghiêng ngả”, “thuở ấy” hiển thị \
                 đúng dấu ở mọi cỡ chữ.";
    let words = div().max_width(560.0).child(text(words).color(token::TEXT_2).selectable());
    let about = "IBM Plex Sans for the interface, Lilex for code and values, both bundled. Weights are 400 \
                 and 600 only.";
    page(
        "Typography",
        about,
        [
            demo("Scale", scale, "text(\"..\").size(size::TEXT_LG.size)"),
            demo("Mono", code, "div().font_family(fonts::MONO).child(text(source))"),
            demo(
                "Selectable paragraph",
                words,
                "text(words).selectable()  // drag, double-click a word, ctrl-a, ctrl-c",
            ),
        ],
    )
}

/// A grid of the colours of some roles: each a sample, its name and its value.
fn swatches(theme: &KitTheme, roles: &[(&str, Token)]) -> Div {
    let mut grid = div().grid_fill(150.0).gap(12.0);
    for &(name, role) in roles {
        let (color, hex) = (theme.color(role), theme.color(role).hex());
        let hex = if color.a >= 1.0 { hex[..7].to_string() } else { hex };
        let sample = div().height(40.0).bg(color).border(1.0).border_color(token::LINE);
        let value =
            div().font_family(fonts::MONO).child(text(hex).size(size::TEXT_XS.size).color(token::TEXT_3));
        grid = grid
            .child(column().gap(4.0).child(sample).child(text(name).size(size::TEXT_SM.size)).child(value));
    }
    grid
}

pub fn colors_page(theme: &KitTheme) -> Div {
    use token::*;
    let planes = [
        ("canvas", CANVAS),
        ("panel", PANEL),
        ("raised", RAISED),
        ("overlay_bg", OVERLAY_BG),
        ("line", LINE),
        ("line_strong", LINE_STRONG),
    ];
    let texts = [("text", TEXT), ("text_2", TEXT_2), ("text_3", TEXT_3), ("text_disabled", TEXT_DISABLED)];
    let accents = [
        ("accent", ACCENT),
        ("accent_tint", ACCENT_TINT),
        ("accent_text", ACCENT_TEXT),
        ("ok", OK),
        ("warn", WARN),
        ("error", ERROR),
        ("info", INFO),
    ];
    let raised = |radius: f32, level: u8, name: &str| {
        let plane = row().align_items(Align::Center).justify(Justify::Center).size(96.0, 96.0);
        let plane = plane.bg(OVERLAY_BG).border(1.0).border_color(LINE_STRONG).radius(radius);
        plane.elevation(level).child(hint(name))
    };
    let levels = line().child(raised(size::RADIUS_MD, 1, "menu")).child(raised(size::RADIUS_LG, 2, "dialog"));
    let about = format!(
        "Design roles of the current theme ({}). Components read roles, never raw colors.",
        theme.name
    );
    page(
        "Colors",
        &about,
        [
            demo("Planes", swatches(theme, &planes), "div().bg(token::PANEL)"),
            demo("Text", swatches(theme, &texts), ""),
            demo("Accent and feedback", swatches(theme, &accents), ""),
            demo("Elevation: overlays only", levels, ""),
        ],
    )
}

pub fn icons_page() -> Div {
    let mut grid = div().grid_fill(112.0).gap(4.0);
    for (name, body) in icons::ALL {
        let cell = column().align_items(Align::Center).gap(8.0).padding_y(12.0).radius(token::RADIUS_MD);
        let cell = cell.hover(|s| s.bg(token::PANEL_HOVER)).child(icon(body).size(18.0).color(token::TEXT));
        grid = grid.child(cell.child(text(name).size(size::TEXT_XS.size).color(token::TEXT_3)));
    }
    let about = format!(
        "{} stroked 24 px icons, drawn as vectors at any size and tinted from the theme.",
        icons::ALL.len()
    );
    let code = "icon(icons::SETTINGS).size(16.0).color(token::TEXT_2)";
    page("Icons", &about, [demo("Catalog", grid, code)])
}
