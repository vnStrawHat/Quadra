//! A package's files put in the folder of the app that runs (specs/22, "Installing"). A
//! program's file cannot be written over or deleted while it runs, but it can be renamed
//! where it is: each file there is moved aside (`<name>.quadra-old`) and the new one put at
//! its name, so the app goes on running what it loaded and its next start is the new version.
//! That start removes what was moved aside.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// What a file moved aside ends with.
const OLD: &str = "quadra-old";
/// Set for the start [`restart`] makes: the run before it is still ending.
const RESTARTED: &str = "QUADRA_UPDATED";

fn aside(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".");
    name.push(OLD);
    PathBuf::from(name)
}

/// The files under `folder`, as paths from it, in order.
fn files(folder: &Path, under: &Path, into: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(folder.join(under))?.collect::<io::Result<_>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = under.join(entry.file_name());
        match entry.file_type()?.is_dir() {
            true => files(folder, &path, into)?,
            false => into.push(path),
        }
    }
    Ok(())
}

/// Puts every file of `package` at its place under `home`: the file that is there is moved
/// aside first, as a running program's must be. All of it or none: when one file cannot be
/// put, those already put are taken back and what was moved aside is moved back. Files of
/// `home` the package does not have stay as they are.
/// ponytail: a folder the user cannot write to (under Program Files) is an error said in
/// words; asking for elevation when an installer puts apps there.
pub fn swap(package: &Path, home: &Path) -> io::Result<()> {
    let mut all = Vec::new();
    files(package, Path::new(""), &mut all)?;
    let mut put: Vec<(PathBuf, bool)> = Vec::new();
    let done = all.iter().try_for_each(|file| {
        let (from, to) = (package.join(file), home.join(file));
        to.parent().map_or(Ok(()), std::fs::create_dir_all)?;
        let was = to.exists();
        if was {
            // One a start has not removed yet; if it cannot go, the rename says why.
            let _ = std::fs::remove_file(aside(&to));
            std::fs::rename(&to, aside(&to))?;
        }
        put.push((to.clone(), was));
        std::fs::copy(&from, &to).map(|_| ())
    });
    if done.is_err() {
        for (to, was) in put.into_iter().rev() {
            let _ = std::fs::remove_file(&to);
            if was {
                let _ = std::fs::rename(aside(&to), &to);
            }
        }
    }
    done
}

/// Removes what [`swap`] moved aside under `home`, as far as it can (the file of a program
/// that still runs stays): how many files are left.
pub fn swept(home: &Path) -> usize {
    let mut all = Vec::new();
    let _ = files(home, Path::new(""), &mut all);
    let old = all.iter().filter(|it| it.extension().is_some_and(|it| it == OLD));
    old.filter(|it| std::fs::remove_file(home.join(it)).is_err()).count()
}

/// Starts the app again, from the file at its name: the new version once [`swap`] put it
/// there. The app then closes; the new start waits for it ([`started`]).
pub fn restart() -> io::Result<()> {
    std::process::Command::new(std::env::current_exe()?).env(RESTARTED, "1").spawn().map(|_| ())
}

/// For the top of an app's `main`, before it asks to be the one instance: after an update,
/// removes what the version before left moved aside. Started by [`restart`], it first waits
/// (10 s at most) for the run before to end, which is when its file can be removed. Without
/// an update it looks at one file's name and returns.
pub fn started() {
    let Ok(exe) = std::env::current_exe() else { return };
    let (old, Some(home)) = (aside(&exe), exe.parent()) else { return };
    if !old.exists() {
        return;
    }
    let waits = if std::env::var_os(RESTARTED).is_some() { 100 } else { 1 };
    for _ in 0..waits {
        if std::fs::remove_file(&old).is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    swept(home);
}
