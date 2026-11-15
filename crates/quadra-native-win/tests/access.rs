//! The bridge to assistive technology (specs/12), read back the way a screen reader reads
//! it: a UI Automation client on another thread finds the controls of a window that is never
//! shown, reads what they say, and asks them to act.

use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use quadra_a11y::{RangeValue, Role, SemanticAction, SemanticNode, Semantics, TreeUpdate};
use quadra_native_win::WinPlatform;
use quadra_platform::{
    Platform, PlatformEvent, PlatformHandler, PlatformWindow, TimerId, WindowHandle, WindowId, WindowOptions,
};
use quadra_prim::Rect;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern,
    IUIAutomationRangeValuePattern, IUIAutomationTextPattern, IUIAutomationTogglePattern, ToggleState_On,
    TreeScope_Descendants, UIA_AutomationIdPropertyId, UIA_ButtonControlTypeId, UIA_InvokePatternId,
    UIA_LevelPropertyId, UIA_PositionInSetPropertyId, UIA_RangeValuePatternId, UIA_SizeOfSetPropertyId,
    UIA_TextPatternId, UIA_TogglePatternId,
};
use windows::core::{BSTR, Result};

const WINDOW: u64 = 1;
const SAVE: u64 = 2;
const DONE: u64 = 3;
const NAME: u64 = 4;
const VOLUME: u64 = 5;
const FILES: u64 = 6;
const ROW: u64 = 7;

fn node(id: u64, role: Role, name: &str, bounds: Rect, children: &[u64]) -> SemanticNode {
    let semantics = Semantics { role, name: name.to_string(), ..Semantics::default() };
    SemanticNode { id, semantics, bounds, focused: false, focusable: true, children: children.to_vec() }
}

/// A window holding a button, a checkbox, an input with a selection, a slider and a tree of
/// seven rows that show, as the tree would describe them.
fn page(done: bool) -> TreeUpdate {
    let mut save = node(SAVE, Role::Button, "Save", Rect::new(10.0, 20.0, 80.0, 24.0), &[]);
    save.semantics.test_id = Some("save".to_string());
    let mut check = node(DONE, Role::Checkbox, "Done", Rect::new(10.0, 60.0, 80.0, 24.0), &[]);
    (check.semantics.test_id, check.semantics.checked) = (Some("done".to_string()), Some(done));
    let mut name = node(NAME, Role::TextInput, "Name", Rect::new(100.0, 20.0, 120.0, 24.0), &[]);
    name.semantics.test_id = Some("name".to_string());
    // "Việt" selected from its end: four characters, six bytes.
    (name.semantics.value, name.semantics.selection) = (Some("Tiếng Việt".to_string()), Some((10, 6)));
    let mut volume = node(VOLUME, Role::Slider, "Volume", Rect::new(100.0, 60.0, 120.0, 20.0), &[]);
    volume.semantics.test_id = Some("volume".to_string());
    volume.semantics.range = Some(RangeValue { value: 40.0, min: 0.0, max: 100.0, step: 5.0 });
    let mut files = node(FILES, Role::Tree, "Files", Rect::new(10.0, 100.0, 200.0, 80.0), &[ROW]);
    files.semantics.set_size = Some(7);
    let mut row = node(ROW, Role::TreeItem, "main.rs", Rect::new(10.0, 100.0, 200.0, 20.0), &[]);
    row.semantics.test_id = Some("row".to_string());
    (row.semantics.level, row.semantics.position) = (Some(2), Some(4));
    let all = [SAVE, DONE, NAME, VOLUME, FILES];
    let window = node(WINDOW, Role::Window, "", Rect::new(0.0, 0.0, 320.0, 200.0), &all);
    let nodes = vec![window, save, check, name, volume, files, row];
    TreeUpdate { nodes, root: WINDOW, focus: None, scale: None }
}

/// The app's part: it sends the tree when it is wanted, and checks the box when asked to.
struct App {
    platform: WinPlatform,
    window: Box<dyn PlatformWindow>,
    wanted: u32,
    asked: Vec<(u64, SemanticAction)>,
    timed_out: bool,
}

impl PlatformHandler for App {
    fn event(&mut self, _: WindowId, event: PlatformEvent) {
        match event {
            PlatformEvent::SemanticsWanted => {
                self.wanted += 1;
                self.window.update_semantics(&page(false));
            }
            PlatformEvent::Semantic { node, action } => {
                if node == DONE {
                    let mut update = page(true);
                    update.nodes.remove(0); // only what changed
                    self.window.update_semantics(&update);
                }
                self.asked.push((node, action));
            }
            _ => {}
        }
    }

    fn frame(&mut self, _: WindowId) {}

    fn timer(&mut self, _: TimerId) {
        self.timed_out = true;
        self.platform.quit();
    }

