//! The select (specs/design, "Select"): a button that shows which of a few options is
//! chosen, and a menu of them with a check on that one; or, for many, a list under a field
//! that narrows it (`search.rs`).

mod search;

use std::rc::Rc;

use quadra_layout::Align;
use quadra_platform::Cursor;
use quadra_reactive::{Signal, signal};
use quadra_tree::{NodeId, Paint};
use quadra_view::{IntoProp, MountCx, NodeRef, Prop, View, by_text, column, div, row, text};

use crate::icon::icon;
use crate::menu::{Ends, MenuItem, rows};
use crate::popup::{EDGE, below, listing, popup};
use crate::{icons, size, token};

pub struct Select {
    options: Rc<Vec<String>>,
    /// A few words of each option in its menu row alone ("" for none).
    details: Rc<Vec<String>>,
    chosen: Signal<usize>,
    width: Option<f32>,
    placeholder: String,
    /// What the field of a select that is searched says while it is empty.
    search: Option<String>,
    label: Option<Prop<String>>,
    test_id: Option<&'static str>,
}

/// A dropdown of `options`: `chosen` holds the index of the one that is chosen.
pub fn select<S: Into<String>>(options: impl IntoIterator<Item = S>, chosen: Signal<usize>) -> Select {
    let options = Rc::new(options.into_iter().map(Into::into).collect());
    let (placeholder, search, label, test_id) = (String::new(), None, None, None);
    Select { options, details: Rc::default(), chosen, width: None, placeholder, search, label, test_id }
}

impl Select {
    pub fn width(mut self, px: f32) -> Self {
        self.width = Some(px);
        self
    }

    /// What the select says, muted, while `chosen` is the index of no option
    /// (`usize::MAX` says nothing is chosen yet).
    pub fn placeholder(mut self, words: impl Into<String>) -> Self {
        self.placeholder = words.into();
        self
    }

    /// A few words of each option, in order, after its name in the menu and not in the field
    /// ("" for an option with none); to a reader its row's description.
    pub fn details<S: Into<String>>(mut self, words: impl IntoIterator<Item = S>) -> Self {
        self.details = Rc::new(words.into_iter().map(Into::into).collect());
        self
    }

    /// A field at the top of the list that narrows it as it is typed into, the best match
    /// first; `ask` is what it says while empty ("Search theme…"). For more options than a menu
    /// holds well.
    pub fn searchable(mut self, ask: impl Into<String>) -> Self {
        self.search = Some(ask.into());
        self
    }

    /// What a reader calls it, a constant or words that change; the option it shows is then
    /// its value. Without one it is called by that option.
    pub fn label<M>(mut self, name: impl IntoProp<String, M>) -> Self {
        self.label = Some(name.into_prop());
        self
    }

    pub fn test_id(mut self, id: &'static str) -> Self {
        self.test_id = Some(id);
        self
    }
}

/// A select's own width.
const OWN: f32 = 160.0;

impl View for Select {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (chosen, open, opener) = (self.chosen, signal(false), NodeRef::new());
        let options = self.options.clone();
        let (count, none) = (options.len(), self.placeholder);
        let value = Rc::new(move || options.get(chosen.get()).cloned().unwrap_or_else(|| none.clone()));
        let tone =
            move || -> Paint { if chosen.get() < count { token::TEXT.into() } else { token::TEXT_3.into() } };
        // The field look, 28 px high; pressed, not typed in, so it answers the pointer; the
        // border is where the focus shows.
        let face =
            row().height(by_text(size::H_MD)).padding_x(token::SPACE_MD).gap(token::SPACE_MD).shrink(0.0);
        let face = crate::layout::field_width(face, self.width, OWN);
        let face = face.align_items(Align::Center).radius(token::RADIUS_MD).bg(token::INPUT_BG);
        let face = face.border(1.0).border_color(token::LINE_STRONG).hover(|s| s.bg(token::PANEL_HOVER));
        let face = face.focus_visible(|s| s.border_color(token::BORDER_FOCUSED));
        let face = face.expanded(open);
        let face = face.cursor(Cursor::Pointer).node_ref(&opener);
        // Enter and Space toggle it too: while it is open the focus is in its menu.
        let face = quadra_headless::button(face).on_click(move || open.update(|open| *open = !*open)).build();
        let face = match self.test_id {
            Some(id) => face.test_id(id),
            None => face,
        };
        let face = match self.label {
            Some(name) => {
                let shown = value.clone();
                face.label(name).value(move || shown())
            }
            None => face,
        };
        // One line: what does not fit is cut with an ellipsis (the menu shows it whole).
        let value = text(move || value()).ellipsis();
        let face = face.child(div().grow(1.0).min_width(0.0).text_color(tone).child(value));
        let face = face.child(icon(icons::CHEVRONS_UP_DOWN).size(12.0).color(token::TEXT_2));
        // The menu is as wide as the field at least: the field's width when it opens.
        let (options, width, told) = (self.options, self.width.unwrap_or(OWN), self.width);
        let field = opener.clone();
        let wide = move |tree: &quadra_tree::Tree| field.get().map_or(width, |it| tree.window_rect(it).width);
        if let Some(ask) = self.search {
            let wide = width.max(220.0) - EDGE;
            let list = popup(open, listing(below(&opener)), Some(opener.clone()), move || {
                search::searched(options.clone(), chosen, open, wide, ask.clone())
            });
            return crate::layout::holder(column(), told).child(face).child(list).mount(cx);
        }
        // The chosen option has a quiet check before it; with none chosen, no room is kept.
        let details = self.details;
        let items = move || {
            let on = chosen.get_untracked();
            let item = |(i, name): (usize, &String)| {
                let item = MenuItem::new(name.clone()).on_select(move || chosen.set(i));
                let item = match details.get(i).filter(|it| !it.is_empty()) {
                    Some(words) => item.detail(words.clone()),
                    None => item,
                };
                if i == on { item.checked(true).quiet() } else { item }
            };
            options.iter().enumerate().map(item).collect()
        };
        let ends = move || Ends { done: Rc::new(move || open.set(false)), leave: None };
        // As wide as the field at least.
        let list = popup(open, listing(below(&opener)), Some(opener.clone()), move || {
            let (least, wide) = (signal(width - EDGE), wide.clone());
            let list = rows(items(), ends()).min_width(move || quadra_layout::Length::Px(least.get()));
            list.on_mount(move |tree, _| least.set(wide(tree) - EDGE))
        });
        crate::layout::holder(column(), told).child(face).child(list).mount(cx)
    }
}
