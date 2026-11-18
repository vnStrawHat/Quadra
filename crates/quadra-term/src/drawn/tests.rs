use super::*;

mod oracle;

/// Boxes of both parities, at 1x and 2x.
const SIZES: [(u32, u32); 6] = [(9, 18), (8, 17), (10, 21), (7, 15), (12, 24), (18, 36)];

fn flipped(m: &[u8], w: u32, x: bool, y: bool) -> Vec<u8> {
    let mut rows: Vec<Vec<u8>> = m.chunks_exact(w as usize).map(<[u8]>::to_vec).collect();
    if x {
        rows.iter_mut().for_each(|row| row.reverse());
    }
    if y {
        rows.reverse();
    }
    rows.concat()
}

fn arms(c: char) -> Option<[u8; 4]> {
    let n = c as u32;
    let at = (n.checked_sub(0x2500).filter(|it| *it < 0x80)? * 4) as usize;
    let arms: Vec<u8> = ARMS.as_bytes()[at..at + 4].iter().map(|b| b - b'0').collect();
    Some([arms[0], arms[1], arms[2], arms[3]]).filter(|it| it != &[0; 4])
}

fn with_arms(want: [u8; 4]) -> char {
    (0x2500..0x2580).filter_map(char::from_u32).find(|&c| arms(c) == Some(want)).expect("a character")
}

const BOTH: &[u32] = &[0x2573, 0x2588, 0x2591, 0x2592, 0x2593];
const SELF_X: &[u32] = &[0x2580, 0x2581, 0x2582, 0x2583, 0x2584, 0x2585, 0x2586, 0x2587, 0x2594];
const SELF_Y: &[u32] =
    &[0x2589, 0x258A, 0x258B, 0x258C, 0x258D, 0x258E, 0x258F, 0x2590, 0x2595, 0xE0B0, 0xE0B1, 0xE0B2, 0xE0B3];
const PAIRS_X: &[(u32, u32)] = &[
    (0x258C, 0x2590),
    (0x258F, 0x2595),
    (0x2596, 0x2597),
    (0x2598, 0x259D),
    (0x2599, 0x259F),
    (0x259A, 0x259E),
    (0x259B, 0x259C),
    (0x256D, 0x256E),
    (0x2570, 0x256F),
    (0x2571, 0x2572),
    (0xE0B0, 0xE0B2),
    (0xE0B1, 0xE0B3),
];
const PAIRS_Y: &[(u32, u32)] = &[
    (0x2580, 0x2584),
    (0x2594, 0x2581),
    (0x2596, 0x2598),
    (0x2597, 0x259D),
    (0x2599, 0x259B),
    (0x259A, 0x259E),
    (0x259C, 0x259F),
    (0x256D, 0x2570),
    (0x256E, 0x256F),
    (0x2571, 0x2572),
];

/// The character `c` mirrored about the vertical middle (`x`) or the horizontal one.
fn mirror(c: char, x: bool) -> Option<char> {
    let n = c as u32;
    if let Some([up, right, down, left]) = arms(c) {
        return Some(with_arms(if x { [up, left, down, right] } else { [down, right, up, left] }));
    }
    let dashes = matches!(n, 0x2504..=0x250B | 0x254C..=0x254F);
    let alone = if x { SELF_X } else { SELF_Y };
    if dashes || BOTH.contains(&n) || alone.contains(&n) {
        return Some(c);
    }
    let pairs = if x { PAIRS_X } else { PAIRS_Y };
    let other = pairs.iter().find_map(|&(a, b)| (a == n).then_some(b).or((b == n).then_some(a)))?;
    char::from_u32(other)
}

