# 25. OneTerm services: vault, sessions, update, crash, terminal log

The UI-free services of OneTerm, ported 1:1 from the Rust app (v0.6.11). Five V modules live under
`apps/oneterm/`, each imported from the app by its directory name:

| Module | Port of | Holds |
|---|---|---|
| `store` | `core::persistence`, `core::schema`, `session-ui` store and tree logic, `state::persist_queue` | atomic JSON documents, `ssh_session.json`, groups, colours, search |
| `vault` | `crates/vault`, `state::vault_state` | master password, `vault.json`, `otv1:` envelopes, lock state machine |
| `update` | `crates/update` | GitHub Releases check, digest check, staging, install with rollback, `update_config.json` |
| `crash` | `app::crash_report`, `app::native_crash` | `crashes/*.crash.txt`, panic and native capture, path redaction |
| `termlog` | `terminal::logging`, `core::terminal_logging` | per-terminal printable-output log files |

Dependencies: `vault` and `update` import `store` for the persistence primitives. `store` imports
neither (a sealed password is an opaque string there). `crash` and `termlog` import nothing of ours
except `quadra.platform.win`. No module touches the UI, the clipboard or a real home directory.

## Rules every module follows

- Every configuration or data path comes from a parameter. There is no `config_dir()` here; the app
  passes its data folder. Tests pass `.tmp/...` under `@VMODROOT`.
- `read_only bool` is the port of "elevated window": it is a parameter, read at start by the app, and
  it is checked inside the deepest write function of each document, never at call sites.
- Filesystem and network work blocks. The UI runs it on a worker and applies the result on the UI
  thread. Long operations are split `begin_*` / `Job.run()` / `apply` for that reason.
- Windows functions are loaded at run time through `win.load_system_dll` / `win.dll_proc`.
- Files stay under 300 lines; the Rust test suites are split per subject.

## Decisions (user away; recorded here)

1. **`vault.json` stays byte-compatible** with the Rust app: same schema 1, same Argon2id parameters,
   same AES-256-GCM, same associated data, same `otv1:` envelope. `crypto.argon2` and `crypto.aes`
   (`new_aes_gcm`) in vlib reproduce the frozen vectors, and the Rust fixture `vault-v1.json` opens and
   decrypts a Rust-sealed password in `vault/document_test.v`. Existing files keep working.
2. **Number fidelity.** `x.json2` decodes every number to `f64`, which would write `65536` as
   `6.5536e+04` and break the Rust loader's integer fields. `store.parse_json` normalizes whole
   numbers to `i64` after decoding, so documents round-trip with integers; unknown fields survive.
   Ceiling: whole numbers above 2^53 lose precision (none are used by these documents).
3. **Strict base64**: `encoding.base64.decode` is lenient, so envelopes and salts are accepted only
   when re-encoding the decoded bytes gives back the input (padding and charset included).
4. **Semver is our own** (`update/version.v`): vlib `semver` orders `1.2.3` below `1.2.3-rc.1`.
5. **Atomic replace** uses `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` loaded at run time, because
   `os.rename` on Windows is `_wrename`, which refuses an existing destination.
