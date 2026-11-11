//! Tokens and the theme (specs/13, "Tokens"): a colour of a style is one colour, or a role
//! that the theme of the window gives a colour to. A style that holds roles is plain data;
//! the style pass looks them up, and looks them up again when the theme changes.

use std::rc::Rc;

use quadra_prim::Color;

use crate::node::NodeId;
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

/// The colours of the roles. A role the theme does not have is transparent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Theme {
    colors: Vec<Color>,
}

impl Theme {
    /// A theme from the colour of each role, in the order of their numbers.
    pub fn new(colors: Vec<Color>) -> Theme {
        Theme { colors }
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
        self.theme = theme;
        let themed =
            |i: &usize| self.nodes[*i].alive && self.nodes[*i].styled.as_ref().is_some_and(|s| s.themed());
        let again: Vec<usize> = (0..self.nodes.len()).filter(themed).collect();
        for index in again {
            let id = NodeId { index: index as u32, generation: self.nodes[index].generation };
            self.restyle.push(id);
        }
        self.shared.wake.set(true);
    }
}
