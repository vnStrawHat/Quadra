//! The log's store and listing (specs/13, "Data").

use super::*;

fn entries(from: usize, n: usize) -> Vec<(i64, String)> {
    let entry = |i: usize| match i % 10 {
        3 => (i as i64 * 1000, format!("\u{1b}[31mERROR\u{1b}[0m request {i} failed")),
        _ => (i as i64 * 1000, format!("INFO request {i} ok")),
    };
    (from..from + n).map(entry).collect()
}

#[test]
fn lines_are_packed_and_read_back() {
    let mut store = Store::default();
    for (at, text) in [(5, "one"), (6, ""), (7, "thrée")] {
        store.push(at, text, 0, None);
    }
    assert_eq!((store.len(), store.line(0), store.line(1), store.line(2)), (3, "one", "", "thrée"));
    assert_eq!((store.time(2), clock(1_793_966_400_000 + 3_723_045)), (7, "13:02:03.045".to_string()));
}

#[test]
fn a_filter_hides_or_dims_by_a_text_or_a_pattern() {
    let log = Log::new();
    log.append(entries(0, 100));
    assert_eq!((log.len(), log.listed()), (100, 100));
    // Whatever the case, and not in the escape sequences.
    assert!(log.set_filter("error", false, false));
    assert_eq!((log.listed(), log.line(0).map(|line| line.0)), (10, Some(3)));
    assert!(log.set_filter("31m", false, false) && log.listed() == 0);
    // Lines that come are tested as they come.
    log.set_filter("error", false, false);
    log.append(entries(100, 14));
    assert_eq!((log.len(), log.listed(), log.line(10).map(|line| line.0)), (114, 12, Some(103)));
    // A pattern: 15, 25 .. 105 (a digit before the 5).
    assert!(log.set_filter(r"request \d+5 ", true, false));
    assert_eq!(log.listed(), 10);
    // A text is a text, not a pattern: its dot is a dot.
    assert!(log.set_filter("request 1.", false, false) && log.listed() == 0);
    // Dimmed, every line is listed, and those that do not match are faint.
    log.set_filter("error", false, true);
    assert_eq!((log.listed(), log.line(3).unwrap().3, log.line(4).unwrap().3), (114, false, true));
    // A pattern that is not one filters nothing.
    assert!(!log.set_filter("(", true, false));
    assert_eq!((log.listed(), log.line(500)), (114, None));
    log.clear();
    assert_eq!((log.len(), log.listed(), log.line(0)), (0, 0, None));
}

#[test]
fn past_its_limit_the_oldest_lines_go_an_eighth_at_a_time() {
    let log = Log::new().limit(80);
    log.append(entries(0, 90));
    assert_eq!(log.len(), 90); // ten over: not yet an eighth
    log.set_filter("error", false, false);
    log.push(90_000, "ERROR the last");
    // Eleven over: they go, and the filter's lines are where they were.
    assert_eq!((log.len(), log.line(0).map(|l| l.0)), (80, Some(2)));
    assert_eq!(log.line(0).map(|l| l.1), Some(13_000));
    assert_eq!(log.line(log.listed() - 1).map(|l| l.2), Some("ERROR the last".to_string()));
}

#[test]
fn streams_that_come_interleaved_read_in_time_order_each_line_with_its_source() {
    let log = Log::new();
    log.append_from(0, [(1000, "api one"), (3000, "api three"), (5000, "api five")]);
    assert_eq!(log.sources(), 1);
    log.set_filter("o", false, false);
    log.append_from(1, [(2000, "db two"), (4000, "db four"), (6000, "db six")]);
    let said: Vec<String> = (0..log.listed()).filter_map(|p| log.line(p).map(|l| l.2)).collect();
    assert_eq!(said, ["api one", "db two", "db four"]);
    assert_eq!((log.source(0), log.source(1), log.sources()), (0, 1, 2));
    log.set_filter("", false, false);
    let all: Vec<String> = (0..log.listed()).filter_map(|p| log.line(p).map(|l| l.2)).collect();
    assert_eq!(all, ["api one", "db two", "api three", "db four", "api five", "db six"]);
    // A line with no time goes at the end.
    log.push(0, "no time");
    assert_eq!(log.line(6).map(|l| l.2).as_deref(), Some("no time"));
}

#[test]
fn levels_are_a_set_fatal_counted_as_error_and_matches_are_marked() {
    let log = Log::new();
    assert!(!log.has_levels());
    let lines = ["INFO up", "DEBUG cache warm", "WARN slow", "ERROR down", "FATAL gone", "plain"];
    log.append(lines.iter().enumerate().map(|(i, l)| (i as i64 + 1, *l)));
    assert!(log.has_levels());
    assert_eq!((log.level(4), Level::Fatal.word()), (Level::Fatal, "FATAL"));
    log.set_levels(&[Level::Error, Level::Debug]);
    let said: Vec<String> = (0..log.listed()).filter_map(|p| log.line(p).map(|l| l.2)).collect();
    assert_eq!(said, ["DEBUG cache warm", "ERROR down", "FATAL gone"]);
    log.set_level(Level::Warn);
    assert_eq!(log.listed(), 3);
    log.set_levels(&[]);
    assert_eq!(log.listed(), 6);
    // Marks: where the text is, in what a line says without its escapes, any case.
    log.push(10, "\u{1b}[31mError\u{1b}[0m: Gone, gone");
    log.set_filter("gone", false, false);
    assert_eq!((log.marks(0).len(), log.marks(0).first()), (1, Some(&(6..10))));
    assert_eq!(log.marks(1), vec![7..11, 13..17]);
}

#[test]
fn a_level_given_on_append_wins_and_a_json_line_says_its_own() {
    let log = Log::new();
    log.append_leveled(0, [(1, Level::Info, "Error budget is fine"), (2, Level::Warn, "plain")]);
    assert_eq!((log.level(0), log.level(1)), (Level::Info, Level::Warn));
    // A JSON line: its level field before a word in capitals in its message.
    log.push(3, r#"{"msg":"ERROR budget is fine","level":"info"}"#);
    log.push(4, r#"{"msg":"no level field","note":"WARN"}"#);
    assert_eq!((log.level(2), log.level(3)), (Level::Info, Level::Warn));
}
