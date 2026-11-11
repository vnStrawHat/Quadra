//! What the keyboard asks of the kit's components (specs/13, "Components"): actions, bound in
//! the kit's keymap, never raw keys.

use quadra_tree::{Action, Keymap};

/// The key context of a control that is activated: a button, a checkbox, a tab.
pub const CONTROL: &str = "control";

/// Do what the control is for: Enter or Space on a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Confirm;

impl Action for Confirm {
    const NAME: &'static str = "kit::confirm";
}

/// The key context of a control that holds one choice of a few: a radio group, tabs.
pub const CHOICE: &str = "choice";

/// To the next of a few, and to the one before: the arrows in a radio group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveNext;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MovePrevious;

impl Action for MoveNext {
    const NAME: &'static str = "kit::move_next";
}

impl Action for MovePrevious {
    const NAME: &'static str = "kit::move_previous";
}

/// The key context of a list: the arrows move its selection and stop at the ends.
pub const LIST: &str = "list";

/// To the first of many, and to the last: Home and End in a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveFirst;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveLast;

impl Action for MoveFirst {
    const NAME: &'static str = "kit::move_first";
}

impl Action for MoveLast {
    const NAME: &'static str = "kit::move_last";
}

/// The key contexts of what holds a number: a slider, which every arrow steps, and a number
/// input, where left and right are the caret's.
pub const SLIDER: &str = "slider";
pub const NUMBER: &str = "number";

/// A step up and a step down, and a larger one each way: the arrows and the page keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Increase;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decrease;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IncreaseMore;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecreaseMore;

impl Action for Increase {
    const NAME: &'static str = "kit::increase";
}

impl Action for Decrease {
    const NAME: &'static str = "kit::decrease";
}

impl Action for IncreaseMore {
    const NAME: &'static str = "kit::increase_more";
}

impl Action for DecreaseMore {
    const NAME: &'static str = "kit::decrease_more";
}

/// The key context of a popup: a menu, a popover, a dialog.
pub const POPUP: &str = "popup";
/// The key context of the rows of a menu.
pub const MENU: &str = "menu";

/// Give up what is open: Escape in a popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancel;

impl Action for Cancel {
    const NAME: &'static str = "kit::cancel";
}

/// The key bindings of the kit over those every window has. An app adds its own to it.
pub fn keymap() -> Keymap {
    let mut keymap = Keymap::default();
    let well_formed = "the kit's bindings are well formed";
    for keys in ["enter", "space"] {
        keymap.bind_in(CONTROL, keys, Confirm).expect(well_formed);
    }
    for keys in ["right", "down"] {
        keymap.bind_in(CHOICE, keys, MoveNext).expect(well_formed);
    }
    for keys in ["left", "up"] {
        keymap.bind_in(CHOICE, keys, MovePrevious).expect(well_formed);
    }
    keymap.bind_in(POPUP, "escape", Cancel).expect(well_formed);
    for context in [MENU, LIST] {
        keymap.bind_in(context, "down", MoveNext).expect(well_formed);
        keymap.bind_in(context, "up", MovePrevious).expect(well_formed);
        keymap.bind_in(context, "enter", Confirm).expect(well_formed);
    }
    keymap.bind_in(MENU, "space", Confirm).expect(well_formed);
    for context in [LIST, SLIDER] {
        keymap.bind_in(context, "home", MoveFirst).expect(well_formed);
        keymap.bind_in(context, "end", MoveLast).expect(well_formed);
    }
    for context in [SLIDER, NUMBER] {
        keymap.bind_in(context, "up", Increase).expect(well_formed);
        keymap.bind_in(context, "down", Decrease).expect(well_formed);
    }
    keymap.bind_in(SLIDER, "right", Increase).expect(well_formed);
    keymap.bind_in(SLIDER, "left", Decrease).expect(well_formed);
    keymap.bind_in(SLIDER, "pageup", IncreaseMore).expect(well_formed);
    keymap.bind_in(SLIDER, "pagedown", DecreaseMore).expect(well_formed);
    keymap
}
