//! Saved hosts and the recent ones (specs/19, "M8 in detail"): rows of words in the
//! settings, so the file reads as the page does.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use oneterm_ssh::{Auth, Target};

use crate::model::Model;
use crate::shells::{Launch, Remote};

const AUTHS: [&str; 4] = ["Auto", "Agent", "Key", "Password"];
/// How many recent targets are kept.
const RECENT: usize = 5;

/// A host the user saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Host {
    pub name: String,
    pub group: String,
    pub remote: Remote,
}

impl Host {
    fn row(&self) -> Vec<String> {
        let (auth, key) = match &self.remote.auth {
            Auth::Agent => (1, String::new()),
            Auth::Key(path) => (2, path.display().to_string()),
            Auth::Password => (3, String::new()),
            Auth::Auto => (0, String::new()),
        };
        vec![self.name.clone(), self.group.clone(), self.remote.target.label(), AUTHS[auth].into(), key]
    }

    fn of(row: &[String]) -> Option<Host> {
        let [name, group, target, auth, key] = row else { return None };
        let auth = match auth.as_str() {
            "Agent" => Auth::Agent,
            "Key" => Auth::Key(PathBuf::from(key)),
            "Password" => Auth::Password,
            _ => Auth::Auto,
        };
        let remote = Remote { target: Target::parse(target)?, auth };
        Some(Host { name: name.clone(), group: group.clone(), remote })
    }

    /// The host's address as a row shows it: `user@host`, its port when not 22.
    pub fn address(&self) -> String {
        self.remote.target.label()
    }
}

/// The index of an auth among the dialogs' choices, and back.
pub(crate) fn auth_of(index: usize, key: String) -> Auth {
    match index {
        1 => Auth::Agent,
        2 => Auth::Key(PathBuf::from(key)),
        3 => Auth::Password,
        _ => Auth::Auto,
    }
}

impl Model {
    /// The saved hosts, in the order saved.
    pub fn hosts(&self) -> Vec<Host> {
        self.prefs().hosts.with(|rows| rows.iter().filter_map(|row| Host::of(row)).collect())
    }

    /// Saves a host, in place of one of the same name.
    pub fn save_host(&self, host: Host) {
        let row = host.row();
        self.prefs().hosts.update(|rows| match rows.iter().position(|it| it.first() == Some(&host.name)) {
            Some(at) => rows[at] = row,
            None => rows.push(row),
        });
    }

    pub fn remove_host(&self, name: &str) {
        self.prefs().hosts.update(|rows| rows.retain(|it| it.first().map(String::as_str) != Some(name)));
    }

    /// Opens a tab connected to a host, or a space beside the focused one; it is recent.
    pub fn connect_to(&self, remote: Remote, here: bool) {
        let label = remote.target.label();
        let launch = Launch { ssh: Some(remote), ..self.launch(None) };
        if here {
            self.split_with(launch)
        } else {
            self.new_tab_with(launch)
        }
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |it| it.as_secs());
        self.prefs().recent.update(|rows| {
            rows.retain(|it| it.first() != Some(&label));
            rows.insert(0, vec![label, now.to_string()]);
            rows.truncate(RECENT);
        });
    }

    /// The recent targets, newest first, with when (seconds since 1970).
    pub fn recent(&self) -> Vec<(Remote, u64)> {
        self.prefs().recent.with(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let [label, at] = &row[..] else { return None };
                    let host = self.hosts().into_iter().find(|it| it.address() == *label);
                    let remote = match host {
                        Some(host) => host.remote,
                        None => Remote { target: Target::parse(label)?, auth: Auth::Auto },
                    };
                    Some((remote, at.parse().unwrap_or(0)))
                })
                .collect()
        })
    }

    /// What a remote space is called: its host's name, else `user@host`.
    pub fn host_name(&self, remote: &Remote) -> String {
        let host = self.hosts().into_iter().find(|it| it.remote.target == remote.target);
        host.map_or_else(|| remote.target.label(), |it| it.name)
    }

    /// Whether a space is connected to a target.
    pub fn connected_to(&self, target: &Target) -> bool {
        self.tabs().with(|tabs| {
            tabs.iter().any(|tab| {
                tab.spaces.with(|spaces| {
                    spaces.iter().any(|space| {
                        space.remote.as_ref().is_some_and(|it| it.target == *target)
                            && space.session.exited().get().is_none()
                    })
                })
            })
        })
    }
}

/// How long ago, in a word or two: `now`, `2 min ago`, `3 h ago`, `yesterday`, `4 d ago`.
pub(crate) fn ago(then: u64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |it| it.as_secs());
    match now.saturating_sub(then) {
        0..60 => "now".into(),
        s @ 60..3600 => format!("{} min ago", s / 60),
        s @ 3600..86_400 => format!("{} h ago", s / 3600),
        86_400..172_800 => "yesterday".into(),
        s => format!("{} d ago", s / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_is_a_row_of_words_and_back() {
        let remote =
            Remote { target: Target::parse("ops@10.20.0.2:2222").unwrap(), auth: Auth::Key("k".into()) };
        let host = Host { name: "prod-lb".into(), group: "Production".into(), remote };
        assert_eq!(host.row(), ["prod-lb", "Production", "ops@10.20.0.2:2222", "Key", "k"]);
        assert_eq!(Host::of(&host.row()), Some(host));
        assert_eq!(Host::of(&["a".into(), "b".into(), "-x".into(), "Auto".into(), String::new()]), None);
        assert!(ago(0).ends_with("d ago"));
    }
}
