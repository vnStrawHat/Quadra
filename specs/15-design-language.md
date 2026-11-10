# 15. Design Language: "One"

Quadra's kit targets developer tools: editors, terminals, cluster managers. People keep these
open for hours and drive them from the keyboard. The language is the one of a modern native
code editor: flat, warm-neutral planes, hairline borders, a single blue accent, small
square-ish controls, and no decoration that the content does not need. Every number below
is a pixel at 100 % scale; the OS text-size setting scales text only.

## Principles

1. **Flat planes, hairline seams.** Regions differ by one small step in tone plus a 1 px
   border. The text area is the deepest plane, panels one step off it, the window chrome
   one step further. No cards in cards, no gradients.
2. **Ghost controls.** Most buttons, list rows and tabs have no fill until hovered. Hover is
   one neutral step, press two. Selection is neutral too; the accent is for text and icons.
3. **The accent is text, not paint.** A toggled button, an active panel toggle, a checked
   box and a matched character turn the accent *color*; nothing gets an accent fill except
   tinted (primary) buttons, the switch track and progress fills, at low opacity.
4. **Focus is a border.** Focus never adds a glow or an outer ring: the 1 px border turns
   `border_focused`. Buttons without a border show focus as their hover fill.
5. **Small type, generous leading.** 14 px UI text at a golden-ratio line height (1.618)
   and a sans with open shapes; code in a programming face at 15 px.
6. **Shortcuts are plain text.** `Ctrl-Shift-P`, muted, after the label. No key caps.
7. **Status at a glance, never by color alone.** Workload status keeps a shape per state.
8. **Instant states, functional motion.** Hover and press change at once. Only overlays and
   toasts animate (150 ms, quintic ease-out); reduced motion turns that off.

## Tokens

### Planes and borders

The palette keeps the kit's role names; each role is one token of the One themes.

| Field | Role | One Dark Pro | One Light |
|---|---|---|---|
| `canvas` | editor, terminal, toolbar, active tab, input fields | `#23272E` | `#FAFAFA` |
| `panel` | side panels, surfaces | `#23272E` | `#EBEBEC` |
| `chrome` | title bar, the window behind empty panes | `#23272E` | `#DCDCDD` |
| `overlay_bg` | menus, tooltips, popovers, pickers, dialogs, toasts | `#1E2227` | `#EBEBEC` |
| `raised` | filled controls (element background) | `#404754` | `#EBEBEC` |
| `panel_hover`, `raised_hover` | ghost and filled hover | `#2C313A` | `#DFDFE0` |
| `press` | pressed, selected rows, keyboard-highlighted rows | `#2C313A` | `#CACACA` |
| `line` | dividers, overlay and input outlines (border variant) | `#3E4452` | `#DFDFE0` |
| `line_strong` | tab and pane borders, outlined hover (border) | `#3E4452` | `#C9C9CA` |
| `border_focused` | focus border | `#3E4452` | `#7D82E8` |
| `scrim` | behind modal dialogs: none | transparent | transparent |
| `status_bg` | the status bar (default `chrome`) | `#1E2227` | `chrome` |
| `tab_bg` | tab bar and inactive tabs (default `panel`) | `#1E2227` | `panel` |
| `scroll_thumb` | scroll thumbs (default `text` at 30 %) | `#4E566680` | `text` 30 % |
| `input_bg` | text fields, steppers and dropdown triggers (default `canvas`) | `canvas` | `canvas` |

Roles a theme may split off; unset, each follows the role in brackets: `selection`
(selected UI text; `accent_tint` in fields, the editor selection in selectable text), `tab_fg` and `tab_active_fg` (tab labels; the
active one is also any tab under the pointer; `text_2`, `text`), `table_head_bg`,
`table_head_fg` (`panel`, `text_2`), `table_row_border` (`line_strong`), `list_even`
(striped rows; none), `scroll_thumb_hover` (`scroll_thumb`), `text_on_status` (on
feedback fills; `text_on_accent`), `error_hover` (`error` 15 % toward `canvas`), `cyan`
(the syntax type color).

### Text

| Field | Role | One Dark Pro | One Light |
|---|---|---|---|
| `text` | primary text and icons | `#DCE0E5` | `#242529` |
| `text_2` | muted: inactive tabs, secondary labels, shortcuts | `#A9AFBC` | `#58585A` |
| `text_3` | placeholder, hints, breadcrumb separators | `#878A98` | `#7E8086` |
| `text_disabled` | disabled labels and icons | `#878A98` | `#7E8086` |
| `accent_text` | selected or toggled labels, links, matches | `#74ADE8` | `#5C78E2` |

