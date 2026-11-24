//! The arrangements of the design language (specs/design/layout.md): a view is built from
//! these and the kit's containers, and writes no distance of its own. Each is a `Div` with
//! the language's gap, so anything a `Div` takes can follow.

use quadra_layout::{Align, Justify, Length, Track};
use quadra_reactive::{Scope, provide, use_context};
use quadra_tree::{NodeId, Role};
use quadra_view::{AnyView, Div, MountCx, View, column, div, row, text};

use crate::{size, token};

/// A column of the lines of one group, 8 apart.
pub fn vstack() -> Div {
    column().gap(token::SPACE_MD)
}

/// A row of controls, or of a control and words, 8 apart and centred across.
pub fn hstack() -> Div {
    row().align_items(Align::Center).gap(token::SPACE_MD)
}

/// A row of buttons at the right end, 8 apart: the main one is the last.
pub fn button_row() -> Div {
    hstack().justify(Justify::End)
}

/// `n` equal columns side by side, 12 apart: fields two to a row.
pub fn columns(n: usize) -> Div {
    let tracks = (0..n.max(1)).map(|_| Track::fr(1.0));
    div().grid(tracks).gap(token::SPACE_LG)
}

/// A column of sections, 24 apart.
pub fn sections() -> Div {
    column().gap(token::SPACE_XXL)
}

/// What takes the room left in a row or a column.
pub fn spacer() -> Div {
    div().grow(1.0)
}

/// The label of a section: 12 px, semibold, in the secondary colour; a heading to a reader.
pub fn section_label(title: impl Into<String>) -> Div {
    let label = div().role(Role::Heading).font_weight(size::WEIGHT_STRONG).text_color(token::TEXT_2);
    label.text_size(size::TEXT_SM.size).line_height(size::TEXT_SM.line_height()).child(text(title.into()))
}

/// A section label over a column of what it heads, 8 apart.
pub fn section(title: impl Into<String>) -> Div {
    vstack().child(section_label(title))
}

/// Provided to what a form holds: a field with no width of its own takes its column's.
#[derive(Clone)]
struct Fills;

/// Whether the control being mounted is in a form (or a dialog's body): with no width told,
/// it fills its column.
pub(crate) fn fills() -> bool {
    use_context::<Fills>().is_some()
}

/// Mounts `view` so that the fields in it fill their columns.
pub(crate) fn filled(cx: &mut MountCx, view: impl View) -> NodeId {
    Scope::child().run(|| {
        provide(Fills);
        view.mount(cx)
    })
}

/// The width of a field: the one it was told, its column's in a form, or its own.
pub(crate) fn field_width(frame: Div, told: Option<f32>, own: f32) -> Div {
    match told {
        Some(px) => frame.width(px),
        None if fills() => frame.width(Length::Fill).min_width(Length::Px(0.0)).shrink(1.0),
        None => frame.width(own),
    }
}

/// A column of fields, 12 apart: each field, and its control when it was told no width, is as
/// wide as the form.
pub struct Form {
    children: Vec<AnyView>,
}

pub fn form() -> Form {
    Form { children: Vec::new() }
}

impl Form {
    pub fn child(mut self, view: impl View + 'static) -> Self {
        self.children.push(AnyView::new(view));
        self
    }

    pub fn children<V: View + 'static>(mut self, views: impl IntoIterator<Item = V>) -> Self {
        self.children.extend(views.into_iter().map(AnyView::new));
        self
    }
}

impl View for Form {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        filled(cx, column().gap(token::SPACE_LG).children(self.children))
    }
}
