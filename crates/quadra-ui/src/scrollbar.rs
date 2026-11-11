//! The scrollbar of a scroll container (specs/06, "Scrolling"): where the thumb is and how
//! long, when it shows, and the drag that scrolls by it.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use quadra_async::{Timers, timeout};
use quadra_layout::{Insets, Length, Position};
use quadra_platform::MouseButton;
use quadra_prim::{Point, Size};
use quadra_reactive::{Memo, Scope, Signal, memo, signal, use_context};
use quadra_tree::{Handle, NodeId, PointerKind, ScrollThumb, Style};

use crate::view::MountCx;

/// The width of the grip, the room at each end of the track, the shortest thumb.
const GRIP: f32 = 14.0;
const INSET: f32 = 4.0;
const SHORTEST: f32 = 25.0;
/// How long the bar stays after the last thing that showed it, and how long it fades.
const STAYS: Duration = Duration::from_secs(1);
const FADES: Duration = Duration::from_millis(400);

/// How a container is scrolled: what its bar shows, and moves.
pub(crate) struct Scrolled {
    pub scroll_y: Signal<f32>,
    /// Where the content really is, and the furthest it goes.
    pub top: Memo<f32>,
    pub end: Memo<f32>,
    pub view: Signal<Size>,
}

/// Shows the bar while something goes on, and for a while after: one timer, set again when
/// it fires too early.
struct Idle {
    handle: Handle,
    owner: Scope,
    shown: Signal<bool>,
    until: Cell<Instant>,
    armed: Cell<bool>,
}

impl Idle {
    fn stir(self: &Rc<Self>) {
        self.until.set(self.handle.now() + STAYS);
        self.shown.set(true);
        if self.owner.is_alive() && !self.armed.replace(true) {
            self.arm(STAYS);
        }
    }

    fn arm(self: &Rc<Self>, after: Duration) {
        let idle = self.clone();
        self.owner.run(|| {
            // ponytail: where nothing provides timers (a bare tree in a test) the bar stays.
            if use_context::<Rc<Timers>>().is_none() {
                return;
            }
            timeout(after, move || {
                let left = idle.until.get().saturating_duration_since(idle.handle.now());
                if left.is_zero() {
                    idle.armed.set(false);
                    idle.shown.set(false);
                } else {
                    idle.arm(left);
                }
            });
        });
    }
}

/// Makes the thumb of a scroll container: a node to put in the viewport after the content.
pub(crate) fn attach(cx: &mut MountCx, viewport: NodeId, scrolled: Scrolled) -> NodeId {
    let Scrolled { scroll_y, top, end, view } = scrolled;
    let place = quadra_layout::Style {
        position: Position::Absolute,
        inset: Insets { top: Length::Px(0.0), right: Length::Px(0.0), ..Insets::default() },
        width: Length::Px(GRIP),
        ..Default::default()
    };
    let thumb =
        cx.tree.insert_widget(Style { layout: place, ..Style::default() }, Box::<ScrollThumb>::default());
    cx.tree.make_layer(thumb, true);

    // As long as the viewport is of the content; content that fits has no thumb.
    let track = move || (view.get().height - 2.0 * INSET).max(0.0);
    let length = memo(move || {
        let (height, end, track) = (view.get().height, end.get(), track());
        if end > 0.0 { (track * height / (height + end)).max(SHORTEST).min(track) } else { 0.0 }
    });
    let writer = cx.handle.clone();
    cx.handle.bind(move || {
        let height = Length::Px(length.get());
        writer.with(move |tree| {
            if tree.is_alive(thumb) {
                let style = quadra_layout::Style { height, ..tree.layout_style(thumb).clone() };
                tree.set_layout_style(thumb, style);
            }
        });
    });
    let writer = cx.handle.clone();
    cx.handle.bind(move || {
        let (end, travel) = (end.get(), track() - length.get());
        let y = INSET + if end > 0.0 { travel * top.get() / end } else { 0.0 };
        writer.with(move |tree| tree.set_layer_offset(thumb, Point::new(0.0, y)));
    });

    // It shows at once, and fades away on the frames of the window: no frame is asked for
    // while it only shows.
    let shown = signal(false);
    let (writer, clock) = (cx.handle.clone(), cx.handle.clone());
    let show = move |shows: f32| {
        writer.with(move |tree| {
            if tree.widget_mut::<ScrollThumb>(thumb).is_some_and(|thumb| thumb.show(shows)) {
                tree.mark_paint(thumb);
            }
        })
    };
    let (fading, first): (Rc<Cell<Option<Instant>>>, _) = (Rc::default(), Cell::new(true));
    cx.handle.bind(move || {
        if shown.get() {
            fading.set(None);
            return show(1.0);
        }
        if first.replace(false) || fading.replace(Some(clock.now())).is_some() {
            return; // never shown yet, or a fade is under way and goes on
        }
        let (fading, now, show) = (fading.clone(), clock.clone(), show.clone());
        clock.each_frame(move || {
            let Some(since) = fading.get() else { return false };
            let part = (now.now() - since).as_secs_f32() / FADES.as_secs_f32();
            show((1.0 - part).max(0.0));
            if part >= 1.0 {
                fading.set(None);
            }
            part < 1.0
        });
    });
    let (handle, owner) = (cx.handle.clone(), Scope::current());
    let until = Cell::new(handle.now());
    let idle = Rc::new(Idle { handle, owner, shown, until, armed: Cell::new(false) });
    let over = idle.clone();
    cx.tree.on_pointer(viewport, PointerKind::Move, move |_| {
        if end.get() > 0.0 {
            over.stir();
        }
    });
    let (moved, first) = (idle.clone(), Cell::new(true));
    cx.handle.bind(move || {
        top.get();
        if !first.replace(false) {
            moved.stir();
        }
    });

    // A press on the grip takes the pointer: the content follows it in proportion.
    // ponytail: a thumb held still for a second fades all the same; it scrolls on.
    let held: Rc<Cell<Option<(f32, f32)>>> = Rc::default();
    let press = held.clone();
    cx.tree.on_pointer(thumb, PointerKind::Down, move |e| {
        if e.button == MouseButton::Left {
            press.set(Some((e.at.y, top.get())));
            e.capture_pointer();
            e.stop();
        }
    });
    let drag = held.clone();
    cx.tree.on_pointer(thumb, PointerKind::Move, move |e| {
        let (Some((from, top)), end, travel) = (drag.get(), end.get(), track() - length.get()) else {
            return;
        };
        if travel > 0.0 {
            scroll_y.set((top + (e.at.y - from) * end / travel).clamp(0.0, end));
        }
    });
    cx.tree.on_pointer(thumb, PointerKind::Up, move |_| held.set(None));
    thumb
}
