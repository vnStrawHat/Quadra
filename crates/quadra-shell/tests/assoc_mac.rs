//! An app asks to be the one that opens its kinds of file (specs/16, "On macOS"), of the
//! bundle `QUADRA_BUNDLE` names: the CI's Mac sets it once it has made Markdown's. It may
//! change which app opens `.md` for the user, so nothing is done where it is not set.
//! Whether the app is then the one is not asked: macOS lets the user agree first (asked of
//! the runner's macOS 26 in Swift, run 45935041715: the workspace never said it was done,
//! and Xcode went on opening `.md`), and a runner has no user.
#![cfg(target_os = "macos")]

use std::path::PathBuf;

use quadra_shell::{Associations, FileTypes, MacShell};

#[test]
fn a_bundle_is_registered_and_asks_to_open_its_kinds() {
    let Some(bundle) = std::env::var_os("QUADRA_BUNDLE") else { return };
    let programs = PathBuf::from(bundle).join("Contents/MacOS");
    let exe = std::fs::read_dir(&programs).unwrap().next().expect("the bundle's program").unwrap().path();
    let types = FileTypes {
        name: "Markdown".into(),
        vendor: "Quadra".into(),
        description: String::new(),
        document: "Markdown document".into(),
        extensions: vec![".md".into(), ".markdown".into()],
        exe,
    };
    MacShell.register(&types).expect("Launch Services reads the bundle");
    assert!(MacShell.choose_default(&types), "the system took the asking");
    let opener = MacShell.opener_of(".md").expect("an app opens .md");
    println!("assoc_mac: {} opens .md", opener.display());
}
