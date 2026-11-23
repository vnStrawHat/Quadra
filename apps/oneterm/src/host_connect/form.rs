//! The Connect dialog's form (specs/19, "The rail"): what the host is (its address, jump host
//! and host key), then the fields to change for this connection, two to a row.

use quadra::{AnyView, Div, Signal, Track, View, column, div, dynamic, row, text};
use quadra_design::{Tone, alert, checkbox, field, fonts, icon, icons, input, select, size, token};

use super::{AUTHS, Form, Go, KEY, PASSWORD, USER};
use crate::hosts::Host;
use crate::locked::shown;

/// Half the body, a gap of 12 between.
const HALF: f32 = 202.0;
const FULL: f32 = 416.0;

/// The header's colour tag, title and group.
pub(super) fn head(title: String, tag: quadra::Paint, group: String) -> Div {
    let mark = div().size(10.0, 10.0).shrink(0.0).radius(2.0).bg(tag);
    let title = div().font_weight(600).text_size(size::TEXT_LG.size).child(text(title));
    let group = div().text_size(12.0).text_color(token::TEXT_2).child(text(group));
    row()
        .grow(1.0)
        .gap(8.0)
        .align_items(quadra::Align::Center)
        .child(mark)
        .child(title)
        .child(div().grow(1.0))
        .child(group)
}

/// Mono text in a size, in the colour given.
pub(super) fn mono(words: impl Into<String>, px: f32, color: quadra::Token) -> Div {
    div().font_family(fonts::MONO).text_size(px).text_color(color).child(text(words.into()))
}

/// Words under a field, as its help is.
fn hint(words: impl Fn() -> String + 'static) -> Div {
    div().text_size(size::TEXT_SM.size).text_color(token::TEXT_2).child(text(words))
}

fn pair(a: impl View + 'static, b: impl View + 'static) -> Div {
    div().grid([Track::fr(1.0), Track::fr(1.0)]).gap_x(12.0).child(a).child(b)
}

/// Address, jump host and host key: a label of 88 and its value.
fn summary(host: &Host, key: &Option<String>) -> Div {
    let target = &host.remote.target;
    let address = row()
        .gap(8.0)
        .align_items(quadra::Align::Center)
        .child(mono(target.host.clone(), 13.0, token::TEXT))
        .child(div().text_size(12.0).text_color(token::TEXT_2).child(text(format!("port {}", target.port))));
    let jump = match host.jump.as_str() {
        "" => div().text_color(token::TEXT_2).child(text("None")),
        name => div().text_color(token::TEXT).child(text(name.to_string())),
    };
    let (mark, role, state, print) = match key {
        Some(print) => (icons::CIRCLE_CHECK, token::OK, "Known", print.clone()),
        None => (icons::TRIANGLE_ALERT, token::WARN, "New", "Not in known hosts".into()),
    };
    let known = row()
        .gap(6.0)
        .align_items(quadra::Align::Center)
        .child(icon(mark).size(14.0).color(role))
        .child(div().text_color(token::TEXT).child(text(state)))
        .child(mono(print, 12.0, token::TEXT_2));
    let mut grid =
        div().grid([Track::px(88.0), Track::fr(1.0)]).gap_y(4.0).align_items(quadra::Align::Center);
    for (name, value) in [("Address", address), ("Jump host", jump), ("Host key", known)] {
        let label =
            row().height(23.0).align_items(quadra::Align::Center).text_size(12.0).text_color(token::TEXT_2);
        grid = grid.child(label.child(text(name))).child(value);
    }
    grid
}

