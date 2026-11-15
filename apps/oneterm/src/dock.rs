//! The SFTP dock (specs/19, "M9 in detail"): 300 px at the workspace's right, the folder of
//! the focused space's connection, its entries, the transfers; files dropped on it upload.

use oneterm_ssh::Entry;
use quadra::{
    Align, AnyView, DroppedFiles, Length, Role, View, by_text, column, div, dynamic, effect, memo, row,
    signal, text,
};
use quadra_design::{
    MenuItem, Variant, button, context_menu, fonts, icon, icon_button, icons, input, menu_icon_button, token,
    tooltip,
};

use crate::locked::shown;
use crate::model::Model;
use crate::panes::size;
use crate::sftp::Dock;

mod transfers;
use transfers::transfers;

/// How many entries the list shows; a longer folder says how many more.
const SHOWN: usize = 2000;

/// The dock, or in the workspace's place the expanded view.
pub(crate) fn dock(model: &Model) -> impl View + use<> {
    let (open, expanded) = (model.ui().dock, model.ui().expanded);
    let d = Dock::new(model, open);
    let views = model.clone();
    dynamic(move || match expanded.get() {
        true => AnyView::new(crate::expanded::view(&views, d)),
        false => AnyView::new(panel(&views, d)),
    })
    // Expanded, it takes the workspace's room.
    .container(
        row()
            .min_width(quadra::Length::Px(0.0))
            .grow(move || if expanded.get() { 1.0 } else { 0.0 })
            .display(shown(move || open.get())),
    )
}

fn panel(model: &Model, d: Dock) -> impl View + use<> {
    let (open, naming) = (model.ui().dock, signal(false));
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
    let (named, host_of) = (model.clone(), model.clone());
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
            MenuItem::new("New folder").on_select(move || naming.set(true)),
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
    let folder_name = signal(String::new());
    let new_folder = div().padding_x(10.0).padding_bottom(6.0).display(shown(move || naming.get())).child(
        input(folder_name).placeholder("New folder name").width(280.0).test_id("sftp-new-folder").on_submit(
            move |name: &str| {
                if !name.trim().is_empty() {
                    d.mkdir(&named, name.trim());
                }
                folder_name.set(String::new());
                naming.set(false);
            },
        ),
    );
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
        .child(new_folder)
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
    let model = model.clone();
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
    quadra::scroll(signal(0.0), list).container(column().grow(1.0).min_height(0.0))
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
    let (opens, menu, kept) = (model.clone(), model.clone(), entry.clone());
    let face = line(&entry).on_click(move |e| match (kept.dir, e.clicks) {
        (true, _) => d.load(&opens, Some(oneterm_ssh::join(&d.dir.get_untracked(), &kept.name))),
        (false, 2) => d.download(&opens, &kept),
        _ => {}
    });
    context_menu(face, move || {
        let (gets, gone, (a, b)) = (menu.clone(), menu.clone(), (entry.clone(), entry.clone()));
        let mut items = vec![MenuItem::new("Delete").on_select(move || d.delete(&gone, &b))];
        if !entry.dir {
            items.insert(0, MenuItem::new("Download").on_select(move || d.download(&gets, &a)));
        }
        items
    })
}
