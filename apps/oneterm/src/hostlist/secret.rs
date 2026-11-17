//! A host's password or key passphrase in its dialog (specs/19, "Saved passwords"): saved
//! sealed with the master password, which is asked for while its key is not in memory, or set
//! there when there is none.

use quadra::{View, column, div};
use quadra_design::{Notice, Tone, field, input};

use super::form::Form;
use crate::locked::shown;
use crate::model::Model;

const W: f32 = 392.0;

/// The password (auth Password) or the key's passphrase (Key file), while that auth is chosen;
/// under it the master password while its key is not in memory, or a new one and again while
/// there is none.
pub(super) fn fields(model: &Model, form: Form, editing: Option<String>) -> impl View + use<> {
    let lock = model.lock();
    let saved = editing.and_then(|it| model.sealed(&it)).is_some();
    let h = quadra_design::size::H_MD;
    let secret = input(form.secret)
        .password()
        .placeholder(if saved { "Saved" } else { "" })
        .test_id("host-secret")
        .width(W)
        .height(h);
    let typed = move || !form.secret.with(String::is_empty);
    let opens = move || typed() && lock.can_lock() && lock.key.get().is_none();
    let sets = move || typed() && !lock.can_lock();
    let master = |id: &'static str, value| input(value).password().test_id(id).width(W).height(h);
    column()
        .gap(12.0)
        .display(shown(move || matches!(form.auth.get(), 2 | 3)))
        .child(field("Password or passphrase", secret).help("Saved sealed with the master password."))
        .child(
            div().display(shown(opens)).child(field("Master password", master("host-master", form.master))),
        )
        .child(
            column()
                .gap(12.0)
                .display(shown(sets))
                .child(
                    field("New master password", master("host-new-master", form.master))
                        .help("1Term has none yet: it also locks the window (Security & lock)."),
                )
                .child(field("Repeat it", master("host-master-again", form.again))),
        )
}

/// Saves the secret typed for a host, sealed for its target `bound`; with none typed the one
/// `kept` stays, sealed again when the target changed (forgotten, with a card, while the data
/// key is not in memory). Another auth than Password and Key file saves none. Without a master
/// password the one typed is set first.
pub(super) fn save(model: &Model, form: Form, name: &str, bound: &str, kept: Option<(String, String)>) {
    let typed = quadra_shell::Zeroizing::new(form.secret.get_untracked());
    if !matches!(form.auth.get_untracked(), 2 | 3) {
        return model.set_sealed(name, None);
    }
    if typed.is_empty() {
        let Some((sealed, was)) = kept else { return model.set_sealed(name, None) };
        if was == bound {
            return model.set_sealed(name, Some(sealed));
        }
        let key = model.lock().key.get_untracked();
        let moved = key.and_then(|key| {
            let secret = crate::vault::open_secret(&key, &sealed, &was)?;
            crate::vault::seal_secret(&key, secret.as_str(), bound)
        });
        if moved.is_none() {
            model.notify(
                Notice::new("Edit SSH host")
                    .body("The target changed: its saved password was forgotten.")
                    .tone(Tone::Warning),
            );
        }
        return model.set_sealed(name, moved);
    }
    let (master, again) =
        (form.master.get_untracked(), quadra_shell::Zeroizing::new(form.again.get_untracked()));
    let (lock, model, name, bound) = (model.lock(), model.clone(), name.to_string(), bound.to_string());
    let sealing = move |model: &Model, why: &str| match model.lock().key.get_untracked() {
        Some(key) => model.set_sealed(&name, crate::vault::seal_secret(&key, &typed, &bound)),
        None => model.notify(
            Notice::new("Add SSH host")
                .body(format!("{why}: the password was not saved."))
                .tone(Tone::Warning),
        ),
    };
    if lock.key.get_untracked().is_some() {
        return sealing(&model, "");
    }
    if lock.can_lock() {
        let opens = model.clone();
        return lock.open(master, move |_| sealing(&opens, "Wrong master password"));
    }
    if master.is_empty() || master != *again {
        let why = if master.is_empty() { "No master password" } else { "The master passwords differ" };
        return sealing(&model, why);
    }
    let sets = model.clone();
    lock.set_password(String::new(), master, move |said| sealing(&sets, said));
}
