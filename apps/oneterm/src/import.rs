//! What OneTerm takes over from OneTerm 0.6, its versions before 0.7 (specs/19, "Taking over from
//! OneTerm 0.6"): the sessions saved in its folder as hosts, and at the first start the settings
//! of its terminal that have one here. Its vault and the passwords it sealed come as they
//! are: OneTerm's vault is of the same format (`vault.rs`).

use std::path::{Path, PathBuf};

use oneterm_ssh::{Auth, Forward, Target, Way};
use quadra_data::{Value, parse_json};

use crate::hosts::Host;
use crate::model::Model;
use crate::shells::Remote;
use crate::vault::Header;

/// A saved session as a host, its colour, and the password sealed with it.
pub(crate) struct Taken {
    pub host: Host,
    pub colour: Option<String>,
    pub sealed: Option<String>,
}

fn text<'a>(of: &'a Value, key: &str) -> Option<&'a str> {
    match of.get(key) {
        Some(Value::Text(text)) => Some(text.as_str()),
        _ => None,
    }
}

fn number(of: &Value, key: &str) -> Option<f64> {
    match of.get(key) {
        Some(Value::Number(written)) => written.parse().ok(),
        _ => None,
    }
}

/// The folders a build of the name before kept under `roots` (`%APPDATA%`, `%LOCALAPPDATA%`)
/// become this app's, where it has none there yet. One that cannot be moved, as that build
/// still runs, stays where it is.
pub fn adopt(roots: &[PathBuf]) {
    for root in roots {
        let (old, new) = (root.join("1Term"), root.join("OneTerm"));
        if old.is_dir() && !new.exists() {
            let _ = std::fs::rename(old, new);
        }
    }
}

/// OneTerm 0.6's folder: `.OneTerm` in the user's.
pub(crate) fn folder() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".OneTerm"))
}

/// The sessions of an `ssh_session.json`, of any of its versions: the first was the list
/// itself. Sessions are known by an id there and hosts by their name here, so a label met
/// again gets a number after it, and a jump host is named by the name its session got.
pub(crate) fn sessions(json: &str) -> Vec<Taken> {
    let Ok(all) = parse_json(json) else { return Vec::new() };
    let list = match (&all, all.get("sessions")) {
        (Value::List(list), _) | (_, Some(Value::List(list))) => list,
        _ => return Vec::new(),
    };
    let mut names: Vec<String> = Vec::new();
    for session in list {
        let label = text(session, "label").filter(|it| !it.trim().is_empty());
        let label = label.or(text(session, "host")).unwrap_or("host").trim().to_string();
        let free = |name: &String| !names.contains(name);
        let name = std::iter::once(label.clone()).chain((2..).map(|n| format!("{label} ({n})"))).find(free);
        names.extend(name);
    }
    let named = |id: f64| {
        let at = list.iter().position(|it| number(it, "id") == Some(id))?;
        names.get(at).cloned()
    };
    let taken = list.iter().zip(&names).filter_map(|(session, name)| {
        let host = text(session, "host")?.trim();
        let port = number(session, "port").map_or(22, |it| it as u16);
        let user = text(session, "username").unwrap_or_default().trim().to_string();
        let target = Target { user, host: host.to_string(), port };
        let auth = match (text(session, "auth_method"), text(session, "key_path")) {
            (Some("private_key"), Some(key)) => Auth::Key(PathBuf::from(key)),
            (Some("private_key"), None) => Auth::Auto,
            (Some("agent"), _) => Auth::Agent,
            _ => Auth::Password,
        };
        // A password saved behind another way in is stale there, and would be tried here
        // as a key's passphrase.
        let sealed = text(session, "sealed_password").filter(|_| auth == Auth::Password);
        let mut remote = Remote::new(target, auth);
        if let Some(Value::List(forwards)) = session.get("port_forwards") {
            remote.forwards = forwards.iter().filter_map(forward).collect();
        }
        Some(Taken {
            host: Host {
                name: name.clone(),
                group: text(session, "group").unwrap_or_default().to_string(),
                remote,
                jump: number(session, "jump_host").and_then(named).unwrap_or_default(),
            },
            colour: text(session, "color").map(str::to_string),
            sealed: sealed.map(str::to_string),
        })
    });
    taken.filter(|it| !it.host.remote.target.host.is_empty()).collect()
}

