//! Window surfaces: a swap chain on a window that is never shown, drawn by the software
//! rasterizer. What the scene says is in the back buffer, and the loop gives frames at the
//! pace the swap chain takes them.

use std::time::Duration;

use quadra_native_win::{D3d11Renderer, WinPlatform};
use quadra_platform::{
    Platform, PlatformEvent, PlatformHandler, PlatformWindow, TimerId, WindowHandle, WindowId, WindowOptions,
};
use quadra_prim::{Color, Rect};
use quadra_render::{Clip, DeviceSize, Quad, Renderer, Scene, SurfaceId, SurfaceTarget};

const RED: u32 = 0xFF0000;

fn scene_with_a_quad() -> Scene {
    let mut scene = Scene::new();
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    let quad =
        Quad { bounds: Rect::new(10.0, 10.0, 20.0, 20.0), background: Color::rgb(RED), ..Quad::default() };
    scene.layer_mut(root).set(&[quad.instance(&Clip::NONE)]);
    scene
}

fn target(window: &dyn PlatformWindow) -> SurfaceTarget {
    let WindowHandle::Win32 { hwnd } = window.handle() else { panic!("not a Win32 window") };
    SurfaceTarget::Win32 { hwnd }
}

/// The colour at a pixel of a surface as 0xRRGGBB.
fn at(r: &mut D3d11Renderer, surface: SurfaceId, x: u32, y: u32) -> u32 {
    let pixels = r.read_pixels(surface).unwrap();
    let i = ((y * pixels.size.width + x) * 4) as usize;
    (pixels.rgba[i] as u32) << 16 | (pixels.rgba[i + 1] as u32) << 8 | pixels.rgba[i + 2] as u32
}

#[test]
fn a_window_surface_holds_what_the_scene_says_through_resizes_and_a_new_device() {
    let p = WinPlatform::new().unwrap();
    let w = p.open_window(&WindowOptions::new("surface").size(200.0, 100.0).hidden()).unwrap();
    let mut r = D3d11Renderer::for_platform(&p, true).unwrap();
    let mut scene = scene_with_a_quad();
    let surface = r.create_surface(target(&*w), DeviceSize { width: 200, height: 100 }).unwrap();
    r.draw(surface, &mut scene, Color::WHITE).unwrap();
    assert_eq!((at(&mut r, surface, 20, 20), at(&mut r, surface, 5, 5)), (RED, 0xFFFFFF));
    r.present(surface).unwrap();

    r.resize_surface(surface, DeviceSize { width: 64, height: 48 }).unwrap();
    r.resize_surface(surface, DeviceSize { width: 0, height: 0 }).unwrap(); // minimized: kept
    r.draw(surface, &mut scene, Color::WHITE).unwrap();
    assert_eq!(r.read_pixels(surface).unwrap().size, DeviceSize { width: 64, height: 48 });
    assert_eq!(at(&mut r, surface, 20, 20), RED);
    r.present(surface).unwrap();

    r.recover().unwrap(); // as after a lost device: the window gets a new swap chain
    scene.invalidate_all();
    r.draw(surface, &mut scene, Color::WHITE).unwrap();
    assert_eq!(at(&mut r, surface, 20, 20), RED);
    r.present(surface).unwrap();
    r.drop_surface(surface);
    assert!(r.present(surface).is_err());
}

/// Draws and presents a frame each time it is given one, and asks for the next.
struct Animation {
    platform: WinPlatform,
    window: Box<dyn PlatformWindow>,
    renderer: D3d11Renderer,
    surface: SurfaceId,
    scene: Scene,
    frames: u32,
    /// How many of the first frames are not presented, as when a draw fails.
    unshown: u32,
    gave_up: bool,
}

impl PlatformHandler for Animation {
    fn event(&mut self, _: WindowId, _: PlatformEvent) {}

