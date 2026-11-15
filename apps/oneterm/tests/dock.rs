//! The SFTP dock (specs/19, "M9 in detail") on a real connection to the engine's test server
//! on 127.0.0.1, whose files are a folder of `.tmp`: it lists the home folder, opens a folder
//! and goes back by the path, downloads on two clicks to the folder set, uploads what is
//! dropped, makes a folder, and says a local space is not one.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use oneterm::{NewSession, OneTerm, Remote, keymap, remote_shell_with};
use oneterm_ssh::Auth;
use oneterm_ssh::testing::{folder, options, serve_files, trust, write_user_key};
use quadra::{DroppedFiles, Role, Settings, provide};
use quadra_term::echo;
use quadra_testing::Harness;

struct App {
    h: Harness,
    served: PathBuf,
    downloads: PathBuf,
}

/// The app with one tab connected to a test server by a key, its dock open.
fn app(name: &str) -> App {
    let dir = folder(name);
    let served = dir.join("served");
    std::fs::create_dir_all(served.join("home/src")).unwrap();
    std::fs::write(served.join("home/config.yaml"), vec![b'x'; 2458]).unwrap();
    let (port, _) = serve_files(11, served.clone());
    trust(port, 11, &dir.join("known_hosts"));
    write_user_key(&dir.join("id"));
    let downloads = dir.join("downloads");
    std::fs::create_dir_all(&downloads).unwrap();

    let settings = Settings::memory();
    settings.value("sftp.downloads", String::new()).set(downloads.display().to_string());
    let (base, key_file) = (dir.clone(), dir.join("id"));
    let sessions: NewSession = Rc::new(move |launch: oneterm::Launch| match launch.ssh {
        Some(_) => remote_shell_with(launch, |_| options(port, &base, Auth::Auto)),
        None => echo(),
    });
    let mut h = Harness::new();
    h.size = quadra::Size::new(1400.0, 800.0);
    h.tree.set_keymap(Rc::new(keymap()));
    h.mount(move || {
        provide(settings.clone());
        let app = OneTerm::new(sessions);
        let target = oneterm_ssh::Target { user: "tester".into(), host: "127.0.0.1".into(), port };
        app.model.connect_to(Remote::new(target, Auth::Key(key_file.clone())), false);
        app.view()
    });
    h.settle();
    let mut a = App { h, served, downloads };
    a.h.get_by_role(Role::Button, "SFTP").click();
    a.h.settle();
    a
}

