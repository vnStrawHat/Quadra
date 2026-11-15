//! Quick connect (specs/19, "M7 in detail"): a target as one types it to `ssh`, how to sign
//! in, and whether it opens a tab or a space beside the focused one.

use std::path::PathBuf;

use oneterm_ssh::{Auth, Target};
use quadra::{Display, Signal, View, column, div, signal, text};
use quadra_design::{Notice, Tone, Variant, checkbox, input, select, token};

use crate::model::Model;
use crate::shells::{Launch, Remote};

const AUTHS: [&str; 4] = ["Auto (agent, keys, password)", "Agent (ssh-agent)", "Key file", "Password"];

#[derive(Clone, Copy)]
struct Form {
    target: Signal<String>,
    auth: Signal<usize>,
    key: Signal<String>,
    here: Signal<bool>,
}

pub(crate) fn dialog(model: &Model) -> impl View + use<> {
    let open = model.connecting();
    let form = Form {
        target: signal(String::new()),
        auth: signal(0),
        key: signal(String::new()),
        here: signal(false),
    };
    let (submits, goes) = (model.clone(), model.clone());
    let body = move || {
        let submits = submits.clone();
        let field = input(form.target)
            .placeholder("user@host, host:port, or an ssh command line")
            .test_id("connect-target")
            .width(370.0)
            .on_submit(move |_| {
                if go(&submits, form) {
                    open.set(false);
                }
            });
        let key = input(form.key).placeholder("Path of the key file").test_id("connect-key").width(416.0);
        column()
            .gap(10.0)
            .padding_x(12.0)
            .width(440.0)
            .child(div().text_size(12.0).text_color(token::TEXT_2).child(text("Target")))
            .child(row_of(text("ssh"), field))
            .child(select(AUTHS, form.auth).width(240.0))
            .child(
                div()
                    .display(move || if form.auth.get() == 2 { Display::Flex } else { Display::None })
                    .child(key),
            )
            .child(checkbox(form.here).label("In current tab (new space)"))
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
    let auth = match form.auth.get_untracked() {
        1 => Auth::Agent,
        2 => Auth::Key(PathBuf::from(form.key.get_untracked())),
        3 => Auth::Password,
        _ => Auth::Auto,
    };
    let launch = Launch { ssh: Some(Remote { target, auth }), ..model.launch(None) };
    if form.here.get_untracked() {
        model.split_with(launch)
    } else {
        model.new_tab_with(launch)
    }
    form.target.set(String::new());
    true
}