    fn frame(&mut self, window: WindowId) {
        assert_eq!(window, self.window.id());
        self.renderer.draw(self.surface, &mut self.scene, Color::WHITE).unwrap();
        if self.frames >= self.unshown {
            self.renderer.present(self.surface).unwrap();
        }
        self.frames += 1;
        if self.frames == 5 {
            self.platform.quit();
        } else {
            self.window.request_frame();
        }
    }

    fn timer(&mut self, _: TimerId) {
        self.gave_up = true;
        self.platform.quit();
    }

    fn wake(&mut self) {}
}

#[test]
fn frames_are_given_as_the_swap_chain_takes_them() {
    paced(0);
}

/// A frame not presented takes a frame from the swap chain that no present gives back: the
/// loop does not wait for it for ever, which would leave the window drawing nothing.
#[test]
fn frames_go_on_after_one_that_was_not_presented() {
    paced(1);
}

fn paced(unshown: u32) {
    let p = WinPlatform::new().unwrap();
    let window = p.open_window(&WindowOptions::new("paced").size(120.0, 80.0).hidden()).unwrap();
    let mut renderer = D3d11Renderer::for_platform(&p, true).unwrap();
    let surface = renderer.create_surface(target(&*window), DeviceSize { width: 120, height: 80 }).unwrap();
    window.request_frame();
    let mut animation = Animation {
        platform: p.clone(),
        window,
        renderer,
        surface,
        scene: scene_with_a_quad(),
        frames: 0,
        unshown,
        gave_up: false,
    };
    let guard = p.set_timer(Duration::from_secs(5), false);
    p.run(&mut animation).unwrap();
    p.cancel_timer(guard);
    assert!(!animation.gave_up, "the swap chain never took a frame: {} drawn", animation.frames);
    assert_eq!(animation.frames, 5);
}

/// Leaves the loop at a wake, or when the guard timer fires.
struct UntilWoken(WinPlatform, bool);

impl PlatformHandler for UntilWoken {
    fn event(&mut self, _: WindowId, _: PlatformEvent) {}
    fn frame(&mut self, _: WindowId) {}
    fn timer(&mut self, _: TimerId) {
        self.0.quit();
    }
    fn wake(&mut self) {
        self.1 = true;
        self.0.quit();
    }
}

#[test]
fn a_starting_renderer_copies_its_frames_to_the_window_then_moves_to_the_device_made_meanwhile() {
    let p = WinPlatform::new().unwrap();
    let w = p.open_window(&WindowOptions::new("starting").size(120.0, 80.0).hidden()).unwrap();
    let mut r = D3d11Renderer::starting(&p);
    assert!(r.is_software()); // ready at once
    let mut scene = scene_with_a_quad();
    let surface = r.create_surface(target(&*w), DeviceSize { width: 120, height: 80 }).unwrap();
    // The first frame, unless the other device was made before it could be drawn.
    if r.draw(surface, &mut scene, Color::WHITE).is_ok() {
        assert_eq!((at(&mut r, surface, 20, 20), at(&mut r, surface, 5, 5)), (RED, 0xFFFFFF));
        r.present(surface).unwrap(); // copied to the window: it has no swap chain yet
    }

    let mut waiting = UntilWoken(p.clone(), false);
    let guard = p.set_timer(Duration::from_secs(20), false);
    p.run(&mut waiting).unwrap(); // the worker wakes the loop when the device is made
    p.cancel_timer(guard);
    assert!(waiting.1, "the worker never woke the loop");
    assert!(r.wants_frame());
    assert_eq!(r.draw(surface, &mut scene, Color::WHITE), Err(quadra_render::RenderError::DeviceLost));
    r.recover().unwrap(); // takes the device that waits; the window gets its swap chain
    assert!(!r.wants_frame());
    scene.invalidate_all();
    r.draw(surface, &mut scene, Color::WHITE).unwrap();
    assert_eq!((at(&mut r, surface, 20, 20), at(&mut r, surface, 5, 5)), (RED, 0xFFFFFF));
    r.present(surface).unwrap();
}
