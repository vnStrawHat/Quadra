//! Display lists, the steps of a frame, and what a backend uploads (specs/07).

use quadra_prim::{Affine, Color, Edges, Rect};
use quadra_render::{
    Clip, DeviceSize, Dirty, FLAG_BORDER, FLAG_GRADIENT, GradientKind, Instance, LayerConstants,
    NullRenderer, Quad, Ramp, RenderError, Renderer, Scene, Step, SurfaceTarget, push_quad,
};

fn quad(x: f32, color: u32) -> Instance {
    Quad { bounds: Rect::new(x, 0.0, 10.0, 10.0), background: Color::rgb(color), ..Quad::default() }
        .instance(&Clip::NONE)
}

fn surface(r: &mut NullRenderer) -> quadra_render::SurfaceId {
    r.create_surface(SurfaceTarget::Offscreen, DeviceSize { width: 800, height: 600 }).unwrap()
}

/// The copy the renderer holds of every layer must be what the scene holds.
fn assert_in_sync(r: &NullRenderer, scene: &Scene, layers: &[quadra_render::LayerId]) {
    for &id in layers {
        assert_eq!(r.uploaded(scene.id(), id), Some(scene.layer(id).instances()), "{id:?}");
    }
}

#[test]
fn an_instance_is_144_bytes_and_quads_follow_their_rules() {
    assert_eq!(std::mem::size_of::<Instance>(), 144);
    let mut out = Vec::new();
    let bounds = Rect::new(0.0, 0.0, 10.0, 10.0);
    push_quad(&mut out, &Quad { bounds, ..Quad::default() }, &Clip::NONE); // transparent: nothing
    push_quad(&mut out, &Quad { background: Color::BLACK, ..Quad::default() }, &Clip::NONE); // empty
    assert!(out.is_empty());
    let both = Quad {
        bounds,
        background: Color::BLACK,
        gradient: Some(Ramp { kind: GradientKind::Conic(90.0), uv: [0.1, 0.2, 0.3, 0.2], slice: 2 }),
        border_widths: Edges::all(1.0),
        border_color: Color::rgb(0xFF0000),
        ..Quad::default()
    };
    push_quad(&mut out, &both, &Clip::NONE); // a gradient and a border: one instance
    assert_eq!(out.len(), 1);
    let i = out[0];
    assert_eq!(
        (i.color2, i.params[..2].to_vec(), i.uv, i.slice, i.flags),
        (
            Color::rgb(0xFF0000).to_array(),
            vec![90.0, 3.0],
            [0.1, 0.2, 0.3, 0.2],
            2,
            FLAG_GRADIENT | FLAG_BORDER
        )
    );
    let inner =
        Clip::NONE.then(Rect::new(0.0, 0.0, 50.0, 50.0), 6.0).then(Rect::new(10.0, 10.0, 100.0, 20.0), 0.0);
    assert_eq!((inner.rect, inner.radius), (Rect::new(10.0, 10.0, 40.0, 20.0), 6.0));
}

#[test]
fn overwriting_in_place_uploads_only_that_range() {
    let (mut scene, mut r) = (Scene::new(), NullRenderer::new());
    let s = surface(&mut r);
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    scene.layer_mut(root).set(&(0..100).map(|i| quad(i as f32, 0x202020)).collect::<Vec<_>>());
    r.draw(s, &mut scene, Color::BLACK).unwrap();
    assert_eq!(
        (r.stats().instances_uploaded, r.stats().draw_calls, r.stats().instances_drawn),
        (100, 1, 100)
    );

    r.draw(s, &mut scene, Color::BLACK).unwrap(); // nothing changed: nothing uploaded
    assert_eq!((r.stats().instances_uploaded, r.stats().layers_uploaded), (0, 0));

    scene.layer_mut(root).patch(40, &[quad(40.0, 0xFF0000), quad(41.0, 0xFF0000)]); // a hover
    scene.layer_mut(root).patch(43, &[quad(43.0, 0x00FF00)]);
    assert_eq!(scene.layer(root).dirty(), Dirty::Range { start: 40, end: 44 });
    r.draw(s, &mut scene, Color::BLACK).unwrap();
    assert_eq!(r.stats().instances_uploaded, 4);
    assert_in_sync(&r, &scene, &[root]);
}

