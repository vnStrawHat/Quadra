//! The expanded SFTP view (specs/19, "M9 in detail"): in the workspace's place, Local at the
//! left and Remote at the right, the transfer buttons between, the transfers under both. The
//! canvas gives the layout; the kit's tokens and parts draw it.

use quadra::{
    Align, AnyView, Length, Role, Signal, View, by_text, column, div, dynamic, effect, row, signal, text,
};
use quadra_design::{button, checkbox, fonts, icon, icon_button, icons, token, tooltip};

use crate::model::Model;
use crate::panes::{compared, visible};
use crate::sftp::Dock;

mod rows;
use rows::{line, transfers};

const SIZE: f32 = 80.0;
const MODIFIED: f32 = 120.0;

pub(crate) fn view(model: &Model, d: Dock) -> impl View + use<> {
    let (follow, compare) = (signal(false), signal(false));
    follow_cwd(model, d, follow);
    let (host_of, refreshes, ups, gets, collapse) =
        (model.clone(), model.clone(), model.clone(), model.clone(), d.expanded);
    let host = move || {
        let space = host_of.active_tab().and_then(|tab| tab.focused());
        let name = space.and_then(|it| it.remote).map(|it| host_of.host_name(&it)).unwrap_or_default();
        format!("{name} \u{B7} ssh session keeps running")
    };
    let head = row()
        .align_items(Align::Center)
        .gap(8.0)
        .padding_x(12.0)
        .height(by_text(42.0))
        .border_bottom(1.0)
        .border_color(token::LINE)
        .child(icon(icons::FOLDER).size(14.0).color(token::ACCENT))
        .child(div().font_weight(600).child(text("SFTP")))
        .child(
            div()
                .grow(1.0)
                .min_width(Length::Px(0.0))
                .text_size(11.0)
                .text_color(token::TEXT_2)
                .child(text(host).ellipsis()),
        )
        .child(checkbox(follow).label("Sync cwd with terminal"))
        .child(tooltip(
            "Show hidden files",
            button(".hidden")
                .toggled(move || d.hidden.get())
                .on_click(move || d.hidden.update(|on| *on = !*on)),
        ))
        .child(tooltip(
            "Collapse SFTP",
            icon_button(icons::WINDOW_RESTORE, "Collapse SFTP to side panel")
                .on_click(move || collapse.set(false)),
        ));
    let local = d.local;
    let local_rows = dynamic(move || {
        let (all, said) = (visible(&local.entries.get(), d.hidden.get()), local.said.get());
        let mut list = column().width(Length::Fill).padding(4.0);
        if !said.is_empty() {
            return AnyView::new(
                list.child(
                    div().width(Length::Fill).padding(8.0).text_color(token::TEXT_2).child(text(said)),
                ),
            );
        }
        for entry in all {
            let (name, picks, opens) = (entry.name.clone(), entry.name.clone(), entry.clone());
            let line = line(&entry, None).selected(move || local.selected.get().as_deref() == Some(&name));
            list = list.child(line.on_click(move |e| match (opens.dir, e.clicks) {
                (true, 2) => local.load(local.dir.get_untracked().join(&opens.name)),
                _ => local.selected.set(Some(picks.clone())),
            }));
        }
        AnyView::new(list)
    });
    let remotes = model.clone();
    let remote_rows =
        dynamic(move || {
            let (all, said) = (visible(&d.entries.get(), d.hidden.get()), d.said.get());
            let mut list = column().width(Length::Fill).padding(4.0);
            if !said.is_empty() {
                return AnyView::new(list.child(
                    div().width(Length::Fill).padding(8.0).text_color(token::TEXT_2).child(text(said)),
                ));
            }
            let locals = if compare.get() { local.entries.get() } else { Vec::new() };
            for entry in all {
                let (name, picks, opens, goes) =
                    (entry.name.clone(), entry.name.clone(), entry.clone(), remotes.clone());
                let line = line(&entry, compared(&entry, &locals))
                    .selected(move || d.selected.get().as_deref() == Some(&name));
                list = list.child(line.on_click(move |e| match (opens.dir, e.clicks) {
                    (true, 2) => d.load(&goes, Some(oneterm_ssh::join(&d.dir.get_untracked(), &opens.name))),
                    _ => d.selected.set(Some(picks.clone())),
                }));
            }
            AnyView::new(list)
        });
    let local_pane = pane(
        "Local",
        move || local.dir.get().display().to_string(),
        move || local.up(),
        move || local.refresh(),
        local_rows,
    );
    let remote_pane = pane(
        "Remote",
        move || d.dir.get(),
        move || {
            let dir = d.dir.get_untracked();
            let up = dir.rsplit_once('/').map_or("/", |it| it.0);
            d.load(&ups, Some(if up.is_empty() { "/".into() } else { up.into() }));
        },
        move || d.refresh(&refreshes),
        remote_rows,
    );
    let sends = model.clone();
    let mid =
        |shape, name: &'static str, f: Box<dyn FnMut()>| tooltip(name, icon_button(shape, name).on_click(f));
    let middle = column()
        .width(56.0)
        .shrink(0.0)
        .align_items(Align::Center)
        .justify(quadra::Justify::Center)
        .gap(8.0)
        .border_left(1.0)
        .border_right(1.0)
        .border_color(token::LINE)
        .child(mid(
            icons::ARROW_RIGHT,
            "Upload selected",
            Box::new(move || {
                if let Some(name) = local.selected.get_untracked() {
                    d.upload(&sends, vec![local.dir.get_untracked().join(name)]);
                }
            }),
        ))
        .child(mid(
            icons::ARROW_LEFT,
            "Download selected",
            Box::new(move || {
                let name = d.selected.get_untracked();
                let entry = d
                    .entries
                    .with_untracked(|all| all.iter().find(|it| Some(&it.name) == name.as_ref()).cloned());
                if let Some(entry) = entry.filter(|it| !it.dir) {
                    d.download(&gets, &entry);
                }
            }),
        ))
        .child(tooltip(
            "Compare",
            icon_button(icons::COLUMNS_2, "Compare")
                .toggled(move || compare.get())
                .on_click(move || compare.update(|on| *on = !*on)),
        ));
    column()
        .grow(1.0)
        .min_width(Length::Px(0.0))
        .bg(token::PANEL)
        .role(Role::Group)
        .label("SFTP")
        .child(head)
        .child(row().grow(1.0).min_height(0.0).child(local_pane).child(middle).child(remote_pane))
        .child(transfers(d))
}

