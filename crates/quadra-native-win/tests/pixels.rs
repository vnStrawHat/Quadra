//! The D3D11 renderer on the software rasterizer: what the scene says is what the pixels show
//! (specs/12, "Pixel snapshots"). No GPU and no window is needed.

use quadra_native_win::D3d11Renderer;
use quadra_prim::{Affine, Color, Corners, Edges, Point, Rect};
use quadra_render::{
    BorderStyle, Clip, DeviceSize, Instance, LayerConstants, Line, LineMode, Pixels, Quad, Renderer, Scene,
    SurfaceTarget, stroke,
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
        backdrop: None,
        mask: None,
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
fn a_dashed_or_dotted_border_shows_the_fill_between_its_marks() {
    let mut r = D3d11Renderer::new(true).unwrap();
    let mut scene = Scene::new();
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    let framed = |y: f32, width: f32, border_style: BorderStyle| Quad {
        bounds: Rect::new(2.0, y, 60.0, 24.0),
        background: Color::rgb(RED),
        border_widths: Edges::all(width),
        border_color: Color::rgb(BLUE),
        border_style,
        ..Quad::default()
    };
    let (dashed, dotted) = (framed(2.0, 2.0, BorderStyle::Dashed), framed(34.0, 4.0, BorderStyle::Dotted));
    scene.layer_mut(root).set(&[dashed.instance(&Clip::NONE), dotted.instance(&Clip::NONE)]);
    let pixels = draw(&mut r, &mut scene);
    // Dashes and gaps of about 6 along the top (58 between the corners' middles is five
    // periods), from a half dash at the corner; the fill shows in a gap.
    assert_eq!(
        (at(&pixels, 2, 3), at(&pixels, 4, 3), at(&pixels, 8, 3), at(&pixels, 14, 3)),
        (BLUE, BLUE, RED, BLUE)
    );
    assert_eq!((at(&pixels, 2, 8), at(&pixels, 2, 14), at(&pixels, 30, 14)), (RED, BLUE, RED)); // the left side (two periods); inside
    // Dots 4 wide every 8 px (seven between the corners' middles): one at the corner, the
    // fill between.
    assert_eq!((at(&pixels, 3, 36), at(&pixels, 7, 36), at(&pixels, 11, 36)), (BLUE, RED, BLUE));
    assert_eq!(at(&pixels, 30, 46), RED);
}

#[test]
fn a_chart_line_covers_each_pixel_by_how_much_of_it_the_line_is_in() {
    let mut r = D3d11Renderer::new(true).unwrap();
    let mut scene = Scene::new();
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    // A line 2 px tall that falls 2.5 px a column, a column an instance: it starts at a
    // quarter of a pixel, and no column starts on a whole one twice running.
    let (tall, fall) = (2.0, 2.5);
    let column = |x: u32| {
        let bounds = Rect::new(8.0 + x as f32, 10.25 + x as f32 * fall - 1.0, 1.0, tall + fall + 2.0);
        let mode = LineMode::Column { rise: fall };
        Line { bounds, color: Color::BLACK, thickness: tall, mode }.instance(&Clip::NONE)
    };
    scene.layer_mut(root).set(&(0..16).map(column).collect::<Vec<_>>());
    let pixels = draw(&mut r, &mut scene);
    for x in 0..16 {
        // How much of each pixel of the column is ink: black on white.
        let ink: Vec<f32> = (0..64).map(|y| 1.0 - (at(&pixels, 8 + x, y) >> 16) as f32 / 255.0).collect();
        // All of the line is there, whatever part of a pixel it starts at...
        let all: f32 = ink.iter().sum();
        assert!((all - tall).abs() < 0.05, "column {x} holds {all} px of ink");
        // ...its middle is where the line's is, to a twentieth of a pixel: nothing was moved
        // to a whole pixel, which is what makes a slope steps...
        let middle = ink.iter().enumerate().map(|(y, ink)| (y as f32 + 0.5) * ink).sum::<f32>() / all;
        let wanted = 10.25 + x as f32 * fall + (tall + fall) / 2.0;
        assert!((middle - wanted).abs() < 0.05, "column {x} is at {middle}, not {wanted}");
        // ...and its ends are shades.
        assert!(ink.iter().filter(|ink| **ink > 0.03 && **ink < 0.97).count() >= 2);
    }
}

#[test]
fn a_stroke_is_as_wide_as_it_says_and_covers_its_joints_once() {
    let mut r = D3d11Renderer::new(true).unwrap();
    let mut scene = Scene::new();
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    // A line 4 px wide with a sharp turn, half see-through: a joint drawn twice is darker.
    let points = [Point::new(10.0, 10.0), Point::new(40.0, 20.0), Point::new(20.0, 50.0)];
    let mut instances = Vec::new();
    stroke(&points, 4.0, Color::BLACK.alpha(0.5), false, &Clip::NONE, |instance| instances.push(instance));
    assert_eq!(instances.len(), 2);
    scene.layer_mut(root).set(&instances);
    let pixels = draw(&mut r, &mut scene);
    let ink = |x: u32, y: u32| 1.0 - (at(&pixels, x, y) >> 16) as f32 / 255.0;
    let all: Vec<f32> = (0..64 * 64).map(|i| ink(i % 64, i / 64)).collect();
    assert!(all.iter().all(|ink| *ink < 0.52), "a pixel was drawn twice");
    // In the middle of a segment the line is whole, and beside it nothing is.
    assert!((ink(25, 15) - 0.5).abs() < 0.02 && ink(25, 8) == 0.0 && ink(25, 22) == 0.0);
    // Its ink is its width along its length and a round end at each end, half see-through.
    let (length, ends) = (1000f32.sqrt() + 1300f32.sqrt(), std::f32::consts::PI * 4.0);
    let (sum, wanted) = (all.iter().sum::<f32>(), 0.5 * (4.0 * length + ends));
    assert!((sum - wanted).abs() < wanted * 0.04, "{sum} px of ink, not {wanted}");
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

#[test]
fn what_a_frame_cost_the_gpu_comes_back_without_waiting() {
    let mut r = D3d11Renderer::new(true).unwrap();
    let mut scene = Scene::new();
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    scene.layer_mut(root).set(&[quad(0.0, 0.0, 64.0, 64.0, RED)]);
    draw(&mut r, &mut scene);
    draw(&mut r, &mut scene);
    assert_eq!(r.stats().gpu, None); // not measured until asked
    r.measure_gpu(true);
    draw(&mut r, &mut scene); // reading the pixels waits for the GPU to finish the frame
    assert_eq!(r.stats().gpu, None); // asked for when the next frame starts
    draw(&mut r, &mut scene);
    let gpu = r.stats().gpu.expect("the first frame's time");
    assert!(gpu > std::time::Duration::ZERO && gpu < std::time::Duration::from_secs(1), "{gpu:?}");
}
