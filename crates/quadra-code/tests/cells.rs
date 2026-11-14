//! Characters in their cells (specs/13, "Code"): a wide character takes two cells and its
//! glyph is in the middle of them. On the fake backend at size 10 every glyph is 6 px
//! wide, as a cell is.

use quadra_code::editor;
use quadra_render::{Kind, Step};
use quadra_testing::Harness;
use quadra_ui::column;

#[test]
fn a_glyph_narrower_than_its_cells_is_in_the_middle_of_them() {
    let mut h = Harness::new();
    h.mount(move || column().size(400.0, 140.0).child(editor("a中b".to_string()).font(10.0, 14.0)));
    h.frame();
    // The glyphs of the first line, from the left; the numbers of the gutter are at its left.
    let mut xs: Vec<f32> = h
        .renderer
        .steps()
        .iter()
        .filter_map(|step| match step {
            Step::Layer { id, .. } => Some(h.tree.scene().layer(*id).instances()),
            _ => None,
        })
        .flatten()
        .filter(|i| i.kind == Kind::Mono as u32 && i.bounds[0] >= 54.0)
        .map(|i| i.bounds[0])
        .collect();
    xs.sort_by(f32::total_cmp);
    assert_eq!(xs.len(), 3, "{xs:?}");
    // "中" takes the cells 1 and 2, its glyph of one cell 3 px in; "b" is in the cell 3.
    assert_eq!((xs[1] - xs[0], xs[2] - xs[0]), (9.0, 18.0));
}
