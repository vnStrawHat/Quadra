# 21. Folio: a Markdown reader and editor

Folio opens `.md` files, shows them rendered, and edits them live. It is for reading and
writing Markdown every day: notes, READMEs, specs, docs with formulas and diagrams. It is
built only with public framework APIs, like Notes (specs/13), and lives in `apps/folio`.
What it needs from the framework (math, diagrams, bundled fonts) goes into the framework,
so every app gets it.

## Principles
- **The document first.** Chrome is one titlebar, an optional outline and a status bar. No
  ribbons, no toolbars full of formatting buttons: Markdown is typed.
- **Instant.** Opening a 1 MB file shows the first screen in under 100 ms. A keystroke in a
  2 000-line document reaches the preview in the same frame (under 4 ms of CPU).
- **Nothing lost.** Unsaved changes are marked, asked about on close, and never discarded by
  a reload.
- **One look.** A flat editor style: panels one step off the canvas, 1 px lines, no shadows
  except on overlays, a muted palette with one accent, a sans for the interface and a mono
  for text that is code.

## Window
```
┌ titlebar ───────────────────────────────────────────────────────────────┐
│ ≡  notes                                                        ─ □ ✕   │
├ outline ─────┬ tabs ─────────────────────────────────────────────────────┤
│ Plan         │ ▤ plan.md ●  │ ▤ ideas.md  ×                         + ⋯  │
│ │ Goals      ├ toolbar ─────────────────────────────────────────────────┤
│ │ Steps      │ plan.md › Plan › Goals     H │ B I S <> Σ │ … │ ✎ ▥ ▯     │
│ │ │ Build    ├ editor ──────────────────┬ preview ──────────────────────┤
│              │  1 # Plan                │  Plan                         │
│              │  3 ## Goals              │  Goals                        │
│ Search…      │  4 - ship the reader     │  • ship the reader            │
├──────────────┴──────────────────────────┴───────────────────────────────┤
│ ☰                                  4:12   412 words, 2 min   Markdown   │
└─────────────────────────────────────────────────────────────────────────┘
```
The look follows a code editor's workbench: title and status bars one step lighter than
the panels (`chrome` in the palette), panels one step lighter than the text area, flat
tabs, a breadcrumb toolbar, muted key labels ("Ctrl+Shift+P"), a spotlight palette.
- **Titlebar** (custom, kit.titlebar): the app menu (≡) and the folder of the shown file,
  like a project name, then the window buttons. Double-click and drag behave as the OS bar.
- **Outline** (left, 220 px, hidden with ctrl-shift-o or the status bar button): the
  headings as a tree with indent guides. The section at the reading line (120 px into the
  preview, or five lines into the editor in Raw) is marked and follows scrolling. A click
  scrolls the editor (Raw, Split) and the preview (Split, Preview) to the heading. A
  search field at the top (32 px, a search icon) filters the headings.
- **Tabs** (kit work tab strip): one per open document, with a file icon, a dot for
  unsaved changes and a close button (shown on hover, middle click closes too); drag
  reorders; + starts a new document.
- **Toolbar**: a breadcrumb on the left (the file, then the headings down to the section
  being read); on the right the formatting tools (Raw and Split) and the layout buttons
  (Raw, Split, Preview).
- **Editor** (kit.code editor): Markdown highlighting (headings, emphasis, code, links,
  lists, quotes, math, fences), line numbers, soft wrap on, find and replace (ctrl-f, ctrl-h)
  as in the code editor.
- **Formatting** (the toolbar, Raw and Split): a Heading menu (normal text, heading 1 to
  3); bold, italic, strikethrough, inline code, inline math; link; bulleted, numbered and
  task lists, quote; an Insert menu (image, code block, math block, Mermaid diagram, table,
  horizontal rule). Icon buttons with tooltips naming the keys. Each action is a command
  (keys, palette), edits the Markdown text as one undo step and keeps the editor focused:
  - marks wrap the selection (or the word at the caret, or insert an empty pair) and come
    off again when the selection is already marked, just outside or just inside;
  - headings, lists and quotes apply to every selected line, and come off when every line
    has them; a list replaces another list's marker, a numbered list counts from 1;
  - links: the selection becomes the label and the URL is selected to type over (a selected
    URL becomes the target instead); blocks go on lines of their own with blank lines around.
- **Preview**: the rendered document in a reading column (max 820 px wide with 40 px side
  padding, centered), scrolled independently. In Split, scrolling the editor moves the preview so
  the block at the editor's top line is at the preview's top (one way: editor → preview).
- **Status bar**: the outline button on the left; the cursor's line:column (not in
  Preview), words and reading time (230 words a minute), the language.
- **Empty state** (no tab): a welcome page (specs/15): actions with their keys (New, Open, Command palette)
  and the five most recent files.

