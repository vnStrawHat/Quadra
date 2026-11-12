//! Sequence diagrams: what a source is read as, and where a layout puts things.

use quadra_prim::Rect;
use quadra_text::{FakeBackend, TextStyle, TextSystem};

use super::{Item, MOST_ACTORS, MOST_ITEMS, Seq, Side, layout};
use crate::mermaid::draw::{Head, Ink, Measure, Op};
use crate::mermaid::{Chart, read};

fn seq(source: &str) -> Seq {
    match read(source) {
        Ok(Chart::Seq(seq)) => seq,
        Ok(_) => panic!("no sequence diagram"),
        Err(fault) => panic!("{fault}"),
    }
}

fn fault(source: &str) -> String {
    read(source).err().expect("a fault")
}

#[test]
fn participants_and_messages() {
    let s = seq(
        "sequenceDiagram\nparticipant A as Alice\nactor B as Bob\nA->>B: hi\nB-->>A: back\nA->B: plain\nA-->B: dashed\nA-xB: lost\nA-)C: async\nA->>+B: activate\nactivate B",
    );
    assert_eq!(s.actors.iter().map(|actor| actor.label.as_str()).collect::<Vec<_>>(), ["Alice", "Bob", "C"]);
    assert_eq!(s.actors.iter().map(|actor| actor.figure).collect::<Vec<_>>(), [false, true, false]);
    let message = |k: usize| match &s.items[k] {
        Item::Message { from, to, text, dashed, head } => (*from, *to, text.as_str(), *dashed, *head),
        other => panic!("{other:?}"),
    };
    assert_eq!(s.items.len(), 7);
    assert_eq!(message(0), (0, 1, "hi", false, Head::Arrow));
    assert_eq!(message(1), (1, 0, "back", true, Head::Arrow));
    assert_eq!((message(2).3, message(2).4), (false, Head::None));
    assert_eq!((message(3).3, message(3).4), (true, Head::None));
    assert_eq!((message(4).4, message(5).4, message(5).1), (Head::Cross, Head::Open, 2));
    assert_eq!(message(6), (0, 1, "activate", false, Head::Arrow));
}

#[test]
fn notes_and_frames() {
    let s = seq(
        "sequenceDiagram\nautonumber\nNote left of A: l\nNote right of A: r\nNote over B,A: both\nloop every s\nA->>B: x\nalt ok\nB->>A: y\nelse bad\nB->>A: z\nend\nend\nopt maybe\nA->>A: self\nend",
    );
    assert!(s.autonumber);
    let notes = s.items.iter().filter_map(|item| match item {
        Item::Note { from, to, side, .. } => Some((*from, *to, *side)),
        _ => None,
    });
    // A note over two is from the one at the left to the one at the right.
    assert_eq!(notes.collect::<Vec<_>>(), [(0, 0, Side::Left), (0, 0, Side::Right), (0, 1, Side::Over)]);
    let frames = s.items.iter().filter_map(|item| match item {
        Item::Open { word, text } => Some(format!("{word} {text}")),
        Item::Branch { text } => Some(format!("- {text}")),
        Item::Close => Some("end".to_string()),
        _ => None,
    });
    assert_eq!(
        frames.collect::<Vec<_>>(),
        ["loop every s", "alt ok", "- bad", "end", "end", "opt maybe", "end"]
    );
}

#[test]
fn frames_never_closed_end_and_faults_say_their_line() {
    assert_eq!(seq("sequenceDiagram\nloop forever\nA->>B: x").items.last(), Some(&Item::Close));
    // A box is read and not drawn: its end closes nothing that is.
    assert_eq!(seq("sequenceDiagram\nrect rgb(0,0,0)\nA->>B: x\nend").items.len(), 1);
    assert_eq!(
        fault("sequenceDiagram\nA->>B: ok\nthis is not a message"),
        "line 3: cannot read `this is not a message`"
    );
    assert_eq!(fault("sequenceDiagram\nA->>: ok"), "line 2: a message needs a receiver: `A->>: ok`");
    assert!(fault("sequenceDiagram\nNote under A: no").starts_with("line 2: a note is"));
    // What a diagram may hold has an end.
    let many: String = (0..=MOST_ACTORS).map(|k| format!("participant p{k}\n")).collect();
    assert_eq!(fault(&format!("sequenceDiagram\n{many}")), "line 202: more than 200 participants");
    let long = "A->>B: x\n".repeat(MOST_ITEMS + 1);
    assert_eq!(fault(&format!("sequenceDiagram\n{long}")), "more than 5000 messages, notes and frames");
}

#[test]
fn columns_are_as_far_apart_as_what_is_between_them_and_frames_hold_their_messages() {
    let s = seq(
        "sequenceDiagram\nparticipant A\nparticipant B\nparticipant C\nA->>C: a rather long message label that needs room\nloop again\nB->>C: inner\nend\nNote over A,B: n",
    );
    let mut ts = TextSystem::new(Box::new(FakeBackend::new()));
    let d = layout::layout(
        &s,
        &mut Measure { ts: &mut ts, base: TextStyle { size: 14.0, ..TextStyle::default() } },
    );
    // The boxes of the participants: three at the top and three at the bottom, in order.
    let boxes = d.ops.iter().filter_map(|op| match op {
        Op::Box { rect, radius, fill: Ink::Panel, .. } if *radius == 4.0 => Some(*rect),
        _ => None,
    });
    let boxes: Vec<Rect> = boxes.collect();
    assert_eq!(boxes.len(), 6);
    let tops: Vec<Rect> = boxes.iter().copied().filter(|rect| rect.y == boxes[0].y).collect();
    assert!(tops.len() == 3 && tops[0].x < tops[1].x && tops[1].x < tops[2].x);
    // The long label fits between A and C: 47 letters of the test font at 0.92 of 14 px.
    let label = 47.0 * 0.6 * 14.0 * 0.92;
    assert!(tops[2].center().x - tops[0].center().x > label);
    // The frame of the loop holds the line of its message.
    let frame = d.ops.iter().find_map(|op| match op {
        Op::Box { rect, fill: Ink::Frame, .. } => Some(*rect),
        _ => None,
    });
    let lines = d.ops.iter().filter_map(|op| match op {
        Op::Line { points, ink: Ink::Accent, .. } => Some(points),
        _ => None,
    });
    let (frame, inner) = (frame.expect("a frame"), lines.collect::<Vec<_>>()[1]);
    assert!(inner.iter().all(|p| frame.contains(*p)), "{inner:?} in {frame:?}");
    assert!(d.width > frame.right() && d.height > boxes[5].bottom());
}