#[test]
fn a_rebuild_copies_clean_ranges_and_child_layers_split_the_runs() {
    let (mut scene, mut r) = (Scene::new(), NullRenderer::new());
    let s = surface(&mut r);
    let (root, child) = (scene.add_layer(), scene.add_layer());
    scene.set_roots(&[root]);
    scene.layer_mut(child).set(&[quad(0.0, 0xAAAAAA), quad(1.0, 0xAAAAAA)]);
    scene.layer_mut(root).set(&[quad(0.0, 1), quad(1.0, 2), quad(2.0, 3), quad(3.0, 4)]);

    // The next list: the first two kept, the child layer, one new instance, the last kept.
    let mut next = scene.layer_mut(root).rebuild();
    assert_eq!(next.copy(0..2), 0);
    assert_eq!(next.child(child), 2);
    assert_eq!(next.push(&[quad(9.0, 9)]), 3);
    assert_eq!((next.copy(3..4), next.len()), (4, 5));
    next.finish();
    assert_eq!(scene.layer(root).breaks(), [(2, child)]);
    assert_eq!(scene.layer(root).instances()[4], quad(3.0, 4));

    r.draw(s, &mut scene, Color::BLACK).unwrap();
    let steps: Vec<_> = r
        .steps()
        .iter()
        .map(|step| match *step {
            Step::Layer { id, .. } => (id == root, 0, 0),
            Step::Run { start, end } => (false, start, end),
        })
        .collect();
    // root, its first run, the child and its run, root again, its second run.
    assert_eq!(
        steps,
        [(true, 0, 0), (false, 0, 2), (false, 0, 0), (false, 0, 2), (true, 0, 0), (false, 3, 5)]
    );
    assert_eq!((r.stats().draw_calls, r.stats().layers_bound, r.stats().instances_drawn), (3, 3, 6));
    assert_in_sync(&r, &scene, &[root, child]);
}

#[test]
fn scrolling_changes_one_constant_and_uploads_nothing() {
    let (mut scene, mut r) = (Scene::new(), NullRenderer::new());
    let s = surface(&mut r);
    let (root, content) = (scene.add_layer(), scene.add_layer());
    scene.set_roots(&[root]);
    scene.layer_mut(content).set(&(0..1000).map(|i| quad(i as f32, 0x808080)).collect::<Vec<_>>());
    let mut list = scene.layer_mut(root).rebuild();
    list.child(content);
    list.finish();
    let viewport = Rect::new(100.0, 50.0, 300.0, 400.0);
    scene.layer_mut(root).constants.transform = Affine::translate(10.0, 0.0);
    scene.layer_mut(content).constants =
        LayerConstants { scissor: Some(viewport), ..LayerConstants::default() };
    r.draw(s, &mut scene, Color::BLACK).unwrap();

    scene.layer_mut(content).constants.transform = Affine::translate(0.0, -120.0); // scroll
    r.draw(s, &mut scene, Color::BLACK).unwrap();
    assert_eq!((r.stats().instances_uploaded, r.stats().layers_uploaded), (0, 0));
    let Step::Layer { world, .. } = r.steps()[0] else { panic!("the content layer is bound first") };
    // Constants compose: the parent's shift, then the scroll; the scissor is in device space.
    assert_eq!(world.transform, Affine::translate(10.0, -120.0));
    assert_eq!(world.scissor, Rect::new(110.0, 50.0, 300.0, 400.0));
}

