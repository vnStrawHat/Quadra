//! The rail (specs/19, "Rail"): 248 px of tabs on the panel colour, a heading with the
//! count, a row a tab, and "New terminal". A click shows a tab, the middle button closes it,
//! two clicks rename it, the right button opens its menu, and rows drag to reorder.

use quadra::{
    Align, AnyView, Display, MouseButton, NodeRef, PointerKind, Role, Signal, View, by_text, column, div,
    dynamic, each, row, signal, text,
};
use quadra_design::{MenuItem, context_menu, fonts, icon, icon_button, icons, input, searched_menu, token};

use crate::marks;
use crate::model::{Model, Tab};
use crate::shells;

/// What a tab's row carries while it is dragged.
pub(crate) struct Dragged(pub u64);

pub(crate) fn rail(model: &Model) -> impl View + use<> {
    let (tabs, vertical) = (model.tabs(), model.vertical());
    let heading = row()
        .align_items(Align::Center)
        .gap(6.0)
        .padding_left(14.0)
        .padding_right(6.0)
        .height(by_text(36.0))
        .text_size(11.0)
        .text_color(token::TEXT_3)
        .child(div().child(text("TABS")))
        .child(div().grow(1.0).font_family(fonts::MONO).child(text(move || tabs.with(Vec::len).to_string())))
        .child(icon_button(icons::LAYOUT_GRID, "Horizontal tabs").on_click(move || vertical.set(false)));
    let shown = model.clone();
    let rows =
        each(move || tabs.get(), |tab| tab.id, move |tab: Signal<Tab>| tab_row(&shown, tab.get_untracked()));
    let (open, opener, making) = (signal(false), NodeRef::new(), model.clone());
    let new = row()
        .align_items(Align::Center)
        .gap(10.0)
        .padding_x(8.0)
        .height(by_text(32.0))
        .radius(6.0)
        .text_color(token::TEXT_2)
        .hover(|s| s.bg(token::PANEL_HOVER).text_color(token::TEXT))
        .role(Role::Button)
        .label("New terminal")
        .on_click(move |_| open.update(|open| *open = !*open))
        .child(icon(icons::PLUS).size(14.0))
        .child(div().grow(1.0).child(text("New terminal")))
        .child(
            div().font_family(fonts::MONO).text_size(11.0).text_color(token::TEXT_3).child(text("Ctrl+T")),
        );
    column()
        .width(248.0)
        .shrink(0.0)
        .bg(token::PANEL)
        .border_right(1.0)
        .border_color(token::LINE)
        .child(heading)
        .child(rows.container(column().gap(2.0).padding_x(8.0).role(Role::TabList).label("Tabs")))
        .child(div().padding_x(8.0).padding_y(4.0).node_ref(&opener).child(new))
        .child(searched_menu(open, &opener, ASK, move |typed: &str| new_menu(&making, typed)))
        .child(crate::hostlist::hosts(model))
}

/// What the `+` menu's field says while empty.
pub(crate) const ASK: &str = "Search hosts, or type user@host…";

/// The `+` menu: the default shell, each shell found, a split of the space with the focus,
/// the hosts; with `typed`, what holds it, after a row that connects to it when it is a target.
pub(crate) fn new_menu(model: &Model, typed: &str) -> Vec<MenuItem> {
    let words = typed.trim().to_lowercase();
    let holds = |what: &str| words.is_empty() || what.to_lowercase().contains(&words);
    let mut items = Vec::new();
    let connect = oneterm_ssh::Target::parse(typed.trim()).filter(|_| !words.is_empty()).map(|target| {
        let (goes, label) = (model.clone(), target.label());
        let remote = crate::shells::Remote::new(target, oneterm_ssh::Auth::Auto);
        MenuItem::new(format!("Connect to {label}")).on_select(move || goes.connect_to(remote.clone(), false))
    });
    if !words.is_empty() {
        let user = typed.contains('@');
        items.extend(connect.clone().filter(|_| user));
        items.extend(found_hosts(model, &holds));
        items.extend(connect.filter(|_| !user));
        return items;
    }
    let (local, splits) = (model.clone(), model.clone());
    items.push(MenuItem::new("Local shell").keys("Ctrl+T").on_select(move || local.new_tab()));
    for shell in shells::found() {
        let (makes, name) = (model.clone(), shell.name);
        items.push(
            MenuItem::new(name).on_select(move || makes.new_tab_with(makes.launch(Some(shell.clone())))),
        );
    }
    let connects = model.ui().connecting;
    items.push(MenuItem::new("Quick connect…").keys("Ctrl+Shift+K").on_select(move || connects.set(true)));
    items.push(MenuItem::new("Split current space").keys("Ctrl+D").on_select(move || splits.split()));
    items.extend(found_hosts(model, &holds));
    let adds = model.ui().adding;
    items.push(MenuItem::separator());
    items.push(MenuItem::new("Add SSH host…").on_select(move || adds.set(true)));
    items
}

