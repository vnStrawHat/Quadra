//! Rules a text is checked against (specs/17, "The components"): plain data, with what to say
//! when one is broken, for the error of a field.

/// A rule a text is checked against: plain data, with what to say when it is broken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    Required(String),
    MinLen(usize, String),
    MaxLen(usize, String),
    Email(String),
}

impl Rule {
    /// What is wrong with `value` by this rule, if anything.
    pub fn check(&self, value: &str) -> Option<&str> {
        let (broken, says) = match self {
            Rule::Required(says) => (value.trim().is_empty(), says),
            Rule::MinLen(least, says) => (value.chars().count() < *least, says),
            Rule::MaxLen(most, says) => (value.chars().count() > *most, says),
            Rule::Email(says) => {
                let at = value.find('@').filter(|at| *at > 0);
                (!at.is_some_and(|at| value[at + 1..].contains('.') && !value.contains(' ')), says)
            }
        };
        broken.then_some(says.as_str())
    }
}

/// What the first broken rule says of a value; empty when none is broken: what
/// a field's error takes.
pub fn validate(value: &str, rules: &[Rule]) -> String {
    rules.iter().find_map(|rule| rule.check(value)).unwrap_or_default().to_string()
}
