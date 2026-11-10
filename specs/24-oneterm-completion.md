# 24. OneTerm command completion and semantic highlight

Two UI-free engines of OneTerm, ported 1:1 from the Rust original (v0.6.11): `crates/completion`
(with the UI-free `CompletionController` of `terminal-view/src/completion/controller.rs`) and
`crates/highlight`. The clone at `.cache/downloads/oneterm` is the behavioural reference; where
this spec is silent, the original decides. Painting the overlay, key handling and merging
classes into cells belong to the terminal view (specs/22, P3).

- `apps/oneterm/completion/` is V `module completion`, `apps/oneterm/highlight/` is
  `module highlight`; the app imports them as `import completion`, `import highlight`.
- Decisions taken without the user (the port was unattended) are marked **Decision**.

## Completion

### Sources and catalogs

| Source | Tag | Where | Lifetime |
|---|---|---|---|
| `memory` | `H` | `History`, shared by all tabs | RAM only, reset on exit |
| `manual` | `C`/`O` | `assets/manual/**`, embedded | static |
| `external` | `C`/`O` | `assets/external/**`, embedded | static |

**Decision: no history persistence.** The original keeps the recorded commands in RAM by
design (requirement R5, docs 01 §4 and 06 §1): only the settings that control history persist.
Persistence of commands would put redaction misses on disk, so it is not added.

One JSON file per command, a recursive node `{ name, options, subcommands? }` (schema 1; a
higher `schema` is rejected; a malformed file is skipped without affecting the others). An
option is `"-a"` or `{ "flag": "--all", "description": "hint" }`; a blank description is dropped.
Source and category come from the folder, as `build.rs` derives them:

| Folder | Source | Category |
|---|---|---|
| `external/cmd`, `external/coreutils`, `external/powershell` | external | the folder |
| `manual/windows/cmd` | manual | `windows` |
| `manual/windows/powershell` | manual | `powershell` |
| `manual/linux`, `manual/common` | manual | the folder |

`catalog_index.v` lists every file with `$embed_file`, sorted by path like `build.rs` (the
order is observable through first-seen dedup). It is generated once from the folder layout.
`catalog.schema.json` and the two `.gitkeep`s are not command nodes and are not copied.
Parsing is lazy and cached per file; listing command names never parses JSON.

Precedence for a name: manual beats external, then the earlier category of the search path wins.

### Shell family

| `ShellFamily` | Kinds | Categories (high to low) | Option triggers | Case |
|---|---|---|---|---|
| `cmd` | cmd | cmd, windows, common | `/` `-` | insensitive |
| `powershell` | powershell, pwsh | powershell, cmd, windows, common | `-` | insensitive |
| `unix` | bash, zsh, sh, custom/unknown | coreutils, linux, common | `-` | sensitive |

`windows_allow_coreutils` appends coreutils and linux to the two Windows families. `force_family`
(`cmd`, `powershell`/`pwsh`, `unix`/`bash`/`zsh`/`sh`) overrides detection.

### Suggestion engine

`suggest` is a pure function of the history, the context (family, line, byte cursor, `now_ms`)
and the params. The line left of the cursor is tokenized with single and double quotes; the
token under the cursor is empty after trailing whitespace.

| Context | Condition | Candidates |
|---|---|---|
| Option | token starts with a trigger and a head exists | active node's options, ancestors' options at frecency -1; unknown command: option tokens from history lines of that command |
| Command | first token | catalog names, first tokens of history; requires `min_prefix_len` unless `suggest_on_empty` and empty |
| Subcommand | a later token and the active node has children | children (tag `C`, counted as manual) and whole-line history |
| Argument | a later token, no children | whole-line history |

Tree walk: the tokens after the head skip option tokens, descend while a child matches, and
stop at the first non-child. The path of names walked is the breadcrumb (`git > remote > add`).

Matching: prefix first (`match_len` = token bytes), then fuzzy subsequence when `fuzzy` is on
(`match_len` 0). Options match case-insensitively. Whole-line history replaces from byte 0.

Ranking: `10*kind + 6*frecency + 4*prefix + 1/(1+len)`, kind 3 history, 2 manual, 1 external;
`frecency = use_count / (1 + age_hours)`. Sort is stable, descending. Dedup by text (family case
rule) keeps the best score and upgrades the tag to the highest precedence (H > manual > external).
The list is cut to `4 * max(1, max_visible_items)`.

### Redaction

`redact` runs before a line enters `History`; `contains_secret` runs again at suggestion time on
history candidates (`redact_sensitive`). Rules, in the original's order:

- Flag with inline value (`--flag=v`, `/FLAG:v`): a secret flag keeps the name, drops the value.
- Attached short form (`-pSECRET`) of a per-command short secret flag keeps `-p`.
- Secret flag with separate value: the next token is dropped.
- `KEY=VALUE`: a secret key or a secret-shaped value drops the whole token.
- `scheme://user:pass@host` loses the userinfo only.
- A credential-shaped standalone token is dropped: `AKIA...`, `ghp_`/`gho_`/`ghs_`/`ghu_`/`ghr_`,
  `github_pat_`, `sk-`, `xox`, PEM markers, `bearer `/`authorization`, JWT, high entropy (at least 20
  chars, 90 % base64 alphabet, letters and digits, Shannon entropy at least 3.5, no separators).
