//! The rail's SSH hosts (specs/19, "M8 in detail"): a filter, the hosts by group, each
//! connecting on a click, and the dialog that adds one.

use oneterm_ssh::Target;
use quadra::{
    Align, AnyView, Length, Role, Signal, View, by_text, column, div, dynamic, effect, row, signal, text,
    untrack,
};
use quadra_design::{
    MenuItem, Notice, Tone, Variant, context_menu, fonts, icon, icon_button, icons, input, select, token,
    tooltip,
};

use crate::hosts::{Host, auth_of};
use crate::model::Model;
use crate::shells::Remote;

const AUTHS: [&str; 4] = ["Auto (agent, keys, password)", "Agent (ssh-agent)", "Key file", "Password"];

/// The heading with its Add button, the filter, the groups (folded by a click on their
/// heading) and their hosts.
pub(crate) fn hosts(model: &Model) -> impl View + use<> {
    let (filter, folded) = (signal(String::new()), signal(Vec::<String>::new()));
    let (shown, adds) = (model.clone(), model.ui().adding);
    let add =
        tooltip("Add SSH host", icon_button(icons::PLUS, "Add SSH host").on_click(move || adds.set(true)));
    let heading = crate::rail::heading("SSH HOSTS", 18.0).child(add);
    let field = input(filter)
        .placeholder("Filter hosts\u{2026}")
        .icon(icons::SEARCH)
        .width(232.0)
        .test_id("host-filter");
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
        let mut list = column().gap(2.0).width(Length::Fill);
        for (n, group) in groups.into_iter().enumerate() {
            let closed = folded.with(|it| it.contains(&group));
            if !group.is_empty() {
                let name = group.clone();
                let fold = move |_: &mut quadra::PointerEvent| {
                    folded.update(|it| match it.iter().position(|g| *g == name) {
                        Some(at) => _ = it.remove(at),
                        None => it.push(name.clone()),
                    })
                };
                let chevron = if closed { icons::CHEVRON_RIGHT } else { icons::CHEVRON_DOWN };
                list = list.child(
                    row()
                        .align_items(Align::Center)
                        .gap(4.0)
                        .padding_x(6.0)
                        .padding_top(if n == 0 { 6.0 } else { 10.0 })
                        .padding_bottom(2.0)
                        .text_size(11.0)
                        .text_color(token::TEXT_2)
                        .role(Role::Button)
                        .label(group.clone())
                        .expanded(!closed)
                        .on_click(fold)
                        .child(icon(chevron).size(12.0))
                        .child(text(group.clone())),
                );
            }
            for host in hosts.iter().filter(|it| it.group == group && !closed) {
                list = list.child(host_row(&shown, host.clone()));
            }
        }
        AnyView::new(list)
    });
    column()
        .grow(1.0)
        .min_height(0.0)
        .child(heading)
        .child(div().padding_x(8.0).padding_bottom(6.0).child(field))
        .child(quadra::scroll(signal(0.0), list).container(column().grow(1.0).min_height(0.0).padding_x(8.0)))
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
        .width(Length::Fill)
        .align_items(Align::Center)
        .gap(8.0)
        .padding_left(22.0)
        .padding_right(8.0)
        .height(by_text(30.0))
        .radius(6.0)
        .text_color(token::TEXT)
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
                .text_color(token::TEXT_2)
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
    let (editing, edited) = (model.ui().editing, host.name.clone());
    vec![
        MenuItem::new("Connect").on_select(move || tab.connect_to(a.clone(), false)),
        MenuItem::new("Connect in a new space").on_select(move || space.connect_to(b.clone(), true)),
        MenuItem::new("Edit…").on_select(move || editing.set(Some(edited.clone()))),
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
    jump: Signal<usize>,
    forwards: Signal<String>,
}

/// The dialogs that save a host: a new one, or one saved before, filled in.
pub(crate) fn add_dialog(model: &Model) -> impl View + use<> {
    let (open, editing, changing) = (model.ui().adding, model.ui().editing, signal(false));
    let s = || signal(String::new());
    let form = Form {
        name: s(),
        group: s(),
        target: s(),
        auth: signal(0),
        key: s(),
        jump: signal(0),
        forwards: s(),
    };
    let (saves, changes, lists, fills) = (model.clone(), model.clone(), model.clone(), model.clone());
    // Asked to edit a host, the form says what it is; closed, it is no longer being edited.
    effect(move || match editing.get() {
        Some(name) => {
            if let Some(host) = untrack(|| fills.hosts()).into_iter().find(|it| it.name == name) {
                untrack(|| fill(&fills, form, &host));
                changing.set(true);
            }
        }
        None => changing.set(false),
    });
    effect(move || {
        if !changing.get() {
            editing.set(None);
        }
    });
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
            .child(select(lists.jump_choices(), form.jump).width(240.0).test_id("host-jump"))
            .child(field(form.forwards, "Local forwards: 8080:localhost:80 ...", "host-forwards"))
    };
    let adds = quadra_design::dialog(open, "Add SSH host", body.clone())
        .action("Cancel", Variant::Secondary, || {})
        .action("Save", Variant::Primary, move || save(&saves, form, None));
    let edits = quadra_design::dialog(changing, "Edit SSH host", body)
        .action("Cancel", Variant::Secondary, || {})
        .action("Save", Variant::Primary, move || save(&changes, form, editing.get_untracked()));
    div().child(adds).child(edits)
}

/// The form as a saved host says.
fn fill(model: &Model, form: Form, host: &Host) {
    let (auth, key) = match &host.remote.auth {
        oneterm_ssh::Auth::Agent => (1, String::new()),
        oneterm_ssh::Auth::Key(path) => (2, path.display().to_string()),
        oneterm_ssh::Auth::Password => (3, String::new()),
        oneterm_ssh::Auth::Auto => (0, String::new()),
    };
    let forwards: Vec<String> = host.remote.forwards.iter().map(oneterm_ssh::Forward::label).collect();
    let jump = model.jump_choices().iter().position(|it| *it == host.jump && !host.jump.is_empty());
    form.name.set(host.name.clone());
    form.group.set(host.group.clone());
    form.target.set(host.address());
    form.auth.set(auth);
    form.key.set(key);
    form.forwards.set(forwards.join(" "));
    form.jump.set(jump.unwrap_or(0));
}

/// Saves the form as a host, in place of `was` (the one edited) when it had another name.
fn save(model: &Model, form: Form, was: Option<String>) {
    let typed = form.target.get_untracked();
    let Some(target) = Target::parse(&typed) else {
        model.notify(Notice::new("Add SSH host").body(format!("Not a target: {typed}")).tone(Tone::Warning));
        return;
    };
    let name = form.name.get_untracked().trim().to_string();
    let name = if name.is_empty() { target.host.clone() } else { name };
    let mut remote = Remote::new(target, auth_of(form.auth.get_untracked(), form.key.get_untracked()));
    for word in form.forwards.get_untracked().split_whitespace() {
        let Some(forward) = oneterm_ssh::Forward::parse(word) else {
            model.notify(
                Notice::new("Add SSH host").body(format!("Not a forward: {word}")).tone(Tone::Warning),
            );
            return;
        };
        remote.forwards.push(forward);
    }
    let jump = model.jump_host(form.jump.get_untracked()).map_or(String::new(), |it| it.name);
    if let Some(was) = was.filter(|it| *it != name) {
        model.remove_host(&was);
    }
    model.save_host(Host { name, group: form.group.get_untracked().trim().into(), remote, jump });
    for field in [form.name, form.group, form.target, form.key, form.forwards] {
        field.set(String::new());
    }
    form.jump.set(0);
}
