//! Quick connect (specs/19, "M7 in detail"): a target as one types it to `ssh`, how to sign
//! in and through which host, whether the SFTP dock opens with it, whether it is saved, and
//! whether it opens a tab or a space beside the focused one.

use oneterm_ssh::Target;
use quadra::{Align, Display, Signal, View, column, div, row, signal, text};
use quadra_design::{Notice, Tone, Variant, checkbox, field, input, select, token};

use crate::model::Model;
use crate::shells::Remote;

const AUTHS: [&str; 4] = ["Auto", "Agent (ssh-agent)", "Key file", "Password"];

#[derive(Clone, Copy)]
struct Form {
    target: Signal<String>,
    auth: Signal<usize>,
    key: Signal<String>,
    here: Signal<bool>,
    save: Signal<bool>,
    jump: Signal<usize>,
    /// Opens the SFTP dock with it.
    sftp: Signal<bool>,
}

pub(crate) fn dialog(model: &Model) -> impl View + use<> {
    let open = model.ui().connecting;
    let form = Form {
        target: signal(String::new()),
        auth: signal(0),
        key: signal(String::new()),
        here: signal(false),
        save: signal(false),
        jump: signal(0),
        sftp: signal(true),
    };
    let (submits, goes, lists) = (model.clone(), model.clone(), model.clone());
    let body = move || {
        let submits = submits.clone();
        let typed = input(form.target)
            .placeholder("user@host, host:port, or an ssh command line")
            .test_id("connect-target")
            .width(370.0)
            .on_submit(move |_| {
                if go(&submits, form) {
                    open.set(false);
                }
            });
        let key = input(form.key).placeholder("Path of the key file").test_id("connect-key").width(416.0);
        let note = div()
            .text_size(11.0)
            .text_color(token::TEXT_2)
            .child(text("Not saved to hosts unless you tick it"));
        let target = field("Target", row_of(text("ssh"), typed))
            .help("Accepts user@host, host:port, or a full ssh command line.");
        let how = row().gap(12.0).child(field("Auth", select(AUTHS, form.auth).width(202.0))).child(field(
            "Jump host",
            select(lists.jump_choices(), form.jump).width(202.0).test_id("connect-jump"),
        ));
        let boxes = row()
            .gap(16.0)
            .align_items(Align::Center)
            .child(checkbox(form.sftp).label("Open SFTP alongside"))
            .child(checkbox(form.save).label("Save as host"));
        let boxes =
            column().gap(10.0).child(boxes).child(checkbox(form.here).label("In current tab (new space)"));
        column()
            .gap(14.0)
            .padding_x(12.0)
            .width(416.0)
            .child(note)
            .child(target)
            .child(how)
            .child(
                div()
                    .display(move || if form.auth.get() == 2 { Display::Flex } else { Display::None })
                    .child(key),
            )
            .child(boxes)
    };
    quadra_design::dialog(open, "Quick connect", body).action("Cancel", Variant::Secondary, || {}).action(
        "Connect",
        Variant::Primary,
        move || _ = go(&goes, form),
    )
}

fn row_of(lead: impl View + 'static, field: impl View + 'static) -> quadra::Div {
    let lead = div().text_color(token::TEXT_2).font_family(quadra_design::fonts::MONO).child(lead);
    quadra::row().gap(8.0).align_items(quadra::Align::Center).child(lead).child(div().grow(1.0).child(field))
}

/// Connects as the form says; false (and a card) when the target does not read.
fn go(model: &Model, form: Form) -> bool {
    let typed = form.target.get_untracked();
    let Some(target) = Target::parse(&typed) else {
        model.notify(Notice::new("Quick connect").body(format!("Not a target: {typed}")).tone(Tone::Warning));
        return false;
    };
    let auth = crate::hosts::auth_of(form.auth.get_untracked(), form.key.get_untracked());
    let mut remote = Remote::new(target, auth);
    let jump = model.jump_host(form.jump.get_untracked());
    remote.jumps = jump.as_ref().map_or(Vec::new(), |it| it.remote.route());
    if form.save.get_untracked() {
        let (name, jump) = (remote.target.host.clone(), jump.map_or(String::new(), |it| it.name));
        model.save_host(crate::hosts::Host { name, group: String::new(), remote: remote.clone(), jump });
    }
    model.connect_to(remote, form.here.get_untracked());
    if form.sftp.get_untracked() {
        model.ui().dock.set(true);
    }
    form.target.set(String::new());
    true
}
