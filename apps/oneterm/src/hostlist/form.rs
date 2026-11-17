//! The dialogs that save a host (specs/19, "M8 in detail"): Add SSH host and Edit SSH host,
//! with the password or passphrase saved sealed (`vault.rs`).

use oneterm_ssh::Target;
use quadra::{Signal, View, column, div, effect, signal, untrack};
use quadra_design::{Notice, Tone, Variant, field, input, select};

use super::AUTHS;
use crate::hosts::tag::{picked, tag_picker, word_of};
use crate::hosts::{Host, auth_of};
use crate::model::Model;
use crate::shells::Remote;

#[derive(Clone, Copy)]
struct Form {
    name: Signal<String>,
    group: Signal<String>,
    target: Signal<String>,
    auth: Signal<usize>,
    key: Signal<String>,
    jump: Signal<usize>,
    forwards: Signal<String>,
    /// What the colour picker holds: `#rrggbb`, or empty for the group's.
    tag: Signal<String>,
    /// The password or passphrase to save, and the master password that seals it while the
    /// data key is not in memory.
    secret: Signal<String>,
    master: Signal<String>,
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
        tag: s(),
        secret: s(),
        master: s(),
    };
    let (saves, changes, lists, fills) = (model.clone(), model.clone(), model.clone(), model.clone());
    let secrets = model.clone();
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
    // The dialog is 440 wide: its body fits in 416, padding included. Its fields are as high
    // as its selects and the colour picker.
    const H: f32 = quadra_design::size::H_MD;
    let body = move || {
        let text_field = |value, hint: &str, id: &'static str| {
            input(value).placeholder(hint).test_id(id).width(392.0).height(H)
        };
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
                        input(form.group).placeholder("Group").test_id("host-group").width(190.0).height(H),
                    ))
                    .child(field("Colour tag", tag_picker(form.tag, form.group))),
            )
            .child(field("Target", text_field(form.target, "user@host or host:port", "host-target")))
            .child(how)
            .child(div().display(crate::locked::shown(move || form.auth.get() == 2)).child(key))
            .child(secret_fields(&secrets, form, editing.get_untracked()))
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
    form.tag.set(model.saved_tag(&host.name).map_or_else(String::new, |(word, _)| picked(&word)));
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
    // The secret saved for the host edited stays unless another is typed.
    let kept = was.as_ref().and_then(|it| model.sealed(it));
    if let Some(was) = was.filter(|it| *it != name) {
        model.remove_host(&was);
    }
    let tag = form.tag.with_untracked(|it| word_of(it));
    model.save_host(Host {
        name: name.clone(),
        group: form.group.get_untracked().trim().into(),
        remote,
        jump,
    });
    model.set_host_tag(&name, tag.as_deref());
    save_secret(model, form, &name, kept);
    for field in
        [form.name, form.group, form.target, form.key, form.forwards, form.tag, form.secret, form.master]
    {
        field.set(String::new());
    }
    form.jump.set(0);
}

/// The password (auth Password) or the key's passphrase (Key file) to save, while that auth is
/// chosen: disabled with no master password; with the master password's field under it while
/// the data key is not in memory.
fn secret_fields(model: &Model, form: Form, editing: Option<String>) -> quadra::Div {
    let lock = model.lock();
    let (can, key) = (lock.can_lock(), lock.key);
    let saved = editing.and_then(|it| model.sealed(&it)).is_some();
    let secret = input(form.secret)
        .password()
        .placeholder(if saved { "Saved" } else { "" })
        .disabled(!can)
        .test_id("host-secret")
        .width(392.0)
        .height(quadra_design::size::H_MD);
    let master =
        input(form.master).password().test_id("host-master").width(392.0).height(quadra_design::size::H_MD);
    let note = match can {
        true => "Saved sealed with the master password.",
        false => "Set a master password in Settings, Security & lock, to save it.",
    };
    column()
        .gap(12.0)
        .display(crate::locked::shown(move || matches!(form.auth.get(), 2 | 3)))
        .child(field("Password or passphrase", secret).help(note))
        .child(
            div()
                .display(crate::locked::shown(move || can && key.get().is_none()))
                .child(field("Master password", master)),
        )
}

/// Saves the secret typed for a host, sealed; with none typed the one `kept` stays. Another
/// auth than Password and Key file saves none.
fn save_secret(model: &Model, form: Form, name: &str, kept: Option<String>) {
    let typed = form.secret.get_untracked();
    if !matches!(form.auth.get_untracked(), 2 | 3) {
        return model.set_sealed(name, None);
    }
    if typed.is_empty() {
        return model.set_sealed(name, kept);
    }
    let (model, name) = (model.clone(), name.to_string());
    let sealing = move |model: &Model| match model.lock().key.get_untracked() {
        Some(key) => model.set_sealed(&name, crate::vault::seal_secret(&key, &typed)),
        None => model.notify(
            Notice::new("Add SSH host")
                .body("Wrong master password: the password was not saved.")
                .tone(Tone::Warning),
        ),
    };
    if model.lock().key.get_untracked().is_some() || !model.lock().can_lock() {
        return sealing(&model);
    }
    let opens = model.clone();
    model.lock().open(form.master.get_untracked(), move |_| sealing(&opens));
}
