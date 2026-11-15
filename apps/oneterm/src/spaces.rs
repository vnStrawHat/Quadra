//! The workspace (specs/19, "Spaces"): the spaces of the tab that shows, side by side in
//! equal columns with lines between, each with its header of 18 px; the focused one has an
//! accent ring. A space whose program ended says so, and a key then closes it.

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
    let tab = dynamic(move || match shown.active_tab() {
        Some(tab) => AnyView::new(columns(&shown, tab)),
        None => AnyView::new(div()),
    });
    tab.container(row().grow(1.0).bg(token::CANVAS))
}

fn columns(model: &Model, tab: Tab) -> impl View + use<> {
    let model = model.clone();
    let spaces = each(
        move || tab.spaces.get(),
        |space| space.id,
        move |space: Signal<Space>| SpaceView { model: model.clone(), tab, space: space.get_untracked() },
    );
    spaces.container(row().grow(1.0).gap(1.0).bg(token::LINE))
}

/// Whether the space is its tab's focused one.
fn focused(tab: Tab, id: u64) -> impl Fn() -> bool + Copy {
    move || tab.spaces.with(|spaces| spaces.iter().position(|it| it.id == id)) == Some(tab.focus.get())
}

struct SpaceView {
    model: Model,
    tab: Tab,
    space: Space,
}

impl View for SpaceView {
    /// The header, the terminal and the ring; the terminal takes the focus whenever the
    /// space is the tab's focused one.
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let SpaceView { model, tab, space } = self;
        let shows = focused(tab, space.id);
        let id = space_view(&model, tab, space).mount(cx);
        let handle = cx.handle.clone();
        effect(move || {
            if shows() {
                handle.with(move |tree| _ = tree.focus_first(id));
            }
        });
        id
    }
}

fn space_view(model: &Model, tab: Tab, space: Space) -> impl View + use<> {
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
    let header = row()
        .align_items(Align::Center)
        .padding_x(8.0)
        .height(by_text(18.0))
        .shrink(0.0)
        .bg(token::PANEL)
        .font_family(fonts::MONO)
        .text_size(11.0)
        .text_color(move || if on() { token::TEXT } else { token::TEXT_2 })
        .child(
            div().grow(1.0).cut_to_parent().child(
                text(move || {
                    let title = title.get();
                    if title.is_empty() { cwd.get() } else { title }
                })
                .ellipsis(),
            ),
        )
        .child(crate::marks::header_signal(s.status(), s.progress()))
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
        .width(0.0)
        .bg(token::CANVAS)
        .on_focus(move |gained| {
            if gained {
                tab.focus.set(index());
            }
        })
        .child(header)
        .child(terminal(&s))
        .child(ring)
}
