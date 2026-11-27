//! An app made the one that opens its kinds of file (specs/16, "On macOS"), asked of the
//! bundle `QUADRA_BUNDLE` names: the CI's Mac sets it once it has made Markdown's. It changes
//! which app opens `.md` for the user, so nothing is done where it is not set.
#![cfg(target_os = "macos")]

use std::path::PathBuf;

use quadra_shell::{Associations, FileTypes, MacShell, same_program};

#[test]
fn a_bundle_is_registered_and_made_the_one_that_opens_its_kinds() {
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
    assert!(MacShell.choose_default(&types), "the bundle is made the one that opens .md");
    let opener = MacShell.opener_of(".md").expect("an app opens .md");
    assert!(same_program(&opener, &types.exe), "{} opens .md, not {}", opener.display(), types.exe.display());
}