Icons take a text color (`text`, `text_2` for decoration, `accent_text` when toggled).

### Accent and feedback

- `accent` (`#74ADE8` / `#5C78E2`) is the editor cursor, the switch, the progress fill and
  the dirty dot. `accent_tint` (`#1E2227` / `#E2E2FA`) is the tinted button fill.
- Feedback colors with a tint each for banners and tinted buttons:

  | Field | One Dark Pro | tint | One Light | tint |
  |---|---|---|---|---|
  | `ok` | `#A1C181` | `#A1C1811A` | `#669F59` | `#DFEADB` |
  | `warn` | `#D19A66` | `#1E2227` | `#A48819` | `#FAF2E6` |
  | `error` | `#C24038` | `#1E2227` | `#D36151` | `#FBDFD9` |
  | `info` | `#74ADE8` | `#1E2227` | `#5C78E2` | `#E2E2FA` |

- **Workload status** (derived; the themes reuse the feedback and syntax hues):

  | Status | Shape | Color |
  |---|---|---|
  | `running` | dot | ok |
  | `pending` | ring | warn |
  | `failed` | triangle | error |
  | `succeeded` | check | cyan (syntax type) |
  | `terminating` | dash | violet (syntax keyword) |
  | `unknown` | square | `text_3` |

### Code and terminal

Theme tokens, light and dark: syntax (keyword, control, string, number, constant, type,
function, variable, property, operator, punctuation, comment, tag, attribute, regexp,
escape, invalid), editor (cursor, active line `#2F343EBF` / `#EBEBECBF`, line number
`#4E5A5F` / `#B4B4BB`, active line number `#D0D4DA` / `#44454B`, selection = accent at 24 %,
inactive selection at half that, find matches, bracket match, indent guide, whitespace,
diff tints) and the terminal's 16 ANSI colors.

### Type

| Role | Face | Size / line height |
|---|---|---|
| `text_xs`: badges, hints | UI | 10 / 16 |
| `text_sm`: meta, status bar, small buttons | UI | 12 / 19 |
| `text_md`: body, controls, menus, tabs | UI | 14 / 23 |
| `text_lg`: large labels | UI | 16 / 26 |
| `text_xl`: dialog and page headlines | UI | 18 / 26 |
| `text_2xl`, `text_3xl`: display | UI semibold | 23, 28 / 1.25 |
| code, terminal | mono | 15 / 24 (1.618) |
| values inside UI (paths, numbers) | mono | the UI size of their row |

- **UI face:** IBM Plex Sans, bundled with the kit (regular, italic, semibold, semibold
  italic). Weight 400 everywhere, 600 for headings and emphasis.
- **Mono face:** Lilex, bundled (regular, italic, bold, bold italic), with programming
  ligatures. The UI face is shaped without contextual alternates (`calt` off), so labels
  keep plain forms; the mono face keeps them.
- The default line height of UI text is 1.618 of its size, rounded to whole pixels.
- Markdown content: body 16 / 1.5, headings 28, 22.4, 19.2, 16, 14, 13.6 at weight 600 and
  line height 1.25; H1 and H2 have a 1 px `line` underline.

### Space, size, shape

- **Spacing:** 1, 2, 4, 6, 8, 10, 12, 16, 20, 24, 32. Padding is usually 4 (rows, bars),
  6 (list rows) or 8 (fields, tooltips).
- **Control heights (`Size`):** xs 18, sm 22, md 28, lg 32 (`h_xs` ... `h_lg`). Comfortable
  density adds 4. Compact is the default and is the reference density.
- **UI scale:** `ui_scale` (1) multiplies the control heights and the paddings of the
  components a design language sizes in rem (a UI font-size setting). `text_step` 1 is for
  a type scale whose md is body text (16): components then take their text one step down
  (`ui_text(.md)` is `text_sm`), so controls stay 14 while plain text is 16.
