# Getting started

From nothing to a window with a form in it. Quadra builds on Windows 10 and 11 with the MSVC toolchain.

## What you need

- Git Bash with coreutils (Git for Windows has both). Every command here is run in it, from the project root.
- Python 3, for the repository's tools.
- rustup. The toolchain is pinned in `rust-toolchain.toml` and installs itself.
- The Visual Studio Build Tools: MSVC and the Windows SDK.

```sh
git clone https://github.com/vnStrawHat/Quadra
cd Quadra
tools/setup.sh        # says what is missing; installs nothing
```

## Look at what there is

The gallery is every component of the kit on a page of its own. Run it before writing anything:

```sh
tools/cargo.sh run -p quadra-gallery
```

`tools/cargo.sh` is cargo with the project's settings; use it for every cargo command. Ctrl-K in the gallery opens the command palette; the button in the title bar changes the theme.

## A first app

{{code examples/gallery/examples/guide.rs#hello}}

Line by line:

- `quadra::app()?` is the application: the platform, the renderer, the frame loop. `quadra_design::install_system(..)` gives it the kit's fonts, key bindings and the theme the system asks for (dark, light or high contrast), and follows the system when it changes.
- `app.window(options, view)` opens a window on what `view` builds. **`view` runs once.** It does not run again when something changes.
- `signal(value)` is a value that can change. A closure that reads it (`move || format!(.., name.get())`) is bound to the property it was given to: when the signal is written, that property updates, and nothing else.
- `root()` is the window's ground: the canvas colour, the text colour, the UI face.
- `card()`, `form()`, `field(..)`, `button_row()` lay the content out. Nothing in this view writes a gap or a width: the containers and the helpers have them ([Building a view](views.md)).

Run it:

```sh
tools/cargo.sh run -p quadra-gallery --example guide
```

## In your own crate

Until Quadra is published, depend on it by path or by git:

```toml
[dependencies]
quadra = { git = "https://github.com/vnStrawHat/Quadra" }
quadra-design = { git = "https://github.com/vnStrawHat/Quadra" }
```

`quadra` is the framework's facade (`quadra::app()`, views, signals, layout values). `quadra-design` is the kit. The charts, the editor, markdown, the data views and the terminal are crates beside them (`quadra-viz`, `quadra-code`, `quadra-md`, `quadra-data`, `quadra-term`).

## Check your work

```sh
tools/check.sh        # format, lints, tests, the repository's rules
tools/shots.sh dark   # every gallery page drawn to .tmp/shots/dark, without a window
```

A view can be tested without a window or a GPU ([Building a view](views.md), "Test it").

## Next

- [Concepts](concepts.md): signals, views, layout, themes, and why a frame costs what changed.
- [Building a view](views.md): the helpers, the containers, forms, lists, overlays, tests.
- [Components](../components/index.md): what each looks like and how it is written.
