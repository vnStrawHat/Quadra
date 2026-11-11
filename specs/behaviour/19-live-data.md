# 19. Live data (M10)

Implementation spec for the data views of specs/16 and the finished `apps/cluster`.
Everything is closure-free per frame (specs/14): rows are built by `uniform_list`, custom
elements paint from their own fields, and listeners are made once.

## Live table (`kit/data`)
- `data.Table` gains keyed updates: `set_keyed_rows(mut cx, t, keys []string, cell)`. Rows
  are matched by key, so the selection, the keyboard cursor and the sort survive an update.
  Rows whose key disappears leave the selection.
- Change flash: `set_keyed_rows` compares every cell of the new data with the value it had
  under the same key (all rows, not just visible ones: 10 000 rows × 7 columns is a few ms
  per update) and records `flash[key:col] = now`. Rows capture the flash map; a changed cell
  is tinted `theme.p.accent_tint` and fades out over 800 ms. The fade is paint-only: a small
  element around the list asks `request_prepaint_after(33)` while a flash runs, and
  `uniform_list` rebuilds the visible rows on every prepaint.
- The cell function is made once by the owner and reads the owner's data (an update passes
  only the new keys), so live updates create no closures.
- `Column.hidden` drops a column; `Column.kind` picks a renderer: `.text`, `.status` (the
  value is a status word drawn with `kit.status_label`) and `.age` (the value is seconds;
  it is drawn relative, `45s`, `12m`, `3h`, `6d`, and sorted as a number).
- Pinned columns are not in M10 (the table scrolls as one block).

## Log view (`kit/data`)
- `LogView` holds lines in one packed byte buffer with an offset per line and an optional
  timestamp (ms) per line: a 100-byte line costs about 112 bytes. `append(lines)`.
- ANSI colors: `term.style_spans(line) (string, []Span)` strips escape sequences and returns
  styled spans using the terminal's SGR code (one implementation of SGR). It runs only for
  painted lines.
- One element paints the visible lines (mono, a timestamp gutter `12:04:05.123`), scrolls
  itself and keeps no per-line layout state beyond the text system's line cache.
- Follow mode: on while the view is at the bottom; scrolling up stops it, reaching the
  bottom again or the Follow button resumes it. New lines scroll a following view.
- Filter: plain text (case-insensitive) or regex; mode `hide` keeps an index of matching
  lines (extended incrementally on append) and `dim` paints non-matching lines at 40 %.
- A new filter scans the whole store once: a byte search for text; for regex it is the
  backtracking `regex` module, seconds at 1 000 000 lines (a ceiling; appends test only the
  new lines).
- Line wrap is not in M10 (long lines scroll horizontally).
- Budget: 1 000 000 lines held; appending 10 000 lines per second keeps a frame under 4 ms.

## Value viewer (`kit/data`)
- `Value` is a tree: null, bool, number, string, list, map (ordered keys).
- `parse_json(s) !Value` and `parse_yaml(s) !Value`. The YAML subset: block mappings and
  sequences by indentation, `- ` items holding maps, flow `{}` and `[]` on one line, plain,
  single- and double-quoted scalars, `|` and `>` block scalars, comments, `---`. Anchors,
  tags and multiple documents are not resolved.
- `ValueView` lists the visible nodes as rows (`uniform_list`): indent, chevron, key in
  `syntax.property`, value in the syntax color of its type, `{n}` or `[n]` for collapsed
  containers. Click or left/right toggles, up/down move, the top two levels start open.
  `Copy path` (ctrl-c) copies the path of the selected node, `.spec.containers[0].image`.

## Charts (`kit/viz`)
- Drawn with quads only, one quad per device pixel column for lines and areas, aligned to the
  pixel grid (fractional columns show seams), so live updates never rasterize paths or fill
  the atlas (the image cache has no eviction). The area is plain quads. The line is one
  `Line` primitive per column in column mode: it carries the line's vertical extent
  (`thick * sqrt(1 + slope²)`) and how far it falls across the column, and the pixel shader
  integrates that trapezoid over each pixel, so every slope gets exact antialiasing at an even
  width.
- `sparkline(values)`: a 1 px line plus an area at 12 %, sized by its parent.
- `line_chart(id, series)`: up to 8 `Series{name, values, color?}` over a shared time axis
  (`start_ms`, `step_ms`), y axis from 0 to a rounded maximum with 4 grid lines and labels,
  time labels at the bottom. Hovering shows a crosshair and the values at that point in
  mono. The chart registers a hover-sensitive hitbox whose mouse-move listener is a plain
  function that asks for a repaint; paint reads the window's mouse position (no state).
- `bar_chart(id, bars)`: labeled bars with values, horizontal.
- Series colors: `theme.series` (8 categorical colors per theme); a series may override.

## apps/cluster (complete)
- Live source: `fake.Cluster.tick()` advances one second deterministically: ages grow,
  CPU and memory move (a 120-sample history per pod), restarts and status changes happen
  on fixed schedules, a pod is replaced every 30 ticks, and each pod writes log lines and
  the cluster emits events. The app ticks every second with `set_interval`; tests call
  `tick` directly.
- Resource table: keyed by namespace/name, status and age renderers, the selection stays on
  the same resource across ticks.
- Detail tabs: Overview (fields plus CPU and memory sparklines), YAML (value viewer),
  Manifest (code editor; Apply parses the YAML and updates the fake resource's image and
  replicas, errors show in the bar), Logs (log view following the pod's stream), Metrics
  (line chart of CPU and memory for a pod; for a deployment, a bar chart of its pods'
  CPU), Shell (pods).
- Events activity: a live table of events (type, reason, object, message, age).
- Harness tests: browse, live updates keep the selection, every detail tab, apply a
  manifest (good and bad), log follow and filter, events, closures flat across ticks.

## Tests and budgets
- `kit/data`: keyed update and flash tests, log store and filter tests, YAML and JSON parser
  tests, value view harness test. `kit/viz`: chart geometry tests (column ranges, axis max).
- `bench/live`: 1 000 000 log lines, 10 000 lines/s append frame time, a 10 000-row keyed
  table update.

## Status (M10 done; deviations)
- Pinned columns and log line wrap are not built (stated above).
- A regex log filter rescans the store in about 1.6 s at 1 000 000 lines; a text filter in
  about 0.3 s. Appends test only the new lines.
- Log lines are not selectable or copyable yet.
- The live table has no filter yet, so "a filter keeps the selection" is not built.
- The value viewer keeps open state by path; set_value never resets it.
- `Cx.touch(model)` was added to core: live sources mutate a model through `peek` and mark
  it changed without the closure that `update` takes (one per tick would leak).
- The Theme's `series` is a dynamic array: V's struct update (`Theme{...dark()}`) does not
  compile with a fixed-array field.
