//! The chrome of a workbench (specs/behaviour/16): the activity bar, a column of the tools of
//! an app, and the panel header, the title strip of a panel.

use std::rc::Rc;

use quadra_layout::{Align, Display, Justify};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, Paint, Role, Side};
use quadra_view::{AnyView, Div, MountCx, View, by_text, column, div, row, text};

use crate::icon::icon;
use crate::tooltip::tooltip;
use crate::{Tone, token};

/// A tool of the app: what it is known by, its icon, and what it is called.
#[derive(Clone)]
pub struct Activity {
    id: &'static str,
    icon: &'static str,
    title: String,
    count: Rc<dyn Fn() -> u32>,
    tone: Tone,
}

impl Activity {
    pub fn new(id: &'static str, icon: &'static str, title: impl Into<String>) -> Activity {
        Activity { id, icon, title: title.into(), count: Rc::new(|| 0), tone: Tone::Primary }
    }

    /// What the count counts: `Tone::Danger` or `Tone::Warning` for problems that wait
    /// (failed pods, firing alerts), a pill of that colour in the cell's corner; the accent
    /// otherwise (unread items).
    pub fn badge_tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// A count in a corner of the icon: what waits in the tool. None shows while it is zero,
    /// and "99+" over ninety-nine.
    pub fn badge(mut self, count: impl Fn() -> u32 + 'static) -> Self {
        self.count = Rc::new(count);
        self
    }
}

pub struct ActivityBar {
    active: Signal<&'static str>,
    items: Vec<Activity>,
    foot: Vec<Activity>,
}

/// The leftmost column of a workbench, 40 px wide: an icon a tool, the one whose id `active`
/// holds in the accent. A click writes `active`.
pub fn activity_bar(active: Signal<&'static str>, items: impl IntoIterator<Item = Activity>) -> ActivityBar {
    ActivityBar { active, items: items.into_iter().collect(), foot: Vec::new() }
}

impl ActivityBar {
    /// Tools kept at the bottom of the bar: settings, the account.
    pub fn footer(mut self, items: impl IntoIterator<Item = Activity>) -> Self {
        self.foot = items.into_iter().collect();
        self
    }
}

/// One tool: 40 px high, a ghost icon of 28 px, its name in a tooltip at its right.
fn tool(active: Signal<&'static str>, it: Activity) -> AnyView {
    let (id, count, more) = (it.id, it.count.clone(), it.count);
    let on = move || active.get() == id;
    let tone = move || -> Paint { if on() { token::ACCENT_TEXT.into() } else { token::TEXT_2.into() } };
    let shown = move || if count() > 0 { Display::Flex } else { Display::None };
    let words = move || if more() > 99 { "99+".to_string() } else { more().to_string() };
    // The count of problems is a pill of their colour, 10 px, 5 px from the top and 4 from
    // the right of the cell. Any other count stands on the corner of the icon in the accent,
    // 9 px, ringed by the panel so it parts from the icon.
    let problem = matches!(it.tone, Tone::Danger | Tone::Warning);
    let fill = if problem { it.tone.role() } else { token::ACCENT };
    let chip = row().height(by_text(14.0)).min_width(14.0).padding_x(if problem { 3.0 } else { 2.0 });
    let chip = chip.radius(7.0).bg(fill).text_size(if problem { 10.0 } else { 9.0 });
    let chip = chip.text_color(token::TEXT_ON_ACCENT).align_items(Align::Center).justify(Justify::Center);
    let badge = match problem {
        true => div().absolute().top(5.0).right(4.0),
        false => div().absolute().top(-2.0).right(-4.0).radius(8.0).padding(1.0).bg(token::PANEL),
    };
    let badge = badge.display(shown).child(chip.child(text(words).weight(500)));
    let glyph = row().size(28.0, 28.0).align_items(Align::Center).justify(Justify::Center);
    let glyph = glyph.radius(token::RADIUS_MD).hover(|s| s.bg(token::PANEL_HOVER)).text_color(tone);
    let face = row().height(40.0).align_items(Align::Center).justify(Justify::Center);
    let face = face.cursor(Cursor::Pointer).role(Role::Tab).label(it.title.clone()).selected(on);
    let face = face.on_click(move |_| active.set(id));
    let glyph = glyph.child(icon(it.icon).size(16.0));
    let face = match problem {
        true => face.child(glyph).child(badge),
        false => face.child(glyph.child(badge)),
    };
    let face = face.child(crate::display::chosen_mark(Side::Left, on));
    AnyView::new(tooltip(it.title, face).side(Side::Right))
}

impl View for ActivityBar {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let active = self.active;
        let bar = column().width(40.0).shrink(0.0).bg(token::PANEL).role(Role::TabList);
        let bar = bar.border_right(1.0).border_color(token::LINE);
        let tools = |items: Vec<Activity>| items.into_iter().map(|it| tool(active, it)).collect::<Vec<_>>();
        bar.children(tools(self.items)).child(div().grow(1.0)).children(tools(self.foot)).mount(cx)
    }
}

/// The title strip of a panel: 28 px on the panel's plane over a line, the title muted.
/// What is added to it (`.child(..)`) stands at its end: quiet icon buttons.
pub fn panel_header(title: impl Into<String>) -> Div {
    let strip = row().align_items(Align::Center).gap(4.0).height(by_text(28.0)).shrink(0.0);
    let strip = strip.padding_left(8.0).padding_right(4.0).bg(token::PANEL);
    let strip = strip.border_bottom(1.0).border_color(token::LINE);
    strip.child(text(title.into()).color(token::TEXT_2)).child(div().grow(1.0))
}
