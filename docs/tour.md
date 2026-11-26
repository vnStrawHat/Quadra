# Where things are

A tour of what exists, crate by crate and phase by phase, for a session that starts fresh:
the names to search for. `docs/progress.md` is the source of truth for what is done and how it
is tested; this file only points. It is history as much as a map: written as the phases ran
(since 2026-11-16 Quadra stands on its own, no port of the V implementation; where this says
"V" it tells how a part began).

Phase R0 is done. **Phase R1, the vertical slice, is done** (docs/progress.md, "The exit of
R1"): the four scenes of the comparison bench run in real windows, and against the V build
every time is equal or a little better (list 0.36 ms, grid 1.29, document 0.70, start-up
41 ms, binary 0.88 MB; docs/benchmarks.md). Idle memory is 1 MB more. The decisions the phase
tested (specs/01, R1 to R5) stand.

What exists, in short (`docs/progress.md` has each crate with what it lacks and its tests):
`quadra-reactive`, `quadra-layout` (done); `quadra-render`, `quadra-text`, `quadra-tree`,
`quadra-view`, `quadra-testing` (passes, layers, atlases, glyph tiles, views, virtual list,
scroll with culling, frame callbacks); `quadra-platform` (traits, the headless platform),
`quadra-app` (windows, the frame loop, recovery), `quadra` (the facade, `quadra::app()`);
`quadra-native-win` (Win32 platform, D3D11 with swap chains and the two-device start,
DirectWrite). Its shader is `shaders/prims.hlsl`, compiled by `tools/build_shaders.sh` (needs
the Windows SDK's fxc) into committed `.cso` files. Its tests use windows that are never shown.

**Phase R2, interaction, is in progress** (specs/00, Phases; specs/09). Done: pointer input,
from the platform's events through hit testing and dispatch (capture, bubble, stop, pointer
capture, hover, press, clicks with counts) to `on_click` and `on_pointer` on views; the
bench's counter counts real clicks. Styles have state variants (`.hover(..)`, `.active(..)`,
`Patch` and `State` in `quadra-tree/src/style.rs`); tokens and themes are R3. The wheel
scrolls `scroll` and `uniform_list`, nested ones included. Focus and keys are in
(`quadra-tree/src/focus.rs`: press to focus, Tab inside a focus scope, the ring, keys along
the focus path, `NodeRef::focus`), and so are actions and the keymap (`action.rs`,
`keymap.rs`; `App::keymap`; Tab is a binding). **The text input is done** (`text_input(value)` in `quadra-view/src/input.rs`, the `TextField`
widget and `TextCommand` in `quadra-tree/src/field.rs`, the buffer in
`quadra-text/src/edit.rs`): typing, the editing keys, pointer selection, undo, the clipboard
(the app provides it as context; the Win32 one is never run by a test, and must not be), a
caret that blinks on a timer (`quadra-async`: `timeout`, `interval`; `Harness::mount` and
`Harness::advance` for tests), and input methods (`PlatformEvent::Composition`,
`PlatformWindow::set_text_input`, `quadra-native-win/src/ime.rs`). `crates/quadra/examples/form.rs`
shows it in a real window. One manual check is open: composing with a real input method
(Vietnamese Telex) on a desktop.
Overlays are in (`quadra-tree/src/overlay.rs`, `overlay(place, view)` in `quadra-view`; the
example has a menu). The semantic tree is in (`quadra-a11y`; `quadra-tree/src/semantics.rs`;
`.role(..)`, `.label(..)`, the state setters; the harness finds controls with `get_by_role`
and the like and drives them with semantic actions: prefer these in new tests). The bridge
to assistive technology is in (`PlatformEvent::SemanticsWanted` and `Semantic`,
`PlatformWindow::update_semantics`; `quadra_a11y::bridge` is the tree as `accesskit` takes it,
`quadra-native-win/src/a11y.rs` and `quadra-native-mac/src/appkit/access.rs` hold its adapters; `tests/access.rs` reads a hidden window through UI Automation). The adapter
panics on an update that is not well formed (specs/12 lists the rules): a change to what the
tree tells a listener goes through `semantics_changed`, and `quadra_a11y::Mirror` in the
random test is what catches a missed case. Lists with rows of different heights are in
(`virtual_list` beside `uniform_list` in `quadra-view/src/list.rs`, heights in
`list/heights.rs`, `Tree::after_layout`), and so are the tab index, cursors and scrolling
into view (`Tree::scroll_into_view`, `on_reveal`; `.reveal(..)` on lists). **Phase R2 is done**
(docs/progress.md, "The exit of R2", maps each interaction test of the V suite to its test
here). Since the tab index: cursors, scrolling into view (`Tree::scroll_into_view`,
`on_reveal`; `.reveal(..)` on lists), the password input and `.on_submit`, the text area
(`text_area`; `TextField::sync`, `Paragraph::place`), text that can be selected, drag and
drop (`quadra-tree/src/drag.rs`, `quadra-view/src/div/drag.rs`, the example `board`), and
`tween` on the window's clock (`Tree::set_clock`, `Handle::now`). **R3 has started** (specs/13;
the design language is specs/design): colours of box styles can be roles of the theme
(`Paint`, `Token`, `Theme` in `quadra-tree/src/theme.rs`; `Tree::set_theme`; the app's theme
is a `Signal<Rc<Theme>>` in the context of views), and the crate `quadra-design` has the names
of the roles (`quadra_design::token`) and the themes (`dark()`, `light()`, `theme_from_json`;
the files are in `crates/quadra-design/themes/`). Text values hold down the tree
(`quadra-tree/src/inherit.rs`: `TextValues`, `Widget::inherit`, the tree's `text_epoch`; a
text has no style of its own). The kit has its fonts (`quadra_design::fonts`), its numbers
(`size`), `install(app, &theme)` and `root()`; `crates/quadra-design/examples/themes.rs` shows
both themes in a real window. The button is in (`quadra-design/src/button.rs`; `action.rs`
has `Confirm` and the kit's keymap), and a disabled node is
not clicked, focused or tabbed to (specs/09). `quadra-vector` is ported (paths, the SVG
of icons, coverage), and the kit has icons (`icon(icons::CHECK)`; the widget is `Drawing`
in `quadra-tree/src/drawing.rs`, its masks are tiles of the glyph atlas) and the text
input (`input(value)`; placeholder, selection and caret colours are text values), and the
selection controls (`checkbox`, `switch`, `radio_group`, `segmented`; `outline` is the
focus mark of what has no border), and popups (`popup.rs`: focus in and back, Escape, a
press outside; `popover`, `menu`, `select`), the tooltip (no node of its own: handlers on
the view's node, a timer, an overlay opened by hand) and the dialog (`Place::Top`), and
shadows (`.elevation(1|2)`, a paint value), `tabs` and `list` (boxes have `.display(..)` and
one-sided borders now). Every scroll view and list has a scrollbar (`quadra-view/src/scrollbar.rs`,
the widget `ScrollThumb`: a layer of its own, shown on activity, faded by a timer). The kit has its display
parts (`display.rs`), `accordion` and `split`; boxes have padding by side and `overflow_x`. Windows can have
their own frame (`WindowOptions::custom_frame`, `PlatformHandler::hit_test`, `.window_region(..)`,
`quadra_view::window()`; Win32 in `quadra-native-win/src/frame.rs`) and the kit its `titlebar`, `toolbar`,
`statusbar`, `breadcrumb`, `sidebar` and status marks, and its form controls (`slider`, `number_input`,
`field` with rules, `textarea`; boxes have `.absolute()` with insets), and `progress`, `progress_busy`,
`spinner`, `skeleton` (compose-only: `Tree::set_layer_turn` beside offset and opacity). Menus have headings,
submenus, scrolling and `context_menu` (`menu/rows.rs`; `popup::open_now` opens a popup from a handler), and
notifications and toasts (`notices()`, `Notice`), and the command palette (`palette.rs`: `fuzzy`, `marked`).
`quadra-text` normalizes (`normalize_nfc` on insert, `fold_diacritics`; tables from `tools/gen_nfc.py`). The kit
has `table` (sort, column widths, selection, copy; no sideways scrolling yet) and `tree`, `range_slider`, `combobox`, `calendar`, `date_picker`, `time_input`, and the smaller
things (`loading`, `indeterminate`, `clearable`, `key_hint`, `confirm`, `menu_button`, `menubar`), the hover
card, the drawer, and the workbench (`quadra_design::work`: activity bar, panel header, tab strip, pane tree,
docking), and `settings` (`settings.rs`: rules and cards). Boxes have a pose (`.translate`, `.scale`,
`.rotate`, `.opacity`: constants of the node's layer), transitions (`.transition(ms)`) and timelines
(`.animate(timeline()..)`): `quadra-tree/src/motion.rs`, run by the tree at the start of a frame.
Rich text is in (`rich_text([span(..)..])`; `Paragraph::rich` with `Shaping` runs, the widget `RichText`,
`Widget::theme` for widgets that paint with roles). **Phase R3 is done** (docs/progress.md, "The exit of
R3": both themes compared, a mean of 1 % of the pixels differ; since 2026-11-14 the design language has more contrast and
colour by the user's choice, so the pictures differ from the V ones by design: translucent hover steps, the accent as the focus border,
tinted buttons with borders, filled checkboxes and switches, `chosen_mark(side, on)` for what is chosen, hues by kind (`token::HUE_*`,
`Tone::Hue`), radii 4/6/6/10, and deeper shadows under a dark theme; the theme gives the shadows of each level, `Theme::with_shadows`). **R4 has started**: `quadra-viz` is done (`sparkline`, `line_chart`, `bar_chart`; the widget `Plot` draws a line a device pixel
column at a time and the shader works out the cover of each pixel, so **a chart line must never show steps**: where the line is
stays a fraction of a pixel, columns are whole device pixels; `QUADRA_SCALE=1.5 tools/shots.sh ..` draws the pages at 150 % to look;
a pointer handler has the size of its node, `e.size`). The live table is the kit's table with `.keys(|row| ..)`
(`quadra-design/src/table/live.rs`: what is chosen follows its keys, changed cells are tinted and fade). `quadra-data` has `Value`,
`parse_json`, `parse_yaml` (its own readers) and `value_view` (the kit's tree with `TreeNode::color`, `.detail`; the colours of
code are roles, `token::SYNTAX_*`), `Log` and `log_view` (a `uniform_list`; following is a scroll place past the end; SGR
colours are `token::ANSI`; the filter by pattern is `regex-lite`, admitted in specs/01 R15). **`quadra-data` is done.** The
kit has `diff(old, new)` (`quadra-design/src/diff.rs`, `diff/lines.rs` the line script, `diff/rows.rs` the rows; `.mode(DiffMode::Inline |
SideBySide)`, `.context(n)` folds; `span(..).background(..)` is the fill behind a span of rich text; `token::DIFF_ADD`, `DIFF_REMOVE`). The
gallery has 71 of the 73 reference pages, each drawn offscreen and compared (`tools/shots.sh`), and `diff`, which has no reference; left: the editor
(`quadra-md` has its parser, `parse`, `parse_inline`, `outline`, linear in the text, with the seeded test of `quadra_testing::fuzz`, and its view,
`markdown(source)`: blocks as a keyed list known by what they say, `view/table.rs` measures columns at mount; `quadra_text::find_urls` is the one URL
detector; rich text has `.strike()` and `.ctrl_links()`, `scroll_x` is a box that scrolls sideways in a page; `quadra-code` has the tokenizers,
`tokenizer_for(name).tokenize(line, state, out)`: V, YAML, Markdown and a table of 27 rows, `LangSpec`, read by one scanner, held by a seeded test
to tokens in order and between characters; they colour the fences; `quadra_md::tex` has formulas: `parse` (nodes; its tables are written by `.tmp/gen_tex.py` from the V tree), `Lay` (boxes by the ink of glyphs,
`TextSystem::ink`), `formula(source).display()` (the widget paints with `quadra_tree::paint_glyph`, rules, and strokes); the gallery has
a page `math` with no reference; rich text takes what is no text (`inline(what)`, `InlineBox`, `Shaping::advance`), which is how `$..$` is drawn: `formula(source).span()`;
`quadra_md::mermaid` has diagrams: `diagram(source)`, sequence diagrams (`seq.rs`, `seq/layout.rs`) and flowcharts (`flow.rs` reads, `flow/rank.rs`, `order.rs`, `place.rs` lay out in layers, `draw.rs` is the list a layout gives,
`paint.rs` paints it; a line is `quadra_render::stroke`, a segment an instance, shaded by distance: use it for any line that is not a chart's; limits are checked while reading), with a page `diagram` with no reference;
`quadra-code` has the buffer (`Buffer`: pieces of 4 KB, their counts in a Fenwick tree, `buffer/counts.rs`); and the document (`Doc` in `doc.rs`, `doc/motion.rs`, `commands.rs`, `lines.rs`, `select.rs`: cursors, `apply(edits, typing)`, undo, the commands; a test of random commands holds it); `quadra_text::display_width` counts cells; next the highlighter, finding and the editor's widget;
images are in: `image(source)` in `quadra-view/src/image.rs` (a path, `&'static [u8]` or bytes; decoded on the pool by `Platform::image_decoder`, WIC in `quadra-native-win/src/image.rs`, `FlatImages` in tests; the widget `Picture` paints pieces of 512 px from the colour atlas, `Fonts::image`; `Images` is the app's cache), and a paragraph of images in a document shows them (`markdown(..).base(folder)`); the gallery has a page `image`)
(R4: `quadra-code`, `quadra-md`) and the terminal (on hold). **R4 is done** (docs/progress.md, "The exit of R4"):
`quadra-code` has the highlighter (`Highlighter`, through the trait `Highlight` so a parser of grammars can be another),
finding (`Query`) and the editor (`editor(text).path(name)`: `EditorWidget` in a scroll view, `EditorCommand` in the
context `editor`, `bind_keys`; the find bar; `quadra_gallery::keymap()` binds them for the gallery and its shots; the
bench's scenes `editor` and `editor-scroll`). Next after R4: local time and the app's language (i18n); the components in the order of specs/13, each with its
page in the gallery (`examples/gallery`) and its test through the semantic tree; tokens for radii and spacing, then the components in the order of specs/13,
popovers and hover cards among them. `quadra-async` is done: `spawn` (a `Task` is cancelled when dropped, `.detach()` leaves it to its scope),
`background(|| ..).await` (the pool; `Result<T, Panicked>`), `inbox_with(limit, Full::Block)` with `sender().extend(..)` and `.on_data(..)`, `sleep`, `debounce`,
`throttle`; the app runs ready tasks after every entry point (`App::request_frames`), tests use `Harness::settle()`. Readers are told the caret and selection of
an input (`Semantics::selection`, a text run in the bridge; `SemanticAction::Select`), range values (`.range(..)`), positions in a set
(`.place_in_set(i)` on items, `.set_size(n)` on their container) and scroll views (`Role::ScrollView` on `scroll`).
**R5, app services, has started** (specs/15, 16): `App::dirs(AppDirs::for_app(..))` (nothing is written without it), `Settings`
(`value(key, default)` is a signal; debounced, atomic), `quadra_app::log`, crash reports of panics, `quadra_app::time` (`Date`, `DateTime`,
`local_now()` by `Platform::utc_offset`), the app's language (`I18n`, `App::i18n`, `t`, `t_with`; `language()` taken at build time for what
runs again; `Locale` formats in Vietnamese and English), window state (`WindowOptions::remember(key)`), pages (`router(first)`, `.view(..)`; Alt-Left/Right) and closing (`.on_close(|| bool)`, `window().close_now()`); `quadra-shell` has `Dialogs`, `Opener`, `Tray`, `FakeShell`, `install(app)` (`App::provide`), `single_instance`, `crash_reports` (OS code in `src/win/`, `cfg(windows)`); packaging (`tools/package.sh`); the API docs are in the gate. **R5 is done but for its manual checks** (docs/progress.md, "The exit of R5"). **R6 is done** (docs/progress.md, "The exit of R6"; specs/17): `quadra-headless` is the components without a look (the gate fails on a colour, role, radius or cursor in it); done: the theme's shadows, `action`, `popup`, `choice`, `press` (and `press::toggle`), `list` (and `list::places`), `disclose`, `split`, `menu`, `pick`, `combo`, `hint`, `dialog`, `notices`, `slider`, `number`, `time`, `rules`, `tree`, `table`, `live`, `settings`, `work`; the second kit is `examples/plain` (`quadra-plain`, only the base and boxes). Each move keeps the gallery's pictures the same to the byte: `.tmp/same.sh` draws both themes and compares them with `.tmp/shots/r6-dark` and `r6-light`, taken before the first move. The renderer has gradients (`Gradient::linear|radial|conic`, `.gradient(..)` on a box; stops mixed in Oklab into a ramp of the colour atlas; specs/07). Borders can be dashed or dotted (`BorderStyle`, `.border_style(..)`). What a frame costs the GPU is `RenderStats::gpu` (timestamps; the bench prints `gpu`). A box blurs what is behind it (`.backdrop_blur(px)`; a layer, its backdrop's mips sampled by its quad). A box is cut to a shape (`.clip_shape(svg)`; a mask in the coverage atlas, a layer constant inherited by the layers inside). The renderer's list of specs/00 (R6) is done; shaders by class only if the GPU measure asks. Since: tree rows tell a reader their level (`.level(n)`; the adapter's levels are from 0),
contexts of bindings are predicates (`editor && !find`), and `Keymap::load(text, &Actions)` reads a
keymap file (`quadra_design::actions()` names the kit's; the editor's commands have no names yet). The text size
(specs/12): `TextSize { zoom, text, system }` (`text_size()` in views), zoom by `Tree::set_zoom`, text alone by
`Tree::set_text_scale`; a box of a line of text in the kit is `.height(by_text(px))`, so new components do the same;
`QUADRA_TEXT=1.5 tools/shots.sh` draws the gallery with larger text. Since 2026-11-15 `quadra-base` is
`quadra-headless`, `quadra-kit` is `quadra-design` and `quadra-ui` is `quadra-view` (the user's names); the headless crate names its components as
widgets too (`button`, `checkbox`, `switch`, `radio_group`/`radio`, `tabs`/`tab`, `accordion_head`/`accordion_body`,
`tooltip`, `hover_card`, `combobox`; specs/17, "Widget names"): a look uses those. One manual check is open: glass over scrolling content in a real window. No phase follows R6 in specs/00: what comes next is the user's to say (the apps come after Quadra is complete).

Room left that the numbers show (docs/benchmarks.md, 2026-11-15): the grid's update is the scene's formatting (66 µs), the
reactive runtime (some 70) and applying 1,200 writes (56), each tried and found lean; a new band of a long page still looks at
each row of a column once (25 ns a row).

**OneTerm has started (2026-11-15, the user's decision)**: `quadra-term` (specs/18) is resumed over the user's own engine
`oneterm-vt` (git tag v0.2.0, a clone to read in `.cache/vendor/oneterm-vt`; never edit that repository), and the app is
`apps/oneterm` (apps/oneterm/specs/oneterm.md), drawn after the user's design canvas "1Term" (the app's name until 2026-11-24). P1 is local shells (milestones M1-M6); P2 is SSH and
SFTP, whose engine the user decides when P2 starts. The user's own terminal app OneTerm 0.6 (a clone to read in
`.cache/vendor/oneterm-app`; never edit it; its CLAUDE.md and AGENTS.md are for that repository) is the reference
for behaviour and settings the design leaves open. **P1 is done** (M1-M6, docs/progress.md): `tools/cargo.sh run -p oneterm-app`; a trial run takes
`ONETERM_HOME=.tmp/oneterm` so it writes nothing outside the project. P2 (SSH over `russh` + `tokio` + `ring`, the user's choice) is done through M10 (`apps/oneterm/crates/ssh`, one of the app's crates since 2026-11-25: `app`, `ssh`, `completion`, `vault`, `shots`, apps/oneterm/specs/oneterm.md, "The repository"; jump hosts, local forwards, SFTP dock and its expanded view, saved hosts; tests only against servers of the test process on 127.0.0.1). Throughput: 592 fps of the fire demo at 147 by 40 with the bundled console host (docs/benchmarks.md). Since 2026-11-24 it takes over from OneTerm 0.6 (`import.rs`: hosts, the vault, the proxy; `adopt`, the folders of the name before), updates itself (`updates.rs` over `quadra-updater`, reached as `prefs/net.rs` says: `quadra_shell::Network`), keeps startup commands as a list (`startup.rs`), and its SFTP dock has a header of sortable columns (`dock/list.rs`).

**The updater** (`quadra-updater`, specs/22, 2026-11-24): `Updater::new(repo, app, version, cache)`, then `check()` (the
latest release of a GitHub repository, if newer, with this build's package and its digest), `fetch(&release, progress)`
(the package checked against the digest and unpacked by the crate's own zip reader) and `install(&package)` (each file of the app's folder moved aside
and the new one put at its name, while the app runs); `restart()` and, at the top of `main`, `started()`. It blocks: an
app calls it on its pool. Under it `quadra_shell::http_get` (the OS's HTTP: its proxy, its certificates) and `sha256`
(specs/16, "Fetching"). Tests only against a server of the test on 127.0.0.1.
