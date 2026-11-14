//! The base's data views with bare parts (specs/17, "Tests"): a tree, a table and settings.

use std::collections::HashSet;
use std::rc::Rc;

use quadra_base::table::{self, Column};
use quadra_base::tree::{self, Branch};
use quadra_platform::{Clipboard, Key, Mods};
use quadra_reactive::{Signal, signal, use_context};
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_ui::{View, column, div, row, text};

fn mounted<V: View + 'static>(view: impl FnOnce() -> V) -> Harness {
    let mut h = Harness::new();
    h.tree.set_keymap(Rc::new(quadra_base::keymap()));
    h.mount(view);
    h.frame();
    h
}

#[derive(Clone)]
struct Thing(&'static str, Vec<Thing>);

impl Branch for Thing {
    type Row = &'static str;

    fn id(&self) -> &str {
        self.0
    }

    fn children(&self) -> &[Thing] {
        &self.1
    }

    fn row(&self, _: bool) -> &'static str {
        self.0
    }
}

#[test]
fn a_tree_opens_by_its_keys_and_selects_by_a_click() {
    let (chosen, open): (Signal<Option<String>>, _) = (signal(None), signal(HashSet::new()));
    let mut h = mounted(move || {
        let roots =
            || vec![Thing("src", vec![Thing("main", vec![]), Thing("lib", vec![])]), Thing("docs", vec![])];
        let face = |at: &tree::Shown<&'static str>, _: tree::Line| div().height(20.0).label(at.row);
        tree::tree(roots, chosen, open, 20.0, face).frame(column().height(200.0))
    });
    assert!(h.semantics().contains("TreeItem \"src\" [collapsed]"), "{}", h.semantics());
    let frame = h.get_by_role(Role::Tree, "").node();
    h.tree.focus(frame);
    h.key(Key::Down, Mods::NONE); // nothing selected: the first
    h.key(Key::Right, Mods::NONE); // opens it
    h.key(Key::Right, Mods::NONE); // goes into it
    h.frame();
    assert_eq!(chosen.get_untracked().as_deref(), Some("main"));
    h.key(Key::Left, Mods::NONE); // out to what holds it
    h.key(Key::Left, Mods::NONE); // closes it
    h.frame();
    assert!(h.semantics().contains("TreeItem \"src\" [selected] [collapsed]"), "{}", h.semantics());
    h.get_by_role(Role::TreeItem, "docs").click();
    assert_eq!(chosen.get_untracked().as_deref(), Some("docs"));
}

#[test]
fn a_table_sorts_selects_by_the_pointer_and_the_keys_and_copies() {
    let names = ["pear", "apple", "fig"];
    let counts = ["3", "10", "2"];
    let chosen = signal(Vec::new());
    let mut h = mounted(move || {
        let col = |counts| Column { width: 60.0, least: 20.0, sortable: true, counts, restless: false };
        let cell = move |row: usize, c: usize| if c == 0 { names[row] } else { counts[row] }.to_string();
        let sheet = table::sheet(vec![col(false), col(true)], Rc::new(|| 3), Rc::new(cell), Some(chosen));
        let title = |c: usize| sheet.title(div().width(60.0).label(["Name", "Count"][c]), c);
        let head = row().child(title(0)).child(title(1));
        let rows = (0..3).map(|place| {
            let (place, s) = (signal(place), sheet.clone());
            sheet.row(row().height(20.0).child(text(move || (s.cell)(s.data(place.get()), 0))), place)
        });
        let frame = sheet.frame(column(), use_context::<Rc<dyn Clipboard>>());
        frame.child(head).children(rows.collect::<Vec<_>>())
    });
    h.get_by_role(Role::Cell, "Count").click(); // as numbers: 2, 3, 10
    h.frame();
    assert!(h.has_text("fig"));
    let frame = h.get_by_role(Role::Table, "").node();
    h.tree.focus(frame);
    h.key(Key::Down, Mods::NONE); // the first place: fig, the third row of the data
    h.key(Key::Down, Mods::SHIFT);
    assert_eq!(chosen.get_untracked(), vec![0, 2]);
    h.key(Key::C, Mods::CTRL);
    assert_eq!(h.clipboard.read_text().as_deref(), Some("fig\t2\npear\t3"));
}

#[test]
fn a_search_keeps_rows_whatever_the_accents_and_shows_a_page_that_keeps_one() {
    use quadra_base::settings::Model;
    let pages =
        vec![vec![vec!["Font size".to_string()]], vec![vec!["Ngôn ngữ".to_string(), "Theme".to_string()]]];
    let query = signal(String::new());
    let model = Model::new(&pages, signal(0), query);
    assert_eq!((model.searching.get(), model.shown.get()), (false, 0));
    query.set("ngon".into());
    assert!(model.searching.get());
    assert_eq!((model.rows[1][0][0].get(), model.rows[1][0][1].get()), (true, false));
    assert_eq!((model.pages[0].get(), model.shown.get()), (false, 1));
}
