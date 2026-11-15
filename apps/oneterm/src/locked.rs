//! How the lock looks (specs/19, "Lock"): the card in the middle of the window, what the
//! window counts as being used, and the Security & lock page of the settings.

use quadra::{
    Align, Display, Div, Justify, MountCx, NodeId, PointerKind, Signal, View, column, div, effect, signal,
    text,
};
use quadra_design::{SettingsGroup, Variant, button, icon, icons, input, number_input, token};

use crate::lock::Lock;
use crate::model::Model;

/// Shown while `on` says so, else not displayed (and so not in the semantic tree).
pub(crate) fn shown(on: impl Fn() -> bool + 'static) -> impl Fn() -> Display + 'static {
    move || if on() { Display::Flex } else { Display::None }
}

/// Keys, presses and moves anywhere in `root` start the idle time again; they go on.
pub(crate) fn watches(root: Div, lock: Lock) -> Div {
    root.on_mount(move |tree, id| {
        tree.on_key_capture(id, true, move |_| lock.active());
        for kind in [PointerKind::Down, PointerKind::Move, PointerKind::Wheel] {
            tree.on_pointer_capture(id, kind, move |_| lock.active());
        }
    })
}

/// The card of the locked window, the focus in its field whenever it locks.
pub(crate) struct Card(pub Model);

impl View for Card {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let lock = self.0.lock();
        let id = card(&self.0).mount(cx);
        let handle = cx.handle.clone();
        effect(move || {
            if lock.locked.get() {
                handle.with(move |tree| _ = tree.focus_first(id));
            }
        });
        id
    }
}

fn card(model: &Model) -> Div {
    let (lock, tabs, unread) = (model.lock(), model.tabs(), model.unread());
    let why = move || match lock.after.get() {
        0 => "Locked \u{B7} sessions are still running".to_string(),
        n => format!("Locked after {n} min idle \u{B7} sessions are still running"),
    };
    let counts = move || {
        let running = tabs.with(|tabs| {
            tabs.iter()
                .map(|tab| {
                    tab.spaces.with(|s| s.iter().filter(|it| it.session.exited().get().is_none()).count())
                })
                .sum::<usize>()
        });
        format!("{running} sessions connected \u{B7} {} notices", unread.get())
    };
    let field = input(lock.typed)
        .password()
        .placeholder("Master password")
        .test_id("unlock")
        .width(296.0)
        .on_submit(move |_| lock.unlock());
    let card = column()
        .width(360.0)
        .padding(32.0)
        .gap(12.0)
        .align_items(Align::Center)
        .radius(10.0)
        .bg(token::RAISED)
        .border(1.0)
        .border_color(token::LINE_STRONG)
        .elevation(2)
        .focus_scope()
        .child(icon(icons::LOCK).size(28.0).color(token::ACCENT))
        .child(div().font_weight(600).text_size(18.0).child(text("1Term is locked")))
        .child(div().text_color(token::TEXT_2).child(text(why)))
        .child(field)
        .child(
            div()
                .text_color(token::ERROR)
                .display(shown(move || !lock.said.with(String::is_empty)))
                .child(text(move || lock.said.get())),
        )
        .child(
            button("Unlock")
                .variant(Variant::Primary)
                .disabled(move || lock.wait.get() > 0)
                .on_click(move || lock.unlock()),
        )
        .child(div().text_size(12.0).text_color(token::TEXT_3).child(text(counts)));
    column()
        .grow(1.0)
        .align_items(Align::Center)
        .justify(Justify::Center)
        .bg(token::CANVAS)
        .display(shown(move || lock.locked.get()))
        .child(card)
}

/// Security & lock: the master password set, changed or removed, and the idle time.
pub(crate) fn security(lock: Lock, after: Signal<f64>) -> SettingsGroup {
    let (current, new, again, said) =
        (signal(String::new()), signal(String::new()), signal(String::new()), signal(String::new()));
    let field = |value: Signal<String>, hint: &str, id: &'static str| {
        input(value).password().placeholder(hint).test_id(id).width(240.0)
    };
    let set = move || {
        if new.get_untracked() != again.get_untracked() {
            return said.set("The two new passwords differ".into());
        }
        if new.with_untracked(String::is_empty) {
            return said.set("The new password is empty".into());
        }
        lock.set_password(current.get_untracked(), new.get_untracked(), move |words| said.set(words.into()));
        for field in [current, new, again] {
            field.set(String::new());
        }
    };
    let remove = move || {
        lock.set_password(current.get_untracked(), String::new(), move |words| said.set(words.into()));
        current.set(String::new());
    };
    let buttons = quadra::row().gap(8.0).child(button("Set password").on_click(set)).child(
        div().display(shown(move || lock.can_lock())).child(button("Remove password").on_click(remove)),
    );
    SettingsGroup::new("Master password")
        .about("Kept as a salted hash, never the password. Without one the window does not lock.")
        .row(
            "Current password",
            "Asked to change or remove one.",
            field(current, "Current", "password-current"),
        )
        .row("New password", "", field(new, "New", "password-new"))
        .row("Repeat it", "", field(again, "Again", "password-again"))
        .row("", "", buttons)
        .row("", "", div().text_color(token::TEXT_2).child(text(move || said.get())))
        .row(
            "Lock after",
            "Minutes without a key or the pointer, 1 to 480.",
            number_input(after).range(1.0, 480.0),
        )
}
