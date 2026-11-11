//! A small SVG reader: `path`, `circle`, `ellipse`, `rect`, `line`, `polyline`, `polygon`
//! and `g`, with `fill`, `stroke` and `stroke-width`, which what is inside a group has too.
//! Enough for sets of icons and simple art. Any text reads as something: what is not
//! understood is skipped.

use quadra_prim::{Color, Point};

use crate::data::numbers;
use crate::path::Path;
use crate::raster::Coverage;

/// A shape of a drawing: its outline and how it is painted.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub path: Path,
    /// `None`: not filled.
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f32,
    /// Painted in the colour of whatever shows the drawing (`currentColor`).
    pub current: bool,
}

/// A drawing: the box its units are in, and its shapes from the back to the front.
#[derive(Clone, Debug, PartialEq)]
pub struct Svg {
    pub view: (f32, f32, f32, f32),
    pub shapes: Vec<Shape>,
}

/// How shapes are painted where nothing nearer says otherwise.
#[derive(Clone, Copy)]
struct Paint {
    fill: Option<Color>,
    fill_current: bool,
    stroke: Option<Color>,
    stroke_current: bool,
    width: f32,
}

/// A colour as an attribute gives it, and whether it is that of whatever shows the drawing.
fn color(value: &str) -> (Option<Color>, bool) {
    let value = value.trim();
    let hex = |digits: &str| u32::from_str_radix(digits, 16).ok().map(Color::rgb);
    match value.strip_prefix('#') {
        _ if value == "none" => (None, false),
        _ if value == "currentColor" => (Some(Color::BLACK), true),
        _ if value == "white" => (Some(Color::WHITE), false),
        Some(digits) if digits.len() == 6 => (hex(digits).or(Some(Color::BLACK)), false),
        Some(digits) if digits.len() == 3 && digits.is_ascii() => {
            let doubled: String = digits.chars().flat_map(|c| [c, c]).collect();
            (hex(&doubled).or(Some(Color::BLACK)), false)
        }
        _ => (Some(Color::BLACK), false),
    }
}

/// The attributes of a tag: `name="value"` or `name='value'`.
fn attributes(tag: &str) -> Vec<(&str, &str)> {
    let (mut out, mut rest) = (Vec::new(), tag);
    while let Some(eq) = rest.find('=') {
        let after = rest[eq + 1..].trim_start();
        let Some(quote) = after.chars().next().filter(|c| matches!(c, '"' | '\'')) else { break };
        let Some(end) = after[1..].find(quote) else { break };
        if let Some(name) = rest[..eq].split_whitespace().last() {
            out.push((name, &after[1..1 + end]));
        }
        rest = &after[end + 2..];
    }
    out
}

fn attribute<'a>(of: &[(&str, &'a str)], name: &str) -> Option<&'a str> {
    of.iter().find(|(key, _)| *key == name).map(|(_, value)| *value)
}

fn length(of: &[(&str, &str)], name: &str) -> f32 {
    attribute(of, name).and_then(|value| value.trim().parse().ok()).unwrap_or(0.0)
}

fn shape(name: &str, of: &[(&str, &str)]) -> Option<Path> {
    let mut path = Path::new();
    let n = |name: &str| length(of, name);
    match name {
        "path" => path.push_data(attribute(of, "d")?),
        "circle" => path.ellipse(n("cx"), n("cy"), n("r"), n("r")),
        "ellipse" => path.ellipse(n("cx"), n("cy"), n("rx"), n("ry")),
        "rect" => {
            let radius = if attribute(of, "rx").is_some() { n("rx") } else { n("ry") };
            path.rounded_rect(n("x"), n("y"), n("width"), n("height"), radius);
        }
        "line" => {
            path.move_to(n("x1"), n("y1"));
            path.line_to(n("x2"), n("y2"));
        }
        "polyline" | "polygon" => {
            let points = numbers(attribute(of, "points")?);
            for (i, pair) in points.chunks_exact(2).enumerate() {
                if i == 0 { path.move_to(pair[0], pair[1]) } else { path.line_to(pair[0], pair[1]) }
            }
            if name == "polygon" {
                path.close();
            }
        }
        _ => return None,
    }
    Some(path)
}