/// The recent targets, then the hosts by group, those whose name or address `holds` takes.
fn found_hosts(model: &Model, holds: &dyn Fn(&str) -> bool) -> Vec<MenuItem> {
    let mut items = Vec::new();
    let recent: Vec<_> = model
        .recent()
        .into_iter()
        .filter(|(remote, _)| holds(&model.host_name(remote)) || holds(&remote.target.label()))
        .collect();
    if !recent.is_empty() {
        items.push(MenuItem::separator());
        items.push(MenuItem::heading("Recent"));
    }
    for (remote, at) in recent.into_iter().take(3) {
        let (goes, name) = (model.clone(), model.host_name(&remote));
        let when = crate::hosts::ago(at);
        items.push(MenuItem::new(name).keys(when).on_select(move || goes.connect_to(remote.clone(), false)));
    }
    let hosts = model.hosts().into_iter().filter(|it| holds(&it.name) || holds(&it.address()));
    let mut group = None;
    for host in hosts {
        if group.as_ref() != Some(&host.group) {
            items.push(MenuItem::separator());
            if !host.group.is_empty() {
                items.push(MenuItem::heading(host.group.clone()));
            }
            group = Some(host.group.clone());
        }
        let (goes, remote) = (model.clone(), host.remote.clone());
        items.push(
            MenuItem::new(host.name.clone())
                .keys(host.detail())
                .on_select(move || goes.connect_to(remote.clone(), false)),
        );
    }
    items
}

/// The menu of a tab: Rename, Duplicate, Close, Close others.
pub(crate) fn tab_menu(model: &Model, id: u64) -> Vec<MenuItem> {
    let (renames, copies, closes, keeps) = (model.renaming(), model.clone(), model.clone(), model.clone());
    vec![
        MenuItem::new("Rename").on_select(move || renames.set(Some(id))),
        MenuItem::new("Duplicate").on_select(move || copies.duplicate(id)),
        MenuItem::separator(),
        MenuItem::new("Close").on_select(move || _ = closes.close_tab(id)),
        MenuItem::new("Close others").on_select(move || keeps.close_others(id)),
    ]
}

/// The title of a tab, or the field that renames it: Enter keeps the name (an empty one
/// gives the program's title back), Escape leaves it as it was.
pub(crate) fn title(model: &Model, tab: Tab) -> impl View + use<> {
    let renaming = model.renaming();
    dynamic(move || {
        if renaming.get() != Some(tab.id) {
            return AnyView::new(text(move || tab.title()).ellipsis());
        }
        let value = signal(tab.name.get_untracked().unwrap_or_else(|| tab.title()));
        let field = input(value).on_submit(move |name: &str| {
            let name = name.trim();
            tab.name.set((!name.is_empty()).then(|| name.to_string()));
            renaming.set(None);
        });
        let field = div()
            .on_key_down(move |e| {
                if e.key == quadra::Key::Escape {
                    renaming.set(None);
                }
            })
            .on_mount(|tree, id| _ = tree.focus_first(id))
            .child(field);
        AnyView::new(field)
    })
}

/// A tab's row: the accent bar and fill while it shows, its icon, its title and a line on
/// what it holds.
fn tab_row(model: &Model, tab: Tab) -> impl View + use<> {
    let active = model.active();
    let on = move || active.get() == tab.id;
    let (shows, closes, moves, menu) = (model.clone(), model.clone(), model.clone(), model.clone());
    let bar = div()
        .absolute()
        .left(0.0)
        .top(8.0)
        .bottom(8.0)
        .width(2.0)
        .bg(token::ACCENT)
        .display(move || if on() { Display::Flex } else { Display::None });
    let words = column()
        .grow(1.0)
        .min_width(0.0)
        .child(div().text_color(token::TEXT).child(title(model, tab)))
        .child(
            div()
                .text_size(11.0)
                .font_family(fonts::MONO)
                .text_color(token::TEXT_2)
                .child(text(move || marks::tab_line(tab)).ellipsis()),
        );
    let face = row()
        .align_items(Align::Center)
        .gap(10.0)
        .padding_x(10.0)
        .height(by_text(46.0))
        .radius(6.0)
        .bg(move || if on() { token::RAISED } else { token::PANEL })
        .hover(|s| s.bg(token::PANEL_HOVER))
        .drop_target(|s| s.bg(token::ACCENT_TINT))
        .role(Role::Tab)
        .label(move || tab.title())
        .selected(on)
        .on_click(move |e| {
            shows.show(tab.id);
            if e.clicks == 2 {
                shows.renaming().set(Some(tab.id));
            }
        })
        .on_pointer(PointerKind::Click, move |e| {
            if e.button == MouseButton::Middle {
                closes.close_tab(tab.id);
            }
        })
        .draggable(move || Dragged(tab.id))
        .on_drop::<Dragged>(move |dragged, _| {
            let to = moves.tabs().with_untracked(|tabs| tabs.iter().position(|it| it.id == tab.id));
            moves.move_tab(dragged.0, to.unwrap_or(usize::MAX));
        })
        .child(bar)
        .child(marks::tab_icon(tab))
        .child(words)
        .child(marks::tab_mark(tab))
        .child(marks::progress_bar(tab));
    context_menu(face, move || tab_menu(&menu, tab.id))
}
