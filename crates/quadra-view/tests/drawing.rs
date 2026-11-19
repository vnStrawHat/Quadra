//! Drawings (specs/13, "Icons"): one painted in the text's colour is a tinted mask; one with
//! colours of its own is painted in them, from the colour atlas.

use std::rc::Rc;

use quadra_prim::Color;
use quadra_render::Kind;
use quadra_testing::Harness;
use quadra_vector::Svg;
use quadra_view::{drawing, row};

/// The kinds of what was painted, in order.
fn painted(svg: &str) -> Vec<(u32, [f32; 4])> {
    let svg = Rc::new(Svg::parse(svg));
    let mut h = Harness::new();
    h.mount(move || row().text_color(Color::rgb(0x336699)).child(drawing(svg).size(32.0)));
    h.frame();
    let layer = h.tree.scene().layer(h.tree.scene().roots()[0]);
    let kinds = [Kind::Mono as u32, Kind::Poly as u32];
    layer.instances().iter().filter(|i| kinds.contains(&i.kind)).map(|i| (i.kind, i.color)).collect()
}

#[test]
fn a_drawing_of_its_own_colours_is_painted_in_them() {
    let icon =
        r#"<svg viewBox="0 0 24 24" stroke="currentColor" fill="none"><path d="M4 4h16v16H4z"/></svg>"#;
    let mono = painted(icon);
    assert_eq!(mono.len(), 1);
    assert_eq!(mono[0].0, Kind::Mono as u32, "a mask");
    let logo = r##"<svg viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" fill="#1E94FF"/><circle cx="12" cy="12" r="5" fill="#FFD000"/></svg>"##;
    let own = painted(logo);
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].0, Kind::Poly as u32, "from the colour atlas");
    assert_eq!(own[0].1, Color::WHITE.to_array(), "not tinted");
}
