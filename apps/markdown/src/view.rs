//! The window (specs/21, "Window" and "Tabs"): the bar, the strip, the active tab's document,
//! and what the window does with keys, drops, later starts and closing.

use std::rc::Rc;
use std::time::Duration;

use quadra::{AnyView, Div, DroppedFiles, Inbox, column, debounce, dynamic, effect, row, show, use_context};
use quadra_design::work::tab_strip;
use quadra_shell::{Launch, Opener};

use crate::files::{self, CloseTab, Files, NewFile, NextTab, OpenFile, PreviousTab, Save, SaveAs};
use crate::model::{Model, PREVIEW};
use crate::search::{Find, FindNext};
use crate::tabs::Tabs;
use crate::{
    ApplyFormat, OpenSettings, ShowLayout, ToggleOutline, bar, outline, panes, settings, status, sync,
    toolbar,
};

/// The wait after the last edit before the preview shows the text.
const PREVIEW_AFTER: Duration = Duration::from_millis(120);

pub(crate) fn window(tabs: &Tabs, launches: Option<Inbox<Launch>>) -> Div {
    let files = Files::new(tabs);
    let window = use_context::<quadra::Window>();
    // The window's title is the active tab's file.
    if let Some(window) = window.clone() {
        let named = tabs.clone();
        effect(move || window.set_title(format!("{} - Markdown", named.current().name())));
    }
    // A later start's files: each a tab, and the window to the front.
    if let Some(launches) = launches {
        let (opens, window) = (tabs.clone(), window.clone());
        launches.on_data(move |launches| {
            for launch in launches.drain(..) {
                for arg in &launch.args {
                    opens.open(&launch.folder.join(arg));
                }
            }
            if let Some(window) = &window {
                window.to_front();
            }
        });
    }
    let (t, c, f, s, a, o, n, p) = (
        tabs.clone(),
        files.clone(),
        files.clone(),
        files.clone(),
        files.clone(),
        files.clone(),
        tabs.clone(),
        tabs.clone(),
    );
    let (closes, drops, formats, settled) = (files.clone(), files.clone(), tabs.clone(), tabs.clone());
    let windows = use_context::<quadra::Windows>();
    let current = tabs.clone();
    let first = tabs.current_untracked();
    let strip = {
        let (tabs, files) = (tabs.clone(), files.clone());
        move || {
            let files = files.clone();
            tab_strip(tabs.strip, tabs.active).on_close(move |id| files.close_tab(id))
        }
    };
    let (many, bars, docs, ends) = (tabs.strip, tabs.clone(), (tabs.clone(), files.clone()), tabs.clone());
    quadra_design::root()
        .on_action::<ShowLayout>(move |&ShowLayout(chosen)| t.current_untracked().layout.set(chosen))
        .on_action::<ToggleOutline>(move |_| first.outline.update(|on| *on = !*on))
        .on_action::<OpenSettings>(move |_| {
            if let Some(windows) = &windows {
                settings::open_window(windows, &settled.current_untracked());
            }
        })
        .on_action::<ApplyFormat>(move |&ApplyFormat(what)| {
            toolbar::apply(&formats.current_untracked(), what)
        })
        .on_action::<Find>({
            let tabs = tabs.clone();
            move |_| {
                let m = tabs.current_untracked();
                match m.layout.get_untracked() {
                    PREVIEW => m.search.show(),
                    _ => m.editor.run(quadra_code::EditorCommand::Find),
                }
            }
        })
        .on_action::<FindNext>({
            let tabs = tabs.clone();
            move |&FindNext(back)| {
                let m = tabs.current_untracked();
                match m.layout.get_untracked() {
                    PREVIEW => m.search.step(back),
                    _ => m.editor.run(quadra_code::EditorCommand::FindNext(back)),
                }
            }
        })
        .on_action::<NewFile>(move |_| f.new_file())
        .on_action::<OpenFile>(move |_| o.open())
        .on_action::<Save>(move |_| s.save(|| {}))
        .on_action::<SaveAs>(move |_| a.save_as(|| {}))
        .on_action::<CloseTab>(move |_| c.close_tab(&current.active.get_untracked()))
        .on_action::<NextTab>(move |_| n.step(1))
        .on_action::<PreviousTab>(move |_| p.step(-1))
        .on_drop(move |dropped: &DroppedFiles, _| {
            for path in dropped.0.iter().filter(|it| files::readable(it)) {
                drops.open_path(path.clone());
            }
        })
        // Closing with unsaved work asks first, tab by tab, and closes once all are answered.
        .on_close(move || {
            let unsaved = closes.unsaved();
            if unsaved && let Some(window) = window.clone() {
                closes.then_all_unsaved(move || window.close_now());
            }
            !unsaved
        })
        .child(
            dynamic(move || AnyView::new(bar::bar(&bars.current(), &files))).container(column().shrink(0.0)),
        )
        .child(show(move || many.with(Vec::len) > 1, strip).container(column().shrink(0.0)))
        .child(
            dynamic(move || AnyView::new(document(&docs.0.current(), &docs.1)))
                .container(column().grow(1.0).min_height(0.0)),
        )
        .child(dynamic(move || AnyView::new(status::bar(&ends.current()))).container(column().shrink(0.0)))
        .child(tabs.current_untracked().notices.host())
}

/// A tab's document: the outline, the toolbar while the editor shows, and the panes.
fn document(m: &Model, files: &Files) -> Div {
    // The preview follows the editor once it stands still.
    let (still, shown) = (debounce(m.edited, PREVIEW_AFTER), m.clone());
    effect(move || {
        still.get();
        shown.refresh();
    });
    let headings = sync::headings(m.text);
    sync::follow(m, headings);
    let layout = m.layout;
    // Preview's find is Preview's alone.
    let open = m.search.open;
    effect(move || {
        if layout.get() != PREVIEW {
            open.set(false);
        }
    });
    let tools = show(move || layout.get() != PREVIEW, {
        let m = m.clone();
        move || toolbar::toolbar(&m)
    });
    let right = column()
        .grow(1.0)
        .min_width(0.0)
        .min_height(0.0)
        .child(tools.container(column().shrink(0.0)))
        .child(panes::body(m, follow(m, files)));
    row().grow(1.0).min_height(0.0).child(outline::outline(m, headings)).child(right)
}

/// What a link does, Ctrl held: a heading of this document is brought into view, an address
/// opens with the OS, another document beside this one opens in a tab.
fn follow(m: &Model, files: &Files) -> panes::Follow {
    let (m, files, opener) = (m.clone(), files.clone(), use_context::<Rc<dyn Opener>>());
    let scope = quadra::Scope::current();
    Rc::new(move |target: &str| {
        if let Some(anchor) = target.strip_prefix('#') {
            let blocks = quadra_md::parse(&m.text.get_untracked());
            let found = quadra_md::outline(&blocks).into_iter().find(|it| it.anchor == anchor);
            if let Some(heading) = found {
                sync::show_heading(&m, heading.line, scope);
            }
        } else if quadra_text::url_openable(target) {
            if let Some(opener) = &opener {
                opener.open(target);
            }
        } else if let Some(folder) = m.folder() {
            let path = folder.join(target);
            if files::readable(&path) {
                files.open_path(path);
            }
        }
    })
}
