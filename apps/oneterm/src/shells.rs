//! The local shells (specs/19, "M4 in detail"): those found on this machine, and what a new
//! space is started with.

use std::path::PathBuf;

/// A program a space can run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shell {
    pub name: &'static str,
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// What a new space starts: a shell (the default when none), in a directory (the app's when
/// none), and how its terminal is set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Launch {
    pub shell: Option<Shell>,
    pub cwd: Option<PathBuf>,
    /// The cursor until a program sets one, and the rows of history kept.
    pub cursor: quadra_term::vt::CursorStyle,
    pub scrollback: u32,
}

/// The shells whose programs are where they are installed, in the menu's order.
pub fn found() -> Vec<Shell> {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let (system, programs) = (var("SystemRoot"), var("ProgramFiles"));
    let under = |root: &Option<PathBuf>, path: &str| root.as_ref().map(|root| root.join(path));
    let known: [(&'static str, Option<PathBuf>, &[&str]); 5] = [
        ("Command Prompt", under(&system, "System32/cmd.exe"), &[]),
        (
            "Windows PowerShell",
            under(&system, "System32/WindowsPowerShell/v1.0/powershell.exe"),
            &["-NoLogo"],
        ),
        ("PowerShell 7", under(&programs, "PowerShell/7/pwsh.exe"), &["-NoLogo"]),
        ("Git Bash", under(&programs, "Git/bin/bash.exe"), &["--login", "-i"]),
        ("WSL", under(&system, "System32/wsl.exe"), &[]),
    ];
    known
        .into_iter()
        .filter_map(|(name, program, args)| {
            let program = program.filter(|path| path.is_file())?;
            Some(Shell { name, program, args: args.iter().map(|arg| arg.to_string()).collect() })
        })
        .collect()
}
