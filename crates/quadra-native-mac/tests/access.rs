//! Readers on macOS (specs/12; specs/23): the view of a window that is not shown is asked
//! what the system's accessibility asks a view, and answers with the window's semantic
//! tree; a press asked of a button arrives as a semantic action. A `main` of its own, as
//! AppKit runs on the main thread alone.

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    mac::run();
    // As the harness says it, for who counts what passed.
    println!("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out");
}

#[cfg(target_os = "macos")]
mod mac {
    use std::time::Duration;

    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2_foundation::{NSArray, NSObject, NSString};
    use quadra_a11y::{Role, SemanticAction, SemanticNode, Semantics, TreeUpdate};
    use quadra_native_mac::MacPlatform;
    use quadra_platform::{
        Platform, PlatformEvent, PlatformHandler, PlatformWindow, TimerId, WindowHandle, WindowId,
        WindowOptions,
    };
    use quadra_prim::Rect;

    const WINDOW: u64 = 1;
    const SAVE: u64 = 2;
    const DONE: u64 = 3;

    fn node(id: u64, role: Role, name: &str, bounds: Rect, children: &[u64]) -> SemanticNode {
        let semantics = Semantics { role, name: name.to_string(), ..Semantics::default() };
        SemanticNode { id, semantics, bounds, focused: false, focusable: true, children: children.to_vec() }
    }

    /// A window holding a button and a checkbox, as the tree would describe them.
    fn page(done: bool) -> TreeUpdate {
        let save = node(SAVE, Role::Button, "Save", Rect::new(10.0, 20.0, 80.0, 24.0), &[]);
        let mut check = node(DONE, Role::Checkbox, "Done", Rect::new(10.0, 60.0, 80.0, 24.0), &[]);
        check.semantics.checked = Some(done);
        let window = node(WINDOW, Role::Window, "", Rect::new(0.0, 0.0, 320.0, 200.0), &[SAVE, DONE]);
        TreeUpdate { nodes: vec![window, save, check], root: WINDOW, focus: None, scale: None }
    }

    /// What accessibility is told an object holds.
    fn children(of: &NSObject) -> Retained<NSArray<NSObject>> {
        // SAFETY: a message every view and every element of accessibility answers.
        unsafe { msg_send![of, accessibilityChildren] }
    }

    /// The element of that title among what the window's element holds.
    fn titled(view: &NSObject, title: &str) -> Retained<NSObject> {
        let window = children(view).firstObject().expect("the window's element");
        let found = children(&window).iter().find(|it| {
            // SAFETY: a message of accessibility's elements.
            let said: Option<Retained<NSString>> = unsafe { msg_send![&**it, accessibilityTitle] };
            said.is_some_and(|said| said.to_string() == title)
        });
        found.unwrap_or_else(|| panic!("no element titled {title}"))
    }

    /// The app's part: it gives the tree when it is wanted, and checks the box when the
    /// button is pressed.
    struct App {
        platform: MacPlatform,
        window: Box<dyn PlatformWindow>,
        wanted: u32,
        asked: Vec<(u64, SemanticAction)>,
        ticks: u32,
    }

    impl PlatformHandler for App {
        fn event(&mut self, _: WindowId, event: PlatformEvent) {
            match event {
                PlatformEvent::SemanticsWanted => {
                    self.wanted += 1;
                    self.window.update_semantics(&page(false));
                }
                PlatformEvent::Semantic { node, action } => {
                    self.asked.push((node, action));
                    self.window.update_semantics(&page(true));
                }
                _ => {}
            }
        }

        fn frame(&mut self, _: WindowId) {}

        /// Each tick asks what a reader would, and the next sees what came of it.
        fn timer(&mut self, _: TimerId) {
            let WindowHandle::AppKit { view } = self.window.handle() else { panic!("not an AppKit window") };
            // SAFETY: the address is of the window's content view, alive as long as the window.
            let view = unsafe { &*(view as *const NSObject) };
            self.ticks += 1;
            match self.ticks {
                1 => {
                    // Nobody asked yet: no tree is wanted. The first question is answered
                    // with a window that holds nothing.
                    assert_eq!(self.wanted, 0);
                    assert_eq!(children(view).count(), 1);
                }
                2 => {
                    assert_eq!(self.wanted, 1, "the tree is wanted once it is asked for");
                    let done = titled(view, "Done");
                    // SAFETY: messages of accessibility's elements.
                    let (checked, pressed): (bool, bool) = unsafe {
                        let value: Retained<NSObject> = msg_send![&*done, accessibilityValue];
                        (
                            msg_send![&*value, boolValue],
                            msg_send![&*titled(view, "Save"), accessibilityPerformPress],
                        )
                    };
                    assert!(!checked && pressed);
                }
                _ => {
                    assert_eq!(self.asked, [(SAVE, SemanticAction::Click)]);
                    let done = titled(view, "Done");
                    // SAFETY: as above.
                    let checked: bool = unsafe {
                        let value: Retained<NSObject> = msg_send![&*done, accessibilityValue];
                        msg_send![&*value, boolValue]
                    };
                    assert!(checked, "the update after the press is told");
                    self.platform.quit();
                }
            }
        }

        fn wake(&mut self) {}
    }

    pub fn run() {
        let platform = MacPlatform::new().unwrap();
        let window =
            platform.open_window(&WindowOptions::new("Quadra test").size(320.0, 200.0).hidden()).unwrap();
        platform.set_timer(Duration::from_millis(20), true);
        let mut app = App { platform: platform.clone(), window, wanted: 0, asked: Vec::new(), ticks: 0 };
        platform.run(&mut app).unwrap();
        assert_eq!((app.wanted, app.ticks), (1, 3));
    }
}
