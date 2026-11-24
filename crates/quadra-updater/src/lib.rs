//! An app's updates from GitHub Releases (specs/22): whether a newer release is published,
//! its package fetched and checked against the digest GitHub publishes for it, and its files
//! put in the app's folder while the app runs, so the next start is the new version.
//!
//! Nothing here is asynchronous or knows a window: [`Updater::check`] and [`Updater::fetch`]
//! block on the network, so an app calls them on its pool and shows what they answer.
//!
//! ```no_run
//! let updater = quadra_updater::Updater::new("owner/app", "app", "1.2.0", ".cache/updates".as_ref());
//! if let Some(release) = updater.check()? {
//!     let package = updater.fetch(&release, &mut |_got, _of| true)?;
//!     updater.install(&package)?;
//!     quadra_updater::restart()?; // then the app closes
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod install;
mod release;

use std::path::{Path, PathBuf};

pub use install::{restart, started, swap, swept};
pub use release::{Release, version};

/// Why an update did not happen, in words for who uses the app (`Display`); the detail, where
/// there is one, is for a log (`Debug`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// GitHub could not be reached, or answered what is no release.
    Network(String),
    /// GitHub answers no more requests from this address for now.
    Limited,
    /// The newer release (its version) has no package this build can take.
    NoPackage(String),
    /// What was fetched is not what GitHub says it published.
    Mismatch,
    Unpack(String),
    Install(String),
    /// The fetch was stopped by who asked for it.
    Cancelled,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Network(_) => f.write_str("GitHub could not be reached. Check the network connection."),
            Error::Limited => {
                f.write_str("GitHub answers no more requests from this network for now. Try later.")
            }
            Error::NoPackage(version) => write!(f, "Version {version} has no package for this system."),
            Error::Mismatch => {
                f.write_str("The package is not the one that was published. Nothing was installed.")
            }
            Error::Unpack(why) => write!(f, "The package could not be unpacked: {why}"),
            Error::Install(why) => write!(f, "The update could not be installed: {why}"),
            Error::Cancelled => f.write_str("Cancelled"),
        }
    }
}

impl std::error::Error for Error {}

/// Where an app's releases are, and what this build of it is. Every field can be set: a test
/// names a server of its own and folders of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Updater {
    /// `owner/name` on GitHub.
    pub repo: String,
    /// The app's name in its packages: `<app>-<version>-<target>.zip`, holding `<app>.exe`.
    pub app: String,
    /// The version that runs.
    pub version: String,
    /// The target this build is for, as packages name it.
    pub target: String,
    /// The root of GitHub's API.
    pub api: String,
    /// The folder the app runs from, where a package's files are put.
    pub home: PathBuf,
    /// Where a package is fetched to and unpacked; the app's own folder for it.
    pub cache: PathBuf,
}

impl Updater {
    /// The updater of the app that runs: `repo` is `owner/name`, `version` usually
    /// `env!("CARGO_PKG_VERSION")`, `cache` a folder of the app's for what is fetched.
    pub fn new(repo: &str, app: &str, version: &str, cache: &Path) -> Updater {
        let home = std::env::current_exe().ok().and_then(|it| it.parent().map(Path::to_path_buf));
        Updater {
            repo: repo.into(),
            app: app.into(),
            version: version.into(),
            target: format!("{}-pc-windows-msvc", std::env::consts::ARCH),
            api: "https://api.github.com".into(),
            home: home.unwrap_or_default(),
            cache: cache.into(),
        }
    }

    /// Puts the files of a package [`Updater::fetch`] gave in the app's folder ([`swap`]).
    pub fn install(&self, package: &Path) -> Result<(), Error> {
        swap(package, &self.home).map_err(|e| Error::Install(e.to_string()))
    }

    fn headers(&self, accept: &'static str) -> [(&'static str, String); 2] {
        [("User-Agent", format!("{}/{}", self.app, self.version)), ("Accept", accept.into())]
    }

    /// A GET with the app's headers: the status, with the body given to `body`.
    fn get(
        &self,
        url: &str,
        accept: &'static str,
        body: &mut dyn FnMut(&[u8]) -> bool,
    ) -> Result<u16, String> {
        let headers = self.headers(accept);
        let headers: Vec<(&str, &str)> =
            headers.iter().map(|(name, value)| (*name, value.as_str())).collect();
        http_get(url, &headers, body)
    }
}

#[cfg(windows)]
use quadra_shell::{http_get, sha256};

#[cfg(not(windows))]
fn http_get(_: &str, _: &[(&str, &str)], _: &mut dyn FnMut(&[u8]) -> bool) -> Result<u16, String> {
    Err("no HTTP on this system yet".into())
}
#[cfg(not(windows))]
fn sha256(_: &[u8]) -> Option<[u8; 32]> {
    None
}
