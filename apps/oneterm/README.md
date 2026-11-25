# OneTerm

A terminal app for Windows: local shells in tabs and spaces side by side, SSH with saved
hosts, an SFTP dock, command completion, a master password. It is built on
[Quadra](https://github.com/vnStrawHat/Quadra), a native GPU-rendered UI framework, and on
the terminal engine `oneterm-vt`.

Versions go on from 0.7.0, the release after 0.6.11. OneTerm 0.6 and before were built on another toolkit; 0.7 is made
to take over their hosts, saved passwords and settings at its first start, and to be the
update their updater installs.

## Layout

| Path | What it is |
|---|---|
| `crates/app` | The app (package `oneterm-app`): its library and its binary `oneterm`, its themes, icon and tests |
| `crates/ssh` | SSH and SFTP over `russh` |
| `crates/completion` | Command completion: catalogs, the line at the cursor, ranking, history |
| `crates/vault` | Saved passwords: the vault's format, sealing and opening |
| `crates/shots` | The app's screens drawn to pictures without a window |
| `scripts` | The release asset, the console host, the notices, the release notes |
| `specs` | What the app does: `19-oneterm.md`, `20-complete.md` |
| `.github/workflows` | `ci.yml` (format, lints, tests; licences, bans, advisories and sources by cargo-deny, `deny.toml`) and `release.yml` |

## Build

Windows, the Rust toolchain `rust-toolchain.toml` pins, Git Bash for the scripts.

```sh
cargo run -p oneterm-app          # the app
cargo test --workspace            # its tests: no window, no GPU; SSH against 127.0.0.1 only
cargo deny check licenses bans advisories sources   # the dependency policy (deny.toml)
scripts/conpty.sh                 # Microsoft's newer console host beside the binaries (optional)
scripts/build-release.sh          # dist/oneterm-<version>-<target>.zip and its .sha256
```

A trial run takes `ONETERM_HOME=<folder>` so that it reads and writes nothing of your own
OneTerm.

## Release

The app's version is `version` in `crates/app/Cargo.toml`. The workflow **Release** is run
by hand with a bump (patch, minor, major), or by pushing a tag `v<version>` once the manifest
has that version. The manifest has the version last released: 0.6.11 now, so that a minor
bump releases 0.7.0. It builds the asset with `scripts/build-release.sh`, commits the bump as
`chore(release): v<version>` and publishes the release with the zip and `SHA256SUMS`. The
app's updater asks a release for `oneterm-<version>-x86_64-pc-windows-msvc.zip` and checks
the digest GitHub publishes for it.

## Inside Quadra's repository

Until it has a repository of its own, this folder is `apps/oneterm` of Quadra's repository,
its crates are members of Quadra's workspace, and Quadra's gate checks them
(`tools/check.sh`). There, `Cargo.root.toml` is the root manifest this folder will have, and
`scripts/export.sh <folder>` writes the folder out as that repository.

## Licence

Apache-2.0 (`LICENSE`, `NOTICE`). What a release holds of others is listed in its
`THIRD-PARTY-NOTICES.txt` (`scripts/notices.py`).
