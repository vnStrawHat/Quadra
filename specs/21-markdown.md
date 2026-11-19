# 21. Markdown

Markdown is an app that opens `.md` files to read them, and edits them. It lives in
`apps/markdown` (package `markdown`, binary `markdown`). The user started it on 2026-11-19.
It is drawn with `quadra-design` in the kit's language. A document is `quadra-md`'s view
(13, "Markdown"), and its source is `quadra-code`'s editor (13, "Code").

## Milestones

Each milestone is committed once `tools/check.sh prod` is green, before the next starts.

| Id | Contents |
|---|---|
| M1 | The window, a file opened from the command line in Preview, the three layouts |
| M2 | The editor: editing, saving, opening, new, the unsaved mark, asking before losing work |
| M3 | Side by side: the panes scroll together by their headings, and a heading chosen on one side is shown on the other |
| M4 | The toolbar: formatting and quick inserts |
| M5 | The logo |

## Window

- One window, titled `<file name> - Markdown`. An unsaved document is `Untitled`. The window
  is 1280 × 860 at first, 85 % of the display at most, at least 560 × 400. It is centred and
  remembered as `main`. It has the OS's frame.
- **The bar.** 40 px tall, along the top, on the panel colour, with a line under it. From left
  to right:
  - the logo at 20 px;
  - the file's name in 13 px, 600 weight;
  - a dot in the accent colour while the text is not saved;
  - a gap;
  - the layout as a segmented control (Raw, Side by side, Preview; 13, "Segmented");
  - Open (`folder-open`) and Save (`save`) as ghost icon buttons with tooltips that name
    their keys.
- **Below the bar.** Under it, while the editor shows, is the toolbar (below). Under that is
  the document in its layout.
