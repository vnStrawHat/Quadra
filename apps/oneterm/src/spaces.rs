//! The workspace (specs/19, "Spaces"): the spaces of the tab that shows, side by side in
//! equal columns with lines between (beside the rail, from three on, the first on the left
//! and the others stacked at its right), each with its header of 18 px; the focused one has
//! an accent ring. A space whose program ended says so, and a key then closes it.

use std::time::Instant;

use quadra::{
    Align, AnyView, Display, MountCx, NodeId, Signal, View, by_text, column, div, dynamic, each, effect, row,
    text,
};
use quadra_design::{fonts, token};
use quadra_term::{Link, terminal};

use crate::model::{Model, Space, Tab};

pub(crate) fn workspace(model: &Model) -> impl View + use<> {
    let shown = model.clone();
    let font = model.prefs().font_size;
    // A new size of text mounts the terminals again.
    let tab = dynamic(move || match (font.get(), shown.active_tab()) {
        (size, Some(tab)) => AnyView::new(columns(&shown, tab, size.clamp(6.0, 48.0) as f32)),
        (_, None) => AnyView::new(div()),
    });
    // The expanded SFTP view takes the workspace's place; the spaces run on, not displayed.
    let expanded = model.ui().expanded;
    tab.container(row().grow(1.0).bg(token::CANVAS).display(crate::locked::shown(move || !expanded.get())))
}

fn columns(model: &Model, tab: Tab, font: f32) -> impl View + use<> {
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
                    font,
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
    font: f32,
    /// In a row (equal widths), or in a stack (equal heights).
    across: bool,
}

impl View for SpaceView {
    /// The header, the terminal and the ring; the terminal takes the focus whenever the
    /// space is the tab's focused one.
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let SpaceView { model, tab, space, font, across } = self;
        let shows = focused(tab, space.id);
        let view = space_view(&model, tab, space, font);
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

fn space_view(model: &Model, tab: Tab, space: Space, font: f32) -> quadra::Div {
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
    let ring = div()
        .absolute()
        .left(0.0)
        .top(0.0)
        .right(0.0)
        .bottom(0.0)
        .border(1.0)
        .border_color(token::ACCENT)
        .display(move || if on() && tab.spaces.with(Vec::len) > 1 { Display::Flex } else { Display::None });
    column()
        .grow(1.0)
        .min_width(0.0)
        .min_height(0.0)
        .bg(token::CANVAS)
        .on_focus(move |gained| {
            if gained {
                tab.focus.set(index());
            }
        })
        .child(header)
        .child(
            column()
                .grow(1.0)
                .min_height(0.0)
                .padding_x(12.0)
                .padding_y(10.0)
                .child(terminal(&s).font(font, (font * 1.54).round())),
        )
        .child(ring)
}
