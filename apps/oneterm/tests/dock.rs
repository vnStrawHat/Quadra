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
    /// The app's folders, and what stands for the system's opener.
    data: PathBuf,
    shell: Rc<quadra_shell::FakeShell>,
}

/// The app with one tab connected to a test server by a key, its dock open as an SSH tab's is.
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
    let (home, shell) = (dir.join("app"), quadra_shell::FakeShell::new());
    let (data, given) = (home.join("data"), shell.clone());
    let sessions: NewSession = Rc::new(move |launch: oneterm::Launch| match launch.ssh {
        Some(_) => remote_shell_with(launch, |_| options(port, &base, Auth::Auto)),
        None => echo(),
    });
    let mut h = Harness::new();
    h.size = quadra::Size::new(1400.0, 800.0);
    h.tree.set_keymap(Rc::new(keymap()));
    h.mount(move || {
        provide(settings.clone());
        provide(Rc::new(quadra::AppDirs::under(&home)));
        given.provide();
        let app = OneTerm::new(sessions);
        let target = oneterm_ssh::Target { user: "tester".into(), host: "127.0.0.1".into(), port };
        app.model.connect_to(Remote::new(target, Auth::Key(key_file.clone())), false);
        app.view()
    });
    h.settle();
    // An SSH tab shows its dock by itself.
    App { h, served, downloads, data, shell }
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
    a.wait(|a| a.entry("Parent folder") && !a.entry("config.yaml"), "the folder src");
    a.h.get_by_test_id("sftp-path").set_value("/home");
    a.h.settle();
    a.h.get_by_role(Role::Button, "Go to the typed path").click();
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
    a.wait(|a| a.h.has_text("Downloading config.yaml") && a.h.has_text("done"), "the transfer done");
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

    // The list's menu, where no entry is.
    let list = a.h.get_by_role(Role::Button, "notes.md").node();
    let below = a.h.tree.window_rect(list);
    let at = quadra::Point::new(below.center().x, below.bottom() + 200.0);
    for input in [quadra_tree::PointerInput::Down, quadra_tree::PointerInput::Up] {
        let now = a.h.now();
        a.h.tree.pointer(input, quadra::MouseButton::Right, at, quadra::Mods::NONE, now);
    }
    a.h.settle();
    a.h.get_by_role(Role::MenuItem, "New folder\u{2026}").click();
    a.h.settle();
    a.h.get_by_test_id("sftp-name").set_value("made");
    a.h.settle();
    a.h.get_by_test_id("sftp-name").focus();
    a.h.key(quadra::Key::Enter, quadra::Mods::NONE);
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
    a.h.get_by_role(Role::Button, "Zoom SFTP").click();
    a.h.settle();
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

    a.h.get_by_role(Role::Button, "Zoom out").click();
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
    h.get_by_role(Role::Button, "Toggle SFTP panel").click();
    h.settle();
    assert!(h.has_text("local space \u{B7} n/a"));
}

#[test]
fn a_file_is_edited_and_sent_back_when_it_is_saved() {
    let mut a = app("dock-edit");
    a.wait(|a| a.entry("config.yaml"), "the home folder");
    a.menu("config.yaml", "Edit\u{2026}");
    let folder = a.data.join("sftp-edit");
    let found = |folder: &std::path::Path| {
        let first = std::fs::read_dir(folder).ok()?.flatten().next()?;
        Some(first.path().join("config.yaml")).filter(|it| it.is_file())
    };
    let whole = |it: &PathBuf| std::fs::metadata(it).is_ok_and(|m| m.len() == 2458);
    a.wait(|_| found(&folder).is_some_and(|it| whole(&it)), "the download");
    let local = found(&folder).unwrap();
    let opened =
        |a: &App| a.shell.asked().iter().any(|it| it.starts_with("open: ") && it.ends_with("config.yaml"));
    a.wait(opened, "opened");
    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(&local, b"edited").unwrap();
    let served = a.served.join("home/config.yaml");
    a.wait(|_| std::fs::read(&served).is_ok_and(|it| it == b"edited"), "sent back");
}

#[test]
fn an_entry_s_properties_show_in_both_views_and_a_failure_is_a_card() {
    let mut a = app("dock-props");
    a.wait(|a| a.entry("config.yaml"), "the home folder");
    a.menu("config.yaml", "Properties\u{2026}");
    a.wait(|a| a.h.has_text("-rw-r--r-- (0644)"), "the dialog");
    for shown in ["File", "2.4 KB (2458 bytes)", "1000", "/home/config.yaml"] {
        assert!(a.h.has_text(shown), "{shown}\n{}", a.h.semantics());
    }
    close(&mut a);
    assert!(!a.h.has_text("-rw-r--r-- (0644)"), "closed");

    a.h.get_by_role(Role::Button, "Zoom SFTP").click();
    a.h.settle();
    a.menu("src", "Properties\u{2026}");
    a.wait(|a| a.h.has_text("drwxr-xr-x (0755)") && a.h.has_text("Folder"), "a folder's");
    close(&mut a);

    std::fs::remove_file(a.served.join("home/config.yaml")).unwrap();
    a.menu("config.yaml", "Properties\u{2026}");
    let card = |a: &App| a.h.semantics().contains("No properties of /home/config.yaml: ");
    a.wait(card, "the card");
}

/// The dialog's Close: the last of the buttons of that name (the window's and the cross before it).
fn close(a: &mut App) {
    let all = a.h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Close");
    assert!(a.h.tree.act(*all.last().unwrap(), &quadra_tree::SemanticAction::Click));
    a.h.settle();
}
