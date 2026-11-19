//! The `icon` scene (specs/21, "Logo"): the logo drawn at each size of an icon into
//! `apps/markdown/assets/logo.ico` (PNG images, straight alpha), the mark alone under 40 px;
//! and a picture of every size, each made four times larger pixel by pixel, on the dark and
//! the light canvas, to look at.

use std::error::Error;
use std::path::Path;

use quadra_prim::Color;
use quadra_vector::Svg;

use crate::png;

const SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// The logo at `side` px, straight RGBA.
fn drawn(side: u32) -> Vec<u8> {
    let source = if side < 40 { markdown::LOGO_SMALL } else { markdown::LOGO };
    let mut rgba = Svg::parse(source).render_rgba(side as usize, side as usize, Color::BLACK);
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        for value in &mut pixel[..3] {
            let straight = (u32::from(*value) * 255 + alpha / 2).checked_div(alpha);
            *value = straight.map_or(0, |it| it.min(255) as u8);
        }
    }
    rgba
}

/// An icon file of PNG images.
fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend([0u16, 1, images.len() as u16].iter().flat_map(|v| v.to_le_bytes()));
    let mut offset = 6 + 16 * images.len() as u32;
    for (side, data) in images {
        let edge = if *side >= 256 { 0 } else { *side as u8 };
        out.extend([edge, edge, 0, 0]);
        out.extend([1u16, 32].iter().flat_map(|v| v.to_le_bytes()));
        out.extend([data.len() as u32, offset].iter().flat_map(|v| v.to_le_bytes()));
        offset += data.len() as u32;
    }
    for (_, data) in images {
        out.extend(data);
    }
    out
}

pub fn icon(out: &Path) -> Result<(), Box<dyn Error>> {
    let images: Vec<(u32, Vec<u8>)> =
        SIZES.iter().map(|&side| (side, png::png(side, side, &drawn(side)))).collect();
    let file = Path::new("apps/markdown/assets/logo.ico");
    std::fs::write(file, ico(&images))?;
    println!("{}", file.display());
    // The sizes under 256, four times larger, side by side on both canvases.
    let (zoom, gap) = (4u32, 16u32);
    let small: Vec<u32> = SIZES.iter().copied().filter(|&side| side < 256).collect();
    let width = small.iter().map(|side| side * zoom + gap).sum::<u32>() + gap;
    let tall = 64 * zoom + 2 * gap;
    let mut sheet = vec![0u8; (width * tall * 2 * 4) as usize];
    for (row, canvas) in [[0x1e, 0x22, 0x27], [0xff, 0xff, 0xff]].iter().enumerate() {
        let top = row as u32 * tall;
        for y in top..top + tall {
            for x in 0..width {
                let at = ((y * width + x) * 4) as usize;
                sheet[at..at + 4].copy_from_slice(&[canvas[0], canvas[1], canvas[2], 255]);
            }
        }
        let mut left = gap;
        for &side in &small {
            let rgba = drawn(side);
            for y in 0..side * zoom {
                for x in 0..side * zoom {
                    let from = (((y / zoom) * side + x / zoom) * 4) as usize;
                    let at = (((top + gap + y) * width + left + x) * 4) as usize;
                    let alpha = u32::from(rgba[from + 3]);
                    for c in 0..3 {
                        let over =
                            u32::from(rgba[from + c]) * alpha + u32::from(sheet[at + c]) * (255 - alpha);
                        sheet[at + c] = (over / 255) as u8;
                    }
                }
            }
            left += side * zoom + gap;
        }
    }
    let file = out.join("icons.png");
    std::fs::write(&file, png::png(width, tall * 2, &sheet))?;
    println!("{}", file.display());
    Ok(())
}