#[test]
fn every_mask_is_the_mirror_of_its_mirrored_character_about_both_middles() {
    let mut checked = 0;
    for c in (0x2500..=0x259F).chain(0xE0B0..=0xE0B3).filter_map(char::from_u32) {
        for x in [true, false] {
            let Some(other) = mirror(c, x) else { continue };
            for (w, h) in SIZES {
                let (a, b) = (mask(c, w, h), mask(other, w, h));
                assert_eq!(
                    flipped(&a, w, x, !x),
                    b,
                    "{c} mirrored {} at {w} x {h} is {other}",
                    if x { "in x" } else { "in y" }
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 1500, "{checked}");
}

/// The columns (`down`) or rows a mask fills wholly along its middle.
fn filled(m: &[u8], w: u32, h: u32, down: bool) -> Vec<usize> {
    let (w, h) = (w as usize, h as usize);
    if down {
        (0..w).filter(|&x| m[(h / 2) * w + x] == 255).collect()
    } else {
        (0..h).filter(|&y| m[y * w + w / 2] == 255).collect()
    }
}

#[test]
fn lines_are_whole_pixels_centred_and_run_to_the_edges() {
    let (w, h) = (9, 18);
    let across = mask('─', w, h);
    assert_eq!(filled(&across, w, h, false), [8, 9], "2 px, as many rows above as below");
    assert!(across.iter().all(|&v| v == 0 || v == 255), "no partly covered pixel");
    assert!((0..w as usize).all(|x| across[8 * w as usize + x] == 255), "edge to edge");
    assert_eq!(filled(&mask('│', w, h), w, h, true), [4], "an odd width: 1 px in the middle");
    assert_eq!(filled(&mask('┃', w, h), w, h, true), [3, 4, 5], "heavy: 3 px");
    assert_eq!(filled(&mask('║', w, h), w, h, true), [3, 5], "double: two light strokes and a gap");
    assert_eq!(filled(&mask('═', w, h), w, h, false), [6, 7, 10, 11], "2 px each across 18");
    // An arc meets the straight lines at their edges.
    let arc = mask('╭', w, h);
    let right: Vec<usize> =
        (0..h as usize).filter(|&y| arc[y * w as usize + w as usize - 1] >= 128).collect();
    assert_eq!(right, [8, 9]);
    let bottom: Vec<usize> =
        (0..w as usize).filter(|&x| arc[(h as usize - 1) * w as usize + x] >= 128).collect();
    assert_eq!(bottom, [4]);
}

#[test]
fn blocks_fill_their_part_and_an_edge_inside_a_pixel_covers_it_by_its_part() {
    let (w, h) = (9, 17);
    let top = mask('▀', w, h);
    let rows: Vec<u8> = top.chunks_exact(w as usize).map(|row| row[0]).collect();
    assert_eq!(rows, [255, 255, 255, 255, 255, 255, 255, 255, 128, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(mask('█', w, h).iter().all(|&v| v == 255));
    assert!(mask('▒', w, h).iter().all(|&v| v == 128));
    assert_eq!(mask('▌', 8, 16).chunks_exact(8).next().unwrap(), [255, 255, 255, 255, 0, 0, 0, 0]);
}

#[test]
fn the_table_has_an_entry_a_character() {
    assert_eq!(ARMS.len(), 0x80 * 4);
    assert_eq!(arms('┼'), Some([1, 1, 1, 1]));
    assert_eq!(arms('╔'), Some([0, 3, 3, 0]));
    assert_eq!(arms('╿'), Some([2, 0, 1, 0]));
}

#[test]
fn in_a_cell_whose_sides_share_a_parity_lines_are_as_thick_down_as_across() {
    for (w, h) in [(9, 19), (8, 18), (7, 17), (18, 38)] {
        for c in ['┼', '╋'] {
            // Across the top row, the up arm; down the first column, the left arm.
            let m = mask(c, w, h);
            let down = m[..w as usize].iter().filter(|&&v| v == 255).count();
            let across = m.chunks_exact(w as usize).filter(|row| row[0] == 255).count();
            assert_eq!(down, across, "{c} at {w} x {h}");
        }
    }
}
