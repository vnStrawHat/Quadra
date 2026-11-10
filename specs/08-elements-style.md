# 08 — Elements and Style

Module `quadra.ui`. Primitive elements on top of `core.Element`. Everything in `kit` is composed from these.

## Primitive elements

| Constructor | Purpose |
|---|---|
| `div()` | Flex or grid container: box styling, children, events, focus, scrolling, semantics |
| `text(s)` | Text leaf: wraps by default, `.truncate()`, `.selectable()`, inherits text style |
| `rich_text(spans)` | Mixed styles and links in one paragraph: `span(s).bold().italic().color(c).size(v).family(f).link(url)`, `.on_link(l)`. `.ctrl_links()` (user content, every Markdown view): links open on ctrl-click only, the pointer shows only while ctrl is held, a plain click selects like any text, and resting on a link shows the hint "Ctrl+click to open" (web and mail links) or "Ctrl+click to follow" (anchors, files) through a hitbox hint (specs/06); the pieces of one link on a row are one hitbox. Without it a plain click opens (app UI links). Words glued across spans never break. `.selectable_in(group)`: one selection over every rich text of a `ui.select_group()` (prepaint the group before its members), with drag across members, double-click word, triple-click member, shift-click, ctrl-a, ctrl-c (members joined by `\n`). Not yet: inline icons, selecting inline boxes |
| `icon(name)` | Kit icon from the bundled icon font (mono, tinted by color) |
| `svg(src)` | SVG from asset or bytes, rasterized at device size, cached |
| `img(src)` | Image from asset, path, bytes or URL; `.fit(.cover \| .contain \| .fill)`, async with placeholder |
| `list(id, count, render)` | Virtualized list of rows sized by their content (below); `.estimate(h)`, `.reveal(i)`, `.scroll_y(y)`. `uniform_list(id, count, row_h, render)` for equal rows |
| `canvas(paint_fn)` | Custom painting: quads, paths (fill, stroke), text, images |

