//! Diagrams drawn from their source (specs/13, "Diagrams"): `diagram(source)` reads what
//! is between the fences of a `mermaid` block once, and is a node as large as the diagram
//! laid out. Flowcharts are drawn; another kind, or a source with a fault, says what is
//! wrong, for who shows it to show the source.

mod curve;
mod draw;
mod flow;
#[cfg(test)]
mod fuzz;
mod paint;
mod scan;
#[cfg(test)]
mod tests;

use quadra_layout::Known;
use quadra_prim::Size;
use quadra_text::TextStyle;
use quadra_tree::{GlyphTile, Inherited, MeasureCx, NodeId, PaintCx, Role, Semantics, Style, Theme, Widget};
use quadra_ui::{MountCx, View};

use draw::{Drawing, Measure};
use flow::Flow;

/// What a diagram is, read from its source.
enum Chart {
    Flow(Flow),
}

/// Reads a diagram: its kind is the first word of the first line that is not blank, a
/// comment or front matter.
fn read(source: &str) -> Result<Chart, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut at = 0;
    while let Some(line) = lines.get(at).map(|line| line.trim()) {
        if line == "---" {
            at += 1 + lines[at + 1..].iter().take_while(|line| line.trim() != "---").count();
        } else if !line.is_empty() && !line.starts_with("%%") {
            break;
        }
        at += 1;
    }
    let header = lines.get(at).ok_or("empty diagram")?.trim().trim_end_matches(';');
    let mut words = header.split_whitespace();
    match words.next().ok_or("no diagram kind")? {
        "graph" | "flowchart" => {
            let dir = words.next().unwrap_or("TD").to_uppercase();
            flow::parse(&dir, &lines[at + 1..], at + 1).map(Chart::Flow)
        }
        kind => Err(format!("diagram kind `{}` is not supported", kind.chars().take(40).collect::<String>())),
    }
}

impl Chart {
    fn layout(&self, m: &mut Measure) -> Drawing {
        match self {
            Chart::Flow(flow) => flow::place::layout(flow, m),
        }
    }

    /// What the diagram is to who cannot see it: its kind, and what its nodes say.
    // ponytail: the links are not said; "A to B" for each when a reader asks for them.
    fn says(&self) -> String {
        let Chart::Flow(flow) = self;
        let labels: Vec<&str> = flow.nodes.iter().map(|node| node.label.as_str()).collect();
        format!("Flowchart: {}", labels.join(", ").replace('\n', " "))
    }
}

/// The widget: lays the diagram out in the font that holds at its node, and paints it in
/// the colours of the theme.
pub struct Diagram {
    chart: Chart,
    style: TextStyle,
    /// Laid out in `style`; none when the style is another since.
    drawing: Option<Drawing>,
    inks: paint::Inks,
    /// The tiles last painted: holding them keeps them in the atlas.
    tiles: Vec<GlyphTile>,
}

impl Widget for Diagram {
    fn is_leaf(&self) -> bool {
        true
    }

    fn measure(&mut self, cx: &mut MeasureCx, _known: Known, _avail: Size) -> Size {
        let (chart, base) = (&self.chart, self.style.clone());
        let drawing = self.drawing.get_or_insert_with(|| chart.layout(&mut Measure { ts: cx.text, base }));
        Size::new(drawing.width, drawing.height)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        self.tiles.clear();
        if let Some(drawing) = &self.drawing {
            paint::paint(drawing, &self.inks, cx, &mut self.tiles);
        }
    }

    fn inherit(&mut self, inherited: &Inherited) {
        if self.style != inherited.style {
            (self.style, self.drawing) = (inherited.style.clone(), None);
        }
    }

    fn theme(&mut self, theme: &Theme) -> bool {
        std::mem::replace(&mut self.inks, paint::inks(theme)) != self.inks
    }

    fn semantics(&self, says: &mut Semantics) {
        if says.role == Role::None {
            says.role = Role::Image;
        }
        if says.name.is_empty() {
            says.name = self.chart.says();
        }
    }
}

pub struct DiagramView {
    chart: Result<Chart, String>,
}

/// A diagram, from its source. The source is read here, once.
pub fn diagram(source: &str) -> DiagramView {
    DiagramView { chart: read(source) }
}

impl DiagramView {
    /// Why the diagram cannot be drawn, if it cannot: a fault with its line, or a kind that
    /// is not drawn. The view is then a node of no size.
    pub fn fault(&self) -> Option<&str> {
        self.chart.as_ref().err().map(String::as_str)
    }
}

impl View for DiagramView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let Ok(chart) = self.chart else { return cx.tree.insert(Style::default()) };
        let (style, inks) = (TextStyle::default(), paint::Inks::default());
        let widget = Diagram { chart, style, drawing: None, inks, tiles: Vec::new() };
        cx.tree.insert_widget(Style::default(), Box::new(widget))
    }
}