/// A forward: what listens, and where it goes. The address it listens on is this machine's
/// own here, whatever it was there.
fn forward(of: &Value) -> Option<Forward> {
    let listen = number(of, "bind_port")? as u16;
    let way = match text(of, "kind")? {
        "local" => Way::Local,
        "remote" => Way::Remote,
        "dynamic" => return Some(Forward { way: Way::Dynamic, listen, host: String::new(), port: 0 }),
        _ => return None,
    };
    let (host, port) = (text(of, "target_host")?.to_string(), number(of, "target_port")? as u16);
    Some(Forward { way, listen, host, port })
}

impl Model {
    /// Adds the hosts OneTerm 0.6 saved in `folder` whose names OneTerm does not have, with their
    /// colours, then its saved passwords: how many hosts, how many passwords, and how many
    /// passwords could not be taken. `settings` (the first start alone) takes the terminal's
    /// settings too.
    pub fn take_over(&self, folder: &Path, settings: bool) -> (usize, usize, usize) {
        let saved = std::fs::read_to_string(folder.join("ssh_session.json")).unwrap_or_default();
        let have: Vec<String> = self.hosts().into_iter().map(|it| it.name).collect();
        let all = sessions(&saved);
        let new: Vec<&Taken> = all.iter().filter(|it| !have.contains(&it.host.name)).collect();
        for taken in &new {
            self.save_host(taken.host.clone());
            self.set_host_tag(&taken.host.name, taken.colour.as_deref());
        }
        let terminal = std::fs::read_to_string(folder.join("terminal.json")).ok().filter(|_| settings);
        if let Some(Ok(terminal)) = terminal.as_deref().map(parse_json) {
            self.take_settings(&terminal);
        }
        // The proxy its updates went through, so the next one arrives the same way. Whether
        // certificates are checked is not taken: off is chosen here, knowingly.
        let updates = std::fs::read_to_string(folder.join("update_config.json")).ok().filter(|_| settings);
        let updates = updates.as_deref().and_then(|it| parse_json(it).ok());
        let proxy = updates.as_ref().and_then(|it| text(it, "proxy_url")).map(str::trim);
        if let Some(proxy) = proxy.filter(|it| !it.is_empty()) {
            let net = self.prefs().net;
            net.proxy.set(1);
            net.proxy_address.set(proxy.to_string());
        }
        let vault = std::fs::read_to_string(folder.join("vault.json")).unwrap_or_default();
        let (passwords, left) = self.take_passwords(&vault, &all);
        (new.len(), passwords, left)
    }

    /// OneTerm 0.6's vault and what it sealed, as they are: its document becomes OneTerm's vault
    /// where there is none, and a session's password its host's secret, where OneTerm has that
    /// host (the name and the target) without one. A vault of OneTerm's own is kept, and then
    /// nothing opens what another sealed: how many were taken, and how many were not.
    fn take_passwords(&self, vault: &str, all: &[Taken]) -> (usize, usize) {
        let sealed = || all.iter().filter_map(|it| Some((&it.host, it.sealed.as_ref()?)));
        let (theirs, ours) = (Header::parse(vault), self.prefs().vault);
        let Some(theirs) = theirs.filter(|_| sealed().count() > 0) else { return (0, sealed().count()) };
        match Header::parse(&ours.get_untracked()) {
            Some(ours) if ours.id != theirs.id => return (0, sealed().count()),
            Some(_) => {}
            None => ours.set(theirs.write()),
        }
        let hosts = self.hosts();
        let (mut taken, mut left) = (0, 0);
        for (host, envelope) in sealed() {
            let here = hosts.iter().any(|it| it.name == host.name && it.remote.target == host.remote.target);
            match (here && theirs.seals(envelope), self.sealed(&host.name)) {
                (true, None) => {
                    self.set_sealed(&host.name, Some(envelope.clone()));
                    taken += 1;
                }
                (true, Some(_)) => {}
                (false, _) => left += 1,
            }
        }
        (taken, left)
    }

