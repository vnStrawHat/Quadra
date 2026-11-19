//! A shell's profile (specs/18, "Semantic highlighting"): the prompts and options only its shell
//! has, over those every profile knows (`scan::prompt_sign`).

use crate::session::Session;

/// What shell runs in a session, for how its lines are read when the shell marks none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShellProfile {
    /// sh, bash, zsh, fish: local ones, WSL, and what SSH reaches.
    #[default]
    Unix,
    /// Command Prompt: its `>` continuation, and options of `/` too.
    Cmd,
    /// PowerShell: its `>>` continuation, and a location behind a provider
    /// (`PS Microsoft.PowerShell.Core\FileSystem::\\server\share>`).
    PowerShell,
    /// A router or a serial line: any word that ends in a sign (`Router#`, `switch>`).
    Dumb,
}

impl ShellProfile {
    /// The sign of a prompt of this shell's own, unmarked.
    pub(super) fn sign(self, c: &[char]) -> Option<usize> {
        match self {
            ShellProfile::Unix => None,
            ShellProfile::Cmd => (c.first() == Some(&'>')).then_some(0),
            ShellProfile::PowerShell => qualified(c).or_else(|| {
                let n = c.iter().take_while(|&&x| x == '>').count();
                (n > 0 && c.get(n).is_none_or(|&x| x == ' ')).then(|| n - 1)
            }),
            ShellProfile::Dumb => {
                let end = c.iter().position(|x| x.is_whitespace()).unwrap_or(c.len());
                let ends = end > 0 && matches!(c[end - 1], '$' | '#' | '%' | '>' | '»');
                ends.then(|| end - 1)
            }
        }
    }

    /// Whether a word that starts with `c` is an option.
    pub(super) fn option(self, c: char) -> bool {
        c == '-' || (c == '/' && matches!(self, ShellProfile::Cmd | ShellProfile::Dumb))
    }
}

/// `PS <module>\<provider>::<path>>`: a qualifier of names, `::`, then a path as a Windows
/// prompt's (no `< | " * ? :`, its last character no space or `-`) up to the first `>`.
fn qualified(c: &[char]) -> Option<usize> {
    let rest = c.strip_prefix(&['P', 'S', ' '])?;
    let colons = rest.windows(2).position(|w| w == [':', ':'])?;
    let name = |part: &[char]| {
        part.first().is_some_and(char::is_ascii_alphabetic)
            && part.iter().all(|x| x.is_ascii_alphanumeric() || matches!(x, '.' | '-'))
    };
    if colons == 0 || !rest[..colons].split(|&x| x == '\\').all(name) {
        return None;
    }
    let start = 3 + colons + 2;
    let gt = start + c[start..].iter().position(|&x| x == '>')?;
    let body = &c[start..gt];
    let bad = |x: &char| matches!(x, '<' | '|' | '"' | '*' | '?' | ':');
    let end = body.last().is_some_and(|&x| !x.is_whitespace() && x != '-');
    (!body.iter().any(bad) && end).then_some(gt)
}

impl Session {
    /// Says what shell runs, for how its lines are read (`Unix` until told); its view reads them
    /// again.
    pub fn set_shell_profile(&self, profile: ShellProfile) {
        if self.0.profile.replace(profile) != profile {
            self.repaint();
        }
    }

    pub fn shell_profile(&self) -> ShellProfile {
        self.0.profile.get()
    }
}
