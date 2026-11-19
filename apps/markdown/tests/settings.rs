//! Settings and About (specs/21): the page writes the settings and the window follows them;
//! Set as default registers Markdown and shows the system's page, through the fake shell
//! (nothing writes the registry).

use std::cell::RefCell;
use std::rc::Rc;

use markdown::{MdApp, Model, PREVIEW, RAW, SIDE_BY_SIDE, keymap, settings_page};
use quadra::{Role, SemanticAction, Settings, provide};
use quadra_shell::FakeShell;
use quadra_testing::Harness;

struct App {
    h: Harness,
    m: Model,
    settings: Rc<Settings>,
    shell: Rc<FakeShell>,
}

/// The window and its settings page side by side, over settings in memory.
fn app(name: &str) -> App {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("settings-{name}.md"));
    let text: String =
        (0..30).map(|k| format!("## Part {k}\n\nOne {k}.\n\nTwo {k}.\n\nThree {k}.\n\n")).collect();
    std::fs::write(&path, text).unwrap();
    let mut h = Harness::new();
    h.size = quadra::Size::new(1600.0, 900.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let (settings, shell) = (Settings::memory(), FakeShell::new());
    let (given, faked, built) = (settings.clone(), shell.clone(), Rc::new(RefCell::new(None)));
    let tell = built.clone();
    h.mount(move || {
        provide(given.clone());
        faked.provide();
        let md = MdApp::new(Some(path));
        *tell.borrow_mut() = Some(md.model.clone());
        let page = settings_page(&md.model, 0);
        quadra::row().grow(1.0).child(quadra::column().width(900.0).child(md.view())).child(page)
    });
    let m: Model = built.borrow_mut().take().unwrap();
    h.settle();
    App { h, m, settings, shell }
}

impl App {
    fn page(&mut self, name: &str) {
        self.h.get_by_role(Role::TreeItem, name).click();
        self.h.settle();
    }

    fn switches(&self) -> Vec<quadra::NodeId> {
        self.h.query(|n| n.semantics.role == Role::Switch)
    }
}

#[test]
fn the_page_writes_the_settings_and_the_window_follows() {
    let mut a = app("follows");
    // The theme, by the segmented control.
    a.page("Appearance");
    a.h.get_by_role(Role::Radio, "Light").click();
    a.h.settle();
    assert_eq!(a.settings.value("appearance.theme", String::new()).get(), "Light");
    // Wrap lines, off: written, and the editor built again without it.
    a.page("Editor");
    let wrap = a.switches()[0];
    a.h.tree.act(wrap, &SemanticAction::Click);
    a.h.settle();
    assert!(!a.settings.value("editor.wrap", true).get());
    assert!(!a.m.prefs.wrap.get_untracked());
    // A file opens in the layout the settings say.
    a.settings.value("layout.opens_in", String::new()).set("Raw".into());
    a.h.settle();
    a.m.show(None, "# New\n".into());
    a.h.settle();
    assert_eq!(a.m.layout.get_untracked(), RAW);
    a.settings.value("layout.opens_in", String::new()).set("Preview".into());
    a.m.show(None, "# Again\n".into());
    a.h.settle();
    assert_eq!(a.m.layout.get_untracked(), PREVIEW);
}

#[test]
fn scroll_together_off_lets_each_pane_go_its_own_way() {
    let mut a = app("together");
    a.h.get_by_role(Role::Radio, "Side by side").click();
    a.h.settle();
    assert_eq!(a.m.layout.get_untracked(), SIDE_BY_SIDE);
    a.page("Layouts");
    let together = a.switches()[0];
    a.h.tree.act(together, &SemanticAction::Click);
    a.h.settle();
    assert!(!a.settings.value("layout.scroll_together", true).get());
    a.m.driver.set(markdown::Driver::Editor);
    a.m.pos.set(80.0);
    a.h.settle();
    assert_eq!(a.m.preview_y.get_untracked(), 0.0, "the preview stays");
}

#[test]
fn set_as_default_registers_and_shows_the_systems_page() {
    let mut a = app("default");
    a.page("About");
    assert!(a.h.has_text("Another app opens .md files, or none."));
    // What the user would choose on the system's page.
    a.shell.set_opener(".md", std::env::current_exe().unwrap());
    a.h.get_by_role(Role::Button, "Set as default\u{2026}").click();
    a.h.settle();
    let asked = a.shell.asked();
    assert!(asked.contains(&"register: Quadra.Markdown.Document .md .markdown".to_string()), "{asked:?}");
    assert!(asked.contains(&"choose default: Markdown".to_string()), "{asked:?}");
    assert!(a.h.has_text("Markdown opens .md files."));
}
