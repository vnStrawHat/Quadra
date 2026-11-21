//! A graph's drawing as SVG: its frames, its links and its nodes as boxes with their labels,
//! at its own size and in the colours of a theme.

use std::fmt::Write;

use quadra_design::token;
use quadra_prim::Point;
use quadra_tree::{Paint, Theme};

use super::links::head;
use super::{Graph, LinkStyle};

/// Words as SVG's text and attributes take them.
pub fn escape(words: &str) -> String {
    let mut out = String::with_capacity(words.len());
    for c in words.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// A colour as `#rrggbb` with its opacity apart.
fn color(theme: &Theme, paint: Paint) -> (String, f32) {
    let c = theme.resolve(paint);
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    (format!("#{:02x}{:02x}{:02x}", byte(c.r), byte(c.g), byte(c.b)), c.a)
}

fn path(points: &[Point]) -> String {
    let mut d = String::new();
    for (k, p) in points.iter().enumerate() {
        _ = write!(d, "{}{:.1} {:.1}", if k == 0 { "M" } else { " L" }, p.x, p.y);
    }
    d
}

pub fn write(graph: &Graph, theme: &Theme, ground: Option<Paint>) -> String {
    let (rects, routes) = graph.plane();
    let b = graph.bounds();
    let (x, y, w, h) = (b.x - 16.0, b.y - 16.0, b.width + 32.0, b.height + 32.0);
    let mut out = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="{x} {y} {w} {h}" font-family="sans-serif">"#
    );
    if let Some((fill, _)) = ground.map(|paint| color(theme, paint)) {
        _ = write!(out, r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}"/>"#);
    }
    let (line, (muted, _)) = (color(theme, token::LINE_STRONG.into()), color(theme, token::TEXT_3.into()));
    for (title, r) in graph.frames(&rects) {
        _ = write!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="6" fill="none" stroke="{}"/>"#,
            r.x, r.y, r.width, r.height, line.0
        );
        let title = escape(&title);
        _ = write!(
            out,
            r#"<text x="{}" y="{}" font-size="12" fill="{muted}">{title}</text>"#,
            r.x + 8.0,
            r.y + 16.0
        );
    }
    for (k, points) in &routes {
        let LinkStyle { color: paint, width, dash, arrow } = (graph.style)(*k);
        let (stroke, alpha) = color(theme, paint);
        let dashes = if dash > 0.0 { format!(r#" stroke-dasharray="{dash} {dash}""#) } else { String::new() };
        let look =
            format!(r#"fill="none" stroke="{stroke}" stroke-opacity="{alpha}" stroke-width="{width}""#);
        _ = write!(out, r#"<path d="{}" {look}{dashes}/>"#, path(points));
        if let Some(head) = head(points, width).filter(|_| arrow) {
            _ = write!(out, r#"<path d="{}" {look}/>"#, path(&head));
        }
    }
    let ((fill, _), (text, _)) = (color(theme, token::PANEL.into()), color(theme, token::TEXT.into()));
    for (node, r) in graph.nodes.iter().zip(&rects) {
        if let Some(own) = &node.svg {
            out.push_str(&own(*r));
            continue;
        }
        _ = write!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="4" fill="{fill}" stroke="{}"/>"#,
            r.x, r.y, r.width, r.height, line.0
        );
        let (cx, cy, label) = (r.center().x, r.center().y, escape(&node.label));
        _ = write!(
            out,
            r#"<text x="{cx}" y="{cy}" font-size="12" fill="{text}" text-anchor="middle" dominant-baseline="middle">{label}</text>"#
        );
    }
    out.push_str("</svg>");
    out
}
