//! A keymap file (specs/09, "The keymap file"): the user's bindings, as text, over an app's
//! defaults. Actions are named by `Action::NAME`; the app says which it has (`Actions`).
//!
//! ```text
//! # Everywhere, until a context is named.
//! ctrl-s = file::save
//! [editor && !find]
//! ctrl-k ctrl-c = editor::comment
//! ctrl-1 = tabs::select 0
//! ```

use std::collections::HashMap;
use std::rc::Rc;

use super::{Binding, Context, Keymap, KeymapError, Keystroke};
use crate::action::{Action, AnyAction, FocusNext, FocusPrevious, GoBack, GoForward};

type Build = Rc<dyn Fn(&str) -> Option<Rc<dyn AnyAction>>>;

/// The actions a keymap file may name, each built from what follows its name.
#[derive(Clone)]
pub struct Actions {
    by_name: HashMap<&'static str, Build>,
}

impl Default for Actions {
    /// The tree's own: moving the focus, and the pages back and forward.
    fn default() -> Actions {
        let mut actions = Actions { by_name: HashMap::new() };
        actions.add(FocusNext).add(FocusPrevious).add(GoBack).add(GoForward);
        actions
    }
}

impl Actions {
    pub fn new() -> Actions {
        Actions::default()
    }

    /// An action that takes nothing after its name.
    pub fn add<A: Action + Clone>(&mut self, action: A) -> &mut Actions {
        self.add_with(move |rest| rest.is_empty().then(|| action.clone()))
    }

    /// An action built from what follows its name (`tabs::select 3`); `None` says that is
    /// wrong.
    pub fn add_with<A: Action>(&mut self, build: impl Fn(&str) -> Option<A> + 'static) -> &mut Actions {
        let build: Build = Rc::new(move |rest| build(rest).map(|a| Rc::new(a) as Rc<dyn AnyAction>));
        self.by_name.insert(A::NAME, build);
        self
    }
}

impl Keymap {
    /// Adds the bindings of a keymap file after those it has, so they win over them. A line
    /// that is wrong (a keystroke, a context, an action not in `actions` or what follows its
    /// name) is reported with its number, and then none of the file is taken.
    pub fn load(&mut self, text: &str, actions: &Actions) -> Result<(), KeymapError> {
        let mut context: Option<Rc<Context>> = None;
        let mut taken = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let wrong = |what: String| KeymapError(format!("line {}: {what}", n + 1));
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                context = Some(Rc::new(Context::parse(name).map_err(|e| wrong(e.0))?));
                continue;
            }
            let (keys, command) =
                line.rsplit_once(" = ").ok_or_else(|| wrong(format!("no \" = \" in {line:?}")))?;
            let keys: Vec<Keystroke> = keys
                .split_whitespace()
                .map(Keystroke::parse)
                .collect::<Result<_, _>>()
                .map_err(|e| wrong(e.0))?;
            let (name, rest) = command.trim().split_once(' ').unwrap_or((command.trim(), ""));
            let build = actions.by_name.get(name).ok_or_else(|| wrong(format!("no action {name:?}")))?;
            let action = build(rest.trim()).ok_or_else(|| wrong(format!("{name} does not take {rest:?}")))?;
            taken.push(Binding { keys, context: context.clone(), action });
        }
        self.bindings.extend(taken);
        Ok(())
    }
}
