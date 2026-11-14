//! The app's language (specs/15, "Language"): tables of strings a language, a locale that is
//! a signal, and lookups that fall back to the default language and then to the key.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use quadra_reactive::{Signal, signal, use_context};
use serde_json::{Map, Value};

use crate::locale::Locale;
use crate::log;

/// What fills a `{name}` of a string: a number, which also picks the plural, or a text.
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Number(f64),
    Text(String),
}

macro_rules! numbers {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg {
            fn from(n: $t) -> Arg {
                Arg::Number(n as f64)
            }
        }
    )*};
}

numbers!(i32, i64, u32, u64, usize, f32, f64);

impl From<&str> for Arg {
    fn from(text: &str) -> Arg {
        Arg::Text(text.to_string())
    }
}

impl From<String> for Arg {
    fn from(text: String) -> Arg {
        Arg::Text(text)
    }
}

pub struct I18n {
    default: Locale,
    locale: Signal<Locale>,
    /// The strings of each language, by key.
    tables: RefCell<HashMap<String, Map<String, Value>>>,
    /// Keys a language was found without: each is logged once.
    missing: RefCell<HashSet<(String, String)>>,
}

impl I18n {
    /// The language strings fall back to; it is also the first locale.
    pub fn new(default: Locale) -> Rc<I18n> {
        let locale = signal(default.clone());
        Rc::new(I18n { default, locale, tables: RefCell::default(), missing: RefCell::default() })
    }

    /// Adds the strings of a language, a JSON object of keys; to those it had.
    pub fn add(&self, language: &str, json: &str) -> Result<(), String> {
        let Value::Object(map) = serde_json::from_str(json).map_err(|e| e.to_string())? else {
            return Err(format!("the strings of {language} are not a JSON object"));
        };
        let language = Locale::new(language).language();
        self.tables.borrow_mut().entry(language).or_default().extend(map);
        Ok(())
    }

    /// The locale: views that read it are shown again when it is another one.
    pub fn locale(&self) -> Signal<Locale> {
        self.locale
    }

    fn find(&self, language: &str, key: &str) -> Option<Value> {
        self.tables.borrow().get(language)?.get(key).cloned()
    }

    /// The string of `key` in the locale, filled with `args`.
    pub fn text(&self, key: &str, args: &[(&str, Arg)]) -> String {
        let locale = self.locale.get();
        let language = locale.language();
        let found = self.find(&language, key).or_else(|| {
            if self.missing.borrow_mut().insert((language.clone(), key.to_string())) {
                log::warn("i18n", format!("no \"{key}\" in {language}"));
            }
            self.find(&self.default.language(), key)
        });
        let Some(found) = found else { return key.to_string() };
        // A string of plural forms takes the one the number `n` asks for.
        let text = match &found {
            Value::String(text) => text.clone(),
            Value::Object(forms) => {
                let n = args.iter().find_map(|(name, arg)| match arg {
                    Arg::Number(n) if *name == "n" => Some(*n),
                    _ => None,
                });
                let form = forms.get(locale.plural(n.unwrap_or(0.0))).or_else(|| forms.get("other"));
                form.and_then(Value::as_str).unwrap_or(key).to_string()
            }
            _ => key.to_string(),
        };
        fill(&text, args, &locale)
    }
}

/// `{name}` replaced by its argument; a number in the locale's form.
fn fill(text: &str, args: &[(&str, Arg)], locale: &Locale) -> String {
    let mut out = text.to_string();
    for (name, arg) in args {
        let value = match arg {
            Arg::Text(text) => text.clone(),
            Arg::Number(n) => locale.number(*n, if n.fract() == 0.0 { 0 } else { 2 }),
        };
        out = out.replace(&format!("{{{name}}}"), &value);
    }
    out
}

/// The app's language as a view keeps it: what it says is looked up in closures that run
/// again later, where the context of the view is no longer around. Ask for it while the view
/// is built.
#[derive(Clone)]
pub struct Language {
    i18n: Option<Rc<I18n>>,
    locale: Signal<Locale>,
}

/// The app's language; English, with no strings, where the app has none.
pub fn language() -> Language {
    match use_context::<Rc<I18n>>() {
        Some(i18n) => Language { locale: i18n.locale, i18n: Some(i18n) },
        None => Language { i18n: None, locale: signal(Locale::en()) },
    }
}

impl Language {
    /// The locale, read as a signal is.
    pub fn locale(&self) -> Locale {
        self.locale.get()
    }

    /// The string of `key`; the key itself where the app has none.
    pub fn t(&self, key: &str) -> String {
        self.t_with(key, &[])
    }

    /// The string of `key`, its `{name}`s filled: `t_with("files.count", &[("n", 3.into())])`.
    pub fn t_with(&self, key: &str, args: &[(&str, Arg)]) -> String {
        match &self.i18n {
            Some(i18n) => i18n.text(key, args),
            None => fill(key, args, &self.locale.get()),
        }
    }
}

/// The string of `key` in the app's language, while a view is built (`language().t(key)`
/// in what runs later).
pub fn t(key: &str) -> String {
    language().t(key)
}

/// The same, with arguments.
pub fn t_with(key: &str, args: &[(&str, Arg)]) -> String {
    language().t_with(key, args)
}

/// The app's locale, while a view is built.
pub fn locale() -> Locale {
    language().locale()
}