impl Svg {
    /// Reads a drawing. Without a `viewBox` its units are a box of 24 by 24.
    pub fn parse(source: &str) -> Svg {
        let mut svg = Svg { view: (0.0, 0.0, 24.0, 24.0), shapes: Vec::new() };
        let root = Paint {
            fill: Some(Color::BLACK),
            fill_current: false,
            stroke: None,
            stroke_current: false,
            width: 1.0,
        };
        let (mut around, mut rest) = (vec![root], source);
        while let Some(open) = rest.find('<') {
            let Some(close) = rest[open..].find('>') else { break };
            let tag = &rest[open + 1..open + close];
            rest = &rest[open + close + 1..];
            if tag.starts_with('/') {
                if around.len() > 1 && matches!(tag.trim(), "/g" | "/svg") {
                    around.pop();
                }
                continue;
            }
            if tag.starts_with(['?', '!']) {
                continue;
            }
            let name = tag.split([' ', '/', '\t', '\n', '\r']).next().unwrap_or("");
            let of = attributes(tag);
            let mut paint = *around.last().unwrap_or(&root);
            if let Some(fill) = attribute(&of, "fill") {
                (paint.fill, paint.fill_current) = color(fill);
            }
            if let Some(stroke) = attribute(&of, "stroke") {
                (paint.stroke, paint.stroke_current) = color(stroke);
            }
            if let Some(width) = attribute(&of, "stroke-width") {
                paint.width = width.trim().parse().unwrap_or(0.0);
            }
            match name {
                "svg" => {
                    let view = attribute(&of, "viewBox").map(numbers).unwrap_or_default();
                    if view.len() == 4 && view.iter().all(|v| v.is_finite()) && view[2] > 0.0 && view[3] > 0.0
                    {
                        svg.view = (view[0], view[1], view[2], view[3]);
                    }
                    around.push(paint);
                }
                "g" if !tag.ends_with('/') => around.push(paint),
                "g" => {}
                _ => {
                    if let Some(path) = shape(name, &of) {
                        let current = paint.fill_current || paint.stroke_current;
                        let (fill, stroke, stroke_width) = (paint.fill, paint.stroke, paint.width);
                        svg.shapes.push(Shape { path, fill, stroke, stroke_width, current });
                    }
                }
            }
        }
        svg
    }

    /// The scale and the offset that fit the drawing's box into a bitmap, centred.
    fn fit(&self, width: usize, height: usize) -> (f32, Point) {
        let (x, y, w, h) = self.view;
        let scale = (width as f32 / w).min(height as f32 / h);
        let offset = Point::new(
            -x * scale + (width as f32 - w * scale) / 2.0,
            -y * scale + (height as f32 - h * scale) / 2.0,
        );
        (scale, offset)
    }

    /// Whether every shape is painted in the colour of whatever shows the drawing: then it
    /// is drawn as a mask and tinted.
    pub fn is_single_color(&self) -> bool {
        self.shapes.iter().all(|shape| shape.current)
    }

    /// The drawing as coverage, a byte a pixel: what is tinted to show an icon.
    pub fn render_mask(&self, width: usize, height: usize) -> Vec<u8> {
        let (mut coverage, (scale, offset)) = (Coverage::new(width, height), self.fit(width, height));
        for shape in &self.shapes {
            let lines = shape.path.flatten(scale, offset, 0.25);
            if shape.fill.is_some() {
                coverage.fill(&lines);
            }
            if shape.stroke.is_some() {
                coverage.stroke(&lines, shape.stroke_width * scale);
            }
        }
        coverage.to_bytes()
    }

    /// The drawing in its own colours, as premultiplied RGBA; `current` is the colour of the
    /// shapes that have none of their own.
    pub fn render_rgba(&self, width: usize, height: usize, current: Color) -> Vec<u8> {
        let (mut out, (scale, offset)) = (vec![0.0f32; width * height * 4], self.fit(width, height));
        for shape in &self.shapes {
            let lines = shape.path.flatten(scale, offset, 0.25);
            for (paint, filled) in [(shape.fill, true), (shape.stroke, false)] {
                let Some(own) = paint else { continue };
                let color = if shape.current { current } else { own };
                let mut coverage = Coverage::new(width, height);
                match filled {
                    true => coverage.fill(&lines),
                    false => coverage.stroke(&lines, shape.stroke_width * scale),
                }
                for (i, pixel) in out.chunks_exact_mut(4).enumerate() {
                    let alpha = coverage.at(i % width, i / width) * color.a;
                    if alpha > 0.0 {
                        for (value, over) in pixel.iter_mut().zip([color.r, color.g, color.b, 1.0]) {
                            *value = over * alpha + *value * (1.0 - alpha);
                        }
                    }
                }
            }
        }
        out.into_iter().map(|value| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8).collect()
    }
}
