//! The SFTP dock (specs/19, "M9 in detail"): 300 px at the workspace's right, the folder of
//! the focused space's connection, its entries, the transfers; files dropped on it upload.

use oneterm_ssh::Entry;
use quadra::{
    Align, AnyView, DroppedFiles, Length, Role, View, by_text, column, div, dynamic, effect, memo, row,
    signal, text,
};
use quadra_design::{
    MenuItem, Variant, button, context_menu, fonts, icon, icon_button, icons, menu_icon_button, token,
    tooltip,
};

use crate::locked::shown;
use crate::model::Model;
use crate::panes::size;
use crate::sftp::Dock;

mod menus;
mod transfers;
pub(crate) use menus::{Ask, local as local_menu, remote as remote_menu, remote_area};
use transfers::transfers;

/// How many entries the list shows; a longer folder says how many more.
const SHOWN: usize = 2000;

/// The dock is open or closed as the user left it for the kind of tab that shows: a local
/// shell's, closed at first, or an SSH space's, open.
fn follow_kind(model: &Model, open: quadra::Signal<bool>) {
    let (p, kinds) = (model.prefs(), model.clone());
    let remote = memo(move || {
        kinds.active_tab().is_some_and(|tab| tab.spaces.with(|all| all.iter().any(|it| it.remote.is_some())))
    });
    let slot = move |remote: bool| if remote { p.dock_ssh } else { p.dock_local };
    effect(move || {
        let wanted = slot(remote.get()).get();
        if quadra::untrack(|| open.get()) != wanted {
            open.set(wanted);
        }
    });
    effect(move || {
        let (now, slot) = (open.get(), slot(quadra::untrack(|| remote.get())));
        if quadra::untrack(|| slot.get()) != now {
            slot.set(now);
        }
    });
}

/// The dock, or in the workspace's place the expanded view.
pub(crate) fn dock(model: &Model) -> impl View + use<> {
    let (open, expanded) = (model.ui().dock, model.ui().expanded);
    let d = Dock::new(model, open);
    follow_kind(model, open);
    let views = model.clone();
    let shows = dynamic(move || match expanded.get() {
        true => AnyView::new(crate::expanded::view(&views, d)),
        false => AnyView::new(panel(&views, d)),
    });
    // Expanded, it takes the workspace's room.
    let grows = move || if expanded.get() { 1.0 } else { 0.0 };
    row()
        .min_width(quadra::Length::Px(0.0))
        .grow(grows)
        .display(shown(move || open.get()))
        .child(shows.container(row().min_width(quadra::Length::Px(0.0)).grow(grows)))
        .child(menus::dialogs(model, d))
}

fn panel(model: &Model, d: Dock) -> impl View + use<> {
    let open = model.ui().dock;
    // The connection of the focused space, again whenever another space has the focus.
    let follows = model.clone();
    let focused = memo(move || follows.active_tab().and_then(|tab| tab.focused()).map(|it| it.id));
    let loads = model.clone();
    effect(move || {
        focused.get();
        if open.get() {
            quadra::untrack(|| d.load(&loads, None));
        }
    });
    let host_of = model.clone();
    let host = move || {
        let space = host_of.active_tab().and_then(|tab| tab.focused());
        let remote = space.and_then(|it| it.remote);
        remote.map(|it| host_of.host_name(&it)).unwrap_or_else(|| "local space \u{B7} n/a".into())
    };
    let (refreshes, drops) = (model.clone(), model.clone());
    // Refresh and New folder wait in a menu: the header has the design's two buttons.
    let more = menu_icon_button(icons::ELLIPSIS, "More", move || {
        let refreshes = refreshes.clone();
        vec![
            MenuItem::new("Refresh").on_select(move || d.refresh(&refreshes)),
            MenuItem::new("New folder\u{2026}").on_select(move || d.ask.folder()),
        ]
    })
    .size(quadra_design::Size::Sm);
    let expand = icon_button(icons::WINDOW_MAXIMIZE, "Expand SFTP")
        .variant(Variant::Secondary)
        .size(quadra_design::Size::Sm)
        .on_click(move || d.expanded.set(true));
    let close =
        icon_button(icons::X, "Close SFTP").size(quadra_design::Size::Sm).on_click(move || open.set(false));
    let head = row()
        .align_items(Align::Center)
        .gap(8.0)
        .padding_left(12.0)
        .padding_right(8.0)
        .height(by_text(42.0))
        .shrink(0.0)
        .border_bottom(1.0)
        .border_color(token::LINE)
        .child(icon(icons::FOLDER).size(14.0).color(token::ACCENT))
        .child(div().font_weight(600).text_size(12.0).child(text("SFTP")))
        .child(
            div()
                .grow(1.0)
                .min_width(Length::Px(0.0))
                .text_size(11.0)
                .text_color(token::TEXT_2)
                .child(text(host).ellipsis()),
        )
        .child(more)
        .child(tooltip("Expand: local and remote side by side", expand))
        .child(tooltip("Hide SFTP panel", close));
    column()
        .width(300.0)
        .shrink(0.0)
        .bg(token::PANEL)
        .border_left(1.0)
        .border_color(token::LINE)
        .label("SFTP")
        .on_drop(move |files: &DroppedFiles, _| d.upload(&drops, files.0.clone()))
        .drop_target(|s| s.bg(token::ACCENT_TINT))
        .child(head)
        .child(crumbs(model, d))
        .child(entries(model, d))
        .child(transfers(d))
}