## Files
- **Open**: ctrl-o (file dialog, several files at once, filter `*.md;*.markdown;*.txt`),
  dropping files on the window, the command line (`folio a.md b.md`), or a recent file.
  Each file opens in a tab after the shown one; a file already open shows its tab; an
  untouched new document is replaced. ctrl-shift-n opens a new, empty window.
- **Session**: the open files and the shown one are kept in the settings and opened again
  at the next start (when no file is given).
- **Save**: ctrl-s writes in place (UTF-8, line endings kept as read); ctrl-shift-s saves as.
  A new document asks for a path on its first save. Writes go to a temp file in the same
  folder and are renamed over the original, so a crash never leaves half a file.
- **Close**: closing a tab or the window with unsaved changes asks (Save, Don't save,
  Cancel), naming every document concerned.
- **Outside changes**: the file's modification time is checked every 2 s (one stat call). Changed on disk and
  unchanged here: reloaded silently, keeping the scroll position. Changed in both: a banner
  offers Reload or Keep mine.
- **Recent files**: the last 10, kept in the app settings (quadra.app settings), with the
  layout and outline visibility per window.
- **Links**: `http(s)` links open in the browser. A relative link to another `.md` opens it
  in a tab. `#anchor` links scroll to the heading.
- **Images**: `![alt](path)` with a path relative to the file, or an absolute one, shown
  scaled to the column width. Remote images show their alt text and the url (no network).

## Keyboard
| Keys | Action |
|---|---|
| ctrl-o / ctrl-s / ctrl-shift-s | open / save / save as |
| ctrl-n / ctrl-shift-n | new document / new window |
| ctrl-1 / ctrl-2 / ctrl-3 | Raw / Split / Preview |
| ctrl-shift-o | show or hide the outline |
| ctrl-shift-p | command palette (every action, with its keys) |
| ctrl-f / ctrl-h | find / replace in the editor |
| ctrl-= / ctrl-minus / ctrl-0 | preview text larger / smaller / default |
| ctrl-w / ctrl-shift-w | close the tab / the window |
| ctrl-tab / ctrl-shift-tab (ctrl-pagedown / ctrl-pageup) | next / previous tab |
| ctrl-b / ctrl-i / ctrl-shift-x / ctrl-e / ctrl-shift-m | bold / italic / strikethrough / inline code / inline math |
| ctrl-k | link |
| ctrl-alt-0 / 1 / 2 / 3 | normal text / heading 1 to 3 |
| ctrl-shift-8 / 7 / 9 / q | bulleted / numbered / task list / quote |

## Markdown
The kit's Markdown module (`kit/md`) grows to cover what everyday documents use. Each block
records its source line, which the outline, the scroll sync and the tests use.
- **Blocks**: ATX headings, paragraphs, bullet, numbered and task lists (`- [ ]`,
  `- [x]`, shown as checkboxes, read-only), fenced code (highlighted by language), quotes
  (nested), rules, pipe tables with alignment, images, math blocks, Mermaid blocks, YAML
  front matter (hidden from the preview, shown as a muted block when it is the only content).
