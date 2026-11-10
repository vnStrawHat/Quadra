# 26. OneTerm Session panel

Status: `apps/oneterm/sessionui/` (V `module sessionui`). A 1:1 port of the Rust crate `session-ui`
(v0.6.11) and of the master-password prompts it triggers (`state::vault_dialogs`, set-up and unlock).
Data and persistence come from `apps/oneterm/store`, secrets from `apps/oneterm/vault` (specs/25). The
SSH engine (`apps/oneterm/ssh`, specs/23) is never imported: connecting is the app's job.

## Public surface

```v
panel := sessionui.new_panel(mut cx, sessionui.PanelOptions{
	store:      store  // &store.SessionStore
	vault:      vault  // &vault.VaultState
	read_only:  false  // an elevated window opens no SSH dialog
	on_connect: cx.listener(...) // core.Listener[sessionui.ConnectRequest]
	on_cancel:  cx.listener(...) // core.Listener[u64]: the attempt id
})
```

Mount it with `core.view_element(panel.any())`. It renders under the app's "Session" panel header; the
"+" there calls `open_new_session`.

Methods on the panel model (`cx.peek(panel.model())`), `mut cx core.Cx`:

| Method | Rust equivalent |
|---|---|
| `open_new_session(mut cx)` | `open_new_saved_session_dialog`, the header "+" |
| `open_quick_connect(mut cx)` | `open_quick_connect_dialog` |
| `open_duplicate(mut cx, DuplicateSpec)` | `open_duplicate_ssh_dialog` |
| `open_connect(mut cx, id u64)` | `open_saved_ssh_session` |
| `open_selected(mut cx)` / `properties_selected(mut cx)` / `delete_selected(mut cx)` | the rebindable `open_session`, `session_property`, `delete_session` actions |
| `refresh(mut cx)` | the store changed outside the panel (vault reset cleared sealed values) |
| `connect_succeeded(mut cx, attempt u64)` | the `Ok` branch of `connect_ssh_session` |
| `connect_failed(mut cx, attempt u64, message string, authentication bool)` | the `Err` branch; `authentication` is `ConnectPhase::Authentication` |
| `host_key_unknown(mut cx, attempt u64, host string, port int, algorithm string, fingerprint string)` | `HostKeyUnknown`: closes the dialog, shows "Unknown SSH Host Key"; Trust and Connect re-emits the request |

`ConnectRequest` carries everything the app needs to build the engine `Config`:

```v
pub struct ConnectRequest {
pub:
	attempt            u64  // names this attempt in every callback
	session_id         ?u64 // none for Quick Connect and Duplicate
	label              string
	host               string
	port               int
	username           string
	credential         Credential
	hops               []HopRequest // outermost first
	forwards           []store.PortForward
	agent_forwarding   bool
	logging            store.LoggingOverride
	shell_integration  bool
	initial_cwd        string // duplicate-at-cwd; '' for none
	tag                string // the DuplicateSpec's tag, unchanged
	accept_fingerprint string // set on the retry after Trust and Connect
}
```

`accept_fingerprint` on the target or a `HopRequest` maps to `HostKeyPolicy.accept_new(...)`; a hop's
own host and port identify which key was accepted, exactly as the Rust retry does. A retry after
Trust and Connect keeps its attempt id.

`connect_failed` message rules (the app composes the text, as `connect_failure_message` does): an error
that already names itself is passed verbatim; anything else gets the `SSH connect failed: ` prefix. A
jump-host failure message must start with `jump host <user>@<host>:<port>:` so the panel can attribute
a rejected saved password to the right row (specs/23 prefixes host errors with the hop label; the app
passes the address).

## Decisions (user away; recorded here)

1. **Sealing happens in the panel.** Rust seals in the panel's dialogs and the V vault is synchronous for
   seal and open (only KDF jobs run on a worker). The panel calls `vault.seal`, and on success
   `store.set_sealed_password`. The app only builds connections.
2. **Quick Connect and the connect dialog are one view** (`ConnectDialog`, three modes: saved session,
   quick connect, duplicate). Rust has two near-identical files; the differences are fields and the
   save options.
3. **Pickers.** The kit's `Select` popup sits below modals (popups layer at 10+id*2, modals at 1000+id),
   and the Rust field needs search and a create footer. `SearchSelect` (a view of this module, drawn with
   `ui.anchored` at layer 6000) serves the group combobox, the jump-host picker and the forward kind.
4. **Color picker.** No kit component. `Custom...` opens a popover of the same module: the eight featured
   colors, a 13 x 10 generated grid, and a hex field. Rust's kit palette is not in the source tree, so
   the grid is the same 130 swatch count, not the same 130 values.
5. **Search filters on every keystroke.** Rust debounces 300 ms; the store's `build_tree` over a saved
   session list is cheap and the no-results text already updated at once in Rust.
6. **Default swatches** are `#56B6C2`, then fixed One Dark Pro colors (`#E06C75`, `#E5C07B`, `#98C379`,
   `#61AFEF`, `#C678DD`, `#E8848F`, `#83C2F4`; no per-theme lookup): the OneTerm
   theme's `base.*` tokens are not in `kit.Theme`, and a saved session stores a hex anyway.
7. **Secrets in memory** follow specs/25 decision 8: typed passwords are V strings and cannot be zeroed.
   `Credential.str()` is redacted so a stray print cannot leak one; nothing here logs, writes to disk or
   sends a secret anywhere but `on_connect`. Only the sealed `otv1:` value reaches the store.
