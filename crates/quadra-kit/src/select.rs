//! The select (specs/behaviour/15, "Select"): a button that shows which of a few options is
//! chosen, and a menu of them with a check on that one.

use std::rc::Rc;

use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, signal};
use quadra_tree::{NodeId, Role};
use quadra_ui::{MountCx, NodeRef, View, column, div, row, text};

use crate::action::{CONTROL, Confirm};
use crate::icon::icon;
use crate::menu::{MenuItem, menu};
use crate::{icons, size, token};

pub struct Select {
    options: Rc<Vec<String>>,
    chosen: Signal<usize>,
    width: f32,
    test_id: Option<&'static str>,
}

/// A dropdown of `options`: `chosen` holds the index of the one that is chosen.
pub fn select<S: Into<String>>(options: impl IntoIterator<Item = S>, chosen: Signal<usize>) -> Select {
    let options = Rc::new(options.into_iter().map(Into::into).collect());
    Select { options, chosen, width: 160.0, test_id: None }
}

impl Select {
    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }
}

impl View for Select {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (chosen, open, opener) = (self.chosen, signal(false), NodeRef::new());
        let options = self.options.clone();
        let value = move || options.get(chosen.get()).cloned().unwrap_or_default();
        // Filled, with a hairline border, 28 px high; the border is where the focus shows.
        let face = row().width(self.width).height(size::H_MD).padding_x(8.0).gap(8.0).shrink(0.0);
        let face = face.align_items(Align::Center).radius(size::RADIUS_MD).bg(token::RAISED);
        let face = face.border(1.0).border_color(token::LINE).hover(|s| s.bg(token::RAISED_HOVER));
        let face = face.focus_visible(|s| s.border_color(token::BORDER_FOCUSED));
        let face = face.role(Role::Button).expanded(open).focusable().key_context(CONTROL);
        let face = face.cursor(Cursor::Pointer).node_ref(&opener);
        let face = face.on_click(move |_| open.update(|open| *open = !*open));
        let face = face.on_action::<Confirm>(move |_| open.set(true));
        let face = match self.test_id {
            Some(id) => face.test_id(id),
            None => face,
        };
        let face = face.child(div().grow(1.0).child(text(value)));
        let face = face.child(icon(icons::CHEVRONS_UP_DOWN).size(12.0).color(token::TEXT_2));
        let options = self.options;
        let items = move || {
            let on = chosen.get_untracked();
            let item = |(i, name): (usize, &String)| {
                MenuItem::new(name.clone()).checked(i == on).on_select(move || chosen.set(i))
            };
            options.iter().enumerate().map(item).collect()
        };
        column().child(face).child(menu(open, &opener, items)).mount(cx)
    }
}
