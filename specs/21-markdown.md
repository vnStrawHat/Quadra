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
| M6 | The outline: a sidebar of the document's headings |
| M7 | Settings, About, and making Markdown the default for `.md` files |
| M8 | Tabs, and one process: files opened from the OS go to tabs of the window there is |
| M9 | The status bar |
| M10 | Find in the preview |

## Window

- One window, titled `<file name> - Markdown`. An unsaved document is `Untitled`. The window
  is 1280 × 860 at first, 85 % of the display at most, at least 560 × 400. It is centred and
  remembered as `main`. It draws its own frame (10, "Windows with their own frame"): the bar
  below is the kit's title bar (13, `titlebar`, 40 tall), the window is dragged by its empty
  room, a double click maximizes it, and minimize, maximize and close are at its right end.
- **The bar.** 40 px tall, along the top, on the chrome colour, with a line under it. From
  left to right:
  - the logo at 20 px;
  - the file's name in 13 px, 600 weight;
  - a dot in the accent colour while the text is not saved;
  - a gap;
  - the layout as a segmented control (Raw, Side by side, Preview; 13, "Segmented");
  - Outline (`panel-left`), Open (`folder-open`), Save (`save`) and Settings (`settings`) as
    ghost icon buttons with tooltips that name their keys;
  - the window's minimize, maximize and close.
- **Below the bar.** At the left, the outline (below), in every layout. At its right, while
  the editor shows, the toolbar (below), and under that the document in its layout.
- **The status bar** (13, "Status bar"), along the bottom, of the active tab:
  - at its left, the document's words, its characters and its reading time at 200 words a
    minute (`1,204 words · 7,310 characters · 6 min read`), counted on the text the preview
    shows, so on the editor's text once it stands still; a word is a run without spaces
    that holds a letter or a digit, so `#`, `-` and `|` are none; characters leave out line
    ends;
  - at its right, while the editor shows, the primary cursor (`Ln 12, Col 5`, and
    `(34 selected)` while there is a selection); then the document's lines and its line
    ends as read (`LF` or `CRLF`, what a save writes).
- **Opening a file.** The command line's arguments, paths, are the files opened at start,
  each a tab. That is how a file opens straight from the OS ("Open with", a file's
  association; several files chosen in Explorer start the app once each, and all go to the
  first: "Tabs"). Without one, the window starts with an untitled empty document. A `.md`,
  `.markdown` or `.txt` file dropped on the window opens it.
- **A byte order mark** at the start of a file is dropped when it is read, and not written
  back on a save.
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
- **Changing the layout builds nothing it built before.** The editor and the preview are
  the two panes of the kit's split (`.alone`): each is built the first time a layout shows
  it, then kept, hidden while another layout does not show it, so another layout is a
  layout (in the dist build, 920 lines: 20 ms to Side by side, 11 back to Preview). The
  focus in a pane that is hidden goes (04, "Removing"). Opening another document, the
  editor's face and wrapping build both again.
- The preview's text column is at most 900 px wide (the setting Text width), centred, with 32
  px of room at the sides.
- The editor wraps lines (prose), uses the kit's mono face at 14 px with lines of 22 px, and
  colours the text as Markdown.
- A link opens with Ctrl held, in the preview and in the editor alike. A document may not be
  the user's, so a plain click opens nothing (13, "Markdown").
  - A link to an address opens with the OS's opener, if it is one that opens
    (`url_openable`).
  - A link to `#anchor` scrolls the preview to that heading.
  - A link to another `.md` file, by a path from this one's folder, opens it in a tab
    ("Tabs").

## Tabs

- **One process.** Markdown starts by asking whether it is the first in the user's session
  (16, "One instance"; the name is `markdown`, or `markdown-trial` under `MARKDOWN_HOME`, so a
  trial run is apart). A later start (files opened from Explorer, "Open with", the
  command line) hands the first its folder and its files and ends; the first opens each file
  as a tab, a relative path from that folder, and brings its window to the front.
- **A tab is a document**: its file, its text, its editor's document, its layout, where its
  panes are and its outline's row. The settings and the notices are the window's.
- **The strip.** Over the content while there are two tabs or more, from the outline's edge
  to the window's (the outline is the tab's, under the bar): the kit's tab strip (13,
  "Tab strip"), each tab the file's name with the unsaved mark, a cross that closes it, the
  tabs dragged to another order. The bar, the title and the toolbar are the active tab's.
- **Where a file opens.** In the tab that has it already; else in the active tab when it is
  untitled, empty and unchanged; else in a new tab after the active one, which becomes active.
  So opening never loses work and asks nothing. A file dropped on the window, a link to
  another document and Ctrl+O all open so.
- **Keys.** Ctrl+N or Ctrl+T: a new untitled tab. Ctrl+W: close the active tab. Ctrl+Tab and
  Ctrl+Shift+Tab: the next and the previous tab, round.
- **Closing.** A tab with unsaved work asks first ("Save changes to <name>?"). Closing the
  last tab leaves an untitled one. Closing the window asks for each tab with unsaved work in
  turn; Cancel on any keeps the window open.
