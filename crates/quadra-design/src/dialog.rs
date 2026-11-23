//! The dialog (specs/behaviour/15, "Dialog"): a question the user answers before going on,
//! and the drawer, which is one along an edge of the window.
//! It hangs 80 px from the top of the window, centred, with no backdrop: the cross at the end
//! of its title, Escape or a press outside gives it up.

use std::rc::Rc;

use quadra_headless::dialog as base;
use quadra_layout::{Align, Justify};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::NodeId;
use quadra_view::{AnyView, Div, IntoProp, MountCx, Place, Prop, View, column, div, row, text};

use crate::button::{Variant, button};
use crate::icon::icon;
use crate::popup::{EDGE, popup};
use crate::{icons, size, token};

mod drawer;

pub use drawer::{Drawer, drawer};

type Act = Rc<dyn Fn()>;
type When = Rc<dyn Fn() -> bool>;
/// A row of the app's in the place of the title.
pub(crate) type Head = Rc<dyn Fn() -> AnyView>;

/// A button at the foot: its label and look, what it does, its key, when it is off, whether
/// pressing it leaves the dialog open, and its label while busy.
struct Action {
    label: String,
    variant: Variant,
    act: Act,
    key: Option<&'static str>,
    off: Option<When>,
    stays: bool,
    busy: Option<(String, When)>,
    /// At the start of the foot, apart from the others.
    start: bool,
    /// Shown only while this says so.
    shown: Option<When>,
}

pub struct Dialog<F> {
    open: Signal<bool>,
    title: String,
    body: F,
    actions: Vec<Action>,
    own: Option<Head>,
}

/// A dialog shown while `open` is true: a title over what `body` builds, each time it opens.
pub fn dialog<V, F>(open: Signal<bool>, title: impl Into<String>, body: F) -> Dialog<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    Dialog { open, title: title.into(), body, actions: Vec::new(), own: None }
}

impl<F> Dialog<F> {
    /// A button at the foot of the dialog, from the left to the right in the order given.
    /// Pressing it runs `f` and closes the dialog.
    pub fn action(mut self, label: impl Into<String>, variant: Variant, f: impl Fn() + 'static) -> Self {
        let (label, act) = (label.into(), Rc::new(f));
        let (off, busy, shown) = (None, None, None);
        self.actions.push(Action {
            label,
            variant,
            act,
            key: None,
            off,
            stays: false,
            busy,
            start: false,
            shown,
        });
        self
    }

    /// A quiet button at the start of the foot, apart from the others (`Edit host…`).
    pub fn aside(mut self, label: impl Into<String>, f: impl Fn() + 'static) -> Self {
        self = self.action(label, Variant::Ghost, f);
        if let Some(last) = self.actions.last_mut() {
            last.start = true;
        }
        self
    }

    /// The last action shows only while `shown` says so.
    pub fn shown(mut self, shown: impl Fn() -> bool + 'static) -> Self {
        if let Some(last) = self.actions.last_mut() {
            last.shown = Some(Rc::new(shown));
        }
        self
    }

    /// A row of the app's in the place of the title, before the cross; the title still names
    /// the dialog to a reader.
    pub fn head<H: View + 'static>(mut self, row: impl Fn() -> H + 'static) -> Self {
        self.own = Some(Rc::new(move || AnyView::new(row())));
        self
    }

    /// The key that does what the last action does, shown after its label (`Enter`).
    pub fn shortcut(mut self, key: &'static str) -> Self {
        if let Some(last) = self.actions.last_mut() {
            last.key = Some(key);
        }
        self
    }

    /// The last action is off while `off` says so: it can be neither pressed nor reached.
    pub fn disabled<M>(mut self, off: impl IntoProp<bool, M>) -> Self {
        if let Some(last) = self.actions.last_mut() {
            last.off = Some(match off.into_prop() {
                Prop::Value(off) => Rc::new(move || off),
                Prop::Reactive(read) => Rc::from(read),
            });
        }
        self
    }

    /// Pressing the last action leaves the dialog open: what it starts closes it when done.
    pub fn stays_open(mut self) -> Self {
        if let Some(last) = self.actions.last_mut() {
            last.stays = true;
        }
        self
    }

    /// While `busy` says so, the last action reads `label` and is off (`Connecting…`).
    pub fn busy(mut self, label: impl Into<String>, busy: impl Fn() -> bool + 'static) -> Self {
        if let Some(last) = self.actions.last_mut() {
            last.busy = Some((label.into(), Rc::new(busy)));
        }
        self
    }

    /// The last action destroys something: in the colour of errors.
    pub fn danger(mut self) -> Self {
        if let Some(last) = self.actions.last_mut() {
            last.variant = Variant::Danger;
        }
        self
    }
}

