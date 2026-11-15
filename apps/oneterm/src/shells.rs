//! The local shells (specs/19, "M4 in detail"): those found on this machine, and what a new
//! space is started with.

use std::path::PathBuf;

use quadra_term::vt::{CursorShape, CursorStyle};

use crate::model::Model;

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
    /// A host to connect to instead of a local shell (P2).
    pub ssh: Option<Remote>,
    /// The SSH keepalive: how often (`None`: never) and how many may go unanswered.
    pub keepalive: (Option<std::time::Duration>, usize),
}

/// A host and how to sign in to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remote {
    pub target: oneterm_ssh::Target,
    pub auth: oneterm_ssh::Auth,
    /// The jump hosts on the way, the first connected to first.
    pub jumps: Vec<oneterm_ssh::Hop>,
    pub forwards: Vec<oneterm_ssh::Forward>,
}

impl Remote {
    pub fn new(target: oneterm_ssh::Target, auth: oneterm_ssh::Auth) -> Remote {
        Remote { target, auth, jumps: Vec::new(), forwards: Vec::new() }
    }

    /// The way through this host to another: its jump hosts, then itself.
    pub fn route(&self) -> Vec<oneterm_ssh::Hop> {
        let mut hops = self.jumps.clone();
        hops.push(oneterm_ssh::Hop { target: self.target.clone(), auth: self.auth.clone() });
        hops
    }
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

impl Model {
    /// What a new space starts as the settings say: a shell (the default one when none is
    /// given), in the active space's directory or home, with the cursor and the history set.
    pub fn launch(&self, shell: Option<Shell>) -> Launch {
        let p = self.prefs();
        let named = p.shell.get_untracked();
        let shell = shell.or_else(|| found().into_iter().find(|it| it.name == named));
        let cwd = match p.start.get_untracked() {
            0 => self.focused_cwd(),
            _ => std::env::var_os("USERPROFILE").map(Into::into),
        };
        let shape =
            [CursorShape::Block, CursorShape::Beam, CursorShape::Underline][p.cursor.get_untracked().min(2)];
        let cursor = CursorStyle { shape, blinking: p.blink.get_untracked() };
        Launch {
            shell,
            cwd,
            cursor,
            scrollback: p.scrollback.get_untracked().clamp(0.0, 1.0e6) as u32,
            ssh: None,
            keepalive: p.keepalive(),
        }
    }

    /// The directory of the focused space of the tab that shows.
    pub(crate) fn focused_cwd(&self) -> Option<std::path::PathBuf> {
        let space = quadra::untrack(|| self.active_tab().and_then(|tab| tab.focused()))?;
        Some(space.session.cwd().get_untracked()).filter(|cwd| !cwd.is_empty()).map(Into::into)
    }
}
