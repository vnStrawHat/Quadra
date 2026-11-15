//! Larger text (specs/12, "Text size"): a button and the rows of a list are as many times
//! taller as text is larger, and the rows stay one after another.

use std::rc::Rc;

use quadra_design::{button, dark, list};
use quadra_reactive::{Signal, memo, provide, signal};
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{TextScale, column, div, text};

struct Page {
    h: Harness,
    scale: Signal<f32>,
}

fn page() -> Page {
    let scale = signal(1.0f32);
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        provide(TextScale(memo(move || scale.get())));
        let rows = list(|| 100, signal(None), |at| text(move || format!("Row {}", at.get())));
        quadra_design::root().child(column().child(button("Save")).child(div().height(300.0).child(rows)))
    });
    h.frame();
    Page { h, scale }
}

impl Page {
    fn set(&mut self, scale: f32) {
        self.scale.set(scale);
        self.h.tree.set_text_scale(scale);
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }

    /// The button's height, and the top and height of the first rows.
    fn look(&mut self) -> (f32, Vec<(f32, f32)>) {
        let save = self.h.get_by_role(Role::Button, "Save").node();
        let height = self.h.tree.rect(save).height;
        let rows = ["Row 0", "Row 1", "Row 2"].map(|name| {
            let row = self.h.get_by_role(Role::ListItem, name).node();
            let rect = self.h.tree.window_rect(row);
            (rect.y, rect.height)
        });
        let first = rows[0].0;
        (height, rows.iter().map(|&(y, h)| (y - first, h)).collect())
    }
}

#[test]
fn a_button_and_the_rows_of_a_list_grow_with_the_text() {
    let mut p = page();
    assert_eq!(p.look(), (28.0, vec![(0.0, 23.0), (23.0, 23.0), (46.0, 23.0)]));
    p.set(1.5);
    assert_eq!(p.look(), (42.0, vec![(0.0, 34.5), (34.5, 34.5), (69.0, 34.5)]));
    p.set(1.0);
    assert_eq!(p.look(), (28.0, vec![(0.0, 23.0), (23.0, 23.0), (46.0, 23.0)]));
}
