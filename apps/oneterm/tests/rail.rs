//! Close tab (specs/19, "Rail"): the tab that shows has it at its end, in the rail and in the
//! strip; a press on it closes that tab and leaves the other one showing.

use std::rc::Rc;

use oneterm::{NewSession, OneTerm, keymap};
use quadra::{Key, Mods, Role, Settings, provide};
use quadra_term::echo;
use quadra_testing::Harness;

fn app(vertical: bool) -> Harness {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let settings = Settings::memory();
    settings.value("tabs.vertical", true).set(vertical);
    let sessions: NewSession = Rc::new(|_| echo());
    h.mount(move || {
        provide(settings.clone());
        OneTerm::new(sessions).view()
    });
    h.settle();
    h
}

fn tabs(h: &Harness) -> Vec<(String, bool)> {
    let found = std::cell::RefCell::new(Vec::new());
    h.query(|n| {
        if n.semantics.role == Role::Tab {
            found.borrow_mut().push((n.semantics.name.clone(), n.semantics.selected));
        }
        false
    });
    found.into_inner()
}

#[test]
fn close_tab_closes_the_tab_that_shows_and_the_other_shows() {
    for vertical in [true, false] {
        let mut h = app(vertical);
        h.key(Key::T, Mods::CTRL);
        h.settle();
        assert_eq!(tabs(&h).len(), 2);
        let closes = h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Close tab");
        assert_eq!(closes.len(), 1, "only the tab that shows has it");
        // A press of the pointer, so the row under the button hears it too, unless stopped.
        let at = h.tree.window_rect(closes[0]).center();
        h.click(at.x, at.y);
        h.settle();
        assert_eq!(tabs(&h), [("Terminal".to_string(), true)], "vertical: {vertical}");
    }
}

#[test]
fn rail_tabs_have_no_line_between_them_and_a_light_fill_under_the_pointer() {
    for folded in [false, true] {
        let mut h = app(true);
        h.key(Key::T, Mods::CTRL);
        h.settle();
        if folded {
            let fold =
                h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Collapse tabs to icons");
            h.tree.act(fold[0], &quadra::SemanticAction::Click);
            h.settle();
        }
        let rows = h.query(|n| n.semantics.role == Role::Tab);
        let (first, second) = (h.tree.window_rect(rows[0]), h.tree.window_rect(rows[1]));
        assert_eq!(h.tree.layout_style(rows[0]).border.bottom, 0.0, "no line under a tab");
        assert_eq!(second.y, first.bottom(), "the next starts where it ends");
        // The first does not show: under the pointer it fills panel_hover at half.
        let at = first.center();
        h.pointer(quadra_tree::PointerInput::Move, at.x, at.y);
        h.settle();
        let hover = h.tree.theme().resolve(quadra_design::token::PANEL_HOVER.alpha(0.5));
        assert_eq!(h.tree.paint_style(rows[0]).background, hover, "folded: {folded}");
        // New terminal is a box of a dashed line, folded or not.
        let new = h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "New terminal")[0];
        let dashed = std::iter::once(new).chain(h.tree.children(new).iter().copied());
        let dashed: Vec<_> =
            dashed.filter(|it| h.tree.paint_style(*it).border_style == quadra::BorderStyle::Dashed).collect();
        assert_eq!(dashed.len(), 1, "folded: {folded}");
    }
}
