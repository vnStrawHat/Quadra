//! File types (specs/16, "File types"): which kinds of file an app opens, what registering it
//! for them writes, and the trait that does it.

use std::path::{Path, PathBuf};

/// An app that opens kinds of file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTypes {
    /// As registered: what the system's Default apps page lists (`Markdown`).
    pub name: String,
    pub vendor: String,
    /// What the app is, one line.
    pub description: String,
    /// What a document of it is called (`Markdown document`).
    pub document: String,
    /// With their dots: `.md`.
    pub extensions: Vec<String>,
    pub exe: PathBuf,
}

impl FileTypes {
    /// `<Vendor>.<Name>.Document`, without spaces.
    pub fn prog_id(&self) -> String {
        format!("{}.{}.Document", self.vendor, self.name).replace(' ', "")
    }
}

/// A string value of the registry under `HKEY_CURRENT_USER`: its key, its name (empty: the
/// key's default value) and its data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegValue {
    pub key: String,
    pub name: String,
    pub data: String,
}

fn value(key: impl Into<String>, name: &str, data: impl Into<String>) -> RegValue {
    RegValue { key: key.into(), name: name.to_string(), data: data.into() }
}

/// What registering `types` writes, in order.
pub fn registration(types: &FileTypes) -> Vec<RegValue> {
    let (id, exe) = (types.prog_id(), types.exe.display().to_string());
    let class = format!(r"Software\Classes\{id}");
    // A Windows path whatever system reads it: its name is what follows its last separator.
    let file = exe.rsplit(['\\', '/']).next().unwrap_or_default().to_string();
    let able = format!(r"Software\{}\{}\Capabilities", types.vendor, types.name);
    let mut out = vec![
        value(&class, "", &types.document),
        value(format!(r"{class}\DefaultIcon"), "", format!("\"{exe}\",0")),
        value(format!(r"{class}\shell\open\command"), "", format!("\"{exe}\" \"%1\"")),
        value(&able, "ApplicationName", &types.name),
        value(&able, "ApplicationDescription", &types.description),
    ];
    for ext in &types.extensions {
        out.push(value(format!(r"Software\Classes\{ext}\OpenWithProgids"), &id, ""));
        out.push(value(format!(r"Software\Classes\Applications\{file}\SupportedTypes"), ext, ""));
        out.push(value(format!(r"{able}\FileAssociations"), ext, &id));
    }
    out.push(value(r"Software\RegisteredApplications", &types.name, &able));
    out
}

/// The system's Default apps page at an app registered for the user.
pub fn default_apps_page(name: &str) -> String {
    let escaped: String = name
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("ms-settings:defaultapps?registeredAppUser={escaped}")
}

/// Whether `opener` is `exe`: the same file, whatever the case of its letters.
pub fn same_program(opener: &Path, exe: &Path) -> bool {
    let plain = |it: &Path| it.display().to_string().to_lowercase().replace('/', "\\");
    plain(opener) == plain(exe)
}

/// Registers an app for its kinds of file: the OS's ([`crate::install`]) or a test's
/// ([`crate::FakeShell`]).
pub trait Associations {
    /// Registers the app for its types, for the user alone, and tells the shell.
    fn register(&self, types: &FileTypes) -> Result<(), String>;
    /// The program that opens files of `extension` (`.md`) now, if any.
    fn opener_of(&self, extension: &str) -> Option<PathBuf>;
    /// Shows where the user makes the app the default (Windows), or asks the system to make
    /// it, which asks the user (macOS): no app may do it by itself.
    fn choose_default(&self, types: &FileTypes) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markdown() -> FileTypes {
        FileTypes {
            name: "Markdown".into(),
            vendor: "Quadra".into(),
            description: "Reads and edits Markdown files.".into(),
            document: "Markdown document".into(),
            extensions: vec![".md".into(), ".markdown".into()],
            exe: PathBuf::from(r"C:\Apps\Markdown\markdown.exe"),
        }
    }

    #[test]
    fn what_registering_writes() {
        let all = registration(&markdown());
        let has = |key: &str, name: &str, data: &str| all.contains(&value(key, name, data));
        assert!(has(r"Software\Classes\Quadra.Markdown.Document", "", "Markdown document"));
        assert!(has(
            r"Software\Classes\Quadra.Markdown.Document\shell\open\command",
            "",
            r#""C:\Apps\Markdown\markdown.exe" "%1""#
        ));
        assert!(has(
            r"Software\Classes\Quadra.Markdown.Document\DefaultIcon",
            "",
            r#""C:\Apps\Markdown\markdown.exe",0"#
        ));
        assert!(has(r"Software\Classes\.md\OpenWithProgids", "Quadra.Markdown.Document", ""));
        assert!(has(r"Software\Classes\.markdown\OpenWithProgids", "Quadra.Markdown.Document", ""));
        assert!(has(r"Software\Classes\Applications\markdown.exe\SupportedTypes", ".md", ""));
        let able = r"Software\Quadra\Markdown\Capabilities";
        assert!(has(&format!(r"{able}\FileAssociations"), ".markdown", "Quadra.Markdown.Document"));
        assert!(has(r"Software\RegisteredApplications", "Markdown", able));
        // Nothing outside the user's own software.
        assert!(all.iter().all(|it| it.key.starts_with(r"Software\")));
        assert_eq!(all.len(), 6 + 3 * 2);
    }

    #[test]
    fn the_page_and_the_program() {
        assert_eq!(default_apps_page("Markdown"), "ms-settings:defaultapps?registeredAppUser=Markdown");
        assert_eq!(default_apps_page("My App"), "ms-settings:defaultapps?registeredAppUser=My%20App");
        let exe = Path::new(r"C:\Apps\Markdown\markdown.exe");
        assert!(same_program(Path::new(r"c:\apps\markdown\MARKDOWN.EXE"), exe));
        assert!(!same_program(Path::new(r"C:\Windows\notepad.exe"), exe));
    }
}
