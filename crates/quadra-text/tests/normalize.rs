//! Normalization for the Latin blocks (specs/08): marks compose onto their letters in either
//! order, a search folds them away, and a mark typed after a letter joins it.

use quadra_text::{EditBuffer, fold_diacritics, normalize_nfc};

#[test]
fn marks_compose_onto_their_letter_in_either_order() {
    // e + circumflex + dot below, in either order, is one letter.
    assert_eq!(normalize_nfc("Vie\u{302}\u{323}t"), "Vi\u{1EC7}t");
    assert_eq!(normalize_nfc("Vie\u{323}\u{302}t"), "Vi\u{1EC7}t");
    assert_eq!(normalize_nfc("Tie\u{302}\u{301}ng"), "Ti\u{1EBF}ng");
    assert_eq!(normalize_nfc("a\u{300}"), "\u{E0}");
    // What is composed already, and what has no accents, is given back as it is.
    for same in ["Vi\u{1EC7}t Nam", "plain ascii", "\u{1F600} \u{4E2D}"] {
        assert!(matches!(normalize_nfc(same), std::borrow::Cow::Borrowed(_)), "{same}");
    }
    // No composed form: the mark stays where it is.
    assert_eq!(normalize_nfc("x\u{301}"), "x\u{301}");
}

#[test]
fn a_search_is_typed_without_accents() {
    assert_eq!(fold_diacritics("H\u{1EA3}i Ph\u{F2}ng"), "Hai Phong");
    assert_eq!(fold_diacritics("\u{110}\u{E0} N\u{1EB5}ng, Vi\u{1EC7}t Nam"), "Da Nang, Viet Nam");
    assert_eq!(fold_diacritics("e\u{323}\u{302}"), "e");
    assert_eq!(fold_diacritics("\u{1EC7}"), "e");
}

#[test]
fn a_mark_typed_after_a_letter_joins_it() {
    let mut field = EditBuffer::new("Vie");
    field.insert("\u{302}"); // the circumflex, then the dot below: as a keyboard sends them
    assert_eq!((field.text(), field.head()), ("Vi\u{EA}", 4));
    field.insert("\u{323}");
    field.insert("t");
    assert_eq!(field.text(), "Vi\u{1EC7}t");
    // Text that comes decomposed is composed where it lands.
    let mut pasted = EditBuffer::new("");
    pasted.insert("Tie\u{302}\u{301}ng Vie\u{323}\u{302}t");
    assert_eq!(pasted.text(), "Ti\u{1EBF}ng Vi\u{1EC7}t");
}
