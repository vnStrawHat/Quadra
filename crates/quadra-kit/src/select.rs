//! The select (specs/behaviour/15, "Select"): a button that shows which of a few options is
//! chosen, and a menu of them with a check on that one; or, for many, a list under a field
//! that narrows it (`search.rs`).

mod search;

use std::rc::Rc;

use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, signal};
use quadra_tree::{NodeId, Paint};
use quadra_ui::{MountCx, NodeRef, View, by_text, column, div, row, text};

use crate::icon::icon;
use crate::menu::{Ends, MenuItem, rows};
use crate::popup::{EDGE, popover};
use crate::{icons, size, token};

pub struct Select {
    options: Rc<Vec<String>>,
    chosen: Signal<usize>,
    width: f32,
    placeholder: String,
    /// What the field of a select that is searched says while it is empty.
    search: Option<String>,
    test_id: Option<&'static str>,
}

/// A dropdown of `options`: `chosen` holds the index of the one that is chosen.
pub fn select<S: Into<String>>(options: impl IntoIterator<Item = S>, chosen: Signal<usize>) -> Select {
    let options = Rc::new(options.into_iter().map(Into::into).collect());
    Select { options, chosen, width: 160.0, placeholder: String::new(), search: None, test_id: None }
}

impl Select {
    pub fn width(mut self, px: f32) -> Self {
        self.width = px;
        self
    }

    /// What the select says, muted, while `chosen` is the index of no option
    /// (`usize::MAX` says nothing is chosen yet).
    pub fn placeholder(mut self, words: impl Into<String>) -> Self {
        self.placeholder = words.into();
        self
    }

    /// A field at the top of the list that narrows it as it is typed into, the best match
    /// first; `ask` is what it says while empty ("Search theme…"). For more options than a menu
    /// holds well.
    pub fn searchable(mut self, ask: impl Into<String>) -> Self {
        self.search = Some(ask.into());
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
        let (count, none) = (options.len(), self.placeholder);
        let value = move || options.get(chosen.get()).cloned().unwrap_or_else(|| none.clone());
        let tone =
            move || -> Paint { if chosen.get() < count { token::TEXT.into() } else { token::TEXT_3.into() } };
        // Filled, with a hairline border, 28 px high; the border is where the focus shows.
        let face = row().width(self.width).height(by_text(size::H_MD)).padding_x(8.0).gap(8.0).shrink(0.0);
        let face = face.align_items(Align::Center).radius(token::RADIUS_MD).bg(token::RAISED);
        let face = face.border(1.0).border_color(token::LINE_STRONG).hover(|s| s.bg(token::RAISED_HOVER));
        let face = face.focus_visible(|s| s.border_color(token::BORDER_FOCUSED));
        let face = face.expanded(open);
        let face = face.cursor(Cursor::Pointer).node_ref(&opener);
        // Enter and Space toggle it too: while it is open the focus is in its menu.
        let face =
            quadra_base::press::press(face).on_click(move || open.update(|open| *open = !*open)).build();
        let face = match self.test_id {
            Some(id) => face.test_id(id),
            None => face,
        };
        let face = face.child(div().grow(1.0).text_color(tone).child(text(value)));
        let face = face.child(icon(icons::CHEVRONS_UP_DOWN).size(12.0).color(token::TEXT_2));
        let (options, width) = (self.options, self.width);
        if let Some(ask) = self.search {
            let wide = width.max(220.0) - EDGE;
            let list = popover(open, &opener, move || {
                search::searched(options.clone(), chosen, open, wide, ask.clone())
            });
            return column().child(face).child(list).mount(cx);
        }
        // The chosen option has a quiet check before it; with none chosen, no room is kept.
        let items = move || {
            let on = chosen.get_untracked();
            let item = |(i, name): (usize, &String)| {
                let item = MenuItem::new(name.clone()).on_select(move || chosen.set(i));
                if i == on { item.checked(true).quiet() } else { item }
            };
            options.iter().enumerate().map(item).collect()
        };
        let ends = move || Ends { done: Rc::new(move || open.set(false)), leave: None };
        // As wide as the field at least.
        let list = popover(open, &opener, move || rows(items(), ends()).min_width(width - EDGE));
        column().child(face).child(list).mount(cx)
    }
}