- Secret flags: the vocabulary (`password`, `passwd`, `pwd`, `pass`, `secret`, `token`, `api-key`,
  `apikey`, `access-key`, `secret-key`, `auth`, `authorization`, `bearer`, `credential`,
  `private-key`, `client-secret`, `session-token`, `otp`, `passphrase`) and names ending `-password`,
  `-passwd`, `-secret`, `-token`.
- Short flags are per command: `-p` for the mysql family, `mariadb`, `sshpass`, `az`, and
  `docker`/`podman`/`nerdctl login`; `-u` and `--user` for `curl`. The command is the first token
  after assignments and `sudo`/`doas`, normalized to a lowercase basename without `.exe`.
- Hygiene: control chars removed, tab to space, at most 4096 bytes on a char boundary, trimmed.

### Config

`Config` is the `completion` group of `terminal.json`. Its field names are the JSON keys and
its defaults the original's, so `config.v` can embed it and V's `json` fills the missing keys.

`enabled` true, `accept_tab` true, `max_history` 500 (0 disables and clears), `min_prefix_len` 1,
`max_visible_items` 8, `sources.memory/manual/external` true, `fuzzy` true,
`inherit_ancestor_options` true, `disable_in_alt_screen` true, `require_prompt_region` true,
`windows_allow_coreutils` false, `force_family` empty (the original's null), `redact_sensitive` true.

### Controller

`Controller` is the per-terminal state machine: gating
(`enabled` and not (`disable_in_alt_screen` and alt screen) and (not `require_prompt_region` or in
prompt region)), recompute only when line, cursor or gating changed, hide a sole exact match,
run-first selection (none until the user navigates), accept, capture with redaction.

**Decision: accept returns `Accept{backspaces, text}`.** `accept()` gives what must be typed: the
remainder after the typed text, or on Cmd/PowerShell backspaces plus the rest in exact suggestion
casing. `Accept.bytes()` is the PTY form (`0x7f` per backspace, then the text).

`strip_prompt` and `visible_window` are ported from the view; both are UI-free.

### Public API

| Call | Purpose |
|---|---|
| `new_controller(kind, config)`, `.sync_settings(kind, config)` | build, apply settings live |
| `.set_alt_screen(on)`, `.set_in_prompt_region(in)`, `.gating_allows()`, `.pre_gate_ok()` | gates |
| `.recompute(line, cursor, now_ms, history, force)`, `.wants_recompute(moved)`, `.dismiss()` | refresh |
| `.is_visible()`, `.suggestions()`, `.selected()`, `.select_first_if_none()`, `.select_next()`, `.select_prev()` | overlay state, `selected` is -1 when none |
| `.typed_len()`, `.max_visible()`, `.accept_tab()`, `.enabled()`, `.family()` | overlay geometry and flags |
| `.accept()`, `.capture(raw, now_ms, mut history)` | apply a suggestion, record a command |
| `.breadcrumb(line, cursor)` | names from the root to the active node |
| `new_history(cap)`, `.record`, `.entries`, `.set_capacity`, `.capacity`, `.clear`, `.is_empty` | the shared store |
| `new_engine()`, `.suggest(history, context, params)`, `.breadcrumb(context, params)` | the pure engine |
| `Suggestion{text, kind, description, match_len, score, replace_from}`, `SuggestionKind.tag()` | one row, tag `H`/`C`/`O` |
| `redact(line)`, `contains_secret(line)` | secret handling |
| `family_from_kind(kind)`, `family_from_config(s)`, `params_from_config(config)` | setup |

## Semantic highlight

`scan_line_into` classifies one **logical** line (wrap-connected rows joined, wide-cell spacers
skipped) into one `u8` `Class` per rune. Three states follow the row role; `RowRole.unmarked`
means the shell said nothing and is the only case where a prompt detector runs.

| Role | Behaviour |
|---|---|
| `prompt` | sign = last prompt glyph before `input_at`, else the last non-space; the head is a path; the tail is command mode |
| `command` | command mode: first token `command`, option tokens `option`, `;`, `\|`, `\|\|`, `&&` reset |
| `output` | the flat matchers below, first match wins per cell |
| `unmarked` | the profile's prompt detector, then the universal one; a prompt is scanned as `prompt`, anything else as `output` |

Output matchers, in order: quoted strings (an unclosed quote runs to the end), `ls -l` permission
block per char, keywords, then MAC, IPv6 and datetime (a match lands only on all-plain cells),
IPv4, paths, numbers, option probe, single-char operators and brackets.

Keywords are the original's five lists (debug, warn, info, success, error), ASCII
case-insensitive, leftmost-longest and non-overlapping, kept only when both ends are word
boundaries. **Decision:** a trie over `[a-z ]` walked at every position replaces the
automaton; the match set is identical.

`Class.url` and `Class.permission` are not produced by the scanner, in the original either: URLs
come from the view's own detection (`kit/term/links.v`) and overwrite classes after the scan.

Merge policy (for the view, original §9): a non-plain class replaces the default foreground only,
never an explicit ANSI color unless `override_ansi`; background, underline and bold/italic are
additive. The exit-code tint (`tint_prompt_sign`) is applied by the view to the prompt run
`RowRoles.last_completed_prompt` returns.

### Styles

`ClassStyles` is a flat array of 32 `ClassStyle{has_fg, fg, has_bg, bg, bold, italic, underline,
override_ansi}`; `assets/default.json` is the original's default. Keys and `"foreground"`,
`"background"`, `"decoration": "underline"`, `"bold"`, `"italic"` as in the original; unknown keys
are ignored. **Decision:** colors are `prim.Color` instead of a local HSLA mirror (nothing here
is above `prim`); `parse_color` accepts `#RGB` and a missing `#` like the original.

### Matchers instead of V's regex

V's `regex` module cannot express the patterns: `|` acts on single tokens, not sequences, and it
has no `\b`. **Decision:** `re.v` and `re_parse.v` are a minimal backtracking matcher: the original patterns are
kept verbatim and compiled once, so leftmost-first alternation and all backtracking behave as in
the original. It supports literals, classes (`[...]`, `[^...]`, `\d`, `\s`), `(?:...)`, `|`, greedy
`? * + {n} {n,m}`, `^ $ \b`. Differences from the original's DFA engine, none reachable on normal
output: `\d` and `\w` use V's Unicode tables plus U+0300-U+036F and U+200C/D (close to, not
identical with, Rust's); an attempt is abandoned after 20000 steps (a guard against pathological
lines, where the DFA stays linear).

