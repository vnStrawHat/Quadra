# 22. Updater: an app's updates from GitHub Releases

`quadra-updater` lets an app find, fetch and install its own newer releases from the
repository that publishes them, so who uses it never visits a release page. It is safe Rust
over two calls of the OS in `quadra-shell` (specs/16, "Fetching"): the OS's HTTP, which
brings the system's proxy and certificates, and its SHA-256.

## Goals

- **Three steps an app shows as it likes**: check, fetch, install. Each is a plain function
  that blocks; the crate knows no window, signal or task, so an app runs them on its pool
  (specs/11) and shows what they answer. Nothing is done unasked: an update that was
  fetched closes no session and restarts nothing.
- **Nothing unchecked is unpacked or run.** A package is installed only when its SHA-256 is
  the digest GitHub publishes for it, and only addresses TLS carried are asked.
- **The app keeps running while it is updated**, and either the whole package is put or
  nothing changed.
- **Few dependencies** (specs/01, R15): HTTP, TLS, proxies and hashing are the OS's; the
  JSON GitHub answers is read with `serde_json`; a zip is read by the crate itself, its
  files inflated by `miniz_oxide`. No program is started to unpack: an archiver is not in
  every Windows.

Not goals, until an app needs them: pre-release channels, a release skipped on purpose,
patches between versions, a signature beside the digest, installing into a folder that
needs elevation, systems other than Windows.

## The release an app publishes

A release of version `1.4.0` is tagged `v1.4.0` or `1.4.0` and carries, for each target it
is built for, the asset `<app>-1.4.0-<target>.zip` (`oneterm-1.4.0-x86_64-pc-windows-msvc.zip`).
The zip holds one folder named as the zip is without `.zip`, with `<app>.exe` and whatever
ships beside it, in the layout of the app's folder; a zip with those files at its top is
taken too. GitHub publishes each asset's SHA-256 (`digest`, `sha256:<hex>`) by itself.

## Checking

`Updater::new(repo, app, version, cache)` describes the app that runs: `repo` is
`owner/name`, `version` what runs (`env!("CARGO_PKG_VERSION")`), `cache` a folder of the
app's for what is fetched; `target` (this build's, `<arch>-pc-windows-msvc`), `api`
(`https://api.github.com`), `home` (the folder of the running program) and `network` (the
system's proxy, certificates checked: `quadra_shell::Network`, which an app sets from its own
settings before it checks or fetches) are filled in and,
as every field, can be set, which is how a test names a server and folders of its own.

`check() -> Result<Option<Release>, Error>` asks `GET <api>/repos/<repo>/releases/latest`,
GitHub's newest release that is no draft and no pre-release, with the `User-Agent`
`<app>/<version>`. A version is three numbers, compared as numbers (`0.10.0` is newer than
`0.9.9`); a tag that is anything else is no release to take. The answer:

- `Ok(None)`: what runs is the latest (the same or an older tag, or no release yet: 404).
- `Ok(Some(release))`: a newer one with this build's asset, an `https` address and a digest
  of 64 hex digits: its `version`, `notes` (the release's page), and the asset's `name`,
  `url`, `size`, `sha256`.
- `Err(NoPackage(version))`: a newer one without that asset or without its digest.
- `Err(Limited)`: GitHub refuses for now (403, 429). `Err(Network(detail))`: no answer, any
  other status, or an answer that is no release.

An `api` that is not `https` is not asked, but for `http://127.0.0.1:<port>`, a server of
the machine itself, which is what tests stand up; the same holds for an asset's address.

## Fetching

`fetch(&release, progress) -> Result<PathBuf, Error>` gets the asset, telling `progress`
the bytes that came and the size (it stops the fetch by answering `false`:
`Err(Cancelled)`); no more than the size GitHub said is taken (512 MiB where it said none).
Then, in this order: the SHA-256 of what came must be the release's (`Err(Mismatch)`
otherwise, and nothing is written); the zip is unpacked from memory into
`<cache>/<app>-<version>/` (emptied first) by `zip.rs`: the files its central directory
lists, stored or deflated, each checked against its length and CRC-32. Refused, with
nothing more written: an entry that would land outside the folder (`..`, a drive, a root, a
backslash), a link, an encrypted entry, another method, more than 2 GiB unpacked, and an
archive that needs zip64. The folder holding `<app>.exe` is the answer (`Err(Unpack(why))`
when there is none). `install` removes that folder once the files are in place.

## Installing

A program's file cannot be written over or deleted while it runs, but it can be renamed
where it is. `install(&package)` (`swap(package, home)`) puts each file of the package at
its place under `home`: the file there is renamed `<name>.quadra-old` beside itself, then
the new one is copied in. The app goes on running the code it loaded; the files at its
names are the new version. All or nothing: when one file cannot be put, those put are
removed and what was moved aside is moved back (`Err(Install(why))`, the OS's words: a
folder that cannot be written to, a second update before the app was started again). Files
of `home` the package does not have stay.

`restart()` starts the program at the app's name again, marked (the environment variable
`QUADRA_UPDATED`) as the start after an update; the app then closes its window as it
always does. Nothing forces it: an app that only installs is updated at its next start.

`started()` goes at the top of `main`, before the app asks to be the one instance
(specs/16): where a file was moved aside for the program itself, it removes what was moved
aside (`swept(home)`); started by `restart()`, it first waits, 10 s at most, for the run
before to end, which is when that file can be removed, so a one-instance app does not find
the old run still there. Without an update it looks at one file's name and returns.

There is no rollback of a version that starts and misbehaves: the files moved aside are
gone once the new version started. An app that needs one keeps the package before.

## Errors

`Error` says in words what happened, for who uses the app (`Display`): GitHub could not be
reached, it answers no more requests for now, the version has no package for this system,
the package is not the one that was published, it could not be unpacked, the update could
not be installed, cancelled. The detail (the OS's error, a status) is in `Debug`, for a log.
Nothing panics on what the network or the disk does.

## Tests

`tests/update.rs`, against a server of the test on 127.0.0.1 that answers as GitHub does and
folders under `.tmp`: the check's answers by tag, asset and digest; a zip fetched, checked
and unpacked, a wrong digest and a stop refused; a zip's entries that would land outside,
a changed byte and a cut archive refused (`zip.rs`); the files of a program that runs replaced,
the second update refused, a failed one taken back, and the files moved aside removed once
the program ended. The OS's calls have their own (specs/16). `quadra_updater::testing` is
that server, what GitHub says of a release, and a package as a release carries it (a zip
it writes itself), for the
tests of an app that updates.
