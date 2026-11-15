//! A block of code in a document: coloured by the tokenizer its language names.

use quadra_code::{TokenKind, role, tokenizer_for};
use quadra_design::{fonts, size, token};
use quadra_reactive::signal;
use quadra_text::find_urls;
use quadra_ui::{Div, column, div, scroll_x, span};

use super::{CODE, OnLink, rich};

/// A run of code: of a kind of token or of none, and a link when it is in a URL.
#[derive(Debug, PartialEq)]
struct Piece {
    text: String,
    kind: Option<TokenKind>,
    link: Option<String>,
}

/// The code as pieces, its lines read in order by the tokenizer of `lang`. A URL is a link
/// whatever tokens it is in, and keeps their colours; what has no colour and is no link is
/// one piece, ends of lines and all.
fn pieces(lang: &str, source: &str) -> Vec<Piece> {
    let (tokenizer, mut state, mut tokens) = (tokenizer_for(lang), 0, Vec::new());
    let (mut out, mut plain) = (Vec::new(), String::new());
    for (n, line) in source.split('\n').enumerate() {
        plain.extend((n > 0).then_some('\n'));
        tokens.clear();
        state = tokenizer.tokenize(line, state, &mut tokens);
        let (urls, mut url, mut at) = (find_urls(line), 0, 0);
        // The tokens and what is between them, then what is after the last.
        let ends =
            tokens.iter().map(|t| (t.start, t.end, Some(t.kind))).chain([(line.len(), line.len(), None)]);
        for (to, kind) in ends.flat_map(|(start, end, kind)| [(start, None), (end, kind)]) {
            while at < to {
                while urls.get(url).is_some_and(|u| u.range.end <= at) {
                    url += 1;
                }
                let (end, link) = match urls.get(url) {
                    Some(u) if u.range.start <= at => (u.range.end.min(to), Some(&u.url)),
                    Some(u) => (u.range.start.min(to), None),
                    None => (to, None),
                };
                if kind.is_none() && link.is_none() {
                    plain.push_str(&line[at..end]);
                } else {
                    out.extend((!plain.is_empty()).then(|| Piece {
                        text: std::mem::take(&mut plain),
                        kind: None,
                        link: None,
                    }));
                    out.push(Piece { text: line[at..end].to_string(), kind, link: link.cloned() });
                }
                at = end;
            }
        }
    }
    out.extend((!plain.is_empty()).then_some(Piece { text: plain, kind: None, link: None }));
    out
}

/// A block of code: the tone of the editor, a hairline, the code face. It scrolls sideways.
pub fn code_box(lang: &str, source: &str, on_link: &Option<OnLink>) -> Div {
    let run = |piece: Piece| {
        let run = span(piece.text);
        let run = piece.kind.map(role).into_iter().fold(run, |run, role| run.color(role));
        piece.link.iter().fold(run, |run, url| run.link(url))
    };
    let runs: Vec<_> = pieces(lang, &source.replace('\t', "    ")).into_iter().map(run).collect();
    let frame = column().padding(12.0).radius(token::RADIUS_LG).bg(token::CANVAS).border(1.0);
    let frame = frame
        .border_color(token::LINE_STRONG)
        .font_family(fonts::MONO)
        .text_size(CODE)
        .line_height(size::LINE);
    if runs.is_empty() { frame } else { frame.child(scroll_x(signal(0.0), div().child(rich(runs, on_link)))) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_is_cut_at_its_tokens_and_at_its_urls() {
        let source = "x = 1 # at http://a.b/c now\ncurl https://d.e/f?g=1\n";
        let got = pieces("sh", source);
        assert_eq!(got.iter().map(|p| p.text.as_str()).collect::<String>(), source);
        let comment = |text: &str, link: Option<&str>| Piece {
            text: text.into(),
            kind: Some(TokenKind::Comment),
            link: link.map(str::to_string),
        };
        let at = got.iter().position(|p| p.text == "# at ").expect("the comment");
        assert_eq!(
            got[at..at + 3],
            [comment("# at ", None), comment("http://a.b/c", Some("http://a.b/c")), comment(" now", None)]
        );
        // A URL the tokenizer reads as several tokens is one link all the same.
        let linked: String = got
            .iter()
            .filter(|p| p.link.as_deref() == Some("https://d.e/f?g=1"))
            .map(|p| p.text.as_str())
            .collect();
        assert_eq!(linked, "https://d.e/f?g=1");
        // A language no tokenizer knows: one piece.
        assert_eq!(pieces("nonsense", "a b\nc").len(), 1);
    }
}
