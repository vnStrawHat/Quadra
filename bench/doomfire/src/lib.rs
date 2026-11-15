//! The fire demo's frames (specs/14, "Terminal throughput"): the whole screen at two fire
//! pixels a cell, `▀` over its two colours as 256-colour SGR written only when they change,
//! the fire spread up from a white last row by the demo's own rule.

use std::time::Instant;

/// The demo's palette, from cold to the white of the source.
const PALETTE: [u8; 26] = [
    0, 233, 234, 52, 53, 88, 89, 94, 95, 96, 130, 131, 132, 133, 172, 214, 215, 220, 220, 221, 3, 226, 227,
    230, 195, 230,
];
/// What ends a row: clear to its end, the next line, its first column.
const NL: &[u8] = b"\x1b[0K\x1b[1E\x1b[1G";
const PIXEL: &[u8] = "\u{2580}".as_bytes();

/// The screen's start: the other buffer, no cursor, home, black on white text, cleared, ASCII.
pub const START: &[u8] = b"\x1b[?1049h\x1b[?25l\x1b[1;1H\x1b[48;5;0m\x1b[38;5;15m\x1b[2J\x1b(B";
/// The screen's end: the main buffer and the cursor back.
pub const END: &[u8] = b"\x1b[?1049l\x1b[?25h\x1b[0K\x1b[1E\x1b[1G";

pub struct Fire {
    width: usize,
    height: usize,
    heat: Vec<u8>,
    seed: u64,
    // The colours of the cell before, kept from frame to frame as the demo keeps them.
    hi: u8,
    lo: u8,
    fg: Vec<Vec<u8>>,
    bg: Vec<Vec<u8>>,
}

impl Fire {
    /// The fire of a screen of `cols` by `rows` cells.
    pub fn new(cols: u16, rows: u16, seed: u64) -> Fire {
        let (width, height) = (cols.max(1) as usize, rows.max(1) as usize * 2);
        let mut heat = vec![0; width * height];
        heat[(height - 1) * width..].fill(PALETTE.len() as u8 - 1);
        let sgr = |kind: u8| (0..=255).map(|n| format!("\x1b[{kind}8;5;{n}m").into_bytes()).collect();
        Fire { width, height, heat, seed: seed | 1, hi: 0, lo: 0, fg: sgr(3), bg: sgr(4) }
    }

    /// A random step of 0 to 3.
    fn step(&mut self) -> usize {
        // xorshift64
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 33) as usize & 3
    }

    /// The fire moves on: each pixel's heat goes up a row, beside itself by the random step,
    /// less 0 or 1 by the step's low bit; a cold pixel cools the one above.
    pub fn spread(&mut self) {
        let w = self.width;
        for x in 0..w {
            for y in 0..self.height {
                let at = y * w + x;
                let heat = self.heat[at];
                if heat == 0 && at >= w {
                    self.heat[at - w] = 0;
                    continue;
                }
                let step = self.step();
                let to = if at > step { at - step + 1 } else { at };
                if to >= w {
                    let less = (step & 1) as u8;
                    self.heat[to - w] = heat.saturating_sub(less);
                }
            }
        }
    }

    /// The frame, into `out` (cleared first).
    pub fn frame(&mut self, out: &mut Vec<u8>) {
        out.clear();
        out.extend_from_slice(b"\x1b[1;1H");
        out.extend_from_slice(&self.bg[0]);
        out.extend_from_slice(&self.fg[0]);
        let w = self.width;
        for y in (0..self.height).step_by(2) {
            for x in 0..w {
                let (hi, lo) = (self.heat[y * w + x], self.heat[(y + 1) * w + x]);
                if lo != self.lo {
                    out.extend_from_slice(&self.bg[PALETTE[lo as usize] as usize]);
                }
                if hi != self.hi {
                    out.extend_from_slice(&self.fg[PALETTE[hi as usize] as usize]);
                }
                out.extend_from_slice(PIXEL);
                (self.hi, self.lo) = (hi, lo);
            }
            out.extend_from_slice(NL);
        }
    }

    /// The status line after a frame, in black: the frames' sizes and the fps so far.
    pub fn status(&self, sizes: &Sizes, fps: f64) -> Vec<u8> {
        let mut line = self.fg[0].clone();
        let (min, avg, max) = (kib(sizes.min), kib(sizes.avg()), kib(sizes.max));
        line.extend_from_slice(format!("mem: {min} min / {avg} avg / {max} max [ {fps:.2} fps ]").as_bytes());
        line
    }
}

/// The smallest, largest and mean frame.
#[derive(Default)]
pub struct Sizes {
    pub min: u64,
    pub max: u64,
    pub sum: u64,
    pub frames: u64,
}

impl Sizes {
    pub fn add(&mut self, bytes: u64) {
        self.min = if self.frames == 0 { bytes } else { self.min.min(bytes) };
        self.max = self.max.max(bytes);
        (self.sum, self.frames) = (self.sum + bytes, self.frames + 1);
    }

    pub fn avg(&self) -> u64 {
        self.sum.checked_div(self.frames).unwrap_or(0)
    }
}

fn kib(bytes: u64) -> String {
    format!("{:.2}KiB", bytes as f64 / 1024.0)
}

/// What a run ends with: `<cols>x<rows> <frames> frames <fps> fps <bytes> bytes a frame`.
pub fn summary(cols: u16, rows: u16, sizes: &Sizes, start: Instant) -> String {
    let fps = sizes.frames as f64 / start.elapsed().as_secs_f64();
    format!("{cols}x{rows} {} frames {fps:.2} fps {} bytes a frame", sizes.frames, sizes.avg())
}

/// `--seconds N`, `--out FILE`, `--size COLSxROWS` from the command line.
pub struct Args {
    pub seconds: Option<f64>,
    pub out: Option<std::path::PathBuf>,
    pub size: Option<(u16, u16)>,
}

pub fn args() -> Args {
    let mut args = Args { seconds: None, out: None, size: None };
    let mut words = std::env::args().skip(1);
    while let Some(word) = words.next() {
        let value = words.next().unwrap_or_default();
        match word.as_str() {
            "--seconds" => args.seconds = value.parse().ok(),
            "--out" => args.out = Some(value.into()),
            "--size" => {
                args.size = value.split_once('x').and_then(|(c, r)| Some((c.parse().ok()?, r.parse().ok()?)))
            }
            _ => eprintln!("doomfire: unknown {word}"),
        }
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_the_screen_in_half_blocks() {
        let mut fire = Fire::new(4, 2, 7);
        let mut out = Vec::new();
        fire.frame(&mut out);
        let text = String::from_utf8(out.clone()).unwrap();
        assert_eq!(text.matches('\u{2580}').count(), 8);
        assert_eq!(text.matches("\x1b[1E").count(), 2);
        // The white source is the last row's lower pixels.
        assert!(text.contains("\x1b[48;5;230m"), "{text:?}");
        for _ in 0..50 {
            fire.spread();
        }
        assert!(fire.heat[..4].iter().all(|it| (*it as usize) < PALETTE.len()));
        let mut sizes = Sizes::default();
        sizes.add(10);
        sizes.add(20);
        assert_eq!((sizes.min, sizes.max, sizes.avg()), (10, 20, 15));
        assert!(String::from_utf8(fire.status(&sizes, 12.5)).unwrap().ends_with("[ 12.50 fps ]"));
    }
}
