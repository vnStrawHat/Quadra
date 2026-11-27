//! Files the system asks the app to open (specs/23, "The app bundle"): a file opened from
//! the Finder or by `open` reaches an app in a bundle as an event of the application, not as
//! an argument, whether the app runs already or is started for it.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use objc2_foundation::NSURL;

/// The path a file's URL names, with its accented letters whole (composed, as text is typed
/// and compared). The system writes a file's URL as its file system writes names, every
/// such letter taken apart, and takes either form of a name for the same file.
// ponytail: a volume that tells the two forms apart (some network shares) would not find a
// name it keeps taken apart; try the path as given first if one is met.
pub fn file_path(url: &NSURL) -> Option<PathBuf> {
    let path = url.path().filter(|_| url.isFileURL())?;
    Some(PathBuf::from(path.precomposedStringWithCanonicalMapping().to_string()))
}

type Listener = Box<dyn FnMut(Vec<PathBuf>) + Send>;

/// Who is told, and what was asked before anyone listened.
static OPENS: Mutex<(Option<Listener>, Vec<PathBuf>)> = Mutex::new((None, Vec::new()));

/// Gives `f` the files the system asks the app to open, each time it asks, on the main
/// thread; what it asked before now is given at once. One listens: another replaces it.
pub fn on_open_files(mut f: impl FnMut(Vec<PathBuf>) + Send + 'static) {
    let mut opens = OPENS.lock().unwrap_or_else(PoisonError::into_inner);
    let waiting = std::mem::take(&mut opens.1);
    if !waiting.is_empty() {
        f(waiting);
    }
    opens.0 = Some(Box::new(f));
}

/// The system asked for these files.
pub(crate) fn opened(files: Vec<PathBuf>) {
    let mut opens = OPENS.lock().unwrap_or_else(PoisonError::into_inner);
    match &mut opens.0 {
        Some(told) => told(files),
        None => opens.1.extend(files),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;

    #[test]
    fn what_was_asked_before_anyone_listened_is_given_first() {
        opened(vec![PathBuf::from("/tmp/a.md")]);
        let (to, seen) = channel();
        on_open_files(move |files| to.send(files).unwrap());
        opened(vec![PathBuf::from("/tmp/b.md"), PathBuf::from("/tmp/c.md")]);
        let seen: Vec<Vec<PathBuf>> = seen.try_iter().collect();
        assert_eq!(seen, [vec![PathBuf::from("/tmp/a.md")], vec!["/tmp/b.md".into(), "/tmp/c.md".into()]]);
    }
}
