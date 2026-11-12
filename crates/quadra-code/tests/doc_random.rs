//! The document under commands at random (specs/13, "Code"): whatever is done with several
//! cursors, they stay in the text, and all of it is undone and done again.

use quadra_code::{Doc, Move};
use quadra_testing::fuzz::Rng;

const MOVES: [Move; 12] = [
    Move::Left,
    Move::Right,
    Move::Up,
    Move::Down,
    Move::WordLeft,
    Move::WordRight,
    Move::Home,
    Move::End,
    Move::DocStart,
    Move::DocEnd,
    Move::PageUp,
    Move::PageDown,
];

#[test]
fn whatever_is_done_is_undone_and_cursors_stay_in_the_text() {
    let start =
        "fn main() {\n\tlet tiếng = \"việt\";\n    // 漢字 😀 e\u{301}\n}\n\nmore (text [here])\nfn main\n";
    let mut rng = Rng::new(5);
    for round in 0..60 {
        let mut d = Doc::new(start);
        for step in 0..120 {
            d.now += rng.below(700) as u64;
            let (extend, flag) = (rng.chance(30), rng.chance(50));
            let (was, command) = ((d.text(), d.cursors.clone()), rng.below(26));
            match command {
                0..=7 => d.move_cursors(*rng.pick(&MOVES), extend, 2),
                8..=10 => d.insert_text(rng.pick(&["a", " ", "é", "(", "x\ny", "漢", ""]), flag),
                11 => d.newline(),
                12 => d.delete_back(flag),
                13 => d.delete_forward(flag),
                14 => d.tab(),
                15 => d.indent_lines(flag),
                16 => d.delete_lines(),
                17 => d.duplicate_lines(),
                18 => d.move_lines(flag),
                19 => d.toggle_comment(),
                20 => d.select_line(),
                21 => d.add_cursor_vertical(flag),
                22 => d.select_next(),
                23 => d.paste(rng.pick(&["one", "one\ntwo", "a\r\nb\r\nc"])),
                24 => drop(d.cut()),
                _ => {
                    let text = d.text();
                    let at = |rng: &mut Rng| text.floor_char_boundary(rng.below(text.len() + 1));
                    d.set_cursor(at(&mut rng), at(&mut rng), flag);
                }
            }
            // The cursors are in the text, between characters, in order, none over another.
            let text = d.text();
            assert!(!d.cursors.is_empty(), "round {round}, step {step}");
            for (i, c) in d.cursors.iter().enumerate() {
                assert!(
                    c.end() <= text.len() && text.is_char_boundary(c.anchor) && text.is_char_boundary(c.head),
                    "round {round}, step {step}: {c:?} after command {command} ({flag}) on {was:?}: {text:?}"
                );
                assert!(
                    i == 0 || d.cursors[i - 1].end() <= c.start(),
                    "round {round}, step {step}: {:?} after command {command} ({flag}) on {was:?}",
                    d.cursors
                );
            }
        }
        // Everything is taken back to the text it started as, and done again to what it was.
        let end = d.text();
        while d.undo() {}
        assert_eq!(d.text(), start, "round {round}");
        while d.redo() {}
        assert_eq!(d.text(), end, "round {round}");
    }
}
