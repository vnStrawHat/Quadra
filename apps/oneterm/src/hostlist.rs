//! The rail's SSH hosts (specs/19, "M8 in detail"): a filter, the hosts by group, each
//! connecting on a click, and the dialog that adds one.

use oneterm_ssh::Target;
use quadra::{Align, AnyView, Length, Role, Signal, View, by_text, column, div, dynamic, row, signal, text};
use quadra_design::{
    MenuItem, Notice, Tone, Variant, context_menu, fonts, icon, icons, input, select, token,
};

use crate::hosts::{Host, auth_of};
use crate::model::Model;
use crate::shells::Remote;

const AUTHS: [&str; 4] = ["Auto (agent, keys, password)", "Agent (ssh-agent)", "Key file", "Password"];

/// The heading, the filter, the groups and their hosts, then "Add SSH host…".
pub(crate) fn hosts(model: &Model) -> impl View + use<> {
    let filter = signal(String::new());
    let (shown, adds) = (model.clone(), model.ui().adding);
    let heading = div()
        .padding_left(14.0)
        .padding_top(14.0)
        .height(by_text(36.0))
        .text_size(11.0)
        .text_color(token::TEXT_3)
        .child(text("SSH HOSTS"));
    let field =
        input(filter).placeholder("Filter hosts").icon(icons::SEARCH).width(232.0).test_id("host-filter");
    let list = dynamic(move || {
        let words = filter.get().to_lowercase();
        let hosts: Vec<Host> = (shown.hosts().into_iter())
            .filter(|it| {
                (it.name.to_lowercase().contains(&words)) || it.address().to_lowercase().contains(&words)
            })
            .collect();
        let mut groups: Vec<String> = Vec::new();
        for host in &hosts {
            if !groups.contains(&host.group) {
                groups.push(host.group.clone());
            }
        }
        let mut list = column().gap(2.0);
        for group in groups {
            if !group.is_empty() {
                list = list.child(
                    div()
                        .padding_x(8.0)
                        .padding_top(6.0)
                        .text_size(11.0)
                        .text_color(token::TEXT_3)
                        .child(text(group.clone())),
                );
            }
            for host in hosts.iter().filter(|it| it.group == group) {
                list = list.child(host_row(&shown, host.clone()));
            }
        }
        AnyView::new(list)
    });
    let add = row()
        .align_items(Align::Center)
        .gap(10.0)
        .padding_x(8.0)
        .height(by_text(30.0))
        .radius(6.0)
        .text_color(token::TEXT_2)
        .hover(|s| s.bg(token::PANEL_HOVER).text_color(token::TEXT))
        .role(Role::Button)
        .label("Add SSH host…")
        .on_click(move |_| adds.set(true))
        .child(icon(icons::PLUS).size(14.0))
        .child(text("Add SSH host…"));
    column()
        .grow(1.0)
        .min_height(0.0)
        .child(heading)
        .child(div().padding_x(8.0).padding_bottom(4.0).child(field))
        .child(quadra::scroll(signal(0.0), list).container(column().grow(1.0).min_height(0.0).padding_x(8.0)))
        .child(div().padding_x(8.0).child(add))
}

/// A host: a dot while a space is connected to it, its name, its address.
fn host_row(model: &Model, host: Host) -> impl View + use<> {
    let (on, opens, menu) = (model.clone(), model.clone(), model.clone());
    let target = host.remote.target.clone();
    let dot = div()
        .size(7.0, 7.0)
        .radius(3.5)
        .shrink(0.0)
        .bg(move || if on.connected_to(&target) { token::OK } else { token::LINE_STRONG });
    let remote = host.remote.clone();
    let face = row()
        .align_items(Align::Center)
        .gap(10.0)
        .padding_x(8.0)
        .height(by_text(28.0))
        .radius(6.0)
        .hover(|s| s.bg(token::PANEL_HOVER))
        .role(Role::Button)
        .label(host.name.clone())
        .on_click(move |_| opens.connect_to(remote.clone(), false))
        .child(dot)
        .child(div().grow(1.0).min_width(Length::Px(0.0)).child(text(host.name.clone()).ellipsis()))
        .child(
            div()
                .font_family(fonts::MONO)
                .text_size(11.0)
                .text_color(token::TEXT_3)
                .child(text(short(&host.remote.target))),
        );
    context_menu(face, move || host_menu(&menu, &host))
}

/// The host and its port when it is not 22, as the rail shows it.
fn short(target: &Target) -> String {
    match target.port {
        22 => target.host.clone(),
        port => format!("{}:{port}", target.host),
    }
}

fn host_menu(model: &Model, host: &Host) -> Vec<MenuItem> {
    let (tab, space, gone) = (model.clone(), model.clone(), model.clone());
    let (a, b, name) = (host.remote.clone(), host.remote.clone(), host.name.clone());
    vec![
        MenuItem::new("Connect").on_select(move || tab.connect_to(a.clone(), false)),
        MenuItem::new("Connect in a new space").on_select(move || space.connect_to(b.clone(), true)),
        MenuItem::separator(),
        MenuItem::new("Remove").on_select(move || gone.remove_host(&name)),
    ]
}

#[derive(Clone, Copy)]
struct Form {
    name: Signal<String>,
    group: Signal<String>,
    target: Signal<String>,
    auth: Signal<usize>,
    key: Signal<String>,
}

/// The dialog that saves a host.
pub(crate) fn add_dialog(model: &Model) -> impl View + use<> {
    let open = model.ui().adding;
    let s = || signal(String::new());
    let form = Form { name: s(), group: s(), target: s(), auth: signal(0), key: s() };
    let saves = model.clone();
    let body = move || {
        let field =
            |value, hint: &str, id: &'static str| input(value).placeholder(hint).test_id(id).width(416.0);
        let key = field(form.key, "Path of the key file", "host-key");
        column()
            .gap(10.0)
            .padding_x(12.0)
            .width(440.0)
            .child(field(form.name, "Name (the host when empty)", "host-name"))
            .child(field(form.group, "Group", "host-group"))
            .child(field(form.target, "user@host or host:port", "host-target"))
            .child(select(AUTHS, form.auth).width(240.0))
            .child(div().display(crate::locked::shown(move || form.auth.get() == 2)).child(key))
    };
    quadra_design::dialog(open, "Add SSH host", body).action("Cancel", Variant::Secondary, || {}).action(
        "Save",
        Variant::Primary,
        move || save(&saves, form),
    )
}

fn save(model: &Model, form: Form) {
    let typed = form.target.get_untracked();
    let Some(target) = Target::parse(&typed) else {
        model.notify(Notice::new("Add SSH host").body(format!("Not a target: {typed}")).tone(Tone::Warning));
        return;
    };
    let name = form.name.get_untracked().trim().to_string();
    let name = if name.is_empty() { target.host.clone() } else { name };
    let auth = auth_of(form.auth.get_untracked(), form.key.get_untracked());
    let host =
        Host { name, group: form.group.get_untracked().trim().into(), remote: Remote { target, auth } };
    model.save_host(host);
    for field in [form.name, form.group, form.target, form.key] {
        field.set(String::new());
    }
}