Image caps: `img` decodes a file only when its header says at most `ui.max_image_side`
(16384) pixels a side and `ui.max_image_pixels` (16 Mi, 64 MiB of RGBA) in all, from a file of
at most `ui.max_image_file_bytes` (256 MiB), whose header claims at most
`ui.max_pixels_per_byte` (16384) pixels per byte of file (no format packs more; a lying header
made the decoder allocate its frame buffers first), and, for PNG, whose chunks all fit in the
file (the decoder sizes its data buffer from a chunk's claimed length). Anything else fails like
a corrupt file (the element stays empty, no pixel decoded). A decode holds two RGBA buffers at once (the decoder's and the
premultiplied copy), so one image costs at most 128 MiB whatever its header claims. Decoding
runs on a worker thread and returns errors, never panics. The atlas shows up to 2047 px a side
(specs/03); larger images fail at upload until images are downscaled.

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
| Box | `bg(c)` `border(n)` `border_color(c)` `rounded(n)` `rounded_full()` `shadow(level)` `opacity(f)` `ring(w, c)` (a band `w` wide just outside the border box, its corners the box's plus `w`; only the band is painted, so a translucent fill stays as it is; one whole-pixel width on every side; an ancestor that clips that axis cuts it) |
| Motion | `translate(x, y)` `scale(k)` `rotate(deg)` (about the center, paint only: layout and hit testing keep the box; text is rasterized again at the scaled size in quarter px steps), `transition(ms)` `transition_with(ms, easing)`: background, border color, opacity, translate, scale and rotation ease to new values, from a state patch (`Patch.translate`, `.scale`, `.rotate`) or a new render. The first paint shows the target at once; a new target starts from the current value. `animate(m)` plays a timeline from the first paint: `ui.tween(ms, patch)` and `ui.tween_with(ms, easing, patch)` ease towards a patch, `ui.pause(ms)` waits, `a + b` runs b after a, `ui.parallel(a, b, ...)` runs them together (a later one wins a property both set), `m.repeat(n)` `m.forever()` `m.alternate()`. Each step starts from where the property is; the end values stay after the last step; restart by changing the element's key |
| Text (inherited) | `text_size` `text_color` `font_weight` `font(family)` `line_height` `tabular_nums()` |
| Overflow | `overflow_hidden()` `overflow_y_scroll()` `overflow_x_scroll()` |
| States | `hover(s Style)` `active(s Style)` `focus_visible(s Style)` `disabled(bool)`; hover groups: `group()` marks a Div, `group_hover(p)` and `group_active(p)` restyle a descendant while its nearest group is hovered or pressed (a close button shown only while its card is hovered). Paint-only, like the other state patches |
| Interaction | `id(s)` `cursor(c)` `track_focus(h)` `tab_index(n)` `key_context(s)` `tooltip(s)` |

Tooltips for parts of an element: `kit.tooltip` wraps a whole element; a link in rich text, a
URL in the editor or in the terminal is a rect inside one. Such a part sets
`HitboxOptions.hint` on its hitbox (specs/06, Hover hints) and `kit.HintLayer`, which the kit's
Host adds after the app root, draws the bubble with the tooltip's code (`tip_bubble`): the same
delay (500 ms), style and placement (above the hitbox with a 6 px gap; at the pointer + 1 with
roomy menus). It closes on a press, a wheel or the pointer leaving, and stays while ctrl is
held (the pointer then shows too). The URL itself is not in the hint: it is a constant, so a
link costs no string per prepaint.
| Events | `on_click` `on_mouse_down/up/move` `on_wheel` `on_key_down` `on_action[T]` `on_drag_start` `on_drop` |
| Semantics | `role(r)` `label(s)` `value(s)` `state(...)` `test_id(s)` |
| Tree | `child(el)` `children(els)` `when(cond, fn (mut Div))` |

- State styles take `Style` values, not closures. Core knows at build time whether a patch is paint-only (07-state).
- Lengths: numbers are logical px; `pct(n)`, `auto`, `fill`, and track sizes such as `fr(1)` and `minmax(200, fr(1))`.
- `test_id` is free in release builds: the harness queries it, and it is stripped with `-d quadra_no_test_ids`.

## Device pixels
Layout stays in fractional logical px; painting puts boxes on the device pixel grid. Every quad painted without a rotation or a scale transform has its edges rounded to whole device pixels in window space (ties toward zero, so a 1 px edge at 150 % stays 1 dp, and two edges at the same position always land on the same pixel) and each border width rounded the same way, at least 1 dp when it is above 0. Clip rects grow outward to whole pixels so they never cut a snapped edge. So a box at y 138.4 paints from 138 with crisp 1 px borders instead of two half-covered rows; a ring, border or separator looks the same on every side and at every scale. Glyphs and terminal cells are snapped as before; lines, strokes, shadows and anything under a rotation or scale (animations) stay exact. The cost is four roundings per quad.

## Clipping
A div clips only the axes it does not leave visible: `overflow_y_scroll()` clips at its top and bottom but not at its sides, so a child's focus ring, shadow or ring band can show beside a vertical scroller (and above or below a horizontal one). `overflow_hidden()` clips both.

## Scrolling
A div with `overflow_*_scroll()` and an `id` keeps its offset in element state and gets its own chunk. Wheel input scrolls by lines, touchpad input scrolls by pixels with momentum supplied by the OS. Scrolling is a compose-level change. Styled scrollbars come from `kit.scroll_area`.

## Variable-height lists
- Unmeasured rows use an estimated height. A row is measured when it first becomes visible, and the real height replaces the estimate.
- Heights are kept in a Fenwick tree, so offset-to-row and row-to-offset lookups are O(log n) for millions of rows.
- When rows above the viewport change height, the scroll position is anchored to the first visible row, so content never jumps.
- `.reveal(row)` works for unmeasured rows: a row below the viewport is measured upward from itself, so it lands exactly at the bottom edge in the same frame.

## Tests
`styled_test.v` (builder and `apply` merging), `paint_class_test.v` (layout or paint classification of every style field), `list_test.v` (Fenwick lookups, anchoring), `tessellate_test.v` for canvas paths.