- **Its cost.** Switching tabs builds the active document's panes again; the other tabs hold their text and their editor's document, not their views.

## Find

- **Ctrl+F** finds in the document; F3 and Shift+F3, the next and the one before.
  - While the editor shows (Raw, Side by side), it is the editor's find bar, with replace
    (Ctrl+H; behaviour/18, "Find"): Ctrl+F gives the editor the focus where it is not.
  - In Preview, a bar at the top right of the preview (specs/design/controls.md, "A find
    bar"; as OneTerm's, 19 "Find"): the query with the search icon, Match case and Whole
    word, `3 of 12` (`No results`), the one before and the next, and a cross. Enter is the
    next, Shift+Enter the one before; Escape closes it. The matches are marked in the
    preview (13, "Markdown", Find), the current one brought into view. Each tab has its own
    query.
- **Its cost.** A find marks the texts of the document: proportional to its text, run on a
  change of the query and after a layout that moved its blocks.

## Files

- **Keys.**
  - Ctrl+N: a new, untitled document (a tab, above).
  - Ctrl+O: open (the system's dialog, Markdown files first), in a tab (above).
  - Ctrl+S: save. An untitled document is saved through Save as.
  - Ctrl+Shift+S: save as.
- **Writing.** A save goes to a file beside the target, which is then renamed over it, so a
  crash never leaves half a file. The text is written as the editor holds it, so CR LF and LF
  stay as they were read. A failure says why in a toast and keeps the work unsaved.
- **The unsaved mark.** It follows the editor's `changed` signal: true while the text differs
  from what was last read or saved.
- **Before work is lost.** Closing a tab or the window asks first: "Save changes to
  <name>?" with Save, Don't save and Cancel. Opening goes to a tab (above), so it asks
  nothing.

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
  - The heading chosen is tinted with the selection colour for 600 ms in the preview; after
    that the preview is free again, and laying out its blocks anew does not bring the heading
    back.
- **A layout changed** keeps where each pane is: an editor built again goes to the position
  kept, once it is laid out at its width.
- **The preview's text.** It follows the editor's text 120 ms after the last edit. Blocks
  whose source did not change keep their nodes.

## Toolbar

- **Where.** The kit's `toolbar()` (45 px on the canvas, a line under it; groups split by
  `vdivider()`), under the bar, while the editor shows (Raw, Side by side). It
  holds ghost icon buttons of 28 px with tooltips that name their keys, in groups split by
  rules. After a press the focus is back in the editor, its selection as the button left
  it. The buttons work on the primary cursor's selection.
- **The buttons.**

| Group | Buttons (icon, key) |
|---|---|
| Text | Bold (`bold`, Ctrl+B), Italic (`italic`, Ctrl+I), Strikethrough (`strikethrough`, Ctrl+Shift+X), Inline code (`code`, Ctrl+E) |
| Headings | Heading 1, 2, 3 (`heading-1`, `heading-2`, `heading-3`, Ctrl+Alt+1, 2, 3) |
| Lists | Bulleted list (`list`, Ctrl+Shift+8), Numbered list (`list-ordered`, Ctrl+Shift+7), Task list (`list-todo`), Quote (`text-quote`, Ctrl+Shift+.) |
| Insert | Link (`link`, Ctrl+K), Image (`image`), Code block (`square-code`), Table (`table`), Horizontal rule (`minus`) |

- **What they do.** Each button is one change, so one undo undoes it.
  - **Text styles.** These wrap the selection in their marks: `**`, `*`, `~~` or a
    backtick.
    - A selection already wrapped, inside or just outside, is unwrapped.
    - With no selection, the word at the caret is wrapped. With no word there, an empty pair
      goes in with the caret between.
  - **Headings.** They set the level of every line the selection touches. A line that is
    already at that level becomes a paragraph.
  - **Lists and quotes.** They toggle their prefix on every line touched: `- `, `1. ` (counted
    on), `- [ ] ` or `> `, after the line's indent. When every line has it already, it comes
    off. A line that has another list's prefix takes this one instead.
  - **Link.** It makes `[selection](url)` with `url` selected. An empty selection gives
    `[text](url)` with `text` selected.
  - **Image.** The same as Link, with `![alt](path)`.
  - **Code block.** It fences the lines touched with ` ``` ` above and below, the caret after
    the opening fence. With nothing selected, it is an empty block, the caret after its opening fence.
  - **Table.** It inserts a table of 3 columns and 2 rows at the start of the next line, with
    the first header selected.
  - **Rule.** It inserts `---` on a line of its own.
  - **Blank lines.** A block that goes in (a code block, a table, a rule) gets a blank line
    before and after it where its neighbours are not blank.
- These are pure functions from the text and the selections to the edits and the selections
  after them (`format.rs`). They are tested without a window.

## Outline

- **Where.** A sidebar 240 px wide at the left, under the bar, on the panel colour with a
  line at its right, in every layout. The bar's Outline button and Ctrl+Shift+O show and
  hide it; it shows at first.
- **What.** The kit's tree (13, "Tree") of the document's headings: a heading holds those of
  deeper levels after it, up to the next of its level or higher. Every heading that holds
  others is open when the headings change, and can be closed. A heading's text is one line,
  cut with "…" where the row ends; its whole text is the row's name for a reader. A document
  without headings shows "No headings" in the faint text colour.
- **Following.** As the panes scroll, the row selected is that of the section at the top:
  the last heading at or above the line at the top of the editor (Raw) or of the preview.
- **Choosing.** A row chosen by the pointer or the keys goes to its heading: the editor's
  caret to the start of its line and the line to the editor's top, the heading to the
  preview's top, tinted for a moment; in each pane that shows. The focus stays in the
  outline. The row chosen stays selected until the user moves a pane: a heading in the last
  screenful does not reach the top, as the panes stop at their ends.
- **Its cost.** The headings are read once for each text the preview shows, shared with the
  panes' following, and so are the pairs that map one pane to the other: the panes scrolling
  costs a search of them. The tree is asked again only when the headings change.

## Settings

Ctrl+, or the bar's Settings button opens the settings in a window of their own (980 × 680,
remembered as `settings`), or brings it to the front; it closes with the main window. It draws
its own frame too: the kit's title bar of 34 px with the caption "Markdown Settings" at 14 px
(`title_size`). Under it, the kit's settings view (13, "Settings"), its pages in a column with a search. Each control
writes its setting to `settings.json` in the app's config folder at once, and the main
window follows. Each row goes back to its default.

What is a setting is what has a value someone may want otherwise, in place of a constant:

| Page | Setting | Choices | Default |
|---|---|---|---|
| Appearance | Theme | System, Dark, Light | System |
| Editor | Text size | 12 to 20 px; the line is 11/7 of it, whole pixels | 14 |
| Editor | Wrap lines | on or off | on |
| Preview | Text width | Narrow (720 px), Normal (900), Wide (1200), Full | Normal |
| Layouts | A file opens in | Preview, Side by side, Raw | Preview |
| Layouts | Scroll together | side by side, the panes follow each other | on |

Kept without being settings, as the window's state: whether the outline shows and the
share of the width side by side (the same file, `window.outline` and `window.split`).

Not settings: the code face's ligatures are off in the whole app, so the source reads as it
is typed (`##`, `|---|`, `->`); the toolbar, the keys and the outline's width are the app's.

## About

The last page of the settings: the logo at 64 px, the name, the version, what it is, the
executable's path, and **Default app**:
- a line that says whether Markdown opens `.md` files now (`Associations::opener_of`
  against the executable);
- **Set as default…**: registers Markdown for `.md` and `.markdown` under the user (16,
  "File types"), then opens the system's Default apps page at Markdown, where the user
  chooses it, as Windows lets no app choose for itself. A note under it says so.
The line is read again when the window comes back to the front.

## Logo

- **The mark.** A blue square with "M" and a down arrow (the mark of Markdown), drawn as if
  still under construction. It stands on light dashed guide lines and a hatched area. The pen
  is caught in the middle of a stroke: a curve with its node handles still showing. A yellow
  circle stands at a corner, as a shape not yet placed.
- **The file.** It is `apps/markdown/assets/logo.svg` (made by `logo.py` beside it), a 64-unit square drawn in what
  `quadra-vector` renders: it has no dashes, opacity or masks, so the dashes and the hatching
  are short lines. The drawing view shows it in its own colours (13, "Icons"). It shows at
  20 px in the bar, and at 96 px in an empty window with how to start. The `logo` scene of
  `bench/markdown-shots` draws it at 256 px on both canvases.
- **The icon.** `assets/logo.ico` holds the logo at 16, 20, 24, 32, 40, 48, 64 and 256 px as
  PNG images. Under 40 px it is `logo-small.svg`, the mark alone (the square with M↓ and the
  circle, without the guides, the hatching and the nodes), as those are too fine to read
  there. The `icon` scene of `bench/markdown-shots` makes the file and a picture of every
  size. `build.rs` links it as the executable's icon (16, "Packaging"), which the window
  takes (10, "Win32 behaviour").

## Tests

The tests drive the controls through the semantic tree:
- the file of the command line opening in Preview, and the three layouts;
- saving through the rename;
- the question before losing work, closing a tab or the window;
- the tabs: where a file opens, closing, the keys, and the files a later start hands over;
- the status bar's counts and cursor;
- find: Ctrl+F in each layout, the count and the next in Preview, a word in code and in a table found, the keys in the field;
- the panes following by headings, both ways, with no echo;
- the heading chosen;
- the toolbar's changes, and their undo;
- the outline: nesting, a long heading cut, the row following the panes, a row chosen;
- the settings followed by the window, and the default app through the fake shell (nothing
  writes the registry).

The formatting functions are unit-tested. Nothing opens a real dialog or writes outside
`.tmp/`.