Performance: each pattern has a first-char set, MAC and IPv6 run only when the row has enough
colons (5 for MAC; `::` or 7 for IPv6), all matchers work on the caller's rune buffer, and
`ScanScratch` and the `out` buffer are reused, so a steady-state scan allocates nothing. Measured
with clang `-prod`: about 9 µs per row over 200,000 scans of 100-char corpus rows.

Patterns (reference, also in `rules.v`):

| Name | Pattern |
|---|---|
| Unix prompt | `^(?:\([^)\s]*\) )?(?:\[[^\]]*\]\|[^\s]*[@:~/\]][^\s]*)?[\$#%](?: \|$)` |
| Windows path body | `[^<>\|"*?:\r\n]*[^\s<>\|"*?:-]`, roots `(?:[A-Za-z]:\|\\\\)` |
| cmd | `^(?:ROOT BODY>[ ]?)\|(?:^>[ ]?)` |
| PowerShell | `^(?:PS(?: PS_ROOT(?:BODY)?)?>[ ]?)\|(?:^>+[ ]?)` |
| dumb | `^[^\s]*[\$#%>»](?: \|$)` |
| universal fallback | windows path half `\|` Unix |
| MAC | `\b[0-9a-fA-F]{2}(?::[0-9a-fA-F]{2}){5}\b` |
| IPv6, datetime | see `rules.v` |

### Public API

| Call | Purpose |
|---|---|
| `scan_line(line, profile, role, input_at)`, `scan_line_into(..., mut scratch, mut out)` | classes per rune; `input_at` -1 means unknown |
| `new_overlay(profile, enabled)`, `.scan_into(line, role, input_at, mut scratch, mut out)` | per-view gate; the fields `profile`, `enabled`, `exit_code`, `has_exit_code` are public |
| `spans(classes, styles)`, `spans_into(classes, styles, mut out)` | merged styled runs (`Span{start, end, class, style}`, rune indices) |
| `highlight_row(line, profile, role, input_at, styles)` | scan plus spans in one call |
| `tint_prompt_sign(mut classes, exit_code)` | prompt sign to success or error |
| `load_default_styles()`, `parse_semantic_json(text)` | themes |
| `profile_for(kind)`, `RowRoles`, `.role_at`, `.is_empty`, `.reset`, `.last_completed_prompt(wraps)` | shell profile and OSC 133 roles |

## Tests

Ported as V tests: all of `engine_tests`, `catalog_tests`, `redact_tests`, the unit tests of
`family`, `history`, `parse`, `terminal-view/completion/tests.rs`, and `scanner_tests`, `profile`,
`rules` (patterns), `role`, `theme`, the overlay and bridge tests. `scan_line_output.golden` is
reproduced: the 144 FNV-1a hashes over every role and profile must equal the original's, which
proves the port byte for byte on that corpus.

## Not ported

`scripts/completion-catalog.py` (regenerates the catalogs; they are checked in) and the Hsla
conversion helpers of the color module.

## Licence

The original is the user's Apache-2.0 project. `completion/assets/NOTICE` keeps the provenance of
the catalogs: `external/cmd` lists flag names extracted from MicrosoftDocs `windows-commands`,
`external/coreutils` from the Debian bookworm coreutils man pages. Only command and option name
tokens were extracted (no prose); no catalog file carries a GPL notice, but the
coreutils man pages themselves are GPL-3+. The catalogs are copied on that basis and the
coreutils folder is called out for the user's review.
