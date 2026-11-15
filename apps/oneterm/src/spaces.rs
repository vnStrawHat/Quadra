//! The workspace (specs/19, "Spaces"): the spaces of the tab that shows, side by side in
//! equal columns with lines between (beside the rail, from three on, the first on the left
//! and the others stacked at its right), each with its header of 18 px; the focused one has
//! an accent ring. A space whose program ended says so, and a key then closes it.

use std::time::Instant;

use std::rc::Rc;

use quadra::{
    Align, AnyView, Clipboard, MountCx, NodeId, Signal, View, by_text, column, div, dynamic, each, effect,
    row, signal, text, untrack, use_context,
};
use quadra_design::{MenuItem, context_menu, fonts, icons, token};
use quadra_term::{Link, Session, terminal};

use crate::model::{Model, Space, Tab};

pub(crate) fn workspace(model: &Model) -> impl View + use<> {
    let shown = model.clone();
    let tab = dynamic(move || match shown.active_tab() {
        Some(tab) => AnyView::new(columns(&shown, tab)),
        None => AnyView::new(div()),
    });
    // The expanded SFTP view takes the workspace's place; the spaces run on, not displayed.
    let expanded = model.ui().expanded;
    tab.container(row().grow(1.0).bg(token::CANVAS).display(crate::locked::shown(move || !expanded.get())))
}

fn columns(model: &Model, tab: Tab) -> impl View + use<> {
    let (model, vertical) = (model.clone(), model.vertical());
    let stacked = quadra::memo(move || vertical.get() && tab.spaces.with(Vec::len) >= 3);
    dynamic(move || {
        let list = |part: fn(&[Space]) -> Vec<Space>, across: bool| {
            let model = model.clone();
            each(
                move || tab.spaces.with(|spaces| part(spaces)),
                |space| space.id,
                move |space: Signal<Space>| SpaceView {
                    model: model.clone(),
                    tab,
                    space: space.get_untracked(),
                    across,
                },
            )
        };
        if !stacked.get() {
            return AnyView::new(
                list(<[Space]>::to_vec, true).container(row().grow(1.0).gap(1.0).bg(token::LINE)),
            );
        }
        let first = list(|spaces| spaces.iter().take(1).cloned().collect(), true)
            .container(row().grow(1.0).width(0.0));
        let rest = list(|spaces| spaces.iter().skip(1).cloned().collect(), false)
            .container(column().grow(1.0).width(0.0).gap(1.0));
        AnyView::new(row().grow(1.0).gap(1.0).bg(token::LINE).child(first).child(rest))
    })
    .container(row().grow(1.0).min_width(0.0))
}

/// Whether the space is its tab's focused one.
fn focused(tab: Tab, id: u64) -> impl Fn() -> bool + Copy {
    move || tab.spaces.with(|spaces| spaces.iter().position(|it| it.id == id)) == Some(tab.focus.get())
}

struct SpaceView {
    model: Model,
    tab: Tab,
    space: Space,
    /// In a row (equal widths), or in a stack (equal heights).
    across: bool,
}

impl View for SpaceView {
    /// The header, the terminal and the ring; the terminal takes the focus whenever the
    /// space is the tab's focused one.
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let SpaceView { model, tab, space, across } = self;
        let shows = focused(tab, space.id);
        let view = space_view(&model, tab, space);
        let id = if across { view.width(0.0) } else { view.height(0.0) }.mount(cx);
        let (handle, locked) = (cx.handle.clone(), model.lock().locked);
        effect(move || {
            // Opened again, the focus comes back to the focused space.
            if shows() && !locked.get() {
                handle.with(move |tree| _ = tree.focus_first(id));
            }
        });
        id
    }
}

