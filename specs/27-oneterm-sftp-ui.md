# 27. OneTerm SFTP Browser (`apps/oneterm/sftpui`)

Port of the original SFTP browser (crate `sftp-ui` at v0.6.11, screenshot
`ssh-sftp-client.png`) to V on Quadra. The panel lives in the right dock under the
`SFTP Browser` header (specs/22); it talks to the SFTP client of specs/23.

## Public API

```v
pub fn new_panel(mut cx core.Cx, opts PanelOptions) core.View[SftpPanel]
pub fn set_session(mut cx core.Cx, v core.View[SftpPanel], s ?&ssh.Sftp)
pub fn set_client(mut cx core.Cx, v core.View[SftpPanel], c Client, key u64) // tests and fakes
pub fn clear_client(mut cx core.Cx, v core.View[SftpPanel])
pub fn follow_cwd(mut cx core.Cx, v core.View[SftpPanel], path string)        // '' = unknown
pub fn set_edit_settings(mut cx core.Cx, v core.View[SftpPanel], s EditSettings)
pub fn set_shortcuts(mut cx core.Cx, v core.View[SftpPanel], m map[string]string)
pub fn run_action(mut cx core.Cx, v core.View[SftpPanel], a SftpAction)
pub fn set_expanded(mut cx core.Cx, v core.View[SftpPanel], on bool)
pub fn expand_button(mut cx core.Cx, v core.View[SftpPanel]) core.Element      // dock header action
pub fn dispose(mut cx core.Cx, v core.View[SftpPanel])                         // on app exit
```

`PanelOptions`: `edit_dir` (root for working copies, under the app's config directory),
`state_path` (the app's `docks.json`, key `sftp_table_state`; `''` = no persistence),
`local_dir` (first folder of the Local pane; default home), `editor` / `edit_limit`
(`EditSettings`, also updatable), `launch` (runs the configured editor; injected in tests),
`on_expanded` (fires on every expand / collapse, so the host dock can give the browser all
of its space).

`Client` is the interface the panel uses, implemented by `ssh.Sftp` and by the test fake.
Every call on it blocks and runs on a worker task.

## Layout

```
+--------------------------------------------------------------------------+
| Download  Remote ---[path]----------------------  <-  sync  refresh  :    |  40 px toolbar
| Name                     v | Date Modified      v | Permissions         v |  30 px header
| [icon] file                 2026-02-09 14:07      -rw-r--r-- (0644)      |  30 px rows
| ...                                                                      |
+--------------------------------------------------------------------------+
| Transfers  1 active, 2 done                                   Clear      |  28 px header
| v name.ext                     ======            23%   x                 |  24 px rows
+--------------------------------------------------------------------------+
```

- No backend: `No SFTP connection.` centered (with the Local pane beside it when expanded).
- Expanded: a horizontal split, Local on the left, Remote on the right. The toolbar of the
  Remote pane gains `Download` and the `Remote` label on its left; the Local toolbar shows
  the `Local` label and an `Upload` button after its menu. Dock header: the expand / collapse
  toggle (`expand_button`).
- Sort cycle per header click: none, descending, ascending; folders always first; the
  selection follows its entry across a sort. Name is pinned; the other columns scroll
  sideways with the wheel, width from dragging the right edge of a header, saved per column.
- Name column fills the leftover width, 100 px minimum.
- Listing failure keeps the previous rows under a banner `Could not open "<cwd>": <error>`.
  Path input with a wrong path shows a red bottom border until the text changes.
- Loading: skeleton rows. Empty: `Empty directory.`

Menus (one list, three renderings, see `SftpAction`): Open, Edit, Download / Upload Files,
Upload Folder, New Folder / Rename, Delete / Properties, Refresh. A directory drops Edit. The
empty area keeps only what needs no selection. The toolbar `⋮` menu adds Follow Terminal Cwd
and the Columns chooser (Name always on).

## Behaviour

- `set_session` keeps per-backend state (cwd, entries, sort, selection, transfers, follow
  flag): switching tabs restores it, a transfer running on a hidden backend keeps its
  progress. Closed backends are purged on the 500 ms tick.
- Listings are generation-guarded; a late answer for an older request is dropped. Relative
  paths (the initial `.`) resolve through `realpath` so an empty listing still knows its cwd.
- Upload: file picker, folder picker, Local pane or OS drop. The remote folder is listed
  first; collisions are matched case-insensitively and asked about once (Replace / Skip or
  Cancel; a folder is "merged"). The batch is sequential; a failure does not stop it.
- Download: a Save dialog, or straight into the Local pane's folder when expanded (asking
  before replacing). Both show in the queue and can be cancelled.
- Edit: size gate (`edit_limit`, 0 = none), download to
  `<edit_dir>/<pid>/<session>/<sanitized name>`, open with the configured editor (or the OS
  default), poll the working copy every 500 ms, ask before uploading (per-session
  `Always upload this file while editing`), warn on a remote change since the baseline, then
  upload through the queue. Copies are removed when the session or panel ends; the next start
  sweeps folders of dead processes (a held file lock marks a live process).
- Follow cwd: the app pushes the active terminal's OSC 7 path with `follow_cwd`; the sync
  button jumps to it, the menu toggle follows every change.

## Security

Remote names are untrusted. Before a remote name becomes a local path component it passes
`ssh.validate_remote_entry_name` (refused otherwise), or is sanitized for a working copy to a
single safe component (`sanitize_file_name`). Names typed by the user go through
`validate_entry_name` (not empty, no `/` or `\`, not `.` or `..`) and must also be
single-component on the remote.

## Decisions

Recorded here because the original is ambiguous, or because the framework differs.

1. **Path box, not breadcrumbs.** The shipped crate and screenshot have an editable path
   input; the breadcrumb in the old design document never landed.
2. **The fourth toolbar icon is "sync to terminal cwd"**, as in the crate, not "new folder".
   New Folder is in the menu.
3. **Default columns: Name, Date Modified, Permissions** (the screenshot and the task),
   not the crate's newer Name, Size, Date Modified. Size, Owner and Group are one click away
   in the Columns menu.
4. **No breadcrumbs, no file icon by type**: two icons as in the crate (folder / file).
5. **`follow_cwd` is pushed** by the app instead of polled from shared terminal state.
6. **Saves are detected by polling** the working copy's (mtime, size) on the 500 ms tick,
   instead of an OS directory watcher; while the mtime is less than two seconds old a content
   hash covers its one-second resolution. A save is handled when the stamp stays still for
   one tick, which replaces the 400 ms debounce.
7. **The file table is built here**: `kit.data.Table` has string cells only, no per-row
   context menu, double click or drag source (see the report).
8. **Column state and dual-pane fields persist in `docks.json`** under `sftp_table_state`
   (widths, visibility, expanded, local_dir), written with a read-merge-write like
   `save_docks`, debounced 1 s.
