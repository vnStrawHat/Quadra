//! The status bar's git (specs/19, "M4 in detail"): the branch of the repository the focused
//! space is in, read from its `.git`, and the commits ahead of its upstream and how many files
//! changed, from `git status` run on the pool with no window. Read again when the directory
//! changes and every 10 s.

use std::path::{Path, PathBuf};
use std::time::Duration;

use quadra::{Scope, Signal, background, effect, interval, signal, spawn};

/// What the status bar says of a repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    pub branch: String,
    /// The commits ahead of the upstream and the files changed; None when `git` could not say.
    pub changes: Option<(usize, usize)>,
}

/// Keeps a signal the repository of the directory `cwd` gives.
pub(crate) fn follow(cwd: impl Fn() -> String + 'static) -> Signal<Option<Repo>> {
    let (repo, tick, owner) = (signal(None), signal(0u32), Scope::current());
    let _ = interval(Duration::from_secs(10), move || tick.update(|n| *n += 1));
    effect(move || {
        tick.get();
        let dir = PathBuf::from(cwd());
        // In the scope it was set up in, where the app's tasks run.
        owner.run(|| {
            spawn(async move {
                if let Ok(found) = background(move || read(&dir)).await {
                    repo.set(found);
                }
            })
            .detach()
        });
    });
    repo
}

/// The repository a directory is in, its branch and its changes.
fn read(dir: &Path) -> Option<Repo> {
    if dir.as_os_str().is_empty() {
        return None;
    }
    let git = dir.ancestors().map(|at| at.join(".git")).find(|git| git.exists())?;
    // A worktree's `.git` is a file that says where its folder is.
    let git = match std::fs::read_to_string(&git) {
        Ok(said) => PathBuf::from(said.strip_prefix("gitdir:")?.trim()),
        Err(_) => git,
    };
    let head = std::fs::read_to_string(git.join("HEAD")).ok()?;
    let head = head.trim();
    let branch = match head.strip_prefix("ref: refs/heads/") {
        Some(name) => name.to_string(),
        None => head.chars().take(7).collect(),
    };
    Some(Repo { branch, changes: changes(dir) })
}

/// What `git status --porcelain --branch` says: the commits ahead, the files changed.
fn changes(dir: &Path) -> Option<(usize, usize)> {
    let mut git = std::process::Command::new("git");
    git.args(["status", "--porcelain", "--branch"]).current_dir(dir);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        git.creation_flags(CREATE_NO_WINDOW);
    }
    let out = git.output().ok().filter(|out| out.status.success())?;
    Some(counts(&String::from_utf8_lossy(&out.stdout)))
}

/// The commits ahead, from the first line (`## main...origin/main [ahead 2, behind 1]`), and
/// the lines after it, a file each.
fn counts(said: &str) -> (usize, usize) {
    let mut lines = said.lines().filter(|line| !line.is_empty()).peekable();
    let head = lines.next_if(|line| line.starts_with("## "));
    let ahead = head
        .and_then(|line| line.split("[ahead ").nth(1))
        .and_then(|rest| rest.split([',', ']']).next())
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or(0);
    (ahead, lines.count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_outside_a_repository_has_none() {
        assert_eq!(read(Path::new("")), None);
        assert_eq!(read(Path::new("Z:/no/such/place")), None);
    }

    #[test]
    fn ahead_and_changes_are_counted() {
        assert_eq!(counts("## main...origin/main [ahead 2, behind 1]\n M a.rs\n?? b\n"), (2, 2));
        assert_eq!(counts("## main...origin/main [ahead 12]\n"), (12, 0));
        assert_eq!(counts("## main\n M a.rs\n"), (0, 1));
        assert_eq!(counts(""), (0, 0));
    }

    #[test]
    fn the_branch_is_read_from_head() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/git-head");
        let git = root.join(".git");
        std::fs::create_dir_all(&git).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/feature/x\n").unwrap();
        assert_eq!(read(&root).map(|repo| repo.branch), Some("feature/x".to_string()));
        std::fs::write(git.join("HEAD"), "0123456789abcdef\n").unwrap();
        assert_eq!(read(&root).map(|repo| repo.branch), Some("0123456".to_string()));
    }
}