- **Opening a file.** The command line's first argument, a path, is the file opened at start.
  That is how a file opens straight from the OS ("Open with", a file's association). Without
  one, the window starts with an untitled empty document. A `.md`, `.markdown` or `.txt` file
  dropped on the window opens it.
- **The app's folders.** These hold its settings and log. `MARKDOWN_HOME=<folder>` puts them in
  that folder instead, for a trial run.

## Layouts

| Layout | What shows | Key |
|---|---|---|
| Raw | the editor alone | Ctrl+1 |
| Side by side | the editor at the left, the preview at the right, with a seam between them that is dragged (the kit's `split`, even at first) | Ctrl+2 |
| Preview | the document rendered, alone | Ctrl+3 |

- A file always opens in **Preview**. Changing the layout keeps the text, cursors, undo and
  where each pane is. The editor's document is kept while it is not shown (`Editor::keep`).
- The preview's text column is at most 760 px wide, centred, with 32 px of room at the sides.
- The editor wraps lines (prose), uses the kit's mono face at 14 px with lines of 22 px, and
  colours the text as Markdown.
- A link opens with Ctrl held, in the preview and in the editor alike. A document may not be
  the user's, so a plain click opens nothing (13, "Markdown").
  - A link to an address opens with the OS's opener, if it is one that opens
    (`url_openable`).
  - A link to `#anchor` scrolls the preview to that heading.
  - A link to another `.md` file, by a path from this one's folder, opens it here. It asks
    first when there is unsaved work.

## Files

- **Keys.**
  - Ctrl+N: a new, untitled document.
  - Ctrl+O: open (the system's dialog, Markdown files first).
  - Ctrl+S: save. An untitled document is saved through Save as.
  - Ctrl+Shift+S: save as.
- **Writing.** A save goes to a file beside the target, which is then renamed over it, so a
  crash never leaves half a file. The text is written as the editor holds it, so CR LF and LF
  stay as they were read. A failure says why in a toast and keeps the work unsaved.
- **The unsaved mark.** It follows the editor's `changed` signal: true while the text differs
  from what was last read or saved.
- **Before work is lost.** New, Open, opening a linked file and closing the window all ask
  first: "Save changes to <name>?" with Save, Don't save and Cancel.

## Side by side

- **Both panes go through headings.**
  - A position in the editor is a fractional line: the line at the top of its view and how
    far down it the view stands (wrapped rows count). This is `Editor::position`.
  - A position in the preview is its scroll offset in pixels.
  - Every heading is an anchor pair: its source line, and its block's top in the preview
    (`Markdown::places`).
  - Two more pairs close the list. The start: line 0 and 0 px. The end: the line count and
    the document's height.
- **The pane being scrolled drives.** The other pane goes to the same point between the two
  anchors around the driver's position. The mapping is linear between them. So a heading at
  the top of one pane is at the top of the other, and the other follows smoothly between
  headings. A follower that cannot scroll that far stops at its end.
- **One pane drives at a time.** The pane that last moved by the user drives. A position that
  only follows is never sent back.
- **Choosing a heading.**
  - When the editor's caret goes to a heading's line, by a key or a press, the preview
    brings that heading to its top.
  - A press on a heading in the preview puts the editor's caret at the start of its line,
    with the line at the top of the editor's view. The focus stays where it was.
  - The heading chosen is tinted with the selection colour for 600 ms in the preview.
- **The preview's text.** It follows the editor's text 120 ms after the last edit. Blocks
  whose source did not change keep their nodes.

## Toolbar

- **Where.** A row of 36 px, under the bar, while the editor shows (Raw, Side by side). It
  holds ghost icon buttons of 28 px with tooltips that name their keys, in groups split by
  rules. A press does not take the focus from the editor: the selection stays, and so does
  the caret.
- **The buttons.**

| Group | Buttons (icon, key) |
|---|---|
| Text | Bold (`bold`, Ctrl+B), Italic (`italic`, Ctrl+I), Strikethrough (`strikethrough`, Ctrl+Shift+X), Inline code (`code`, Ctrl+E) |
| Headings | Heading 1, 2, 3 (`heading-1`, `heading-2`, `heading-3`, Ctrl+Alt+1, 2, 3) |
| Lists | Bulleted list (`list`, Ctrl+Shift+8), Numbered list (`list-ordered`, Ctrl+Shift+7), Task list (`list-todo`), Quote (`text-quote`, Ctrl+Shift+.) |
| Insert | Link (`link`, Ctrl+K), Image (`image`), Code block (`square-code`), Table (`table`), Horizontal rule (`minus`) |

- **What they do.** Each button is one change, so one undo undoes it.
  - **Text styles.** These wrap each selection in their marks: `**`, `*`, `~~` or a
    backtick.
    - A selection already wrapped, inside or just outside, is unwrapped.
    - With no selection, the word at the caret is wrapped. With no word there, an empty pair
      goes in with the caret between.
  - **Headings.** They set the level of every line the selections touch. A line that is
    already at that level becomes a paragraph.
  - **Lists and quotes.** They toggle their prefix on every line touched: `- `, `1. ` (counted
    on), `- [ ] ` or `> `. A line that has another list's prefix takes this one instead.
  - **Link.** It makes `[selection](url)` with `url` selected. An empty selection gives
    `[text](url)` with `text` selected.
  - **Image.** The same as Link, with `![alt](path)`.
  - **Code block.** It fences the lines touched with ` ``` ` above and below, the caret after
    the opening fence. With nothing selected, it is an empty block with the caret inside.
  - **Table.** It inserts a table of 3 columns and 2 rows at the start of the next line, with
    the first header selected.
  - **Rule.** It inserts `---` on a line of its own.
  - **Blank lines.** A block that goes in (a code block, a table, a rule) gets a blank line
    before and after it where its neighbours are not blank.
- These are pure functions from the text and the selections to the edits and the selections
  after them (`format.rs`). They are tested without a window.

## Logo

- **The mark.** A blue square with "M" and a down arrow (the mark of Markdown), drawn as if
  still under construction. It stands on light dashed guide lines and a hatched area. The pen
  is caught in the middle of a stroke: a curve with its node handles still showing. A yellow
  circle stands at a corner, as a shape not yet placed.
- **The file.** It is `apps/markdown/assets/logo.svg`, a 64-unit square drawn in what
  `quadra-vector` renders. It shows in the bar and in the empty window. Its picture at 256 px
  is `apps/markdown/assets/logo.png`.
- **Not yet.** The window's and the executable's icon wait for a window icon in
  `quadra-platform`.

## Tests

The tests drive the controls through the semantic tree:
- the file of the command line opening in Preview, and the three layouts;
- saving through the rename;
- the question before losing work;
- the panes following by headings, both ways, with no echo;
- the heading chosen;
- the toolbar's changes, and their undo.

The formatting functions are unit-tested. Nothing opens a real dialog or writes outside
`.tmp/`.
