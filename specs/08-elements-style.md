# 08 — Elements and Style

Module `quadra.ui`. Primitive elements on top of `core.Element`. Everything in `kit` is composed from these.

## Primitive elements

| Constructor | Purpose |
|---|---|
| `div()` | Flex or grid container: box styling, children, events, focus, scrolling, semantics |
| `text(s)` | Text leaf: wraps by default, `.truncate()`, `.selectable()`, inherits text style |
| `rich_text(spans)` | Mixed styles and links in one paragraph: `span(s).bold().italic().color(c).size(v).family(f).link(url)`, `.on_link(l)`. Words glued across spans never break. Not yet: inline icons, selection |
| `icon(name)` | Kit icon from the bundled icon font (mono, tinted by color) |
| `svg(src)` | SVG from asset or bytes, rasterized at device size, cached |
| `img(src)` | Image from asset, path, bytes or URL; `.fit(.cover \| .contain \| .fill)`, async with placeholder |
| `list(id, count, render)` | Virtualized list of rows sized by their content (below); `.estimate(h)`, `.reveal(i)`, `.scroll_y(y)`. `uniform_list(id, count, row_h, render)` for equal rows |
| `canvas(paint_fn)` | Custom painting: quads, paths (fill, stroke), text, images |

`div()` is the only element with the full box-style builder. Leaves have their own few methods and are wrapped in a `div()` for padding, background or borders. One builder implementation instead of one per element type.

## Style values
A style is a plain value. It can be built once, stored in a constant and applied many times:

```v
const card = ui.style().flex_col().gap(12).p(16).rounded(8).border(1)

ui.div().apply(card).bg(t.surface).child(...)
```
- `Style` holds a layout part (`layout.Style`) and a paint part (`PaintStyle`), plus a bitmask of set fields so `apply` merges in O(fields set).
- `div()` returns `&Div`; every builder method mutates and returns it, so chains never copy.
- Kit components accept `.style(s)` to let apps override their defaults without forking them.

## Builder methods

| Group | Methods |
|---|---|
| Flex | `flex_row()` `flex_col()` `wrap()` `gap(n)` `gap_x(n)` `gap_y(n)` `grow()` `shrink_0()` `basis(l)` |
| Grid | `grid()` `cols(tracks...)` `rows(tracks...)` `auto_rows(t)` `areas(rows)` `area(name)` `col(start, span)` `row(start, span)` `col_span(n)` `dense()` |
| Align | `justify_*()` `items_*()` `content_*()` `self_*()` |
| Size | `w(l)` `h(l)` `size(l)` `w_full()` `h_full()` `min_w` `max_w` `min_h` `max_h` `aspect(r)` |
| Spacing | `p` `px` `py` `pt/pr/pb/pl` and `m*` |
| Position | `absolute()` `top/right/bottom/left(n)` `inset(n)` `z(n)` (paint order among siblings) |
| Box | `bg(c)` `border(n)` `border_color(c)` `rounded(n)` `rounded_full()` `shadow(level)` `opacity(f)` `ring(w, c)` (a band `w` wide just outside the border box, its corners the box's plus `w`; only the band is painted, so a translucent fill stays as it is; a clipping ancestor cuts it) |
| Motion | `translate(x, y)` `scale(k)` `rotate(deg)` (about the center, paint only: layout and hit testing keep the box; text is rasterized again at the scaled size in quarter px steps), `transition(ms)` `transition_with(ms, easing)`: background, border color, opacity, translate, scale and rotation ease to new values, from a state patch (`Patch.translate`, `.scale`, `.rotate`) or a new render. The first paint shows the target at once; a new target starts from the current value. `animate(m)` plays a timeline from the first paint: `ui.tween(ms, patch)` and `ui.tween_with(ms, easing, patch)` ease towards a patch, `ui.pause(ms)` waits, `a + b` runs b after a, `ui.parallel(a, b, ...)` runs them together (a later one wins a property both set), `m.repeat(n)` `m.forever()` `m.alternate()`. Each step starts from where the property is; the end values stay after the last step; restart by changing the element's key |
| Text (inherited) | `text_size` `text_color` `font_weight` `font(family)` `line_height` `tabular_nums()` |
| Overflow | `overflow_hidden()` `overflow_y_scroll()` `overflow_x_scroll()` |
| States | `hover(s Style)` `active(s Style)` `focus_visible(s Style)` `disabled(bool)`; hover groups: `group()` marks a Div, `group_hover(p)` and `group_active(p)` restyle a descendant while its nearest group is hovered or pressed (a close button shown only while its card is hovered). Paint-only, like the other state patches |
| Interaction | `id(s)` `cursor(c)` `track_focus(h)` `tab_index(n)` `key_context(s)` `tooltip(s)` |
| Events | `on_click` `on_mouse_down/up/move` `on_wheel` `on_key_down` `on_action[T]` `on_drag_start` `on_drop` |
| Semantics | `role(r)` `label(s)` `value(s)` `state(...)` `test_id(s)` |
| Tree | `child(el)` `children(els)` `when(cond, fn (mut Div))` |

- State styles take `Style` values, not closures. Core knows at build time whether a patch is paint-only (07-state).
- Lengths: numbers are logical px; `pct(n)`, `auto`, `fill`, and track sizes such as `fr(1)` and `minmax(200, fr(1))`.
- `test_id` is free in release builds: the harness queries it, and it is stripped with `-d quadra_no_test_ids`.

## Scrolling
A div with `overflow_*_scroll()` and an `id` keeps its offset in element state and gets its own chunk. Wheel input scrolls by lines, touchpad input scrolls by pixels with momentum supplied by the OS. Scrolling is a compose-level change. Styled scrollbars come from `kit.scroll_area`.

## Variable-height lists
- Unmeasured rows use an estimated height. A row is measured when it first becomes visible, and the real height replaces the estimate.
- Heights are kept in a Fenwick tree, so offset-to-row and row-to-offset lookups are O(log n) for millions of rows.
- When rows above the viewport change height, the scroll position is anchored to the first visible row, so content never jumps.
- `.reveal(row)` works for unmeasured rows: a row below the viewport is measured upward from itself, so it lands exactly at the bottom edge in the same frame.

## Tests
`styled_test.v` (builder and `apply` merging), `paint_class_test.v` (layout or paint classification of every style field), `list_test.v` (Fenwick lookups, anchoring), `tessellate_test.v` for canvas paths.
