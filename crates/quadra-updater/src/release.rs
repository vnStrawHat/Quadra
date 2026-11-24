//! What is published (specs/22, "Checking", "Fetching"): the latest release of a repository,
//! the package in it for this build, and that package fetched, checked and unpacked.

use std::path::PathBuf;

use serde_json::Value;

use crate::{Error, Updater, sha256};

/// A fetch of a package whose size GitHub did not say stops here.
const MOST: u64 = 512 * 1024 * 1024;

/// A release newer than what runs, and its package for this build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// `1.4.0`, without the tag's `v`.
    pub version: String,
    /// The release's page, where its notes are.
    pub notes: String,
    /// The package's name, address, size in bytes and SHA-256 in hex, as GitHub publishes them.
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

/// `1.4.0` or `v1.4.0` as its three numbers; none of anything else, a pre-release included.
pub fn version(tag: &str) -> Option<[u64; 3]> {
    let mut numbers = tag.trim().trim_start_matches('v').split('.').map(|it| it.parse::<u64>().ok());
    let all = [numbers.next()??, numbers.next()??, numbers.next()??];
    numbers.next().is_none().then_some(all)
}

/// Only what TLS carried is fetched; a server of the machine itself is one a test stands up.
fn allowed(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://127.0.0.1:")
}

impl Updater {
    /// The latest release (GitHub's: no draft, no pre-release), if it is newer than what runs
    /// and has this build's package with a digest. `Ok(None)`: what runs is the latest.
    pub fn check(&self) -> Result<Option<Release>, Error> {
        let url = format!("{}/repos/{}/releases/latest", self.api, self.repo);
        let mut said = Vec::new();
        let mut all = |piece: &[u8]| {
            said.extend_from_slice(piece);
            said.len() < 4 * 1024 * 1024
        };
        let status = allowed(&url).then(|| self.get(&url, "application/vnd.github+json", &mut all));
        match status.unwrap_or_else(|| Err(format!("not asked: {url}"))).map_err(Error::Network)? {
            200 => {}
            // A repository with no release yet.
            404 => return Ok(None),
            403 | 429 => return Err(Error::Limited),
            other => return Err(Error::Network(format!("GitHub answered {other}"))),
        }
        let release: Value = serde_json::from_slice(&said).map_err(|e| Error::Network(e.to_string()))?;
        let text = |of: &Value, key: &str| of[key].as_str().unwrap_or_default().to_string();
        let Some(newer) =
            version(&text(&release, "tag_name")).filter(|it| Some(*it) > version(&self.version))
        else {
            return Ok(None);
        };
        let shown = format!("{}.{}.{}", newer[0], newer[1], newer[2]);
        let name = format!("{}-{shown}-{}.zip", self.app, self.target);
        let assets = release["assets"].as_array().map(Vec::as_slice).unwrap_or_default();
        let package = assets.iter().find(|it| it["name"] == name.as_str()).and_then(|it| {
            let digest = text(it, "digest").strip_prefix("sha256:")?.to_ascii_lowercase();
            let hex = digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit());
            let url = text(it, "browser_download_url");
            (hex && allowed(&url)).then(|| (url, it["size"].as_u64().unwrap_or(0), digest))
        });
        let Some((url, size, sha256)) = package else { return Err(Error::NoPackage(shown)) };
        Ok(Some(Release { version: shown, notes: text(&release, "html_url"), name, url, size, sha256 }))
    }

    /// Fetches a release's package into the cache, checks it against its digest, unpacks it,
    /// and gives the folder that holds the app's files. `progress` is told the bytes that came
    /// and the package's size, and stops the fetch by answering `false`. What a fetch before
    /// left of this version is removed first; nothing unchecked is unpacked.
    /// ponytail: the package is held in memory while it comes (tens of megabytes); written
    /// through to its file when packages grow past that.
    pub fn fetch(
        &self,
        release: &Release,
        progress: &mut dyn FnMut(u64, u64) -> bool,
    ) -> Result<PathBuf, Error> {
        let (most, mut bytes, mut stopped) =
            (if release.size > 0 { release.size } else { MOST }, Vec::new(), false);
        let mut take = |piece: &[u8]| {
            bytes.extend_from_slice(piece);
            stopped = !progress(bytes.len() as u64, release.size);
            !stopped && bytes.len() as u64 <= most
        };
        let status = self.get(&release.url, "application/octet-stream", &mut take);
        match status {
            _ if stopped => return Err(Error::Cancelled),
            Ok(200) => {}
            Ok(other) => return Err(Error::Network(format!("the package's address answered {other}"))),
            Err(why) => return Err(Error::Network(why)),
        }
        let hex: String = sha256(&bytes).unwrap_or_default().iter().map(|b| format!("{b:02x}")).collect();
        if hex != release.sha256 {
            return Err(Error::Mismatch);
        }
        let folder = self.cache.join(format!("{}-{}", self.app, release.version));
        let unpack = |why: std::io::Error| Error::Unpack(why.to_string());
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).map_err(unpack)?;
        crate::zip::unpack(&bytes, &folder).map_err(unpack)?;
        // The package's own folder, named as the package is, or its files at the top.
        let exe = format!("{}.exe", self.app);
        let inside = folder.join(release.name.trim_end_matches(".zip"));
        let package = [inside, folder].into_iter().find(|it| it.join(&exe).is_file());
        package.ok_or_else(|| Error::Unpack(format!("{exe} is not in {}", release.name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_three_numbers_and_orders_as_numbers() {
        assert_eq!(version("v0.7.0"), Some([0, 7, 0]));
        assert_eq!(version(" 1.10.3 "), Some([1, 10, 3]));
        assert!(version("0.10.0") > version("0.9.9") && version("1.0.0") > version("0.99.99"));
        for no in ["", "1.2", "1.2.3.4", "1.2.x", "0.7.0-rc.1", "nightly", "v1.2.3+build"] {
            assert_eq!(version(no), None, "{no}");
        }
        assert!(allowed("https://github.com/o/r/releases/download/v1/a.zip"));
        assert!(allowed("http://127.0.0.1:4000/a.zip") && !allowed("http://example.com/a.zip"));
        assert!(!allowed("http://127.0.0.1.example.com/a.zip") && !allowed("file:///c:/a.zip"));
    }
}