/// The folder as parts, each going there: `/ home / trung / apps`.
fn crumbs(model: &Model, d: Dock) -> impl View + use<> {
    let model = model.clone();
    let parts = dynamic(move || {
        let dir = d.dir.get();
        if dir.is_empty() {
            return AnyView::new(div());
        }
        let mut line = row()
            .gap(4.0)
            .padding_x(12.0)
            .padding_y(6.0)
            .border_bottom(1.0)
            .border_color(token::LINE)
            .font_family(fonts::MONO)
            .text_size(11.0)
            .text_color(token::TEXT_2)
            .cut_to_parent();
        let names: Vec<&str> = dir.split('/').filter(|it| !it.is_empty()).collect();
        let mut at = "/".to_string();
        if names.is_empty() {
            line = line.child(text("/"));
        }
        // `/home / trung / apps`, the last in the text colour.
        for (i, name) in names.iter().enumerate() {
            at = oneterm_ssh::join(&at, name);
            let (goes, path) = (model.clone(), at.clone());
            let last = i + 1 == names.len();
            let part = div()
                .text_color(if last { token::TEXT } else { token::TEXT_2 })
                .hover(|s| s.text_color(token::TEXT))
                .role(Role::Link)
                .label(path.clone())
                .on_click(move |_| d.load(&goes, Some(path.clone())));
            if i > 0 {
                line = line.child(text("/"));
            }
            let name = if i == 0 { format!("/{name}") } else { name.to_string() };
            line = line.child(part.child(text(name)));
        }
        AnyView::new(line)
    });
    parts.container(div())
}

fn entries(model: &Model, d: Dock) -> impl View + use<> {
    let (model, model_of) = (model.clone(), model.clone());
    let list = dynamic(move || {
        let said = d.said.get();
        if said == crate::sftp::LOCAL {
            return AnyView::new(local(&model));
        }
        if !said.is_empty() {
            return AnyView::new(div().padding(10.0).text_color(token::TEXT_2).child(text(said)));
        }
        let all = crate::panes::visible(&d.entries.get(), d.hidden.get());
        let mut list = column().width(Length::Fill).padding_x(6.0).padding_y(4.0);
        let dir = d.dir.get_untracked();
        if dir != "/" && !dir.is_empty() {
            let (up, goes) = (dir.rsplit_once('/').map_or("/", |it| it.0).to_string(), model.clone());
            let up = if up.is_empty() { "/".to_string() } else { up };
            let entry = Entry { name: "..".into(), dir: true, size: 0, modified: 0 };
            list = list.child(line(&entry).on_click(move |_| d.load(&goes, Some(up.clone()))));
        }
        for entry in all.iter().take(SHOWN) {
            list = list.child(entry_row(&model, d, entry.clone()));
        }
        if all.len() > SHOWN {
            list = list.child(
                div()
                    .padding(8.0)
                    .text_color(token::TEXT_3)
                    .child(text(format!("{} more", all.len() - SHOWN))),
            );
        }
        AnyView::new(list)
    });
    let area = column().grow(1.0).min_height(0.0);
    context_menu(quadra::scroll(signal(0.0), list).container(area), menus::remote_area(&model_of, d))
}

/// A row: its icon, its name, its size.
fn line(entry: &Entry) -> quadra::Div {
    let shape = if entry.dir { icons::FOLDER } else { icons::FILE };
    let size = if entry.dir { "\u{2014}".to_string() } else { size(entry.size) };
    row()
        .align_items(Align::Center)
        .gap(8.0)
        .padding_x(8.0)
        .height(by_text(27.0))
        .radius(5.0)
        .hover(|s| s.bg(token::PANEL_HOVER))
        .role(Role::Button)
        .label(entry.name.clone())
        .child(icon(shape).size(14.0).color(if entry.dir { token::ACCENT } else { token::TEXT_2 }))
        .child(div().grow(1.0).min_width(Length::Px(0.0)).child(text(entry.name.clone()).ellipsis()))
        .child(div().text_size(11.0).text_color(token::TEXT_2).child(text(size)))
}

/// What the dock shows for a local space: why it is empty, and a button that shows the
/// first tab with a remote space.
fn local(model: &Model) -> impl View + use<> {
    let found = model.tabs().get_untracked().into_iter().find_map(|tab| {
        tab.spaces
            .with_untracked(|spaces| spaces.iter().find_map(|it| it.remote.clone()))
            .map(|it| (tab.id, it))
    });
    let mut view = column()
        .grow(1.0)
        .align_items(Align::Center)
        .justify(quadra::Justify::Center)
        .gap(10.0)
        .padding(24.0)
        .text_size(12.0)
        .text_color(token::TEXT_2)
        .child(icon(icons::FOLDER).size(24.0).color(token::TEXT_3))
        .child(text(crate::sftp::LOCAL));
    if let Some((id, remote)) = found {
        let shows = model.clone();
        let label = format!("Show {}", model.host_name(&remote));
        view = view.child(button(label).variant(Variant::Secondary).on_click(move || shows.show(id)));
    }
    view
}

fn entry_row(model: &Model, d: Dock, entry: Entry) -> impl View + use<> {
    let (opens, kept) = (model.clone(), entry.clone());
    let face = line(&entry).on_click(move |e| match (kept.dir, e.clicks) {
        (true, _) => d.load(&opens, Some(oneterm_ssh::join(&d.dir.get_untracked(), &kept.name))),
        (false, 2) => d.download(&opens, &kept),
        _ => {}
    });
    context_menu(face, menus::remote(model, d, &entry))
}