    fn wake(&mut self) {
        self.platform.quit(); // the reader is done
    }
}

/// What the reader found.
#[derive(Debug, PartialEq)]
struct Read {
    name: String,
    is_button: bool,
    /// The button's size on the screen, in device pixels.
    size: (i32, i32),
    checked_before: bool,
    checked_after: bool,
    /// The selected text of the input, and the slider's value and maximum.
    selected: String,
    volume: (f64, f64),
    /// The row of the tree: its level, its place in the set from 1, and the set's size.
    row: (i32, i32, i32),
}

/// A screen reader's side, on a thread of its own: calls into the window's thread wait for
/// its loop to answer.
fn read(hwnd: isize) -> Result<Read> {
    // SAFETY: COM calls on interfaces this thread made, with arguments that live through
    // each call; the window handle is only a number to look the window up by.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
        let window = uia.ElementFromHandle(HWND(hwnd as *mut _))?;
        let find = |id: &str| -> Result<IUIAutomationElement> {
            let by_id =
                uia.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(BSTR::from(id)))?;
            window.FindFirst(TreeScope_Descendants, &by_id)
        };
        let (save, done) = (find("save")?, find("done")?);
        let text: IUIAutomationTextPattern = find("name")?.GetCurrentPatternAs(UIA_TextPatternId)?;
        let selected = text.GetSelection()?.GetElement(0)?.GetText(-1)?.to_string();
        let range: IUIAutomationRangeValuePattern =
            find("volume")?.GetCurrentPatternAs(UIA_RangeValuePatternId)?;
        let volume = (range.CurrentValue()?, range.CurrentMaximum()?);
        let row = find("row")?;
        let number = |id| -> Result<i32> { i32::try_from(&row.GetCurrentPropertyValue(id)?) };
        let row = (
            number(UIA_LevelPropertyId)?,
            number(UIA_PositionInSetPropertyId)?,
            number(UIA_SizeOfSetPropertyId)?,
        );
        let bounds = save.CurrentBoundingRectangle()?;
        let toggle: IUIAutomationTogglePattern = done.GetCurrentPatternAs(UIA_TogglePatternId)?;
        let checked_before = toggle.CurrentToggleState()? == ToggleState_On;
        let invoke: IUIAutomationInvokePattern = save.GetCurrentPatternAs(UIA_InvokePatternId)?;
        invoke.Invoke()?;
        toggle.Toggle()?;
        // The app answers on its own thread: the new state is there a moment later.
        let start = Instant::now();
        while toggle.CurrentToggleState()? != ToggleState_On && start.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(Read {
            name: save.CurrentName()?.to_string(),
            is_button: save.CurrentControlType()? == UIA_ButtonControlTypeId,
            size: (bounds.right - bounds.left, bounds.bottom - bounds.top),
            checked_before,
            checked_after: toggle.CurrentToggleState()? == ToggleState_On,
            selected,
            volume,
            row,
        })
    }
}

#[test]
fn a_screen_reader_finds_the_controls_of_a_window_and_asks_them_to_act() {
    let platform = WinPlatform::new().unwrap();
    let window = platform.open_window(&WindowOptions::new("a11y").size(320.0, 200.0).hidden()).unwrap();
    let WindowHandle::Win32 { hwnd } = window.handle() else { panic!("not a Win32 window") };
    let scale = window.scale();
    let (waker, (found, result)) = (platform.waker(), channel());
    let reader = std::thread::spawn(move || {
        let found: Sender<_> = found;
        let _ = found.send(read(hwnd).map_err(|e| e.to_string()));
        waker.wake();
    });
    platform.set_timer(Duration::from_secs(30), false);
    let mut app = App { platform: platform.clone(), window, wanted: 0, asked: Vec::new(), timed_out: false };
    platform.run(&mut app).unwrap();
    assert!(!app.timed_out, "the reader never finished");
    reader.join().unwrap();

    let read = result.recv().unwrap().unwrap();
    let size = ((80.0 * scale).round() as i32, (24.0 * scale).round() as i32);
    let (selected, volume) = ("Việt".to_string(), (40.0, 100.0));
    let (name, checked_before, checked_after) = ("Save".to_string(), false, true);
    let row = (2, 5, 7);
    let want = Read { name, is_button: true, size, checked_before, checked_after, selected, volume, row };
    assert_eq!(read, want);
    // The tree was asked for once, and each request came back as semantic actions: the
    // adapter gives what can have the focus the focus before it clicks it.
    assert_eq!(app.wanted, 1);
    use SemanticAction::{Click, Focus};
    assert_eq!(app.asked, [(SAVE, Focus), (SAVE, Click), (DONE, Focus), (DONE, Click)]);
}
