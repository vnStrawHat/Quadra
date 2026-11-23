//! What the host editor shows (specs/19, "M8 in detail"), two fields to a row: name; group and
//! colour tag; address and port; username and jump host; auth and its password or passphrase,
//! the key file; the master password when one is needed; startup command and forwards.

use quadra::{AnyView, Color, Div, Paint, Signal, Track, View, column, div, dynamic, row, text};
use quadra_design::{Tone, alert, field, input, select, size, token};

use super::AUTHS;
use super::form::Form;
use crate::hosts::tag::{tag_of_group, tag_picker};
use crate::locked::shown;
use crate::model::Model;

const H: f32 = size::H_MD;
const HALF: f32 = 202.0;
const FULL: f32 = 416.0;

fn pair(a: impl View + 'static, b: impl View + 'static) -> Div {
    div().grid([Track::fr(1.0), Track::fr(1.0)]).gap_x(12.0).child(a).child(b)
}

/// The header: the tag the host will have, and the title.
pub(super) fn head(form: Form, title: String) -> Div {
    let mark = dynamic(move || {
        let paint: Paint = match Color::parse_hex(form.tag.get().trim()) {
            Some(color) => color.into(),
            None => tag_of_group(form.group.get().trim()).1.into(),
        };
        AnyView::new(div().size(10.0, 10.0).radius(2.0).bg(paint))
    });
    let title = div().font_weight(600).text_size(size::TEXT_LG.size).child(text(title));
    row().grow(1.0).gap(8.0).align_items(quadra::Align::Center).child(mark).child(title)
}

/// The form; `confirming` puts the question of deleting it at its end.
pub(super) fn body(model: &Model, form: Form, editing: Option<String>, confirming: Signal<bool>) -> Div {
    let text_field = |value, hint: &str, id: &'static str, width: f32| {
        input(value).placeholder(hint).test_id(id).width(width).height(H)
    };
    let auth = form.auth;
    let address = div()
        .grid([Track::fr(1.0), Track::px(88.0)])
        .gap_x(12.0)
        .child(field("Address", text_field(form.target, "Host name or IP", "host-target", 316.0).mono()))
        .child(field("Port", text_field(form.port, "22", "host-port", 88.0).mono()));
    let user =
        field("Username", text_field(form.user, "Ask when connecting", "host-user", HALF)).help("Optional.");
    let jump = field("Jump host", select(model.jump_choices(), form.jump).width(HALF).test_id("host-jump"));
    let saved = editing.as_ref().and_then(|it| model.sealed(it)).is_some();
    let secret = dynamic(move || {
        let label = if auth.get() == 2 { "Passphrase" } else { "Password" };
        let entry = input(form.secret).password().reveal().placeholder(if saved { "Saved" } else { "" });
        AnyView::new(field(label, entry.test_id("host-secret").width(HALF).height(H)))
    });
    let secret = div().display(shown(move || matches!(auth.get(), 2 | 3))).child(secret);
    let hint =
        div().text_size(size::TEXT_SM.size).text_color(token::TEXT_2).child(text(move || match auth.get() {
            0 => "Tries the keys in ssh-agent and ~/.ssh, then asks for a password.",
            1 => "Uses the keys loaded in ssh-agent.",
            2 => "The passphrase is optional. Left empty, 1Term asks for it when you connect.",
            _ => "The password is optional. Left empty, 1Term asks for it when you connect.",
        }));
    let how = pair(field("Auth method", select(AUTHS, auth).width(HALF)), secret);
    let name = editing.clone().unwrap_or_default();
    let question =
        alert(Tone::Danger, &format!("Delete {name}?"), "Its saved password and startup command go too.");
    column()
        .gap(12.0)
        .width(FULL)
        .child(field("Name", text_field(form.name, "The host when empty", "host-name", FULL)))
        .child(pair(
            field("Group", text_field(form.group, "Group", "host-group", HALF)),
            field("Colour tag", tag_picker(form.tag, form.group)),
        ))
        .child(address)
        .child(pair(user, jump))
        .child(column().gap(4.0).child(how).child(hint))
        .child(
            div()
                .display(shown(move || auth.get() == 2))
                .child(field("Key file", crate::sshkeys::picker(model, form.key, FULL, "host-key"))),
        )
        .child(super::secret::fields(model, form))
        .child(field(
            "Startup command",
            text_field(form.startup, "Written once the shell is open", "host-startup", FULL),
        ))
        .child(field(
            "Forwards",
            text_field(form.forwards, "8080:localhost:80 R9000:localhost:3000 D1080", "host-forwards", FULL),
        ))
        .child(div().display(shown(move || confirming.get())).child(question))
}