#[test]
fn a_layer_cut_to_nothing_is_skipped_with_what_is_inside_it() {
    let (mut scene, mut r) = (Scene::new(), NullRenderer::new());
    let s = surface(&mut r);
    let (root, panel, inner) = (scene.add_layer(), scene.add_layer(), scene.add_layer());
    scene.set_roots(&[root]);
    scene.layer_mut(inner).set(&[quad(0.0, 1)]);
    let mut list = scene.layer_mut(panel).rebuild();
    list.push(&[quad(0.0, 2)]);
    list.child(inner);
    list.finish();
    let mut list = scene.layer_mut(root).rebuild();
    list.push(&[quad(0.0, 3)]);
    list.child(panel);
    list.push(&[quad(5.0, 4)]);
    list.finish();
    scene.layer_mut(panel).constants = LayerConstants {
        scissor: Some(Rect::new(900.0, 0.0, 50.0, 50.0)),
        opacity: 0.5,
        ..LayerConstants::default()
    };
    r.draw(s, &mut scene, Color::BLACK).unwrap(); // the panel is off the 800 px surface
    assert_eq!((r.stats().draw_calls, r.stats().layers_bound), (2, 1)); // root is not bound twice

    scene.layer_mut(panel).constants.scissor = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
    scene.layer_mut(inner).constants.opacity = 0.5;
    r.draw(s, &mut scene, Color::BLACK).unwrap();
    assert_eq!(r.stats().draw_calls, 4);
    let opacities: Vec<f32> = r
        .steps()
        .iter()
        .filter_map(|step| match step {
            Step::Layer { world, .. } => Some(world.opacity),
            Step::Run { .. } => None,
        })
        .collect();
    assert_eq!(opacities, [1.0, 0.5, 0.25, 1.0]); // opacity multiplies down
}

#[test]
fn removed_layers_draw_nothing_and_their_buffers_are_freed() {
    let (mut scene, mut r) = (Scene::new(), NullRenderer::new());
    let s = surface(&mut r);
    let (root, child) = (scene.add_layer(), scene.add_layer());
    scene.set_roots(&[root]);
    scene.layer_mut(child).set(&[quad(0.0, 1)]);
    scene.layer_mut(root).set(&[quad(0.0, 2), Instance::marker(child)]);
    r.draw(s, &mut scene, Color::BLACK).unwrap();
    assert_eq!((r.buffers(), r.stats().draw_calls, scene.len()), (2, 2, 2));
    scene.remove_layer(child);
    r.draw(s, &mut scene, Color::BLACK).unwrap(); // the marker still stands: it draws nothing
    assert_eq!((r.buffers(), r.stats().draw_calls), (1, 1));
    let reused = scene.add_layer(); // the same id, another layer: uploaded as new
    scene.layer_mut(reused).set(&[quad(7.0, 7)]);
    r.draw(s, &mut scene, Color::BLACK).unwrap();
    assert_in_sync(&r, &scene, &[root, reused]);
    r.forget_scene(scene.id());
    assert_eq!(r.buffers(), 0);
}

#[test]
fn a_lost_device_is_recovered_by_uploading_everything_again() {
    let (mut scene, mut r) = (Scene::new(), NullRenderer::new());
    let s = surface(&mut r);
    let root = scene.add_layer();
    scene.set_roots(&[root]);
    scene.layer_mut(root).set(&[quad(0.0, 1), quad(1.0, 2)]);
    r.draw(s, &mut scene, Color::BLACK).unwrap();
    r.lose_device();
    assert_eq!(r.draw(s, &mut scene, Color::BLACK), Err(RenderError::DeviceLost));
    r.recover().unwrap();
    scene.invalidate_all();
    r.draw(s, &mut scene, Color::BLACK).unwrap();
    assert_eq!(r.stats().instances_uploaded, 2);
    assert_in_sync(&r, &scene, &[root]);
    assert_eq!(r.read_pixels(s).unwrap().rgba.len(), 800 * 600 * 4);
    r.drop_surface(s);
    assert_eq!(r.present(s), Err(RenderError::BadSurface));
}
