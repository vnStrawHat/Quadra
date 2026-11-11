//! The D3D11 renderer on the software rasterizer: what the scene says is what the pixels show
//! (specs/12, "Pixel snapshots"). No GPU and no window is needed.

use quadra_native_win::D3d11Renderer;
use quadra_prim::{Affine, Color, Corners, Edges, Rect};
use quadra_render::{
    Clip, DeviceSize, Instance, LayerConstants, Pixels, Quad, Renderer, Scene, SurfaceTarget,
};

const RED: u32 = 0xFF0000;
const BLUE: u32 = 0x0000FF;

fn quad(x: f32, y: f32, w: f32, h: f32, color: u32) -> Instance {
    Quad { bounds: Rect::new(x, y, w, h), background: Color::rgb(color), ..Quad::default() }
        .instance(&Clip::NONE)
}

/// The colour at a pixel as 0xRRGGBB.
fn at(pixels: &Pixels, x: u32, y: u32) -> u32 {
    let i = ((y * pixels.size.width + x) * 4) as usize;
    (pixels.rgba[i] as u32) << 16 | (pixels.rgba[i + 1] as u32) << 8 | pixels.rgba[i + 2] as u32
}

fn draw(r: &mut D3d11Renderer, scene: &mut Scene) -> Pixels {
    let surface = r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 64, height: 64 }).unwrap();
    r.draw(surface, scene, Color::WHITE).unwrap();
    let pixels = r.read_pixels(surface).unwrap();
    r.drop_surface(surface);
    pixels
}

#[test]
fn a_quad_is_where_the_scene_puts_it() {
    let mut r = D3d11Renderer::new(true).unwrap();
    assert!(r.is_software());
    let mut scene = Scene::new();
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    scene.layer_mut(root).set(&[quad(10.0, 10.0, 20.0, 20.0, RED)]);
    let pixels = draw(&mut r, &mut scene);
    assert_eq!((at(&pixels, 20, 20), at(&pixels, 10, 10), at(&pixels, 29, 29)), (RED, RED, RED));
    assert_eq!(
        (at(&pixels, 9, 20), at(&pixels, 30, 20), at(&pixels, 40, 40)),
        (0xFFFFFF, 0xFFFFFF, 0xFFFFFF)
    );
    assert_eq!((r.stats().draw_calls, r.stats().instances_uploaded), (1, 1));
}

#[test]
fn a_layer_is_moved_cut_and_faded_by_its_constants() {
    let mut r = D3d11Renderer::new(true).unwrap();
    let mut scene = Scene::new();
    let (root, child) = (scene.add_layer(), scene.add_layer());
    scene.set_roots(&[root]);
    scene.layer_mut(child).set(&[quad(0.0, 0.0, 30.0, 30.0, BLUE)]);
    scene.layer_mut(root).set(&[quad(0.0, 0.0, 8.0, 8.0, RED), Instance::marker(child)]);
    scene.layer_mut(child).constants = LayerConstants {
        transform: Affine::translate(20.0, 20.0),
        scissor: Some(Rect::new(20.0, 20.0, 10.0, 30.0)), // only the left third shows
        opacity: 1.0,
    };
    let pixels = draw(&mut r, &mut scene);
    assert_eq!((at(&pixels, 4, 4), at(&pixels, 25, 25), at(&pixels, 25, 45)), (RED, BLUE, BLUE));
    assert_eq!((at(&pixels, 35, 25), at(&pixels, 15, 25)), (0xFFFFFF, 0xFFFFFF)); // cut, and not there

    scene.layer_mut(child).constants.opacity = 0.5; // half blue over white
    scene.layer_mut(child).constants.transform = Affine::translate(22.0, 20.0);
    let pixels = draw(&mut r, &mut scene);
    let faded = at(&pixels, 25, 25);
    assert!((faded >> 16) > 0x70 && (faded >> 16) < 0x90 && faded & 0xFF == 0xFF, "{faded:06x}");
    assert_eq!(at(&pixels, 21, 25), 0xFFFFFF); // moved two pixels right: nothing at 21 now
    assert_eq!(r.stats().instances_uploaded, 0); // constants only
}

#[test]
fn overwriting_in_place_changes_the_pixels_and_uploads_the_range() {
    let mut r = D3d11Renderer::new(true).unwrap();
    let mut scene = Scene::new();
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    let row: Vec<_> = (0..6).map(|i| quad(i as f32 * 10.0, 0.0, 10.0, 10.0, RED)).collect();
    scene.layer_mut(root).set(&row);
    let surface = r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 64, height: 64 }).unwrap();
    r.draw(surface, &mut scene, Color::WHITE).unwrap();
    scene.layer_mut(root).patch(3, &[quad(30.0, 0.0, 10.0, 10.0, BLUE)]);
    r.draw(surface, &mut scene, Color::WHITE).unwrap();
    let pixels = r.read_pixels(surface).unwrap();
    assert_eq!((at(&pixels, 25, 5), at(&pixels, 35, 5), at(&pixels, 45, 5)), (RED, BLUE, RED));
    assert_eq!(r.stats().instances_uploaded, 1);
}

#[test]
fn corners_borders_and_clips_are_antialiased_shapes() {
    let mut r = D3d11Renderer::new(true).unwrap();
    let mut scene = Scene::new();
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    let round = Quad {
        bounds: Rect::new(8.0, 8.0, 40.0, 40.0),
        background: Color::rgb(RED),
        border_widths: Edges::all(4.0),
        border_color: Color::rgb(BLUE),
        radii: Corners::all(12.0),
        ..Quad::default()
    };
    let clip = Clip::NONE.then(Rect::new(0.0, 0.0, 40.0, 64.0), 0.0); // the right edge is clipped off
    scene.layer_mut(root).set(&[round.instance(&clip)]);
    let pixels = draw(&mut r, &mut scene);
    assert_eq!(at(&pixels, 28, 28), RED); // inside
    assert_eq!(at(&pixels, 10, 28), BLUE); // the border
    assert_eq!(at(&pixels, 8, 8), 0xFFFFFF); // outside the rounded corner
    assert_eq!((at(&pixels, 38, 28), at(&pixels, 42, 28)), (RED, 0xFFFFFF)); // clipped at 40
}

#[test]
fn a_new_device_draws_the_same_after_recover() {
    let mut r = D3d11Renderer::new(true).unwrap();
    let mut scene = Scene::new();
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    scene.layer_mut(root).set(&[quad(0.0, 0.0, 16.0, 16.0, RED)]);
    let surface = r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 32, height: 32 }).unwrap();
    r.draw(surface, &mut scene, Color::WHITE).unwrap();
    r.recover().unwrap(); // as after a lost device
    scene.invalidate_all();
    r.draw(surface, &mut scene, Color::WHITE).unwrap();
    assert_eq!(at(&r.read_pixels(surface).unwrap(), 8, 8), RED);
    assert_eq!(r.stats().instances_uploaded, 1);
}
