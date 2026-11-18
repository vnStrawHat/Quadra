//! What the Unicode name of a drawn character says it is.

/// What a name says is drawn.
#[derive(Debug)]
pub(super) enum Says {
    /// Up, right, down, left: 0 none, 1 light, 2 heavy, 3 double; and whether it is an arc.
    Arms([u8; 4], bool),
    /// So many dashes of a weight, across (or down).
    Dashes(usize, u8, bool),
    /// The diagonals: lower left to upper right, upper left to lower right.
    Diagonal(bool, bool),
    /// Parts of the box filled, as fractions of it: x0, y0, x1, y1.
    Blocks(Vec<[f64; 4]>),
    Shade(f64),
    /// A separator pointing right (or left): solid or a chevron.
    Separator(bool, bool),
}

pub(super) fn says(name: &str) -> Says {
    if let Some(rest) = name.strip_prefix("BOX DRAWINGS ") {
        return boxes(rest);
    }
    let words: Vec<&str> = name.split(' ').collect();
    match words[..] {
        [kind, "SHADE"] => Says::Shade(match kind {
            "LIGHT" => 0.25,
            "MEDIUM" => 0.5,
            _ => 0.75,
        }),
        ["SOLID" | "RIGHT" | "LEFT", ..] if !name.ends_with("BLOCK") => {
            Says::Separator(name.contains("RIGHT"), name.starts_with("SOLID"))
        }
        ["QUADRANT", ..] => Says::Blocks(
            name["QUADRANT ".len()..]
                .split(" AND ")
                .map(|it| {
                    let (x, y) = (
                        f64::from(u8::from(it.ends_with("RIGHT"))),
                        f64::from(u8::from(it.starts_with("LOWER"))),
                    );
                    [x / 2.0, y / 2.0, x / 2.0 + 0.5, y / 2.0 + 0.5]
                })
                .collect(),
        ),
        ["FULL", "BLOCK"] => Says::Blocks(vec![[0.0, 0.0, 1.0, 1.0]]),
        [side, .., "BLOCK"] => {
            let count = |w: &str| {
                ["", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN"].iter().position(|it| *it == w)
            };
            let a = match words[1..words.len() - 1] {
                ["HALF"] => 0.5,
                [n, "QUARTER" | "QUARTERS"] => count(n).expect("a number") as f64 / 4.0,
                [n, "EIGHTH" | "EIGHTHS"] => count(n).expect("a number") as f64 / 8.0,
                _ => panic!("{name}"),
            };
            Says::Blocks(vec![match side {
                "UPPER" => [0.0, 0.0, 1.0, a],
                "LOWER" => [0.0, 1.0 - a, 1.0, 1.0],
                "LEFT" => [0.0, 0.0, a, 1.0],
                "RIGHT" => [1.0 - a, 0.0, 1.0, 1.0],
                _ => panic!("{name}"),
            }])
        }
        _ => panic!("a name the oracle does not read: {name}"),
    }
}

fn boxes(rest: &str) -> Says {
    let words: Vec<&str> = rest.split(' ').collect();
    let weight = |w: &str| match w {
        "LIGHT" | "SINGLE" => Some(1),
        "HEAVY" => Some(2),
        "DOUBLE" => Some(3),
        _ => None,
    };
    if let Some(at) = words.iter().position(|w| *w == "DASH") {
        let n =
            ["DOUBLE", "TRIPLE", "QUADRUPLE"].iter().position(|it| *it == words[at - 1]).expect("dashes") + 2;
        return Says::Dashes(n, weight(words[0]).expect("a weight"), words[at + 1] == "HORIZONTAL");
    }
    if words.contains(&"DIAGONAL") {
        let cross = rest.ends_with("CROSS");
        return Says::Diagonal(
            cross || rest.contains("UPPER RIGHT TO LOWER LEFT"),
            cross || rest.contains("UPPER LEFT TO LOWER RIGHT"),
        );
    }
    let (mut arms, mut last) = ([0u8; 4], None);
    for clause in rest.split(" AND ") {
        let w = clause.split(' ').find_map(weight).or(last).expect("a weight");
        last = Some(w);
        for word in clause.split(' ') {
            let sides: &[usize] = match word {
                "UP" => &[0],
                "RIGHT" => &[1],
                "DOWN" => &[2],
                "LEFT" => &[3],
                "HORIZONTAL" => &[1, 3],
                "VERTICAL" => &[0, 2],
                _ => &[],
            };
            sides.iter().for_each(|&s| arms[s] = w);
        }
    }
    Says::Arms(arms, words.contains(&"ARC"))
}
