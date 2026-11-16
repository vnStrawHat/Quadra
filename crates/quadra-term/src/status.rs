//! OSC 7501, the program status protocol (rev 0.3; specs/18, "Signals an app reads"): a
//! program says what it is doing, the session keeps one record per id and tells the app the
//! strongest. A report is untrusted: one that breaks a rule is dropped whole.

/// What a program says it is doing, weakest first: the summary is the strongest record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProgramState {
    #[default]
    Idle,
    Done,
    Working,
    Error,
    Blocked,
}

const RECORDS: usize = 64;

/// The records of one terminal, by id, each with the tick of its last report.
#[derive(Debug, Default)]
pub(crate) struct Statuses {
    records: Vec<(String, ProgramState, u64)>,
    tick: u64,
}

/// What a report asked for.
#[derive(Debug, PartialEq)]
pub(crate) enum Report {
    /// `?`: the terminal says it listens.
    Query,
    /// The records changed; the new summary.
    Changed(ProgramState),
    /// Nothing changed: a report that broke a rule.
    Dropped,
}

impl Statuses {
    /// Takes the body of an OSC 7501 (what follows `7501;`).
    pub fn report(&mut self, body: &[u8]) -> Report {
        if body == b"?" {
            return Report::Query;
        }
        let Ok(body) = std::str::from_utf8(body) else { return Report::Dropped };
        if body.len() > 4096 - 9 {
            return Report::Dropped;
        }
        let (mut state, mut id, mut title, mut msg) = ("", "", "", "");
        for pair in body.split(':') {
            let Some((key, value)) = pair.split_once('=') else { return Report::Dropped };
            let fits = |b: u8| b.is_ascii_alphanumeric() || b"_.,+/=-".contains(&b);
            let keyed = (1..=16).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_lowercase());
            if !keyed || !value.bytes().all(fits) {
                return Report::Dropped;
            }
            match key {
                "state" => state = value,
                "id" => id = value,
                "title" => title = value,
                "msg" => msg = value,
                _ => {}
            }
        }
        let st = match state {
            "idle" | "clear" => ProgramState::Idle,
            "working" => ProgramState::Working,
            "done" => ProgramState::Done,
            "blocked" => ProgramState::Blocked,
            "error" => ProgramState::Error,
            _ => return Report::Dropped,
        };
        if (!id.is_empty() && !valid_id(id)) || !valid_text(title, 192) || !valid_text(msg, 2048) {
            return Report::Dropped;
        }
        if state == "clear" {
            let under = format!("{id}/");
            self.records.retain(|(at, ..)| !id.is_empty() && at != id && !at.starts_with(&under));
        } else {
            self.tick += 1;
            match self.records.iter_mut().find(|(at, ..)| at == id) {
                Some(record) => (record.1, record.2) = (st, self.tick),
                None => {
                    if self.records.len() >= RECORDS
                        && let Some(oldest) = (0..self.records.len()).min_by_key(|&i| self.records[i].2)
                    {
                        self.records.remove(oldest);
                    }
                    self.records.push((id.to_string(), st, self.tick));
                }
            }
        }
        Report::Changed(self.summary())
    }

    /// OSC 133 A, a new prompt: what was working, waiting or idle is over. True when that
    /// changed the records.
    pub fn prompt(&mut self) -> bool {
        let before = self.records.len();
        self.records.retain(|(_, st, _)| matches!(st, ProgramState::Done | ProgramState::Error));
        self.records.len() != before
    }

    pub fn summary(&self) -> ProgramState {
        self.records.iter().map(|(_, st, _)| *st).max().unwrap_or_default()
    }

    /// The id of the record the summary is, the latest of the strongest: what it is about
    /// (`deploy`); empty when it has none.
    pub fn named(&self) -> String {
        let strongest = self.records.iter().max_by_key(|(_, st, tick)| (*st, *tick));
        strongest.map_or_else(String::new, |(id, ..)| id.clone())
    }
}

/// Up to 8 segments split by `/`, each 1 to 32 of letters, digits and `_.+-`; 128 bytes.
fn valid_id(id: &str) -> bool {
    let segment = |s: &str| {
        (1..=32).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.+-".contains(&b))
    };
    id.len() <= 128 && id.split('/').count() <= 8 && id.split('/').all(segment)
}

/// Base64 of at most `max` bytes of UTF-8 without control characters; empty is valid.
fn valid_text(text: &str, max: usize) -> bool {
    if text.is_empty() {
        return true;
    }
    if text.len() > max.div_ceil(3) * 4 {
        return false;
    }
    match decode_base64(text) {
        Some(bytes) if !bytes.is_empty() && bytes.len() <= max => match String::from_utf8(bytes) {
            Ok(s) => s.chars().all(|c| c >= ' ' && !('\u{7f}'..'\u{a0}').contains(&c)),
            Err(_) => false,
        },
        _ => false,
    }
}

/// Standard base64 with optional padding; `None` on any other byte.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let value = |b: u8| match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let (mut out, mut acc, mut bits) = (Vec::new(), 0u32, 0);
    for b in text.trim_end_matches('=').bytes() {
        acc = acc << 6 | u32::from(value(b)?);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_by_id_and_the_strongest_is_the_summary() {
        let mut s = Statuses::default();
        assert_eq!(s.report(b"state=working:id=build"), Report::Changed(ProgramState::Working));
        assert_eq!(s.report(b"state=blocked:id=deploy/push"), Report::Changed(ProgramState::Blocked));
        assert_eq!(s.named(), "deploy/push");
        assert_eq!(s.report(b"state=done:id=deploy/push"), Report::Changed(ProgramState::Working));
        assert_eq!(s.named(), "build");
        // `clear` of an id takes what is under it too; of none, everything.
        assert_eq!(s.report(b"state=clear:id=deploy"), Report::Changed(ProgramState::Working));
        assert_eq!(s.report(b"state=clear"), Report::Changed(ProgramState::Idle));
        assert_eq!(s.report(b"?"), Report::Query);
    }

    #[test]
    fn a_report_that_breaks_a_rule_is_dropped_whole() {
        let mut s = Statuses::default();
        assert_eq!(s.report(b"state=sleeping"), Report::Dropped);
        assert_eq!(s.report(b"state=working:id=a//b"), Report::Dropped);
        assert_eq!(s.report(b"state=working:title=!!"), Report::Dropped);
        // "aGk=" is "hi": a title is checked, then let go.
        assert_eq!(s.report(b"state=error:title=aGk="), Report::Changed(ProgramState::Error));
        // A title with a control character in it ("\x01").
        assert_eq!(s.report(b"state=done:title=AQ=="), Report::Dropped);
        assert_eq!(s.summary(), ProgramState::Error);
    }

    #[test]
    fn a_prompt_ends_what_was_working_and_keeps_what_ended() {
        let mut s = Statuses::default();
        s.report(b"state=working:id=a");
        s.report(b"state=done:id=b");
        assert!(s.prompt() && s.summary() == ProgramState::Done);
        assert!(!s.prompt());
        for n in 0..70 {
            s.report(format!("state=working:id=n{n}").as_bytes());
        }
        assert_eq!(s.records.len(), 64); // the oldest went first
    }
}
