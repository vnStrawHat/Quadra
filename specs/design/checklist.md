# Checklist

For whoever builds a view, person or agent: go down this list before calling it done. The
first part is what the gate checks; the second is what only a look can tell.

## What `tools/check.sh` checks

`tools/check_design.py` reads the kit (`crates/quadra-design`), the gallery and the apps and
fails on:

- a number in what `.gap(..)`, `.gap_x(..)`, `.gap_y(..)`, `.padding(..)` and its sides are
  given (written there, chosen between two, or kept in a `const` of the file and given by
  its name) that is not 0, 1 or a step of the distance ladders (2, 4, 6, 8, 12, 16, 24, 32);
- a literal in `.radius(..)` that is not 0 or a step of the radius ladder (2, 4, 6, 10);
- a literal in `.text_size(..)` or `text(..).size(..)` that is not a step of the type scale
  (10, 12, 14, 16, 18, 23, 28).

A number that must be off a ladder (a shape drawn to a measure, a width that is content)
carries its reason on the same line: `// design: <why>`. The check counts them; a view that
needs more than one or two is not following the language.

## Before writing

1. Find the pattern in patterns.md that is nearest and start from its code.
2. Name the container (dialog, popover, card, page, panel). Its inset and gap are the
   kit's; do not set them.
3. List the things in the body and what each is to its neighbour: parts of one thing (2 to
   8), siblings (12), groups (16), sections (24).

## While writing

4. No bare `column()` or `row()` with more than one child and no gap. Use `vstack()`,
   `form()`, `hstack()`, `button_row()`, `columns(n)`, `section(..)`.
5. No width on a field inside a `form()`. No height on a control. No size on a button
   unless the whole row has it.
6. Every text is a row of the table in type.md: no size or colour picked alone.
7. Every colour is a token. Every icon is 14 (16 in navigation and titles).
8. Every control has a name a reader hears: a `field(label, ..)`, a `.label(..)`, or its own
   words. An icon button has a tooltip.
9. Text that can be long has `min_width(0)` and `.ellipsis()` or wraps; what stands beside
   it has `shrink(0)`.

## When it is built: look at it

Draw it (`tools/shots.sh`, or the app's own shots) in both themes and check:

10. **Edges**: one left edge down the container. Nothing touches a border. The inset is the
    same on all four sides.
11. **Gaps**: three sizes of gap at most in the view, and they mean within, between,
    sections.
12. **Rows**: controls in a row have one height and one centre line; text beside them is
    on their baseline.
13. **Weight**: one title, one primary button, one thing in the accent (but for chosen
    siblings). Squint: what stands out is what matters.
14. **Lines**: a hairline only where two different things meet. No box in a box.
15. **Squeeze it**: at half the width nothing overlaps or leaves its container; long names
    end with `…`.
16. **Empty, busy, wrong**: the view with nothing in it, while it loads, and after a
    failure each say so in place.
17. **Keys**: Tab goes through it in reading order; Enter does the main action; Escape
    leaves.
18. **Light theme**: the same view, with the same care.

## Review questions

- Could this view be built from a pattern with fewer parts of its own?
- Is a number followed by what it counts (`.unit("s")`)?
- Is every distance in it gone? (A view of helpers and tokens writes none; a container's
  width is the one number it may give.)
- If a control here were moved to another app of the kit, would it look at home?
