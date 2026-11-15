//! The context of a binding (specs/09, "Keys, contexts and actions"): a predicate over the
//! names from the focus to the root. `editor` holds where the focus is in an editor;
//! `editor && !find` where it is not in its find bar too; `a || b` in either.

use std::rc::Rc;

use super::KeymapError;

/// Clauses of which one must hold; in each, names that must be around the focus (`true`) or
/// must not (`false`).
#[derive(Debug)]
pub(crate) struct Context(Vec<Vec<(bool, Rc<str>)>>);

impl Context {
    pub fn name(name: &str) -> Context {
        Context(vec![vec![(true, Rc::from(name))]])
    }

    /// Parses `a`, `!a`, `a && b`, `a || b && !c` (`&&` binds closer than `||`).
    pub fn parse(text: &str) -> Result<Context, KeymapError> {
        let wrong = || KeymapError(format!("no such context: {text:?}"));
        let mut clauses = Vec::new();
        for clause in text.split("||") {
            let mut terms = Vec::new();
            for term in clause.split("&&") {
                let term = term.trim();
                let (wanted, name) = match term.strip_prefix('!') {
                    Some(name) => (false, name.trim()),
                    None => (true, term),
                };
                let ok = !name.is_empty()
                    && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b));
                terms.push((wanted, Rc::from(ok.then_some(name).ok_or_else(wrong)?)));
            }
            clauses.push(terms);
        }
        Ok(Context(clauses))
    }

    /// How near the focus it holds, the nearest the largest (a clause of only `!` holds as
    /// far as a binding for everywhere: 0); `None` where it does not.
    pub fn near(&self, contexts: &[&str]) -> Option<usize> {
        let at = |name: &str| contexts.iter().position(|c| *c == name).map(|at| contexts.len() - at);
        let clause = |terms: &Vec<(bool, Rc<str>)>| {
            let mut near = 0;
            for (wanted, name) in terms {
                match (at(name), wanted) {
                    (Some(n), true) => near = near.max(n),
                    (None, false) => {}
                    _ => return None,
                }
            }
            Some(near)
        };
        self.0.iter().filter_map(clause).max()
    }
}