- **Icons:** Lucide outlines on a 16 grid, sizes 10, 12, 14 (default in chrome), 16.
- **Radii:** `radius_sm` 2 (checkbox, key), `radius_md` 4 (buttons, list rows, chips,
  tooltips' rows), `radius_lg` 6 (inputs, segmented groups, code blocks), `radius_xl` 8
  (every overlay: menus, tooltips, popovers, pickers, dialogs, toasts). Tabs, panels and
  tables are square.
- **Chrome:**

  | Element | Size |
  |---|---|
  | title bar | 32 |
  | tab bar | 32 (the active tab has no bottom border) |
  | toolbar / breadcrumbs | 45 (32 row, 6 padding, 1 px `line` bottom) |
  | panel header | 28 |
  | tree and sidebar rows | 26 |
  | status bar | 30 (22 buttons, 4 padding) |
  | activity bar | 40 wide |
  | splitter | 1 px `line_strong`, 6 px grab area |

### Elevation

Overlays are `overlay_bg` with a 1 px `line` border and `radius_xl`.

| Level | Used by | Shadows (x, y, blur, black alpha) |
|---|---|---|
| 1 | menus, tooltips, popovers, combobox lists | 0 2 3 .12; 0 1 0 .06 |
| 2 | pickers, dialogs, notifications, toasts | 0 2 3 .12; 0 3 6 .08; 0 6 12 .04; 0 1 0 .12 |

Pickers and dialogs hang 80 px from the window top, centered, with no backdrop; a click
outside dismisses them. Drawers, which the reference has no counterpart for, follow the
same rule: no backdrop, an outside click closes them.

### States

| State | Ghost items (buttons, rows, tabs) | Filled and outlined items |
|---|---|---|
| hover | `panel_hover` | `raised_hover`; outlined border becomes `line_strong` |
| press | `press` | `press` |
| selected | `press` fill, label stays `text` | label `accent_text` |
| toggled button | no fill, icon and label `accent_text` | same |
| focus (keyboard) | hover fill | border `border_focused` |
| disabled | `text_disabled`, no hover | same, fill kept |
| drop target | `#83899480` / `#7E808780` fill | same |

## Components

- **Button.** Height by size, padding 4 (xs, sm) or 8 (md, lg), gap 4, radius 4, label
  14 (12 at xs), weight 400, icon 14 (16 at md and lg). Variants: `ghost` (no fill),
  `secondary` (outlined: `line` border, transparent fill), `outline` (outlined ghost),
  `primary` (tinted: `accent_tint` fill, `text` label), `danger` (tinted with `error`'s
  tint), `link` (accent text). Icon buttons are square at size sm (22) with a 14 icon.
  Toggled: the label and icon turn `accent_text`; labelled toggles (filter chips, scope
  tabs) also take the `press` fill, icon toggles do not. A dropdown trigger (`.caret()`)
  ends in a 16 chevron-down in its label color at 75 %, with gap 8, and takes the
  `input_bg` fill. `.compact()` narrows the padding (sm 6, md and lg 8) and keeps the button
  at least as wide as it is tall (a title-bar menu: sm, compact, ghost).
- **Checkbox:** 16 box in a 20 hit area, radius 2, 1 px `line_strong`; the check is 14,
  `accent_text`. **Switch:** 32×20 track with 2 px inner padding inside a 2 px
  `line_strong` ring 1 px off it (38×26 in all), 12 knob in `text`; off: `raised` track
  with a `line_strong` border, knob at 50 %; on: `accent` at 40 % with an `accent_text`
  20 % border.
- **Controls** (`Theme.ctl`, the `controls` object of a theme file): shapes that differ
  between design languages, off by default (One). `plain_toggles`: the switch is a 36×20
  track (`line` off, `accent` on) with a 16 `text` thumb inset 2 and no ring, the checkbox
  a 16 box with radius 4, `input_bg` fill and a `line` border, checked: an `accent` fill
  with a 12 `text_on_accent` check; label gap 8; disabled at 50 % opacity.
  `select_input`: see Select; its menu then lists the options as 14 px rows padded 7 x 4,
  hover `panel_hover` at 70 %, the chosen row filled with `panel_hover` and a trailing 12
  check, radius_md, padding 4, at most 320 tall.
  The others, each a family of shapes (theme_controls.v has the numbers):
  `solid_buttons` (a solid `accent` primary, danger at 20 / 30 / 40 % with `error` text,
  outlined buttons on `line` at 30 / 50 / 70 %, ghost hover `panel_hover` at 50 %, disabled
  text at 50 %; padding 4 / 8 / 10 / 12, gap 8 from md; icon squares hold 12 / 14 / 16
  icons), `roomy_inputs` (single-line inputs the md control height tall, 32 at rem 16, the text
  centred in it; padding 10 × 8, gap 6, placeholder `text_2`; a focused input also gets a 3 px `border_focused` 50 % ring outside its border, on any focus), `roomy_menus` (rows 26 tall and 2 apart, text 12 from
  the edge, min width 128, max 500, `radius_md`, a 1 px `text` 10 % ring instead of the
  border, 2 px separators 2 off their rows, 12 icons, the check in the row color, the
  shortcut 12 `text_2` 12 after the label, disabled `text_2`, 14 px section labels,
  submenus 8 into their parent and 4 up; tooltips padding 8 × 2, `radius_md`, gap 12, the
  shortcut 12, placed at the pointer + 1, inside the window), `plus_keys`
  (`Ctrl+Shift+C`, modifiers Ctrl, Alt, Shift, Win, `Page Up`), `card_dialogs` (`canvas`
  fill, `radius_xl`, a tenth of the window from the top, at most the window less 32 wide,
  padding 16, title 16 semibold at line height 1.25, body `text`, title-body gap 8 and
  body-footer 16, footer gap 8 with no line, a 24 ghost close button 8 from the top right;
  `ModalOptions.alert`: no close button and outside clicks ignored), `tinted_notices`
  (notifications and toasts in one stack at the bottom right 16 from the edges, 382 wide,
  `radius_xl`, the kind color at 4 % fill and 30 % border, title and text in it, padding
  16 × 14, gap 12, the 14 semibold title, a 20 close button shown only while the card is hovered; `info` is `info`;
  they hide after 5 s unless they have actions), `accent_chrome` (the title bar shades
  from `chrome` 45 % toward `canvas` at the top down to `chrome`; caption buttons 34 wide
  with 14 icons, hover `raised_hover`, close `error` with `text_on_status`; tabs
  padded 12, the active one with a 2 px `accent` line on top and seams on both sides only,
  `tab_fg` / `tab_active_fg` labels, the close button always shown: 16, radius 3, a 12
  `text_2` icon, hover `text_2` at 15 %; status dividers 1 × 12 in `line`; split and dock
  resize handles a 1 px `line` hairline carrying a 3 px `text_2` pill, 20 long at 35 % while
  hovered and 44 long at 90 % while dragged) and
  `fixed_scrollbars` (thumbs always shown, radius 4, `scroll_thumb_hover` and 8 wide under
  the pointer or while dragged; a terminal's thumb is 8 wide, 2 from the edge, at least 24
  long, `#808080` at 80 %, radius 2, shown 2 s and fading over 1 s).
- **Select (dropdown):** filled: `raised` with a 1 px `line` border, radius 4, 28 tall,
  padding 8, the value in 14 `text`, a 12 up-down chevron in `text_2`; hover
  `raised_hover`; focused: `border_focused`. Its menu is the menu below, with a check on
  the chosen row. `.compact()` sizes it to its value and opens the menu below with its
  right edge on the button's. With `Controls.select_input` the trigger takes the
  `input_bg` fill, padding 10, gap 8 and a 16 chevron-down (`text_2`; compact: the value's
  color at 75 %).
- **Segmented control:** an outlined group (1 px `line_strong` at 60 %, radius 6) of cells
  split by 1 px dividers; sm cells 22 tall with 12 labels, md and lg 28 / 32 with 14; the
  chosen cell `accent_tint` with an `accent_text` label.
- **Text input:** `input_bg` fill (default `canvas`), 1 px `line`, radius 6, padding 8 × 6, 14 text in the
  editor foreground (`code.editor.foreground`: `#ACB2BE` / `#242529`), placeholder
  `text_3`; focused: `border_focused`; invalid: an `error` border, focused or not, and
  the form field shows the message under it in 12 px `error`. No hover style. `height`
  fixes a single-line input's height (the cards settings use 32).
- **Number input:** a text input with a 12 up and down chevron pair on its right, 160
  wide. **Stepper** (`stepper: true`): one framed control 128 × 32: `input_bg`, 1 px
  `line`, `radius_lg`; a 32 wide `−` button, the value centred, a 32 wide `+` button.
  The buttons are ghost with a 16 icon in `text`, hover `line` at 40 %, press at 60 %,
  only their outer corners rounded. Arrow keys step too.
- **Menu:** level 1 overlay, min width 200, 4 vertical padding. Rows are 23 tall, inset 4
  from the edge with 6 inner padding (text 10 from the edge), radius 4; hover
  `panel_hover`, keyboard highlight `press`. Icons 14 in `text_2`, only when some item has
  one. Shortcut right-aligned, `text_2`, 16 from the label. Separator: 1 px `line`, 6 above
  and below, edge to edge. Section header: 20 tall, 12 px `text_2`. Check marks
  `accent_text`, 14 px in the icon column (`checked` on a `MenuItem`). A submenu (`items` on a
  `MenuItem`): a 12 px `chevron_right` at the row's right end; it opens on hover, click or
  right arrow, left arrow closes it; submenus overlap their row by 2 px. A long menu is capped
  at half the window or 450 px and scrolls. A labelled separator is rule, 12 px `text_2`
  label, rule; a `heading` row is a section header. A row may lead with an 8 px color square
  (`swatch`). Closing a popup gives the focus back
  to what had it when the popup opened (its trigger), so keys and actions reach the window.
- **Tooltip:** level 1, padding 8 × 4, 14 text, max width 288, shortcut muted 16 to the
  right, an optional meta line under it in 12 `text_2`, shown after 500 ms.
- **List row:** 23 tall (dense) or 32 (picker rows, 4 padding), padding 6, radius 4 when
  inset; tree indent 20 with 1 px indent guides; selected `press`.
- **Tabs:** 32 tall, 4 padding, label 14 in `text_2` on `panel` (inactive) and `text` on
  `canvas` (active), 1 px `line_strong` seams on the side away from the active tab, no
  hover fill. The dirty mark is a 6 px `accent` dot, always shown; the close button (18,
  14 icon, in `text`) shows on hover.
- **Picker / command palette:** 544 wide (palette 608), at most 384 tall, level 2. Query
  row 36 tall with 10 padding over a 1 px `line`; rows as list rows; matches `accent_text`;
  shortcuts muted on the right; an optional footer with a top `line`.
- **Dialog:** level 2, 440 wide, header padding 12 / 8 / 4, headline 16, body 14
  `text_2`, footer padding 8 with a top `line`, buttons right-aligned, gap 4.
- **Notifications:** stacked at the bottom right above the status bar, 12 from the edges,
  newest at the bottom, gap 8; each 448 wide, level 2 on `panel`, padding 12: a 16 status
  icon, the title over a `text_2` body, a 22 close button, optional action buttons (sm,
  right-aligned, gap 4; the primary one tinted) whose click also closes the card. They stay
  until closed unless given a timeout.
  Toasts: bottom center, padding 6 / 10, a status icon, auto-hide after 10 s.
- **Scrollbar:** 6 px pill thumb inset 4 from the edge, at least 25 long, `text` at 30 %.
  Every scrolling container has one (scroll areas, overflow containers, lists, the code
  editor, the terminal); strips the wheel scrolls sideways (tab strips) have none. It is
  hidden until the pointer moves over the container, the wheel scrolls it or its content
  moves any other way (keys, the app), stays 1 s after the last such activity, then fades
  out over 400 ms. A shown thumb can be dragged; its 14 px track is the grip.
- **Title bar:** 32, `chrome`; left a 22 ghost icon button (hamburger) that opens the app
  menu, then the document or project name in 12 px; app controls; caption buttons 36 wide
  with 10 px glyphs from Segoe Fluent Icons (Windows 11) or Segoe MDL2 Assets (Windows
  10), close hovers `#E81120`. `.title_size(16)` makes the title a window caption at body
  size, 12 from the left edge (a settings window: 34 tall).
- **Status bar:** 30, `chrome`, padding 8, items gap 12, labels 12; panel toggles are 22
  icon buttons with 14 `text_2` icons whose active state is an accent icon (no fill);
  groups split by a 1 × 14 `line_strong` divider.
- **Settings:** a page column 226 wide (padding 10, a right `line_strong` hairline): the
  search field, page rows 28 tall with a 12 chevron and a 14 label (hover `panel_hover`,
  the chosen page `raised_hover` with a 1 px `line` border, radius 4), a keybinding hint
  at the bottom over a `line`. The page: padding 40 left, 32 right, 24 top and bottom; a
  header row (scope toggles, then actions on the right), the title in `text_xl`, then
  groups: a mono 12 `text_2` header over a `line`, and rows of a 14 title over a 12
  `text_2` description that wraps, the control on the right, 16 above and below, a `line`
  under each. A search shows only matching rows, and pages with none leave the column.
  A page row's chevron (16 hit area) expands the page's groups under it as 26 tall
  `text_2` rows indented 28; picking a group picks its page and scrolls the group's header
  to the top.
- **Settings, cards** (`.cards()`): the layout of outlined setting cards.
  - Navigation: 250 wide, `panel`, then a 1 px `line` on the right. The search field, full
    width, 12 from the top and the sides. Below it, 12 in, page rows 8 apart: 28 tall,
    padding 8, gap 8, radius 4, 14 text: a 16 icon, the label, and when the page has more
    than one titled group a 20 ghost button with a 16 chevron (right; down while open).
    Hover `press` at 80 %; the chosen row `press` with a medium (500) label. Clicking a row
    picks the page and opens its groups; the chevron alone toggles them. The groups list
    under the row, 4 apart, inside a 1 px `line_strong` rule on the left (14 in, then 10
    to the rows), 2 above and below; they are page rows without icons, as wide as the column
    allows (their hover fills the row, not the label). A picked group is
    the chosen row (its page row is not), and the group scrolls to the top of the page.
  - Page header: padding 16, a 1 px `line` under it; the title at 16 (26 line) on the left, the
    header actions on the right (gap 4); an optional page description under it, 14
    `text_2`, 12 below the title.
  - Groups: the page scrolls under the header; each group has 16 around it; its title 16
    `text_2` at line height 1.25 with an optional 14 `text_2` description 4 under it, then
    16 below them the card: a 1 px `line` border, `radius_lg`, padding 16, rows 16 apart.
    A group without a title is the card alone.
  - Rows: the title (14 `text`, 20 line) over the description (14 `text_2`, wraps) on the left, at
    most 60 % of the row; the control on the right, centred, 12 from the text. A row with
    neither title nor description gives the control the whole row.
  - Search keeps the rows whose title or description contains the query (any case),
    the groups that keep a row and the pages that keep a group. When the chosen page has
    none left the first remaining page shows.
- **Keybinding hint:** the keys in a box (radius 4, padding 2, 1 px border of the surface
  blended 16 % toward `text`, fill blended 10 % toward `accent_text`; half in light
  themes) at 10 px, then the text, all mono italic 12 in `text_disabled`.
- **Toolbar / breadcrumbs:** 45, `canvas`; breadcrumbs in the mono face at 14, `text_2`,
  separated by `›` in `text_3`.
- **Keybinding text:** one run per keystroke, modifiers then key joined by `-`, in the order
  Ctrl, Win, Alt, Shift (`Ctrl-Shift-P`, `Shift-PageUp`), chords separated by a space,
  `text_2` (`Controls.plus_keys`: `Ctrl+Shift+P`, `Shift+Page Up`). `key_label` writes One's
  form, `key_text(keys, t)` the theme's.
- **Charts, live tables, calendars, status shapes** have no counterpart in the reference
  editor; they are built from the tokens, sizes and states above (rows 23 or 26, 1 px
  `line` grid, `text_2` axes in 12 px, the theme's series colors).

## Themes

`dark()` is One Dark Pro and `light()` is One Light; they share every token name. Both are
theme files bundled in the kit (`quadra/kit/themes/*.json`), and any theme loads the same way:
`kit.load_theme(path)` or `kit.theme_from_json(text)`. A theme file is an object:

```json
{
	"name": "Night",
	"mode": "dark",
	"palette": { "canvas": "#16181D", "accent": "#7AA2F7" },
	"syntax": { "keyword": "#BB9AF7" },
	"editor": { "selection": "#7AA2F740" },
	"terminal": { "background": "#16181D", "ansi": ["#15161E", "..."] },
	"series": ["#7AA2F7", "..."]
}
```

- `mode` is `dark`, `light` or `high_contrast`; the theme starts from `light()` when it is
  `light`, else from `dark()`, so a file lists only the colors that differ.
- The keys are the field names of the tables above (`palette`), of `Syntax`, `EditorColors`
  and `TerminalColors` (`ansi` takes 16 colors), and `series` the chart colors.
- Colors are `#RRGGBB` or `#RRGGBBAA`. An unknown key, a bad color or a wrong count is an
  error that names its path (`palette.canvass`, `terminal.ansi[3]`).
- `Theme.to_json()` writes a theme with every color, a starting point for a new one.
- Colors a theme leaves unset (alpha 0) get `finish()`'s defaults.

`high_contrast()` follows the OS high-contrast setting: black planes, white text and
borders, yellow focus. Apps may override sizes and faces with struct update syntax.

## Component notes for the developer-tool kit (specs/16)

- Per-frame builders (terminal grid, editor lines, log rows, live tables) go through
  `core.tracked` and contain no `fn [..]` in their hot path (specs/14, Memory).
- Everything stays headless-testable: the terminal runs against a fake PTY in tests, and the
  editor against an in-memory buffer.

## Status

Implemented as above. Under One Dark Pro the switch's on track is `accent` at 40 %; the
reference build this was matched against draws a more saturated blue there.