8. **Attempts** live in the panel (`attempts`, keyed by a counter). `connect_succeeded/failed` for an
   unknown attempt are ignored, so a late result after Cancel is safe.
9. **Close detection without a kit hook.** The kit has no modal close callback, and Escape, the title-bar
   X and an outside click close a modal with no notice. The panel reads the `kit.Overlays` global in
   render; a tracked dialog whose modal is gone is released and, if an attempt is running, cancelled
   (`on_cancel`). Rust's `on_cancel` hook is thus covered for every way of closing.
10. **Move to Group, New group...** keep the Rust behavior: it opens the session dialog focused on Group.
11. **Elevated windows.** `read_only` blocks the four dialog entry points; the panel is otherwise inert,
    since Rust has no panel in an elevated window at all.
12. **Master password prompts ported: set-up and unlock.** Change and reset belong to Settings.
13. **Bind address check is shape only.** `forward_rows.is_ip` recognises IPv4 and the IPv6 shape and
    calls no resolver, since a bind address never needs DNS. A malformed IPv6 of the right shape passes.
14. **Notices are `kit.push_toast`**, the kit's one notification surface, at the Rust call sites.

## Behaviors

Everything below is the Rust crate's behavior; the tests pin these.

- **Tree.** `store.build_tree`: ungrouped sessions by label, then groups sorted with members by label. A
  group row is a chevron plus its name, pl = 16 * depth + 12. A session row: an 8 px color square, the
  label truncated, the right-aligned muted `user@host:port` (`store.session_subtitle`). Groups start
  open; a click toggles. Up/Down move, Right opens or enters, Left closes or goes to the parent, Enter
  toggles a group or opens a session. Right click selects the row first.
- **Empty states.** No sessions: "No SSH session yet. Right-click -> New Session." Search with no
  results: `No sessions found for "<query>".`. The list area keeps the blank-area New Session menu.
- **Context menus.** Session: Open, Properties, Duplicate Saved Session, Move to Group >, separator, New
  Session, separator, Delete (danger). Group: Rename Group..., separator, New Session. Blank area: New
  Session. Delete confirms in a danger-styled dialog, then toasts "SSH session deleted.".
- **Session dialog** (560 wide): Label*, Color, Host*, Port, Username, Authentication (Password, Private
  Key, SSH Agent), Private Key* + Browse, the saved password editor (Password only), Advanced disclosure
  (Jump host, agent forwarding, Port forwards; open when configured), Group, Logging (Use global, On,
  Off). Save validates in Rust's order: forwards, jump chain, form, password plan. A refused forward or
  jump host opens Advanced.
- **Password plan.** Same table as `session_dialog.rs`: unchanged address keeps the envelope, a changed
  host/port/user reseals it (after unlocking), a blank username drops it with a note, Remove removes it,
  ticked Save seals the typed one. Non-password methods store none.
- **Connect dialog.** Title `Connect to <label> (<user>@<host>:<port>)` or `Connect to <label>` without a
  username; hop blocks first; banner `ssh://...`; Username field accepting `user[@host[:port]]` when the
  session has none; auth with secrets; Save username (forced while Save password is ticked); inline
  error; Connect/Connecting button. Saved passwords show as "Using saved password" once the vault is
  unlocked; a locked vault asks once for the whole chain.
- **Quick Connect.** Title `SSH Quick Connect`: Host*, Port, Username*, jump host picker, auth with
  secrets, Save to SSH Sessions (saved only after a successful connect). Target resolution:
  `resolve_target` (explicit fields win over the username string, port 22 by default).
- **Rejected saved password.** Auth failure on a row using a saved password turns that row into an empty
  input with "Update saved password" ticked (target) or a note (hop); the envelope stays; the next
  connect dialog remembers it while the session still holds that envelope.
- **Host key.** `Unknown SSH Host Key` with host (+ `(jump host for <label>)`), algorithm and SHA-256
  fingerprint, Trust and Connect in danger style, Cancel.

## Files

| File | Holds |
|---|---|
| `events.v` | `Credential`, `HopRequest`, `ConnectRequest`, `DuplicateSpec` |
| `parse.v` | `parse_user_host_port`, `parse_port`, `resolve_target` |
| `panel.v`, `panel_rows.v`, `panel_menu.v`, `panel_attempts.v` | the view, tree rows, context menus, connection callbacks |
| `search_select.v` | `SearchSelect` |
| `color_row.v` | swatches, color popover |
| `fields.v` | field, scrolling body, inline error and toast helpers |
| `auth_form.v`, `auth_render.v` | `AuthForm` |
| `session_form.v` | pure form validation, the password plan, new-password rules and strength hint |
| `session_dialog.v`, `session_submit.v` | New/Edit dialog |
| `connect_dialog.v`, `connect_hops.v`, `connect_submit.v` | Connect/Quick Connect/Duplicate dialog |
| `jump_hops.v`, `forward_rows.v` | hop forms and picker, port-forward rows |
| `vault_dialog.v` | set-up and unlock, `ensure_unlocked` |
| `small_dialogs.v` | rename group, delete and host-key confirmation |

## Not ported

- Rust's `Rejected` persistence is process-wide, kept here in the panel: same lifetime for the one panel
  the app creates.
- The accessibility labels the Rust toolkit needed are the kit's defaults here.