fn space_view(model: &Model, tab: Tab, space: Space) -> quadra::Div {
    let s = space.session.clone();
    let index = move || tab.spaces.with(|spaces| spaces.iter().position(|it| it.id == space.id).unwrap_or(0));
    let on = focused(tab, space.id);
    let (title, cwd, exited) = (s.title(), s.cwd(), s.exited());
    // Once the program ended it says so, and the next key closes the space.
    let (said, closes, gone) = (s.clone(), model.clone(), space.id);
    effect(move || {
        if let Some(code) = exited.get() {
            let words = format!("\r\n[process exited {code}]\r\n");
            said.feeder().feed(words.as_bytes(), Instant::now());
            let closes = closes.clone();
            said.connect(Link::new(move |_| _ = closes.close_space(gone)));
        }
    });
    // Where it is: the directory, after the user and host of a remote space; a shell that
    // says no directory, its title.
    let (named, remote) = (model.clone(), space.remote.clone());
    let place = move || {
        let dir = cwd.get();
        let dir = if dir.is_empty() { title.get() } else { dir };
        match &remote {
            Some(remote) if cwd.with(String::is_empty) => {
                format!("{}@{} {dir}", remote.target.user, named.host_name(remote))
            }
            Some(remote) => format!("{}@{}:{dir}", remote.target.user, named.host_name(remote)),
            None => dir,
        }
    };
    let header = row()
        .align_items(Align::Center)
        .gap(8.0)
        .padding_x(8.0)
        .height(by_text(18.0))
        .shrink(0.0)
        .bg(token::PANEL)
        .font_family(fonts::MONO)
        .text_size(11.0)
        .text_color(token::TEXT_2)
        .child(
            div()
                .grow(1.0)
                .cut_to_parent()
                .text_color(move || if on() { token::TEXT } else { token::TEXT_2 })
                .child(text(place).ellipsis()),
        )
        .child(crate::marks::header_signal(&s, model.prefs(), title))
        .child(text(move || exited.get().map(|code| format!("exited {code}")).unwrap_or_default()));
    // The focused space's ring is its border, so nothing lies over the terminal for the pointer.
    let ring = move || if on() && tab.spaces.with(Vec::len) > 1 { token::ACCENT } else { token::CANVAS };
    column()
        .grow(1.0)
        .min_width(0.0)
        .min_height(0.0)
        .bg(token::CANVAS)
        .border(1.0)
        .border_color(ring)
        .on_focus(move |gained| {
            if gained {
                tab.focus.set(index());
            }
        })
        .child(header)
        .child(column().grow(1.0).min_height(0.0).padding_x(12.0).padding_y(10.0).child(menu_of(
            model,
            &s,
            space.id,
            terminal(&s),
        )))
}

/// The space's menu for the other button (specs/19, "Spaces"), when the settings have it; the
/// terminal follows the settings of the pointer and the wheel.
fn menu_of(model: &Model, s: &Session, id: u64, view: quadra_term::Terminal) -> impl View + use<> {
    let p = model.prefs();
    let (settings, look) = (signal(untrack(|| p.term())), signal(untrack(|| p.look())));
    effect(move || settings.set(p.term()));
    // A new look is followed by the terminal as it is: its screen stays.
    effect(move || look.set(p.look()));
    let (s, model, clipboard) = (s.clone(), model.clone(), use_context::<Rc<dyn Clipboard>>());
    context_menu(view.settings(settings).look(look), move || {
        if !p.context_menu.get_untracked() {
            return Vec::new();
        }
        let chosen = s.selection_text();
        let (copies, pastes, (all, clears)) = (clipboard.clone(), clipboard.clone(), (s.clone(), s.clone()));
        let (pasted, (splits, closes)) = (s.clone(), (model.clone(), model.clone()));
        vec![
            MenuItem::new("Copy")
                .icon(icons::COPY)
                .keys("Ctrl+Shift+C")
                .disabled(chosen.is_none())
                .on_select(move || {
                    if let (Some(clipboard), Some(text)) = (&copies, &chosen) {
                        clipboard.write_text(text);
                    }
                }),
            MenuItem::new("Paste").keys("Ctrl+Shift+V").on_select(move || {
                if let Some(clipboard) = &pastes {
                    pasted.paste(&clipboard.read_text().unwrap_or_default());
                }
            }),
            MenuItem::new("Select all").on_select(move || all.select_all()),
            MenuItem::new("Clear scrollback").on_select(move || clears.clear_history()),
            MenuItem::separator(),
            MenuItem::new("Split space")
                .icon(icons::COLUMNS_2)
                .keys("Ctrl+D")
                .on_select(move || splits.split()),
            MenuItem::new("Close space").keys("Ctrl+W").danger().on_select(move || {
                _ = closes.close_space(id);
            }),
        ]
    })
}