6. **Lock files** are `os.filelock` files `.<document>.lock` beside the document, taken with a 10 s
   wait. Unlike the Rust ones they are deleted at release (V's implementation); the guarantee, one
   writer at a time across processes, is the same.
7. **Vault and store state machines without an executor.** No `Entity`/`Task`, so the Rust `run`
   shape becomes `begin_*` (checks and marks busy, returns a `VaultJob`), `VaultJob.run()` (the
   Argon2id work, no shared state, any thread), `apply(result)` on the owner's thread. A dropped job
   cannot exist (the owner applies what it spawned). Events are drained with `take_events()`.
8. **Key material**: the data key and every derived key are `[]u8` zeroed on lock or as soon as they
   are not needed. Limits, stated plainly: V strings are immutable (the typed password cannot be
   cleared by this module), vlib's Argon2 working memory and AES key schedule are freed by the
   collector without being cleared.
9. **V panic capture.** V has no panic hook: `panic()` prints to stderr and calls `exit(1)`. So
   `crash.install` redirects stderr to the staging file `<identity>.stderr.tmp` whose header (version,
   OS, architecture) is written first, and the next start promotes it to `.crash.txt` only when it
   contains `V panic` (a clean exit leaves no marker and the file is deleted). The report carries V's
   message and backtrace as printed; "Thread" and "Location" are not separate fields.
10. **Native capture** (OS half in `quadra/native/sys`; the report headers stay in `crash/capture.v`):
    `SetUnhandledExceptionFilter` through `dll_proc`, a pre-opened handle and
    `WriteFile` on a bounded stack buffer, chaining to the previous filter, as the Rust callback does.
    CRT invalid-parameter/purecall hooks and the out-of-memory allocator are not ported: V code calls
    neither CRT path, and the collector has its own failure mode.
11. **Staging owner liveness**: `OpenProcess` denied means alive, invalid parameter means dead. The
    current process's own identity is always skipped (Rust relies on share-delete there).
12. **Update identity** (`current_version`, `target_triple`, `repository`) are parameters; the defaults
    are the constant `update.repository_default` (`vnStrawHat/OneTerm`) and `current_target_triple()`
    (`x86_64-pc-windows-msvc` on Windows x64). Asset names match the Rust release workflow, so the V
    build would pick up `oneterm-<version>-x86_64-pc-windows-msvc.zip` from that repository.
13. **Certificates**: `net.http` defaults to `validate: false`; `GitHubClient` always passes
    `validate: config.verify_certificates`, and logs a warning when off.
14. **Zip reading** is our own small central-directory reader over `compress.deflate`, because
    `compress.szip` exposes no external attributes (symlink detection) or expansion budget. Tar is read
    with a small header walker over `compress.gzip`. Both load the archive in memory; the download cap
    (512 MiB) bounds it. Ceiling: a very large archive costs that much memory while staging.
15. **Helper script location**: the Windows install helper `.cmd` lives in the update cache directory,
    not in the user temp folder.
16. **Terminal log stripper** is a small ground/ESC/CSI/OSC/DCS state machine: `quadra.kit.term.Parser`
    feeds a `Screen` and is being changed by others.
17. **Signature verification** stays out, exactly as in the Rust app: TLS plus the published SHA-256.

## store

`store.atomic_write`, `atomic_write_sealed`, `update_json_file`, `update_secret_json_file`,
`remove_document`, `quarantine_file`, `backup_path`, `parse_json`, `encode_pretty`, `schema_version`,
`migrate_json_value`, `PersistQueue`: as `docs/agents/persistence.md` describes.

- Sensitivity: ordinary (`.bak` = previous generation), private (same), secret (`.bak` = new document).
- Quarantine name: `.<name>.invalid-<pid>-<unix>-<sequence>`, never overwriting an earlier one, and a
  no-op (`none`) when `read_only`.

Session document (`ssh_session.json`, schema 2): v0 array and v1 object migrate in memory and re-save,
ids assigned by position, hand-edited rows without a unique id repaired. A file that does not parse
is quarantined and the store starts empty; any other read failure sets `persist_blocked` and the
store never writes. `sealed_password` non-string loads as absent and is written back byte for byte.
`.bak` is scrubbed only when a write drops or replaces a sealed value.

`SessionStore`: `sessions`, `get`, `add`, `update`, `remove`, `duplicate`, `set_group`, `rename_group`,
`set_sealed_password`, `clear_sealed_passwords`, `jump_chain` (`max_jump_hops` = 4). Saves go through a
newest-wins single-flight `PersistQueue`, drained on a spawned thread (`flush()` blocks until idle).

Tree logic: `build_tree(sessions, query)` (ungrouped first by label, then groups sorted, trimmed names,
case-insensitive match on label/host/user/group), `menu_entries` (store order), `session_color_hex`,
`session_subtitle`.

## vault

`init` (create-only), `unlock` (uses the header held in memory, never the file), `reset`, `UnlockedVault`
(`seal`, `open`, `change_password`, `needs_rehash`, `lock`). KDF floor 19 MiB/2/1, default 64 MiB/3/1,
ceiling 1 GiB/16/4. `vault.json` is never quarantined and never defaulted; rewrap is refused unless the
file is still the unlocked header (`vault_replaced`, `changed_elsewhere`) and preserves unknown fields.

`VaultState`: phases uninitialised / locked / unlocked / unavailable plus `screen_locked`, back-off after
4 wrong passwords (2, 4, 8, 16, then 30 s), lock-while-busy wins, seal re-reads the file and drops the
key when the vault was reset elsewhere, a usable vault starts screen-locked. `VaultEvent`:
`initialised`, `unlocked`, `locked`, `screen_locked`, `reset` (emitted before the file goes, so owners
wipe their sealed values first).

## update

`UpdateConfig` with field-level merges (`save_preferences_to`, `UpdateCheckCache.save_to`), quarantine
of unreadable documents, ETag reuse only for the same app version, `should_auto_check_at`. `UpdateManager`
takes a `ReleaseClient` (the production one is `GitHubClient`, tests pass a canned double), filters
drafts, prereleases by channel, invalid tags and skipped versions, requires exact asset name, digest and
HTTPS. `download_and_stage` caps the body, verifies SHA-256 before extraction, extracts under a 2 GiB
budget refusing absolute, parent, drive-rooted and symlink entries, validates the package and removes
the staging directory on any failure.

Install: `install_staged_update(staged, current_exe, cache_dir, read_only)` is `manual_install` when
the install directory is not writable. Windows writes the helper `.cmd` and spawns it hidden with paths
in the environment; Unix replaces only the shipped entries with `replace_directory_contents_and_launch`
and rolls back on any failure. A `read_only` window installs nothing.

Before the Windows helper starts, `sys.terminate_console_hosts_in(install_dir)` ends every
`OpenConsole.exe` whose image path lies inside the install directory (the bundled ConPTY host, specs/17):
one that outlived the app would hold the files the helper overwrites, and the update would roll back.
It matches on the full image path, never the bare name, so other apps' hosts are untouched; each one
ended is written to `update.log`.

## crash

`crashes_dir_in`, `prepare_capture_paths`, `install` (see 9, 10), `load_pending_reports` (promotes dead
owners' staging, combines native and panic reports for one identity, redacts the home prefix and rewrites
the file, keeps the newest 20), `delete_pending_report` (refused outside the managed store), `redact_user_home`,
`new_issue_url`. File name: `YYYYMMDDTHHMMSSmmmZ-p<PID>-<8 hex>.crash.txt`. A read-only window uses
`crashes/elevated/` and the app shows no dialog there.

## termlog

`TerminalLogController`: `start`, `stop`, `process`, `state`, `take_error`, `set_identity`. Files
`%n_%Y-%m-%d_%H-%M-%S.log`, records `[%Y-%m-%d %H:%M:%S] %msg`, append or overwrite, identity sanitized.
Printable text only (no escape sequences, no control characters); CR and LF end a record, BS removes the
last character. A failure closes the writer and surfaces through `take_error`; the terminal is never
blocked. Thread-safe through a mutex.

## Tests

Ported from the Rust suites into `*_test.v` beside the code. Nothing touches the network, GitHub, the
clipboard or a home directory; install and rollback run in `.tmp/`; no real executable is replaced.

## Not ported (UI or framework work, listed so nothing is lost)

- Dialogs, tree rendering, settings pages, tab markers (the UI agent uses the APIs above).
- `docks.json` and the rest of `crates/state` that is UI layout.
- The Windows helper's real run cannot be exercised by tests (manual smoke).
- macOS bundle replacement: a macOS install reports `manual_install` until the platform exists.
- CRT-level panic hooks: `SIGSEGV` is reset to default in `crash.install` so an access violation reaches
  the exception filter; V's own handler would exit first and lose the exception code.