/// The remote pane follows the focused space's directory while Sync is on, when it is a path
/// of the server (`/..`, or a `file://` URL of one).
fn follow_cwd(model: &Model, d: Dock, follow: Signal<bool>) {
    let model = model.clone();
    effect(move || {
        let Some(space) = model.active_tab().and_then(|tab| tab.focused()) else { return };
        let cwd = space.session.cwd().get();
        if !follow.get() || space.remote.is_none() {
            return;
        }
        let path = cwd
            .strip_prefix("file://")
            .map_or(cwd.as_str(), |rest| rest.find('/').map_or("", |at| &rest[at..]));
        if path.starts_with('/') && quadra::untrack(|| d.dir.get()) != path {
            let (model, path) = (model.clone(), path.to_string());
            quadra::untrack(move || d.load(&model, Some(path)));
        }
    });
}

/// A pane: its label, its path, Up and Refresh, the header row and the rows.
fn pane(
    label: &'static str,
    path: impl Fn() -> String + 'static,
    up: impl FnMut() + 'static,
    again: impl FnMut() + 'static,
    rows: impl View + 'static,
) -> impl View {
    let quiet = |shape, name: &'static str| icon_button(shape, name).size(quadra_design::Size::Xs);
    let (up_name, again_name) = match label {
        "Local" => ("Up (local)", "Refresh local"),
        _ => ("Up (remote)", "Refresh remote"),
    };
    let bar = row()
        .align_items(Align::Center)
        .gap(8.0)
        .padding_x(12.0)
        .height(by_text(36.0))
        .border_bottom(1.0)
        .border_color(token::LINE)
        .child(
            div()
                .text_size(11.0)
                .font_weight(600)
                .text_color(token::TEXT_2)
                .child(text(label.to_uppercase())),
        )
        .child(
            div()
                .grow(1.0)
                .min_width(Length::Px(0.0))
                .font_family(fonts::MONO)
                .text_size(11.0)
                .text_color(token::TEXT_2)
                .child(text(path).ellipsis()),
        )
        .child(quiet(icons::ARROW_UP, up_name).on_click(up))
        .child(quiet(icons::REFRESH_CW, again_name).on_click(again));
    let header = row()
        .gap(12.0)
        .padding_x(16.0)
        .height(by_text(26.0))
        .align_items(Align::Center)
        .text_size(11.0)
        .text_color(token::TEXT_2)
        .border_bottom(1.0)
        .border_color(token::LINE)
        .child(div().grow(1.0).child(text("Name")))
        .child(div().width(SIZE).child(text("Size")))
        .child(div().width(MODIFIED).child(text("Modified")));
    let rows = column().width(Length::Fill).child(rows);
    let list = quadra::scroll(signal(0.0), rows).container(column().grow(1.0).min_height(0.0));
    column()
        .grow(1.0)
        .width(0.0)
        .min_width(Length::Px(0.0))
        .role(Role::Group)
        .label(label)
        .child(bar)
        .child(header)
        .child(list)
}
