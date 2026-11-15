//! A connection's files over SFTP (specs/19, "M9 in detail"). Each call blocks on the
//! engine's runtime: an app makes them on its pool, never on its UI thread.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use russh::client::Handle;
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::runtime;
use crate::session::Checker;

const MOST: usize = 100_000;
const CHUNK: usize = 64 * 1024;

/// An entry of a remote folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub dir: bool,
    pub size: u64,
}

/// SFTP on a connection; a copy is the same session.
#[derive(Clone)]
pub struct Files {
    sftp: Arc<SftpSession>,
}

fn said(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl Files {
    pub(crate) fn open(handle: Arc<Handle<Checker>>) -> Result<Files, String> {
        runtime().block_on(async move {
            let channel = handle.channel_open_session().await.map_err(said)?;
            channel.request_subsystem(true, "sftp").await.map_err(said)?;
            let sftp = SftpSession::new(channel.into_stream()).await.map_err(said)?;
            Ok(Files { sftp: Arc::new(sftp) })
        })
    }

    /// The folder the server starts in.
    pub fn home(&self) -> Result<String, String> {
        runtime().block_on(self.sftp.canonicalize(".")).map_err(said)
    }

    /// A folder's entries: folders first, then by name whatever the case.
    pub fn list(&self, dir: &str) -> Result<Vec<Entry>, String> {
        let read = runtime().block_on(self.sftp.read_dir(dir)).map_err(said)?;
        let mut entries: Vec<Entry> = read
            .filter(|it| it.file_name() != "." && it.file_name() != "..")
            .take(MOST)
            .map(|it| {
                let meta = it.metadata();
                Entry { name: it.file_name(), dir: meta.is_dir(), size: meta.len() }
            })
            .collect();
        entries.sort_by(|a, b| {
            b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(entries)
    }

    pub fn mkdir(&self, path: &str) -> Result<(), String> {
        runtime().block_on(self.sftp.create_dir(path)).map_err(said)
    }

    /// Removes a file, or an empty folder.
    pub fn remove(&self, path: &str, dir: bool) -> Result<(), String> {
        match dir {
            true => runtime().block_on(self.sftp.remove_dir(path)).map_err(said),
            false => runtime().block_on(self.sftp.remove_file(path)).map_err(said),
        }
    }

    /// Copies a remote file to `local` through `local.part`; `progress` is told the bytes done
    /// and the whole.
    pub fn download(
        &self,
        remote: &str,
        local: &Path,
        progress: impl Fn(u64, u64),
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        let part = local.with_file_name(format!("{}.part", file_name(local)?));
        let done = runtime().block_on(async {
            let mut file = self.sftp.open(remote).await.map_err(said)?;
            let whole = file.metadata().await.map(|it| it.len()).unwrap_or(0);
            let mut out = std::fs::File::create(&part).map_err(said)?;
            let (mut buffer, mut got) = (vec![0u8; CHUNK], 0u64);
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err("cancelled".to_string());
                }
                let n = file.read(&mut buffer).await.map_err(said)?;
                if n == 0 {
                    break;
                }
                out.write_all(&buffer[..n]).map_err(said)?;
                got += n as u64;
                progress(got, whole.max(got));
            }
            out.sync_all().map_err(said)
        });
        match done.and_then(|()| std::fs::rename(&part, local).map_err(said)) {
            Ok(()) => Ok(()),
            Err(why) => {
                let _ = std::fs::remove_file(&part);
                Err(why)
            }
        }
    }

    /// Copies a local file to `remote` through `remote.part`.
    pub fn upload(
        &self,
        local: &Path,
        remote: &str,
        progress: impl Fn(u64, u64),
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        let part = format!("{remote}.part");
        let done = runtime().block_on(async {
            let mut input = std::fs::File::open(local).map_err(said)?;
            let whole = input.metadata().map_or(0, |it| it.len());
            let mut file = self.sftp.create(part.as_str()).await.map_err(said)?;
            let (mut buffer, mut sent) = (vec![0u8; CHUNK], 0u64);
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err("cancelled".to_string());
                }
                let n = std::io::Read::read(&mut input, &mut buffer).map_err(said)?;
                if n == 0 {
                    break;
                }
                file.write_all(&buffer[..n]).await.map_err(said)?;
                sent += n as u64;
                progress(sent, whole.max(sent));
            }
            file.shutdown().await.map_err(said)?;
            // SFTP 3 renames onto nothing: what was there goes first.
            let _ = self.sftp.remove_file(remote).await;
            self.sftp.rename(part.as_str(), remote).await.map_err(said)
        });
        if done.is_err() {
            let _ = runtime().block_on(self.sftp.remove_file(part.as_str()));
        }
        done
    }
}

fn file_name(path: &Path) -> Result<String, String> {
    path.file_name().map(|it| it.to_string_lossy().into_owned()).ok_or_else(|| "no file name".into())
}

/// Whether a remote name can be a local file's name: no separator, drive, `..` or control.
pub fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.chars().any(|c| matches!(c, '/' | '\\' | ':') || c.is_control())
}

/// A remote folder and a name in it.
pub fn join(dir: &str, name: &str) -> String {
    match dir {
        "/" => format!("/{name}"),
        _ => format!("{}/{name}", dir.trim_end_matches('/')),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_paths() {
        assert!(safe_name("config.yaml") && safe_name("tệp"));
        for bad in ["", ".", "..", "a/b", "a\\b", "C:x", "a\nb"] {
            assert!(!safe_name(bad), "{bad:?}");
        }
        assert_eq!(join("/", "etc"), "/etc");
        assert_eq!(join("/home/t/", "a"), "/home/t/a");
    }
}