/// The whole form; the focus starts in `missing`.
pub(super) fn body(
    model: &crate::model::Model,
    host: &Host,
    form: Form,
    missing: u8,
    key_known: &Option<String>,
    go: Go,
    secret_at: &quadra::NodeRef,
) -> Div {
    let saved = model.sealed(&host.name).is_some();
    let entry = |value: Signal<String>, id: &'static str, width: f32| {
        let go = go.clone();
        input(value).test_id(id).width(width).on_submit(move |_| go())
    };
    let focused = |view: Div, at: u8| match missing == at {
        true => view.on_mount(|tree, id| tree.after_layout(move |tree| _ = tree.focus_first(id))),
        false => view,
    };
    let auth = form.auth;
    let saved_auth = form.saved_auth.get_untracked();
    let no_user = host.remote.target.user.is_empty();
    let user = field("Username", entry(form.user, "connect-user", HALF).mono().placeholder("Required"));
    let user = if no_user { user.help("This host has no saved username.") } else { user };
    let how = column()
        .gap(4.0)
        .child(field("Auth method", select(AUTHS, auth).width(HALF).test_id("connect-auth")))
        .child(hint(move || match auth.get() == saved_auth {
            true => "Saved for this host.".into(),
            false => format!("Saved for this host: {}", AUTHS[saved_auth]),
        }));
    let saved_hint = if saved { "Saved" } else { "" };
    let key = crate::sshkeys::picker(model, form.key, HALF, "connect-key");
    let passphrase = entry(form.password, "connect-passphrase", HALF).password().reveal();
    let passphrase = field(
        "Passphrase",
        passphrase.placeholder(if saved { "Saved" } else { "Empty if the key has none" }),
    );
    let passphrase = if saved { passphrase.help("Saved for this host.") } else { passphrase };
    let keys = pair(focused(div().child(field("Key file", key)), KEY), passphrase);
    let password = entry(form.password, "connect-password", FULL).password().reveal().placeholder(saved_hint);
    let password = field("Password", password);
    let password = if saved { password.help("Saved for this host.") } else { password };
    let says = |words: &'static str| div().text_color(token::TEXT_2).child(text(words));
    let failure = dynamic(move || match form.failed.get() {
        why if why.is_empty() => AnyView::new(div()),
        why => AnyView::new(alert(Tone::Danger, "Could not connect", &why)),
    });
    let mut body = column()
        .gap(12.0)
        .width(FULL)
        .child(failure)
        .child(summary(host, key_known))
        .child(pair(focused(div().child(user), USER), how));
    body = body
        .child(div().display(shown(move || auth.get() == 2)).child(keys))
        .child(focused(
            div().node_ref(secret_at).display(shown(move || matches!(auth.get(), 0 | 3))).child(password),
            PASSWORD,
        ))
        .child(
            div()
                .display(shown(move || auth.get() == 0))
                .child(says("Tries ssh-agent and the keys in ~/.ssh, then the password.")),
        )
        .child(
            div()
                .display(shown(move || auth.get() == 1))
                .child(says("Uses the keys loaded in ssh-agent, nothing to type.")),
        );
    if key_known.is_none() {
        body = body.child(trust(form.trust));
    }
    let save = checkbox(form.save).test_id("connect-save").label(move || match auth.get() {
        2 => "Save username, auth method and passphrase to this host".to_string(),
        0 | 3 => "Save username, auth method and password to this host".to_string(),
        _ => "Save username and auth method to this host".to_string(),
    });
    let here = checkbox(form.here).test_id("connect-here").label("Open in the current tab as a new space");
    body.child(column().gap(8.0).child(save).child(here))
}

/// A host key 1Term has not seen: checked with its owner, trusted here.
fn trust(trust: Signal<bool>) -> Div {
    let words = "1Term has not seen this host key before. Check the fingerprint with the host's owner.";
    let note = row()
        .gap(8.0)
        .padding(8.0)
        .radius(token::RADIUS_MD)
        .bg(token::WARN_TINT)
        .border(1.0)
        .border_color(token::WARN.alpha(0.2))
        .child(icon(icons::TRIANGLE_ALERT).size(14.0).color(token::WARN))
        .child(div().grow(1.0).min_width(0.0).text_color(token::TEXT).child(text(words)));
    let tick = checkbox(trust).test_id("connect-trust").label("Trust this key and add it to known hosts");
    column().gap(8.0).child(note).child(tick)
}
