# Colour

Colour says what a thing is, never how important the designer felt that day. Three kinds of
colour exist and they do not mix: **planes** (where you are), **text** (how loud it is), and
**meaning** (the accent, feedback, hues). Every colour in a view is a token of the theme
(`quadra_design::token`); a literal colour in a view is a bug, but for content that is a colour
(a swatch the user chose, a terminal's cells).

Rules:

1. **A plane is chosen by depth, not by taste.** `canvas` is where the work is (editor,
   terminal, fields); `panel` is beside it; `chrome` is the window's frame; `overlay_bg` floats
   over all three. A view never invents a fourth tone to "set something apart": it uses a
   hairline.
2. **Three text colours, one job each.** `text` is what is read; `text_2` is what explains
   (labels, descriptions, inactive items); `text_3` is what may be missed (placeholders,
   separators, counts). `text_disabled` is for what cannot be used. Nothing else colours text
   but meaning.
3. **The accent marks one thing a view.** What is chosen, focused or on. Never a decoration,
   never a large fill, never two accents side by side but for the chosen state of siblings.
4. **Feedback colours are for states of the world**: `ok`, `warn`, `error`, `info`. A red
   button deletes; red text is an error; nothing else is red.
5. **Hues tell kinds apart and are never alone**: a shape, a letter or a label goes with each.
6. **Contrast**: text on its plane is 4.5:1 or more (`text_3` 3:1, it is never the only
   carrier of a fact); a border that carries a state (focus, error) is 3:1 against its plane.

## Planes and borders

The palette keeps the kit's role names; each role is one token of the One themes.

| Field | Role | One Dark Pro | One Light |
|---|---|---|---|
| `canvas` | editor, terminal, toolbar, active tab, input fields | `#23272E` | `#FAFAFA` |
| `panel` | side panels, surfaces | `#1E2227` | `#EBEBEC` |
| `chrome` | title bar, the window behind empty panes | `#1A1D22` | `#DCDCDD` |
| `overlay_bg` | the plane of every overlay: menus, tooltips, popovers, pickers, dialogs, toasts (the One themes give it the `panel`'s colour; another theme may set it apart) | `#1E2227` | `#EBEBEC` |
| `raised` | filled controls (element background) | `#404754` | `#EBEBEC` |
| `panel_hover`, `raised_hover` | ghost and filled hover | `#ABB2BF14` | `#DFDFE0` |
| `press` | pressed, selected rows, keyboard-highlighted rows | `#ABB2BF29` | `#CACACA` |
| `line` | dividers, hairlines, the grid of tables | `#31363F` | `#DFDFE0` |
| `line_strong` | outlines of controls, overlays and cards; tab and pane seams | `#3E4452` | `#C9C9CA` |
| `border_focused` | focus border: the accent | `#74ADE8` | `#5C78E2` |
| `scrim` | behind modal dialogs: none | transparent | transparent |
| `status_bg` | the status bar (default `chrome`) | `#1A1D22` | `chrome` |
| `tab_bg` | tab bar and inactive tabs (default `panel`) | `#1E2227` | `panel` |
| `scroll_thumb` | scroll thumbs (default `text` at 30 %) | `#4E566680` | `text` 30 % |
| `input_bg` | text fields, steppers and dropdown triggers (default `canvas`) | `canvas` | `canvas` |

Roles a theme may split off; unset, each follows the role in brackets: `selection`
(selected text, in fields and in selectable text: the accent at 30 % in both themes;
`accent_tint`), `tab_fg` and `tab_active_fg` (tab labels; the
active one is also any tab under the pointer; `text_2`, `text`), `table_head_bg`,
`table_head_fg` (`panel`, `text_2`), `table_row_border` (`line_strong`), `list_even`
(striped rows; none), `scroll_thumb_hover` (`scroll_thumb`), `text_on_status` (on
feedback fills; `text_on_accent`), `error_hover` (`error` 15 % toward `canvas`), `cyan`
(the syntax type color).

## Text

| Field | Role | One Dark Pro | One Light |
|---|---|---|---|
| `text` | primary text and icons | `#DCE0E5` | `#242529` |
| `text_2` | muted: inactive tabs, secondary labels, shortcuts | `#A9AFBC` | `#58585A` |
| `text_3` | placeholder, hints, breadcrumb separators | `#878A98` | `#7E8086` |
| `text_disabled` | disabled labels and icons, a step past `text_3` | `#636B78` | `#A2A3A7` |
| `accent_text` | selected or toggled labels, links, matches | `#74ADE8` | `#5C78E2` |

Icons take a text color (`text`, `text_2` for decoration, `accent_text` when toggled).

## Accent and feedback

- `accent` (`#74ADE8` / `#5C78E2`) is the editor cursor, a switch that is on, a checked
  box, the marks of what is chosen, the progress fill and the dirty dot. `accent_tint`
  (`#74ADE829` / `#5C78E229`, the accent at 16 %) fills tinted buttons and the chosen
  segment.
- Feedback colors with a tint each for banners and tinted buttons. Error text is readable
  (4.5:1 and more) on the canvas of its theme:

  | Field | One Dark Pro | tint | One Light | tint |
  |---|---|---|---|---|
  | `ok` | `#A1C181` | `#A1C1811A` | `#669F59` | `#DFEADB` |
  | `warn` | `#D19A66` | `#D19A6626` | `#A48819` | `#A4881926` |
  | `error` | `#E06C75` | `#E06C7526` | `#CA3A2E` | `#CA3A2E26` |
  | `info` | `#74ADE8` | `accent_tint` | `#5C78E2` | `accent_tint` |

- **Workload status** (derived; the themes reuse the feedback and syntax hues):

  | Status | Shape | Color |
  |---|---|---|
  | `running` | filled square | ok |
  | `pending` | hollow square | warn |
  | `failed` | filled triangle | error |
  | `succeeded` | check | cyan (syntax type) |
  | `terminating` | dash | violet (syntax keyword) |
  | `unknown` | filled diamond | `text_3` |
  | `none` | nothing, its room kept | |

- **Hues** (principle 9): kinds, each with a shape, letter or label beside it. The light
  ones are darker, readable as text on `#FAFAFA` (4.3:1 and more). A badge takes one as its
  tone (`Tone::Hue(token::HUE_BLUE)`), an icon and a row of a tree as their colour
  (`icon(..).color(..)`, `TreeNode::color`, which colours the row's icon too).

  | Field | One Dark Pro | One Light | Used for |
  |---|---|---|---|
  | `hue_red` | `#E06C75` | `#CA3A2E` | deleted, errors |
  | `hue_orange` | `#D19A66` | `#A0650A` | warnings |
  | `hue_yellow` | `#E5C07B` | `#8F6B00` | modified |
  | `hue_green` | `#98C379` | `#3B8539` | new, added |
  | `hue_cyan` | `#56B6C2` | `#0B7499` | types, kinds of objects |
  | `hue_blue` | `#61AFEF` | `#3566D6` | functions, information |
  | `hue_purple` | `#C678DD` | `#A626A4` | keywords, modules |

## Code and terminal

Theme tokens, light and dark: syntax (keyword, control, string, number, constant, type,
function, variable, property, operator, punctuation, comment, tag, attribute, regexp,
escape, invalid), editor (cursor, active line `#2F343EBF` / `#EBEBECBF`, line number
`#4E5A5F` / `#B4B4BB`, active line number `#D0D4DA` / `#44454B`, selection = accent at 24 %,
inactive selection at half that, find matches, bracket match, indent guide, whitespace,
diff tints) and the terminal's 16 ANSI colors.

## Themes

`dark()` is One Dark Pro and `light()` is One Light; they share every token name. Both are
theme files bundled in the kit (`crates/quadra-design/themes/*.json`), and any theme loads the same way:
`theme_from_json(text)` (specs/13). A theme file is an object:

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

`high_contrast(colors)` follows the OS high-contrast setting: the colours the user chose for
it (planes, text and borders, the highlight for focus and what is chosen; specs/13). Apps may override sizes and faces with struct update syntax.
