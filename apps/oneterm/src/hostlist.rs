//! The rail's SSH hosts (specs/19, "M8 in detail"): a filter, the hosts by group, each
//! connecting on a click, and the dialog that adds one.

use oneterm_ssh::Target;
use quadra::{
    Align, AnyView, Length, Role, Signal, View, by_text, column, div, dynamic, effect, row, signal, text,
    untrack,
};
use quadra_design::{
    MenuItem, Notice, Tone, Variant, context_menu, field, fonts, icon, icon_button, icons, input, select,
    token, tooltip,
};

use crate::hosts::{Host, auth_of};
use crate::model::Model;
use crate::shells::Remote;

const AUTHS: [&str; 4] = ["Auto", "ssh-agent", "Key file", "Password"];
/// The colour tags a dialog offers: the group's, then each of [`crate::hosts::TAGS`].
const TAG_NAMES: [&str; 8] = ["By group", "Red", "Orange", "Yellow", "Green", "Cyan", "Blue", "Purple"];

/// The heading with its Add button, the filter, the groups (a chevron, the name, the count;
/// folded by a click) and their hosts under a line.
pub(crate) fn hosts(model: &Model) -> impl View + use<> {
    let (filter, folded) = (signal(String::new()), signal(Vec::<String>::new()));
    let (shown, adds) = (model.clone(), model.ui().adding);
    let add = tooltip(
        "Add SSH host",
        icon_button(icons::PLUS, "Add SSH host").muted().on_click(move || adds.set(true)),
    );
    let heading = crate::rail::heading("SSH hosts").child(add);
    let field = input(filter).placeholder("Filter hosts").height(28.0).width(232.0).test_id("host-filter");
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
        let mut list = column().gap(1.0).width(Length::Fill);
        for group in groups {
            let closed = folded.with(|it| it.contains(&group));
            let count = hosts.iter().filter(|it| it.group == group).count();
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
                    line()
                        .role(Role::Button)
                        .label(group.clone())
                        .expanded(!closed)
                        .on_click(fold)
                        .child(icon(chevron).size(14.0).color(token::TEXT_2))
                        .child(div().grow(1.0).min_width(0.0).child(text(group.clone()).ellipsis()))
                        .child(address(count.to_string())),
                );
            }
            // A group's hosts stand under a line from its chevron; those of no group do not.
            let mut under = column().gap(1.0).width(Length::Fill);
            if !group.is_empty() {
                under = under.padding_left(5.0).border_left(1.0).border_color(token::LINE);
            }
            for host in hosts.iter().filter(|it| it.group == group && !closed) {
                under = under.child(host_row(&shown, host.clone()));
            }
            let inset = if group.is_empty() { 0.0 } else { 14.0 };
            list = list.child(div().width(Length::Fill).padding_left(inset).child(under));
        }
        AnyView::new(list)
    });
    column()
        .grow(1.0)
        .min_height(0.0)
        .child(heading)
        .child(div().padding_x(8.0).padding_bottom(6.0).child(field))
        .child(
            quadra::scroll(signal(0.0), list)
                .container(column().grow(1.0).min_height(0.0).padding_x(6.0).padding_bottom(8.0)),
        )
}

/// A row of the list: 26 px, radius 6, 8 px in, its parts 8 px apart.
fn line() -> quadra::Div {
    row()
        .width(Length::Fill)
        .height(by_text(26.0))
        .align_items(Align::Center)
        .gap(8.0)
        .padding_x(8.0)
        .radius(6.0)
        .text_color(token::TEXT)
        .hover(|s| s.bg(token::PANEL_HOVER))
}

/// The words at a row's right: in mono, 12 px, the third text colour.
fn address(words: String) -> quadra::Div {
    div().font_family(fonts::MONO).text_size(12.0).text_color(token::TEXT_3).child(text(words))
}

/// A host: the square of its colour tag, its name, its address.
fn host_row(model: &Model, host: Host) -> impl View + use<> {
    let (opens, menu) = (model.clone(), model.clone());
    let (tag, color) = model.host_tag(&host);
    let square = div()
        .size(10.0, 10.0)
        .radius(2.0)
        .shrink(0.0)
        .bg(color)
        .role(Role::Image)
        .label(format!("Colour tag: {tag}"));
    let remote = host.remote.clone();
    let face = line()
        .role(Role::Button)
        .label(host.name.clone())
        .on_click(move |_| opens.connect_to(remote.clone(), false))
        .child(square)
        .child(div().grow(1.0).min_width(Length::Px(0.0)).child(text(host.name.clone()).ellipsis()))
        .child(address(short(&host.remote.target)));
    context_menu(face, move || host_menu(&menu, &host))
}

/// The host and its port when it is not 22, as the rail shows it.
pub(crate) fn short(target: &Target) -> String {
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
    tag: Signal<usize>,
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
        tag: signal(0),
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
    // The dialog is 440 wide: its body fits in 416, padding included.
    let body = move || {
        let text_field =
            |value, hint: &str, id: &'static str| input(value).placeholder(hint).test_id(id).width(392.0);
        let key = text_field(form.key, "Path of the key file", "host-key");
        let how = quadra::row().gap(12.0).child(field("Auth", select(AUTHS, form.auth).width(190.0))).child(
            field("Jump host", select(lists.jump_choices(), form.jump).width(190.0).test_id("host-jump")),
        );
        column()
            .gap(12.0)
            .padding_x(12.0)
            .width(416.0)
            .child(field("Name", text_field(form.name, "The host when empty", "host-name")))
            .child(
                quadra::row()
                    .gap(12.0)
                    .child(field(
                        "Group",
                        input(form.group).placeholder("Group").test_id("host-group").width(190.0),
                    ))
                    .child(field("Colour tag", select(TAG_NAMES, form.tag).width(190.0).test_id("host-tag"))),
            )
            .child(field("Target", text_field(form.target, "user@host or host:port", "host-target")))
            .child(how)
            .child(div().display(crate::locked::shown(move || form.auth.get() == 2)).child(key))
            .child(field(
                "Local forwards",
                text_field(form.forwards, "8080:localhost:80 ...", "host-forwards"),
            ))
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
    form.tag.set(model.saved_tag(&host.name).map_or(0, |at| at + 1));
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
    let tag = form.tag.get_untracked().checked_sub(1).map(|at| crate::hosts::TAGS[at].0);
    model.save_host(Host {
        name: name.clone(),
        group: form.group.get_untracked().trim().into(),
        remote,
        jump,
    });
    model.set_host_tag(&name, tag);
    for field in [form.name, form.group, form.target, form.key, form.forwards] {
        field.set(String::new());
    }
    form.jump.set(0);
    form.tag.set(0);
}
