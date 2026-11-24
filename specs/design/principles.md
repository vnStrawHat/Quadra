# Principles

Quadra's kit is for tools people keep open all day and drive from the keyboard: terminals,
editors, cluster managers, log viewers. The work is the content. The interface is the frame
around it: quiet, exact, the same everywhere, and out of the way.

## Quadra's four

Four things make a Quadra app look like one. Each is cheap to follow and easy to check.

### 1. Hairlines, not boxes

A window is a few flat planes, each one small step of tone from the next, split by 1 px
seams. The content plane (`canvas`) is the deepest; panels are one step off it, the window's
frame one more.

- A region is set apart by its plane or by a hairline, never by both a fill and a border and
  a shadow.
- Only what floats over the window casts a shadow (menus, dialogs, notifications).
- No card inside a card. A group inside a card is a hairline or a heading.
- No gradients, no glows. A focus is a border that changes colour, not a halo.
- Fewer lines is better: rows of a list have no lines between them; a line stands only
  where two different things meet (a header and its body, a body and its actions).

### 2. Squares for marks

The small signs that say a state are squares and straight bars: a status is a square (filled
running, hollow pending), a tag's colour is a square, a bullet is a square, a tick box is a
square, what is chosen has a straight 2 px bar on its edge, progress is a straight bar.

- Circles mean three things only: it turns (a spinner), it is one of many (a radio), it is a
  person (an avatar).
- A mark is never the only carrier of a fact: a shape, a letter or a word goes with a colour
  (a status keeps a shape of its own; "modified" is yellow and an `M`).
- Marks take no room when they come and go: a row keeps its text where it was.

### 3. Mono for what the machine said

Two faces, two voices. The UI face is the app talking and the user naming things. The mono
face is the machine's own words: a path, a number with a unit, an address, a hash, a key, a
time, an id, a count, a percentage.

- `~/apps/api`, `124 ms`, `10.20.1.14`, `Ctrl-Shift-P`, `64%`, `3f1c2ab`: mono.
- "Production", "Add SSH host", "Font size": the UI face.
- Mono text takes the size of the text around it; it is not smaller, not boxed, not tinted.

### 4. One accent, used as a mark

One colour says "this one": what is chosen, what has the focus, what is on. It appears as a
2 px bar, a 1 px border, the colour of a label or an icon, and as small fills (a tick box, a
switch, a progress bar). Its tint (16 %) fills the primary button and the chosen segment.

- Never a large accent fill. Never the accent for decoration or for a heading.
- One primary button a view. Two things in the accent side by side are siblings of which one
  is chosen.
- Hover is not the accent: hover is a neutral step (`panel_hover`), press two.

## What follows from them

5. **Everything is on a ladder.** Distances, heights, radii, text and icon sizes each have
   four steps (README). A view has no number of its own: it takes a step, by name where the
   kit has one.
6. **One look for one job.** Everything you type in or choose from is a *field* (the same
   fill, border, height and corners: input, select, combobox, number, date, key). Everything
   you press is a *button*. Everything that lists is *rows*. A new control joins a family; it
   does not get a look of its own.
7. **Default is right.** A control with no size is 28 tall; text with no size is 14; a
   container with no padding is wrong, so the kit's containers come with theirs. The shortest
   code gives the designed result.
8. **Dense, not cramped.** Small type with open leading (14 on 23), controls 28 tall, rows 24
   to 28, but never two things touching: the least distance between two things is 4, and
   between two controls 8.
9. **Instant states, working motion.** Hover and press change at once. Only what appears and
   leaves animates (150 ms, ease out), and what is busy turns or runs. Reduced motion turns
   that off.
10. **Keys first.** Every action has a key or is reachable by Tab and arrows; shortcuts are
    written as plain muted mono text after a label (`Ctrl-Shift-P`), not as key caps.
11. **Words are short and plain.** Sentence case everywhere ("Add SSH host", not "Add SSH
    Host"). A button says what it does ("Delete host", not "OK"). An ellipsis ends a label
    that opens something that asks more ("Browse…"). No full stop after a label; one after a
    sentence of help.
12. **Colour blind and screen-reader safe by construction.** State is never colour alone;
    every control has a name a reader hears (`.label(..)`); contrast is 4.5:1 for text.
