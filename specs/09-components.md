# 09 — Components and Theme

Module `quadra.kit`. Styled components built only from `quadra.ui`, `quadra.core` and `quadra.app`.

## Two kinds of component
- **Builders** (button, badge, checkbox...) are created in `render()` each frame. The parent owns the state they show, and they report changes through `core.Listener[E]` callbacks. The tables below show only the payload type, for example `.on_change(bool)`.
- **Views** (input, select, table...) are created once with `cx.new_view`, kept as a field, and placed with `cx.child_view(v.field)`. They own internal state such as caret, scroll or sort, and expose a typed model for the data the parent cares about.

Every component sets its own semantics (role, label, state) and a default `test_id`, and accepts `.style(s)` overrides.

## Theme

```v
pub struct Theme {
pub:
	name   string
	mode   Mode            // .light | .dark | .high_contrast
	colors ColorTokens     // background, foreground, surface, surface_hover, muted, muted_fg,
	                       // border, ring, overlay, shadow, primary(+fg, hover), danger(+fg),
	                       // success, warning, info, selection, link
	radius RadiusTokens    // sm 4, md 6, lg 8, full
	type_  TypeTokens      // family, mono_family, sizes xs 12 .. 3xl 30, weights
	density Density        // .compact | .comfortable: control heights 18/22/28/32 or 22/26/32/36
}
pub fn load_theme(json string) !Theme        // user and app themes from JSON files
pub fn theme(cx &core.Cx) &Theme             // tracked read of the global theme
pub fn set_theme(mut cx core.Cx, t Theme)
pub fn follow_system(mut cx core.Cx, light Theme, dark Theme)   // dark mode, accent, high contrast
```
- `follow_system` swaps the theme on OS changes and tints `primary` from the system accent color when the theme allows it. When the OS reports high contrast, the built-in high-contrast theme wins over both.
- The OS text-scale setting multiplies all type sizes. Layouts must not assume fixed text heights.
- Built-in themes: light (One Light), dark (One Dark Pro) and high contrast (specs/15); more load from JSON theme files (`kit.load_theme`). Themes are plain values, so switching one invalidates only views that read the theme.
- Focus: the control's 1 px border turns `border_focused` (borderless controls show their hover fill), focus-visible only. Disabled: `text_disabled` labels, no hover, removed from tab order.

## v1 components

| Group | Component | Kind | Key API |
|---|---|---|---|
| Actions | `button` | builder | `.label` `.icon` `.primary/secondary/ghost/danger/link()` `.size` `.loading` `.disabled` `.on_click` |
| | `icon_button`, `toggle_button` | builder | `.icon` `.tooltip` `.pressed` `.on_change(bool)` |
| | `segmented` | builder | `.options` `.selected` `.on_change(int)` |
| Forms | `input` | view | placeholder, value, password, prefix, suffix, clear button, `.on_change(string)` `.on_submit` |
| | `textarea` | view | multi-line, auto-grow up to max rows, same buffer as input |
| | `number_input` | view | min, max, step, format, arrow keys and wheel; `stepper: true` is a framed `− value +` control |
| | `checkbox`, `switch` | builder | `.checked` `.indeterminate` `.label` `.on_change(bool)` |
| | `radio_group` | builder | `.options` `.selected` `.on_change(int)` |
| | `slider`, `range_slider` | builder | `.range(min, max, step)` `.width` `.on_change(f32)`; the range slider has two focusable thumbs that never cross, `.on_change(Range)` |
| | `select`, `combobox` | view | options, search, multi-select with chips, async options |
| | `date_picker` | view | calendar popover, locale-aware format from `quadra.app` i18n |
| | `new_calendar` | view | the month grid on its own; keeps the shown month, `on_date(Date)` |
| | `time_input` | builder | hours and minutes as focusable segments: arrows step (wrapping), digits type, `.minute_step` `.on_change(TimeOfDay)` |
| | `form`, `field` | view / builder | label, help, error text, `required`, validation rules, submit state |
| Data | `table` | view | virtualized rows and columns, sort, column resize and reorder, sticky header, row selection, custom cells |
| | `tree` | view | virtualized, lazy children, keyboard expand/collapse, multi-select |
| | `virtual_list` | builder | `ui.list` plus selection, keyboard navigation, empty state |
| | `badge`, `avatar`, `kbd`, `tag` | builder | small display helpers |
| | `progress`, `spinner`, `skeleton` | builder | determinate/indeterminate, paint-only animation (the spinner is a GPU stroke at each frame's angle) |
| Layout | `card`, `divider`, `accordion` | builder | |
| | `split` | view | resizable panes with min sizes, collapse, persisted ratios |
| | `scroll_area` | builder | styled scrollbars, auto-hide |
| | `tabs` | view | underline/pill, closable, overflow menu |
| Navigation | `sidebar` | view | groups, icons, collapsible to icons only |
| | `menubar`, `toolbar`, `breadcrumb` | builder | menubar uses `menu` popups and keymap labels |
| | `command_palette` | view | fuzzy search over registered actions, shows shortcuts as muted labels (`key_label`: "Ctrl+Shift+P"); a spotlight near the top of the window (`ModalSide.top`: no title, no dimming) |
| Overlays | `tooltip`, `popover`, `menu`, `context_menu` | builder / view | submenus, shortcut labels, dismiss on outside click and Escape; a menu is at least 220 px wide and grows for long labels, with an icon column only when an item has an icon |
| | `modal`, `confirm` | service | focus trap, Escape, backdrop; `confirm` returns the choice to a listener |
| | `toast` | service | stack, action button, auto-dismiss, pause on hover |
| | `notify` | service | notification cards stacked at the bottom right (title, body, close, optional actions), kept until closed or for `timeout_ms` |
| | `tooltip` | builder | `.keys` (a shortcut after the text) and `.meta` (a line under it); buttons take `.tooltip_keys(text, keys)` |
| Window | `titlebar` | builder | custom titlebar regions for snap layouts, caption buttons, `.menu(items)` behind a hamburger button |
| | `statusbar` | builder | left and right items; `status_button` (icon-only panel toggles), `status_divider`, `status_item` |
| | `settings` | builder | preferences in pages: a searchable page column whose chevrons expand the groups, groups of rows (title, description, control); the app owns the controls, the query and the chosen page; the component keeps the expansion and the scroll position. `.cards()` is the outlined-card layout (specs/15): page icons, a page header with the actions on the right, an optional page description |
| | `key_hint` | builder | a keyboard hint: the keys in a box, then what they do |

## Text input internals (`kit/input/`)
- `buffer.v`: rope of lines (piece table for > 1 MB), selections as anchor and head byte offsets, NFC on insert, grapheme-aware movement.
- `history.v`: undo and redo, typing coalesced per word, selection restored on undo.
- `actions.v`: actions and default keymap, including word movement and deletion.
- `mouse.v`: click, drag, double-click word, triple-click line.
- `view.v`: render, caret blink as a paint invalidation, horizontal scroll, IME preedit, `set_ime_rect`.

## Table internals (`kit/table/`)
Column model, row source interface (`row_count()`, `cell(row, col)`), sort state, selection model and the view are separate files. Rows and columns are both virtualized, so a table of 1,000,000 × 50 costs the same per frame as one screenful.

## Tests
Unit tests for buffer, history, validation rules, table sort and selection. Interaction tests for every component run on the headless harness (11-devtools-testing).