impl App {
    /// Settles until `done`, 20 s at most: the engine works on threads of its own.
    fn wait(&mut self, done: impl Fn(&App) -> bool, what: &str) {
        let end = Instant::now() + Duration::from_secs(20);
        loop {
            // The app's timers move with the harness's clock.
            self.h.advance(Duration::from_millis(10));
            self.h.settle();
            if done(self) {
                return;
            }
            assert!(Instant::now() < end, "never: {what}\n{}", self.h.semantics());
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn pane(&self, name: &str) -> bool {
        !self.h.query(|n| n.semantics.name == name && n.semantics.role != Role::Button).is_empty()
    }

    /// A click of the other button on an entry, then a row of its menu.
    fn menu(&mut self, name: &str, row: &str) {
        let node = self.h.get_by_role(Role::Button, name).node();
        let at = self.h.tree.window_rect(node).center();
        for input in [quadra_tree::PointerInput::Down, quadra_tree::PointerInput::Up] {
            let now = self.h.now();
            self.h.tree.pointer(input, quadra::MouseButton::Right, at, quadra::Mods::NONE, now);
        }
        self.h.settle();
        self.h.get_by_role(Role::MenuItem, row).click();
        self.h.settle();
    }

    fn entry(&self, name: &str) -> bool {
        !self.h.query(|n| n.semantics.role == Role::Button && n.semantics.name == name).is_empty()
    }
}

#[test]
fn the_dock_lists_moves_about_and_downloads() {
    let mut a = app("dock");
    a.wait(|a| a.entry("config.yaml") && a.entry("src"), "the home folder");
    assert!(a.h.has_text("2.4 KB"));
    a.h.get_by_role(Role::Button, "src").click();
    a.wait(|a| a.entry("..") && !a.entry("config.yaml"), "the folder src");
    a.h.get_by_role(Role::Link, "/home").click();
    a.wait(|a| a.entry("config.yaml"), "home again");

    let row = a.h.get_by_role(Role::Button, "config.yaml").node();
    let at = a.h.tree.window_rect(row).center();
    for _ in 0..2 {
        for input in [quadra_tree::PointerInput::Down, quadra_tree::PointerInput::Up] {
            a.h.pointer(input, at.x, at.y);
        }
    }
    let got = a.downloads.join("config.yaml");
    a.wait(|_| got.is_file(), "the download");
    a.wait(|a| a.h.has_text("config.yaml \u{2190} local") && a.h.has_text("done"), "the transfer done");
    assert_eq!(std::fs::read(&got).unwrap().len(), 2458);
}

#[test]
fn dropped_files_upload_and_a_folder_is_made() {
    let mut a = app("dock-up");
    a.wait(|a| a.entry("config.yaml"), "the home folder");
    let local = a.downloads.join("notes.md");
    std::fs::write(&local, b"# notes").unwrap();
    drop_on_dock(&mut a, &local);
    let up = a.served.join("home/notes.md");
    a.wait(|a| up.is_file() && a.entry("notes.md"), "the upload listed");

    a.h.get_by_role(Role::Button, "More").click();
    a.h.settle();
    a.h.get_by_role(Role::MenuItem, "New folder\u{2026}").click();
    a.h.settle();
    a.h.get_by_test_id("sftp-name").set_value("made");
    a.h.settle();
    a.h.get_by_role(Role::Button, "Create").click();
    let made = a.served.join("home/made");
    a.wait(|a| made.is_dir() && a.entry("made"), "the folder made");
}

#[test]
fn an_entry_s_menu_copies_its_path_renames_and_deletes_it() {
    let mut a = app("dock-menu");
    a.wait(|a| a.entry("config.yaml"), "the home folder");
    a.menu("config.yaml", "Copy path");
    assert_eq!(a.h.clipboard.read_text().as_deref(), Some("/home/config.yaml"));
    a.menu("config.yaml", "Rename\u{2026}");
    a.h.get_by_test_id("sftp-name").set_value("app.yaml");
    a.h.settle();
    a.h.get_by_role(Role::Button, "Rename").click();
    let renamed = a.served.join("home/app.yaml");
    a.wait(|a| renamed.is_file() && a.entry("app.yaml"), "the file renamed");

    a.menu("app.yaml", "Delete\u{2026}");
    assert!(a.h.has_text("Delete the file app.yaml from the server? It cannot be undone."));
    a.h.get_by_role(Role::Button, "Delete").click();
    a.wait(|a| !renamed.exists() && !a.entry("app.yaml"), "the file deleted");
}

#[test]
fn the_expanded_view_moves_files_between_its_panes_and_compares() {
    let mut a = app("dock-wide");
    a.wait(|a| a.entry("config.yaml"), "the home folder");
    std::fs::write(a.downloads.join("notes.md"), b"# notes").unwrap();
    a.h.get_by_role(Role::Button, "Expand SFTP").click();
    a.h.settle();
    assert!(!a.h.query(|n| n.semantics.name == "Collapse SFTP").is_empty(), "the title bar can collapse it");
    assert!(
        a.pane("Local") && a.pane("Remote"),
        "two panes
{}",
        a.h.semantics()
    );
    a.h.get_by_role(Role::Button, "Refresh local").click();
    a.wait(|a| a.entry("notes.md"), "the local folder");

    a.h.get_by_role(Role::Button, "notes.md").click();
    a.h.settle();
    a.h.get_by_role(Role::Button, "Upload selected").click();
    let up = a.served.join("home/notes.md");
    let both = |a: &App| {
        a.h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "notes.md").len() == 2
    };
    a.wait(|a| up.is_file() && both(a), "the upload listed on both sides");

    a.h.get_by_role(Role::Button, "config.yaml").click();
    a.h.settle();
    a.h.get_by_role(Role::Button, "Download selected").click();
    let got = a.downloads.join("config.yaml");
    a.wait(|a| got.is_file() && a.h.has_text("0 active · 2 done"), "the download into the local pane");

    // The remote copy is years older than the local one.
    let old = std::time::UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    std::fs::File::options()
        .write(true)
        .open(a.served.join("home/config.yaml"))
        .unwrap()
        .set_modified(old)
        .unwrap();
    a.h.get_by_role(Role::Button, "Refresh remote").click();
    a.h.settle();
    a.h.get_by_role(Role::Button, "Compare").click();
    a.wait(|a| a.h.has_text("older"), "the comparison");

    a.h.get_by_role(Role::Button, "Collapse SFTP").click();
    a.h.settle();
    assert!(!a.pane("Local") && !a.h.semantics().contains("SFTP expanded"), "the dock again");
    assert!(a.entry("config.yaml"), "the dock again");
}

fn drop_on_dock(a: &mut App, path: &Path) {
    let dock = a.h.query(|n| n.semantics.name == "SFTP" && n.semantics.role != Role::Button)[0];
    let at = a.h.tree.window_rect(dock).center();
    a.h.tree.os_drag_enter(Rc::new(DroppedFiles(vec![path.to_path_buf()])), at);
    a.h.tree.os_drop(at);
}

#[test]
fn a_local_space_is_not_an_ssh_space() {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1400.0, 800.0);
    let sessions: NewSession = Rc::new(|_| echo());
    h.mount(move || OneTerm::new(sessions).view());
    h.settle();
    h.get_by_role(Role::Button, "SFTP").click();
    h.settle();
    assert!(h.has_text("local space \u{B7} n/a"));
}
