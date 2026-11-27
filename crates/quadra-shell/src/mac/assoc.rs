//! File types on macOS (specs/16, "On macOS"): an app's kinds of file are those its bundle
//! names (`Info.plist`), which Launch Services reads when it registers the bundle. Which app
//! opens a kind is asked of it, and an app may make itself the one.

use std::ffi::c_void;
use std::path::{Path, PathBuf};

use objc2::rc::Retained;
use objc2_foundation::{NSBundle, NSString, NSURL, ns_string};

use super::MacShell;
use crate::files::{Associations, FileTypes};

/// `kLSRolesAll`: whatever an app does with a file of the kind.
const ANY_ROLE: u32 = 0xFFFF_FFFF;

// ponytail: Launch Services' calls of C, which Apple calls deprecated since macOS 12 and
// keeps; `NSWorkspace`'s (macOS 12, and blocks) when the oldest macOS served has them.
#[link(name = "CoreServices", kind = "framework")]
unsafe extern "C" {
    fn UTTypeCreatePreferredIdentifierForTag(
        class: &NSString,
        tag: &NSString,
        conforming_to: *const c_void,
    ) -> *mut NSString;
    fn LSCopyDefaultApplicationURLForContentType(
        kind: &NSString,
        roles: u32,
        error: *mut c_void,
    ) -> *mut NSURL;
    fn LSSetDefaultRoleHandlerForContentType(kind: &NSString, roles: u32, bundle_id: &NSString) -> i32;
    fn LSRegisterURL(bundle: &NSURL, update: u8) -> i32;
}

/// The system's name of the kind of file an extension (`.md`) is of.
fn kind_of(extension: &str) -> Option<Retained<NSString>> {
    let tag = NSString::from_str(extension.trim_start_matches('.'));
    let by_extension = ns_string!("public.filename-extension");
    // SAFETY: the strings live through the call, which gives a string the caller owns, or
    // none.
    unsafe { Retained::from_raw(UTTypeCreatePreferredIdentifierForTag(by_extension, &tag, std::ptr::null())) }
}

/// The bundle a program is in: `Name.app`, over `Contents/MacOS/name`.
fn bundle_of(exe: &Path) -> Option<&Path> {
    exe.ancestors().find(|it| it.extension().is_some_and(|kind| kind == "app"))
}

impl Associations for MacShell {
    /// Has Launch Services read the app's bundle again: its kinds are the bundle's.
    fn register(&self, types: &FileTypes) -> Result<(), String> {
        let exe = types.exe.display();
        let bundle = bundle_of(&types.exe)
            .ok_or_else(|| format!("{exe} is in no app bundle, where macOS reads an app's kinds of file"))?;
        let url = NSURL::from_file_path(bundle).ok_or_else(|| format!("{exe} is no path of a file"))?;
        // SAFETY: the URL lives through the call.
        match unsafe { LSRegisterURL(&url, 1) } {
            0 => Ok(()),
            status => Err(format!("Launch Services did not take {}: error {status}", bundle.display())),
        }
    }

    /// The program of the app that opens the kind: the one its bundle names.
    fn opener_of(&self, extension: &str) -> Option<PathBuf> {
        let kind = kind_of(extension)?;
        // SAFETY: the string lives through the call, which gives a URL the caller owns, or
        // none; no error is asked for.
        let app = unsafe {
            Retained::from_raw(LSCopyDefaultApplicationURLForContentType(
                &kind,
                ANY_ROLE,
                std::ptr::null_mut(),
            ))
        }?;
        quadra_native_mac::file_path(&*NSBundle::bundleWithURL(&app)?.executableURL()?)
    }

    /// Makes the app the one that opens its kinds: on macOS an app may.
    fn choose_default(&self, types: &FileTypes) -> bool {
        let bundle = bundle_of(&types.exe).and_then(NSURL::from_file_path);
        let id = bundle.and_then(|url| NSBundle::bundleWithURL(&url)).and_then(|it| it.bundleIdentifier());
        let Some(id) = id else { return false };
        let kinds: Vec<Retained<NSString>> = types.extensions.iter().filter_map(|it| kind_of(it)).collect();
        // SAFETY: both strings live through each call.
        let made = |kind: &Retained<NSString>| unsafe {
            LSSetDefaultRoleHandlerForContentType(kind, ANY_ROLE, &id) == 0
        };
        !kinds.is_empty() && kinds.iter().all(made)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_every_mac_opens_has_its_program_and_an_unknown_one_none() {
        let opener = MacShell.opener_of(".txt").expect("an app opens text");
        assert!(opener.is_absolute() && bundle_of(&opener).is_some(), "{}", opener.display());
        assert_eq!(MacShell.opener_of(".quadra-no-such-kind"), None);
        assert_eq!(bundle_of(Path::new("/A/Name.app/Contents/MacOS/name")), Some(Path::new("/A/Name.app")));
        let alone = FileTypes {
            name: "Alone".into(),
            vendor: "Quadra".into(),
            description: String::new(),
            document: String::new(),
            extensions: vec![".md".into()],
            exe: "/usr/bin/true".into(),
        };
        assert!(MacShell.register(&alone).is_err() && !MacShell.choose_default(&alone));
    }
}