    /// The settings of OneTerm 0.6's `terminal.json` that have one here.
    fn take_settings(&self, of: &Value) {
        let (p, part) = (self.prefs(), |name: &str| of.get(name).cloned().unwrap_or(Value::Null));
        let (font, scroll, cursor, ssh) = (part("font"), part("scroll"), part("cursor"), part("ssh"));
        let on = |of: &Value, key: &str| match of.get(key) {
            Some(Value::Bool(on)) => Some(*on),
            _ => None,
        };
        text(&font, "family").into_iter().for_each(|it| p.look.font_family.set(it.to_string()));
        number(&font, "size").into_iter().for_each(|it| p.look.font_size.set(it.clamp(8.0, 32.0)));
        number(&scroll, "scrollback_history").into_iter().for_each(|it| p.scrollback.set(it));
        on(&cursor, "blink").into_iter().for_each(|it| p.blink.set(it));
        on(&ssh, "keepalive_enabled").into_iter().for_each(|it| p.keepalive_on.set(it));
        number(&ssh, "keepalive_interval_secs").into_iter().for_each(|it| p.keepalive_secs.set(it));
        number(&ssh, "keepalive_max").into_iter().for_each(|it| p.keepalive_max.set(it));
    }

    /// What a start of the app does once: takes over from OneTerm 0.6's folder, and says what it
    /// took. A trial run (`ONETERM_HOME`) and a test take nothing: the app's `main` asks.
    pub fn take_over_once(&self) {
        let done = self.prefs().taken_over;
        if done.get_untracked() {
            return;
        }
        done.set(true);
        // Nothing taken is nothing to say at a start; the settings' button says it.
        let took = folder().filter(|it| it.is_dir()).map(|it| self.take_over(&it, true));
        took.filter(|it| *it != (0, 0, 0)).into_iter().for_each(|it| self.say_taken(it));
    }

    /// A card that says what was taken over.
    pub(crate) fn say_taken(&self, (hosts, passwords, left): (usize, usize, usize)) {
        let count = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
        let (h, p) = (count(hosts, "host"), count(passwords, "saved password"));
        let mut body = match (hosts, passwords) {
            (0, 0) => "OneTerm 0.6 saved no host or password that is not here already.".to_string(),
            (_, 0) => format!("{h} taken over from OneTerm 0.6."),
            (0, _) => format!("{p} taken over from OneTerm 0.6."),
            _ => format!("{h} and {p} taken over from OneTerm 0.6."),
        };
        if passwords > 0 {
            body += " The master password is the one set in OneTerm 0.6.";
        }
        if left > 0 {
            body += &format!(" {} could not be taken: OneTerm asks again.", count(left, "saved password"));
        }
        self.notify(quadra_design::Notice::new("OneTerm 0.6").body(body));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_become_hosts_by_name_with_their_ways_in() {
        let json = r##"{"schema_version":2,"next_session_id":9,"sessions":[
            {"id":1,"label":"prod","host":"10.0.0.1","port":22,"username":"deploy","group":"infra",
             "color":"#56B6C2","sealed_password":"otv1:AAEC"},
            {"id":4,"label":"prod","host":"10.0.0.2","port":2222,"username":"ops","group":null,
             "auth_method":"private_key","key_path":"C:/keys/id","jump_host":1,
             "port_forwards":[{"kind":"local","bind":"127.0.0.1","bind_port":8080,"target_host":"db","target_port":5432},
                              {"kind":"dynamic","bind":"127.0.0.1","bind_port":1080}]},
            {"id":5,"label":"","host":"10.0.0.3","auth_method":"agent"},
            {"id":6,"label":"no address"}]}"##;
        let all = sessions(json);
        let names: Vec<&str> = all.iter().map(|it| it.host.name.as_str()).collect();
        assert_eq!(names, ["prod", "prod (2)", "10.0.0.3"]);
        let (first, second) = (&all[0], &all[1]);
        assert_eq!((first.host.address(), first.host.group.as_str()), ("deploy@10.0.0.1".into(), "infra"));
        assert!(first.sealed.is_some() && first.colour.as_deref() == Some("#56B6C2"));
        assert_eq!(first.host.remote.auth, Auth::Password);
        assert_eq!(second.host.address(), "ops@10.0.0.2:2222");
        assert_eq!(
            (second.host.jump.as_str(), &second.host.remote.auth),
            ("prod", &Auth::Key("C:/keys/id".into()))
        );
        let ways: Vec<String> = second.host.remote.forwards.iter().map(Forward::label).collect();
        assert_eq!(ways, ["8080:db:5432", "D1080"]);
        assert_eq!(all[2].host.remote.auth, Auth::Agent);
        // The first version was the list itself; what is no such file gives nothing.
        assert_eq!(sessions(r#"[{"label":"old","host":"h","port":22}]"#).len(), 1);
        assert!(sessions("not json").is_empty() && sessions("{}").is_empty());
    }
}
