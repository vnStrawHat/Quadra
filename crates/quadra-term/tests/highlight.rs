//! Semantic highlighting (specs/18, "The view"): text in the default foreground takes the
//! colour of what it means, a program's colours stay, prompts are told by OSC 133 or by their
//! look, and a paint scans only the lines that changed.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_prim::Color;
use quadra_reactive::{Signal, signal};
use quadra_render::{Kind, Step};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Highlight, Session, TermColors, TermLook, TerminalWidget, terminal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, Role};
use quadra_view::column;

const FG: Color = Color::rgb(0xC0C0C0);
const RED: Color = Color::rgb(0xE00000);
const GREEN: Color = Color::rgb(0x00E000);
const BLUE: Color = Color::rgb(0x0000E0);
const PINK: Color = Color::rgb(0xE000E0);
const YELLOW: Color = Color::rgb(0xE0E000);
const LIGHT_BLUE: Color = Color::rgb(0x40A0FF);
const ORANGE: Color = Color::rgb(0xFF8000);
const SIGN: Color = Color::rgb(0x8080FF);

fn colors() -> TermColors {
    let mut colors = TermColors { foreground: Some(FG), ..TermColors::default() };
    for (slot, color) in [(1, RED), (2, GREEN), (4, BLUE), (5, PINK), (11, YELLOW), (12, LIGHT_BLUE)] {
        colors.ansi[slot] = Some(color);
    }
    colors.highlights[Highlight::Number.index()] = Some(ORANGE);
    colors.highlights[Highlight::PromptSign.index()] = Some(SIGN);
    colors
}

fn term(highlighting: bool) -> (Harness, Session, Signal<TermLook>, NodeId) {
    let mut h = Harness::new();
    let look = signal(TermLook {
        size: 10.0,
        line_height: 1.4,
        highlighting,
        colors: colors(),
        ..TermLook::default()
    });
    let slot = Rc::new(RefCell::new(None));
    let out = slot.clone();
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).look(look))
    });
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    let node = h.query(|n| n.semantics.role == Role::TextInput)[0];
    (h, s, look, node)
}

/// The instances of `kind` drawn, a row after another from the left: their bounds and colour.
fn drawn(h: &mut Harness, kind: Kind) -> Vec<([f32; 4], Color)> {
    h.frame();
    let scene = h.tree.scene();
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        Step::Layer { id, .. } => Some(scene.layer(*id).instances()),
        _ => None,
    });
    let color = |c: [f32; 4]| Color { r: c[0], g: c[1], b: c[2], a: c[3] };
    let mut out: Vec<_> =
        layers.flatten().filter(|i| i.kind == kind as u32).map(|i| (i.bounds, color(i.color))).collect();
    out.sort_by(|a, b| a.0[1].total_cmp(&b.0[1]).then(a.0[0].total_cmp(&b.0[0])));
    out
}

/// The colours of the glyphs, a row after another.
fn inks(h: &mut Harness) -> Vec<Color> {
    drawn(h, Kind::Mono).into_iter().map(|it| it.1).collect()
}

fn feed(h: &mut Harness, s: &Session, bytes: &[u8]) {
    s.feeder().feed(bytes, Instant::now());
    h.settle();
}

const LINE: &[u8] = b"error ok /usr 42 \x1b[35mfailed\x1b[0m";

#[test]
fn what_text_means_takes_its_colour_and_a_programs_colour_stays() {
    let (mut h, s, look, _) = term(true);
    feed(&mut h, &s, LINE);
    let mut want = vec![RED; 5];
    want.extend([GREEN; 2]);
    want.extend([BLUE; 4]);
    want.extend([ORANGE; 2]);
    want.extend([PINK; 6]);
    assert_eq!(inks(&mut h), want, "error, ok, a path, a number in the look's colour, a coloured word");
    look.update(|it| it.highlighting = false);
    h.settle();
    let mut plain = vec![FG; 13];
    plain.extend([PINK; 6]);
    assert_eq!(inks(&mut h), plain, "off: as the program said");
}

#[test]
fn a_url_is_coloured_and_underlined() {
    let (mut h, s, _, _) = term(true);
    feed(&mut h, &s, b"see https://x.test");
    assert_eq!(inks(&mut h)[3..], [LIGHT_BLUE; 14]);
    let lines =
        drawn(&mut h, Kind::Quad).into_iter().filter(|(b, c)| b[3] == 1.0 && *c == LIGHT_BLUE).count();
    assert_eq!(lines, 14, "a line under each cell");
}

/// A first line, then a prompt with `ls -l` typed, its output, a status, and the next prompt.
fn session(status: u8) -> Vec<u8> {
    let mut bytes = b"start\r\n\x1b]133;A\x07me$ \x1b]133;B\x07ls -l\r\n\x1b]133;C\x07out\r\n".to_vec();
    bytes.extend(format!("\x1b]133;D;{status}\x07\x1b]133;A\x07me$ \x1b]133;B\x07").as_bytes());
    bytes
}

#[test]
fn a_prompt_by_its_marks_and_the_status_of_what_it_ran() {
    let (mut h, s, _, _) = term(true);
    feed(&mut h, &s, &session(1));
    let start = vec![FG; 5];
    let prompt = [FG, FG, RED, YELLOW, YELLOW, PINK, PINK];
    let after = [FG, FG, FG, FG, FG, SIGN];
    let want: Vec<Color> = start.into_iter().chain(prompt).chain(after).collect();
    assert_eq!(inks(&mut h), want, "the sign of the prompt that ran it tinted by the status 1");

    let (mut h, s, _, _) = term(true);
    feed(&mut h, &s, &session(0));
    assert_eq!(inks(&mut h)[7], GREEN, "0 is success");
}

#[test]
fn a_prompt_by_its_look() {
    let (mut h, s, _, _) = term(true);
    feed(&mut h, &s, b"me@box:~/src$ git log");
    let mut want = vec![FG; 7];
    want.extend([BLUE; 5]);
    want.push(SIGN);
    want.extend([YELLOW; 3]);
    want.extend([FG; 3]);
    assert_eq!(inks(&mut h), want, "the path, the sign, the command");
}

#[test]
fn a_paint_scans_only_the_lines_that_changed() {
    let (mut h, s, _, node) = term(true);
    let scanned = |h: &mut Harness| h.tree.widget_mut::<TerminalWidget>(node).unwrap().lines_scanned();
    let first = scanned(&mut h);
    feed(&mut h, &s, b"one 1\r\ntwo 2\r\nthree 3\r\n");
    h.frame();
    let fed = scanned(&mut h);
    assert_eq!(fed, first + 3, "the three lines written");
    h.tree.mark_paint(node);
    h.frame();
    assert_eq!(scanned(&mut h), fed, "nothing changed");
    feed(&mut h, &s, b"four");
    h.frame();
    assert_eq!(scanned(&mut h), fed + 1, "the line written");
}
