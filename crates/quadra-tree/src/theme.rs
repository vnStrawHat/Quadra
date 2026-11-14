//! Tokens and the theme (specs/13, "Tokens"): a colour of a style is one colour, or a role
//! that the theme of the window gives a colour to. A style that holds roles is plain data;
//! the style pass looks them up, and looks them up again when the theme changes.

use std::rc::Rc;

use quadra_prim::Color;

use crate::node::{NodeId, flag};
use crate::tree::Tree;

/// A role of the theme: a small number the theme has a colour for. The names are those of
/// whoever defines the theme: the kit, for its design language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Token(pub u16);

impl Token {
    /// The role's colour, more transparent: `ACCENT.alpha(0.12)`.
    pub fn alpha(self, alpha: f32) -> Paint {
        Paint::Token(self, alpha)
    }
}

/// A colour of a style: one colour, or a role of the theme and how opaque it is taken.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    Color(Color),
    Token(Token, f32),
}

impl Default for Paint {
    fn default() -> Paint {
        Paint::Color(Color::TRANSPARENT)
    }
}

impl From<Color> for Paint {
    fn from(color: Color) -> Paint {
        Paint::Color(color)
    }
}

impl From<Token> for Paint {
    fn from(token: Token) -> Paint {
        Paint::Token(token, 1.0)
    }
}

/// One shadow a box above the window casts: how far down, how blurred, in what colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cast {
    pub down: f32,
    pub blur: f32,
    pub color: Color,
}

/// The colours of the roles, and the shadows of each level above the window. A role the
/// theme does not have is transparent; a level it has no shadows for casts none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Theme {
    colors: Vec<Color>,
    shadows: Vec<Vec<Cast>>,
}

impl Theme {
    /// A theme from the colour of each role, in the order of their numbers.
    pub fn new(colors: Vec<Color>) -> Theme {
        Theme { colors, shadows: Vec::new() }
    }

    /// The same colours, with the shadows of each level above the window: the first set is
    /// that of level 1.
    pub fn with_shadows(self, shadows: Vec<Vec<Cast>>) -> Theme {
        Theme { shadows, ..self }
    }

    /// The shadows a node of a level above the window casts under this theme.
    pub(crate) fn casts(&self, level: u8) -> &[Cast] {
        let set = (level as usize).checked_sub(1).and_then(|at| self.shadows.get(at));
        set.map_or(&[], Vec::as_slice)
    }

    pub fn color(&self, token: Token) -> Color {
        self.colors.get(token.0 as usize).copied().unwrap_or(Color::TRANSPARENT)
    }

    /// The colour a style's value stands for under this theme.
    pub fn resolve(&self, paint: Paint) -> Color {
        match paint {
            Paint::Color(color) => color,
            Paint::Token(token, alpha) => {
                let color = self.color(token);
                color.alpha(color.a * alpha)
            }
        }
    }
}

impl Tree {
    /// The theme the tree's styles are resolved with.
    pub fn theme(&self) -> &Rc<Theme> {
        &self.theme
    }

    /// Gives the tree another theme. Every node whose style holds a role is resolved again;
    /// what comes out equal flags nothing, so a theme that changes colours lays nothing out.
    pub fn set_theme(&mut self, theme: Rc<Theme>) {
        if *self.theme == *theme {
            return;
        }
        let cast_again = self.theme.shadows != theme.shadows;
        (self.theme, self.snap) = (theme, true); // a new theme shows at once: nothing eases to it
        let themed =
            |i: &usize| self.nodes[*i].alive && self.nodes[*i].styled.as_ref().is_some_and(|s| s.themed());
        let again: Vec<usize> = (0..self.nodes.len()).filter(themed).collect();
        for index in again {
            let id = NodeId { index: index as u32, generation: self.nodes[index].generation };
            self.restyle.push(id);
        }
        // A widget that paints with roles of the theme is painted again with the new ones.
        for index in 0..self.nodes.len() {
            let node = &mut self.nodes[index];
            let widget = node.widget.as_mut().filter(|_| node.alive);
            // So is a node above the window whose shadows are others now.
            let casts = cast_again && node.alive && node.paint.shadows > 0;
            if casts || widget.is_some_and(|widget| widget.theme(&self.theme)) {
                self.mark(
                    NodeId { index: index as u32, generation: self.nodes[index].generation },
                    flag::PAINT,
                );
            }
        }
        self.shared.wake.set(true);
    }
}
