//! A saved host's Connect dialog (specs/19, "The rail"): `Connect to <name>`, what is saved
//! in fields to change for this connection; the tab, or the space, opens only once the shell
//! is open, a failure is an alert in the dialog.

mod username;

use oneterm_ssh::Ask;
use quadra::{AnyView, NodeRef, Signal, View, column, div, dynamic, effect, signal, text, untrack};
use quadra_design::{Tone, Variant, alert, field, input, select};
use username::overridden;

use crate::hosts::Host;
use crate::locked::shown;
use crate::model::{Model, Space};
use crate::remote::{Conn, Phase};
use crate::vault::Secret;

const AUTHS: [&str; 4] = ["Auto", "ssh-agent", "Key file", "Password"];
/// The field the focus starts in, the first missing.
const USER: u8 = 1;
const KEY: u8 = 2;
const PASSWORD: u8 = 3;

/// The saved host whose dialog shows, and the space connecting from it.
#[derive(Clone, Copy)]
pub struct Opening {
    /// Its name, and whether it opens a space beside the focused one.
    pub host: Signal<Option<(String, bool)>>,
    /// Placed once its shell opens, ended when it fails or is given up.
    pub space: Signal<Option<Space>>,
}

impl Opening {
    pub(crate) fn new() -> Opening {
        Opening { host: signal(None), space: signal(None) }
    }
}

#[derive(Clone, Copy)]
struct Form {
    user: Signal<String>,
    auth: Signal<usize>,
    key: Signal<String>,
    password: Signal<String>,
    /// Why the last try failed, for the alert.
    failed: Signal<String>,
    /// The password was refused: the focus goes to it.
    refused: Signal<bool>,
}

impl Form {
    fn fill(&self, host: &Host) {
        let (auth, key) = match &host.remote.auth {
            oneterm_ssh::Auth::Agent => (1, String::new()),
            oneterm_ssh::Auth::Key(path) => (2, path.display().to_string()),
            oneterm_ssh::Auth::Password => (3, String::new()),
            oneterm_ssh::Auth::Auto => (0, String::new()),
        };
        self.auth.set(auth);
        self.key.set(key);
        self.failed.set(String::new());
        self.user.set(host.remote.target.user.clone());
        crate::vault::wipe(self.password);
    }
}

pub(crate) fn dialog(model: &Model) -> impl View + use<> {
    let (opening, open) = (model.ui().opening, signal(false));
    let form = Form {
        user: signal(String::new()),
        auth: signal(0),
        key: signal(String::new()),
        password: signal(String::new()),
        failed: signal(String::new()),
        refused: signal(false),
    };
    watch(model, form);
    effect(move || {
        let has = opening.host.with(Option::is_some);
        if open.get_untracked() != has {
            open.set(has);
        }
    });
    // Given up (Cancel, the cross, Escape): the connecting too.
    effect(move || {
        if open.get() {
            return;
        }
        untrack(|| {
            if opening.host.with(Option::is_some) {
                opening.host.set(None);
            }
            if let Some(space) = opening.space.get() {
                opening.space.set(None);
                crate::split::end(space);
            }
        });
    });
    let model = model.clone();
    dynamic(move || {
        let Some((name, here)) = opening.host.get() else { return AnyView::new(div()) };
        let Some(host) = untrack(|| model.hosts()).into_iter().find(|it| it.name == name) else {
            return AnyView::new(div());
        };
        untrack(|| form.fill(&host));
        let saved = untrack(|| model.sealed(&name).is_some());
        let missing = match form.auth.get_untracked() {
            _ if host.remote.target.user.is_empty() => USER,
            2 if form.key.with_untracked(String::is_empty) => KEY,
            3 if !saved => PASSWORD,
            _ => 0,
        };
        let (goes, submits) = (model.clone(), model.clone());
        let named = name.clone();
        let go = std::rc::Rc::new(move || connect(&submits, &named, here, form));
        let secret_at = NodeRef::new();
        let refocus = secret_at.clone();
        effect(move || {
            if form.refused.get() {
                refocus.focus_first();
                form.refused.set(false);
            }
        });
        let body = move || body(&host, form, missing, saved, go.clone(), secret_at.clone());
        let busy = move || opening.space.with(Option::is_some);
        let named = name.clone();
        let dialog = quadra_design::dialog(open, format!("Connect to {name}"), body)
            .action("Cancel", Variant::Secondary, || {})
            .action("Connect", Variant::Primary, move || connect(&goes, &named, here, form))
            .shortcut("Enter")
            .stays_open()
            .busy("Connecting\u{2026}", busy);
        AnyView::new(dialog)
    })
}