- **Inline**: bold, italic, bold-italic, strikethrough (`~~`), code, links, autolinks
  (`<https://...>` and bare `https://...`), inline math, hard breaks (two spaces or `\`).
- **Not handled**: raw HTML (shown as text), setext headings, indented code, reference
  links, footnotes. They are rare in the documents Folio is for and render as plain text.
- **Headings** get anchors (lowercased, spaces to `-`, punctuation dropped), as most hosts
  make them, for `#anchor` links.

## Math (`kit/md/math`)
TeX math, the subset that notes and technical docs use, laid out as boxes in the TeX way
and painted with the text and stroke primitives. Glyphs come from Cambria Math (shipped with
Windows); without it, the default font.
- **Where**: `$...$` inline (not when the `$` is followed by a space or a digit after a
  space, so prices stay text), `$$...$$` display (alone or spanning lines), fenced
  ```` ```math ```` blocks.
- **Supported**: letters (italic) and digits (upright); `^` and `_` with braces, both on
  one base; `\frac`, `\dfrac`, `\tfrac`, `\binom`; `\sqrt` and `\sqrt[n]`; `\left` / `\right`
  with `( ) [ ] \{ \} | \| \langle \rangle .` growing to the content; big operators `\sum
  \prod \int \oint \iint \bigcup \bigcap \lim \max \min \sup \inf` with limits above and below
  in display, at the side inline; function names `\sin \cos \tan \log \ln \exp \det` and
  `\operatorname{}`; accents `\hat \bar \vec \dot \ddot \tilde \overline \underline`;
  fonts `\mathbf \mathit \mathrm \mathsf \mathtt \mathbb \mathcal \text`; Greek letters,
  relations, arrows and operators by their TeX names (mapped to Unicode); spacing `\, \: \;
  \! \quad \qquad`; `matrix`, `pmatrix`, `bmatrix`, `vmatrix`, `cases` and `aligned`
  environments with `&` and `\\`.
- **Errors**: an unknown command or unbalanced braces shows the source in the danger color
  with the problem as a tooltip; the rest of the document renders.
- **Layout**: each atom is a box (width, height above the baseline, depth below). Styles
  display, text, script and scriptscript scale by 1, 1, 0.7 and 0.5. Fraction bars and
  radicals are lines and paths; delimiters grow by size variants, then by drawn paths beyond
  the font's largest. Inline math sits on the text baseline inside rich text: rich text gets
  inline boxes (a run with a fixed size and baseline, painted by its element).

## Diagrams (`kit/md/mermaid`)
Mermaid source in ```` ```mermaid ```` fences, drawn natively (no browser, no script).
- **Flowcharts**: `graph` / `flowchart` with `TD`, `TB`, `BT`, `LR`, `RL`; nodes
  `id`, `id[text]`, `id(text)`, `id([text])`, `id((text))`, `id{text}`, `id[[text]]`,
  `id[(text)]`, `id>text]`; edges `-->`, `---`, `-.->`, `==>`, `--o`, `--x`, with labels
  `-->|text|` and `-- text -->`; chains `a --> b --> c` and `a & b --> c`; `subgraph ... end`
  (a framed group); `%%` comments; `classDef`, `style` and `click` lines are ignored.
- **Sequence diagrams**: `participant` / `actor` with `as` aliases, messages `->`, `-->`,
  `->>`, `-->>`, `-x`, `-)`, notes `Note left of / right of / over`, `loop`, `alt` / `else`,
  `opt` frames, `autonumber`, activation is ignored.
- **Other kinds** (class, state, gantt, pie, ...) show the source as a code block with a
  note that the kind is not supported.
- **Flowchart layout**: layered. Ranks by longest path from the sources (cycles broken
  by reversing back edges found by DFS), nodes ordered in each rank by barycenter sweeps
  (four passes, down and up), coordinates by a median alignment, edges as polylines through
  dummy nodes and smoothed into curves, arrowheads as filled paths. Labels sit on the edge's
  middle segment on the canvas color.
- **Look**: nodes on the panel color with a 1 px line border and the text color, the
  accent for edges and arrows; it follows the theme. Diagrams wider than the column scroll
  sideways.
- **Errors**: a line that does not parse shows the source with the line marked.

## Look
- **Theme**: two presets in the kit, `dark` (One Dark Pro) and `light` (One Light), following the system
  setting until one is chosen in the palette (the choice is saved). Canvas `#23272E` /
  `#FAFAFA`, panels a step off it, lines `#3E4452` / `#DFDFE0`, text `#DCE0E5` / `#242529`,
  accent `#74ADE8` / `#5C78E2`; syntax colors from the same family (keywords purple, strings green, numbers
  orange, headings red in the editor).
- **Fonts**: the kit's bundled faces (specs/15): IBM Plex Sans for the interface and the
  preview, Lilex for the editor and code.
- **Sizes**: the kit's (specs/15): interface 14 px, editor 15 px with a 1.618 line
  height, preview body 16 px with a 1.5 line height, headings 1.75, 1.4, 1.2, 1, 0.875,
  0.85 em with the first two underlined by a 1 px line.

## Performance
- The preview parses the whole document on every change (the parser is linear: a 2 000-line
  document parses in well under 1 ms) and caches the laid-out math and diagrams by source,
  so a keystroke re-lays out only the block being typed in.
- Large documents: blocks are the unit of layout; the preview builds every block (documents
  are small enough), measured in `bench`. Above 10 000 lines it switches to a virtual list
  of blocks (`ui.list`).

## Tests
- `kit/md`: parser cases for every block and inline kind, with source lines.
- `kit/md/math`: box metrics of fractions, scripts and roots; a pixel test of a formula.
- `kit/md/mermaid`: parsing every node and edge form; layout has no overlapping nodes and
  edges end at node borders; a pixel test of a flowchart and a sequence diagram.
- `apps/folio`: open a sample, switch the three layouts, type and see the preview change,
  click an outline entry and see both panes scroll, save, close with unsaved changes.
- `bench`: a keystroke in a 2 000-line document.

## Plan
1. Spec (this), the parser's source lines and new blocks, the Markdown tokenizer.
2. The app: window, layouts, outline, files, settings, status bar.
3. Theme presets and bundled fonts (in-memory fonts in the text system).
4. Math: parser, box layout, painting; display then inline (rich text inline boxes).
5. Mermaid: flowcharts, then sequence diagrams.
6. Walkthrough: use it on real documents, fix what gets in the way, record it in
   docs/progress.md.