/// The title of a dialog, or the app's row in its place, and the quiet cross at its end that
/// gives the dialog up.
fn head(title: &str, own: Option<AnyView>, open: Signal<bool>) -> Div {
    let cross = row().size(22.0, 22.0).shrink(0.0).align_items(Align::Center).justify(Justify::Center);
    let cross = cross.radius(token::RADIUS_MD).hover(|s| s.bg(token::PANEL_HOVER));
    let cross = base::close(cross.cursor(Cursor::Pointer), open);
    let cross = cross.child(icon(icons::X).size(14.0).color(token::TEXT_2));
    let headline = match own {
        Some(own) => column().grow(1.0).min_width(0.0).align_items(Align::Stretch).child(own),
        None => div().grow(1.0).font_weight(600).child(text(title.to_string()).size(size::TEXT_LG.size)),
    };
    let head = row().align_items(Align::Center).gap(4.0).padding_x(12.0).padding_top(8.0);
    head.padding_bottom(4.0).child(headline).child(cross)
}

/// A question with two answers: `yes` does `on_yes`, Cancel does nothing, and both close the
/// dialog.
pub fn confirm(
    open: Signal<bool>,
    title: impl Into<String>,
    message: impl Into<String>,
    yes: impl Into<String>,
    on_yes: impl Fn() + 'static,
) -> Dialog<Box<dyn Fn() -> AnyView>> {
    let message = message.into();
    let ask = dialog(
        open,
        title,
        Box::new(move || AnyView::new(text(message.clone()))) as Box<dyn Fn() -> AnyView>,
    );
    ask.action("Cancel", Variant::Secondary, || {}).action(yes, Variant::Primary, on_yes)
}

impl<V, F> View for Dialog<F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (open, title, body, actions) = (self.open, self.title, self.body, Rc::new(self.actions));
        let own = self.own;
        let content = move || {
            let head = head(&title, own.as_ref().map(|own| own()), open);
            let middle = div().padding_x(12.0).padding_y(8.0).text_color(token::TEXT_2).child(body());
            let page = base::page(column().width(440.0 - EDGE), title.clone());
            let page = page.child(head).child(middle);
            if actions.is_empty() {
                return page;
            }
            page.child(foot(open, &actions))
        };
        popup(open, (Place::Top(80.0), 2), None, content).mount(cx)
    }
}

/// The seam and the buttons: those at the start, room, then the others at the end.
fn foot(open: Signal<bool>, actions: &[Action]) -> Div {
    let mut foot =
        row().gap(4.0).padding(8.0).align_items(Align::Center).border_top(1.0).border_color(token::LINE);
    let starts = actions.iter().filter(|it| it.start);
    let ends = actions.iter().filter(|it| !it.start);
    for at in starts {
        foot = foot.child(face(open, at));
    }
    foot = foot.child(div().grow(1.0));
    for at in ends {
        foot = foot.child(face(open, at));
    }
    foot
}

/// One button of the foot.
fn face(open: Signal<bool>, at: &Action) -> Div {
    let press: Rc<dyn Fn()> = match at.stays {
        true => at.act.clone(),
        false => Rc::new(base::answer(open, at.act.clone())),
    };
    let face = match at.busy.clone() {
        Some((busy_label, busy)) => {
            let (label, busy) = (at.label.clone(), busy.clone());
            let says = Rc::new(move || if busy() { busy_label.clone() } else { label.clone() });
            // The label alone names it, not the key after it.
            let names = says.clone();
            button(move || says()).label(move || names())
        }
        None => button(at.label.clone()),
    };
    let face = face.variant(at.variant).on_click(move || press());
    let face = match at.key {
        Some(key) => face.shortcut(key),
        None => face,
    };
    let (off, busy) = (at.off.clone(), at.busy.clone().map(|it| it.1));
    let face = match (off, busy) {
        (None, None) => face,
        (off, busy) => {
            face.disabled(move || off.as_ref().is_some_and(|it| it()) || busy.as_ref().is_some_and(|it| it()))
        }
    };
    let shown = at.shown.clone();
    let display = move || match shown.as_ref().is_none_or(|it| it()) {
        true => quadra_layout::Display::Flex,
        false => quadra_layout::Display::None,
    };
    div().display(display).child(face)
}