/// The fields: the user, the auth, the key file, the password (or passphrase); the alert of
/// the last failure over them. The focus is in `missing`.
fn body(
    host: &Host,
    form: Form,
    missing: u8,
    saved: bool,
    go: std::rc::Rc<dyn Fn()>,
    secret_at: NodeRef,
) -> quadra::Div {
    let entry = |value: Signal<String>, id: &'static str| {
        let go = go.clone();
        input(value).test_id(id).width(414.0).on_submit(move |_| go())
    };
    let focused = |view: quadra::Div, at: u8| match missing == at {
        true => view.on_mount(|tree, id| tree.after_layout(move |tree| _ = tree.focus_first(id))),
        false => view,
    };
    let auth = form.auth;
    let user = entry(form.user, "connect-user").mono().placeholder("user, or user@host:port");
    let key = entry(form.key, "connect-key").placeholder("Path of the key file");
    let hint = if saved { "Saved" } else { "" };
    let password = entry(form.password, "connect-password").password().placeholder(hint);
    let passphrase = entry(form.password, "connect-passphrase").password().placeholder(hint);
    let target = &host.remote.target;
    let mut place = format!("{}:{}", target.host, target.port);
    if !host.jump.is_empty() {
        place.push_str(&format!(" \u{B7} through {}", host.jump));
    }
    let failure = dynamic(move || match form.failed.get() {
        why if why.is_empty() => AnyView::new(div()),
        why => AnyView::new(alert(Tone::Danger, "Could not connect", &why)),
    });
    let secrets = div()
        .node_ref(&secret_at)
        .child(
            div()
                .display(shown(move || auth.get() != 1 && auth.get() != 2))
                .child(field("Password", password)),
        )
        .child(div().display(shown(move || auth.get() == 2)).child(field("Passphrase", passphrase)));
    column()
        .gap(12.0)
        .width(414.0)
        .child(failure)
        .child(text(place))
        .child(focused(div().child(field("Username", user)), USER))
        .child(field("Auth", select(AUTHS, form.auth).width(201.0)))
        .child(focused(div().display(shown(move || auth.get() == 2)).child(field("Key file", key)), KEY))
        .child(focused(secrets, PASSWORD))
}

/// Connects as the form says, unless it already does.
fn connect(model: &Model, name: &str, here: bool, form: Form) {
    let opening = model.ui().opening;
    if opening.space.with_untracked(Option::is_some) {
        return;
    }
    let Some(host) = model.hosts().into_iter().find(|it| it.name == name) else { return };
    form.failed.set(String::new());
    let mut remote = host.remote.clone();
    remote.target = overridden(&remote.target, &form.user.get_untracked());
    remote.auth = crate::hosts::auth_of(form.auth.get_untracked(), form.key.get_untracked());
    let typed = form.password.get_untracked();
    crate::vault::wipe(form.password);
    // The password typed, else the saved one (bound to the saved target), else the empty one.
    let secret = match typed.is_empty() && remote.target == host.remote.target && model.sealed(name).is_some()
    {
        false => Secret::new(typed),
        true => match model.saved_secret(&host.remote) {
            Some(secret) => secret,
            // ponytail: a locked vault asks the master password and connects as before (the tab
            // at once); through this dialog when that matters.
            None if model.lock().can_lock() => {
                opening.host.set(None);
                return model.connect_all(vec![remote], here);
            }
            None => Secret::new(String::new()),
        },
    };
    let launch = crate::shells::Launch {
        ssh: Some(remote),
        secret: Some(secret),
        startup: model.startup(name),
        ..model.launch(None)
    };
    let space = model.space(launch);
    match space.session.kept::<Conn>() {
        Some(_) => opening.space.set(Some(space)),
        // A session that does not connect (a test's): placed at once.
        None => {
            opening.host.set(None);
            model.place(space, here);
        }
    }
}

/// Follows the space connecting: placed once its shell opens; a failure, or the password
/// refused, ends it and says why.
fn watch(model: &Model, form: Form) {
    let (opening, model) = (model.ui().opening, model.clone());
    effect(move || {
        let Some(space) = opening.space.get() else { return };
        let Some(conn) = space.session.kept::<Conn>() else { return };
        let phase = conn.phase.get();
        untrack(|| {
            let why = match phase {
                Phase::Done if space.session.exited().get().is_none() => {
                    let here = opening.host.get().is_some_and(|it| it.1);
                    opening.space.set(None);
                    opening.host.set(None);
                    return model.place(space, here);
                }
                Phase::Ask(Ask::Failed { reason }) => crate::asking::sentence(&reason),
                Phase::Ask(Ask::Password { again: true, .. }) => {
                    form.refused.set(true);
                    "Permission denied: the password was refused.".into()
                }
                _ => return,
            };
            opening.space.set(None);
            crate::split::end(space);
            form.failed.set(why);
        });
    });
}

impl Model {
    /// A space connected: in a tab of its own, or beside the focused one; its target recent.
    pub(crate) fn place(&self, space: Space, here: bool) {
        let label = space.remote.as_ref().map(|it| it.target.label());
        match self.active_tab().filter(|_| here) {
            Some(tab) => self.open_space(tab, space),
            None => self.insert_tab(usize::MAX, space),
        }
        if let Some(label) = label {
            self.remember(label);
        }
    }
}
