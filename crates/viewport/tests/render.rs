//! docs/specs/VIEWPORT.md §6. Needs a GPU adapter; skips without one unless
//! FORGE_REQUIRE_GPU=1.

mod common;

use std::sync::{Mutex, MutexGuard, OnceLock};

use common::{boxed, unit_box};
use forge_geometry::selection::Selection;
use forge_viewport::camera::Camera;
use forge_viewport::gizmo::{Gizmo, GizmoMode};
use forge_viewport::grid::grid_lines;
use forge_viewport::render::{
    DrawMesh, Frame, Gpu, MeshBuffers, Palette, Settings, Shading, render_to_rgba,
};
use glam::DVec3;

const W: u32 = 200;
const H: u32 = 150;

/// One device shared by all tests, used one test at a time: many devices rendering in
/// parallel threads hung a driver once, and the suite must not depend on that.
fn gpu() -> Option<(MutexGuard<'static, ()>, &'static Gpu)> {
    static GPU: OnceLock<Option<Gpu>> = OnceLock::new();
    static LOCK: Mutex<()> = Mutex::new(());
    let gpu = GPU.get_or_init(|| match Gpu::new_headless() {
        Ok(g) => Some(g),
        Err(e) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {e}")
        }
        Err(e) => {
            eprintln!("skipping GPU test: {e}");
            None
        }
    });
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    gpu.as_ref().map(|g| (guard, g))
}

struct Image(Vec<u8>);

impl Image {
    fn at(&self, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * W + x) * 4) as usize;
        [self.0[i], self.0[i + 1], self.0[i + 2]]
    }
    /// Pixel under a world point.
    fn under(&self, cam: &Camera, p: DVec3) -> [u8; 3] {
        let s = cam.project(p, W as f64, H as f64).unwrap();
        self.at(s.x as u32, s.y as u32)
    }
    /// Whether any pixel in the 5×5 block around a world point matches `c`.
    fn near(&self, cam: &Camera, p: DVec3, c: [u8; 3]) -> bool {
        let s = cam.project(p, W as f64, H as f64).unwrap();
        let (cx, cy) = (s.x as i64, s.y as i64);
        (-2..=2).any(|dy| (-2..=2).any(|dx| close(self.at((cx + dx) as u32, (cy + dy) as u32), c)))
    }
}

fn u8c(c: [f32; 3], k: f32) -> [u8; 3] {
    c.map(|x| (x * k * 255.0).round() as u8)
}

fn close(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter()
        .zip(b)
        .all(|(x, y)| (*x as i32 - y as i32).abs() <= 1)
}

fn render(gpu: &Gpu, cam: &Camera, meshes: &[DrawMesh], settings: Settings) -> Image {
    let frame = Frame {
        instanced: &[],
        meshes,
        grid: None,
        lines: &[],
        gizmo: None,
        settings,
    };
    Image(render_to_rgba(gpu, W, H, cam, &frame))
}

fn settings(shading: Shading) -> Settings {
    Settings {
        shading,
        ..Settings::default()
    }
}

#[test]
fn empty_frame_is_background() {
    let Some((_lock, gpu)) = gpu() else { return };
    let img = render(gpu, &Camera::default(), &[], Settings::default());
    assert_eq!(img.0.len(), (W * H * 4) as usize);
    let bg = u8c(Palette::default().background, 1.0);
    for (x, y) in [(0, 0), (W - 1, H - 1), (W / 2, H / 2)] {
        assert!(close(img.at(x, y), bg), "{:?} vs {bg:?}", img.at(x, y));
    }
}

#[test]
fn flat_and_smooth_shading() {
    let Some((_lock, gpu)) = gpu() else { return };
    let cam = Camera::default();
    let mesh = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    let color = [0.7, 0.7, 0.7];
    let items = [DrawMesh { mesh: &mesh, color }];
    let full = u8c(color, 1.0);
    let corner = DVec3::new(0.85, 0.85, 1.0);

    let flat = render(gpu, &cam, &items, settings(Shading::Flat));
    assert!(
        close(flat.under(&cam, DVec3::new(0., 0., 1.)), full),
        "{:?}",
        flat.under(&cam, DVec3::Z)
    );
    assert!(
        close(flat.under(&cam, corner), full),
        "flat faces are uniform"
    );

    let smooth = render(gpu, &cam, &items, settings(Shading::Smooth));
    assert!(close(smooth.under(&cam, DVec3::new(0.01, 0.0, 1.)), full));
    let dark = smooth.under(&cam, corner);
    assert!(
        (dark[0] as i32) < full[0] as i32 - 20,
        "smooth corner {dark:?} vs {full:?}"
    );
}

#[test]
fn material_preview_image_baseline() {
    use forge_geometry::CornerUvs;
    use forge_material::{IrNode, Material, ShadeCtx, TextureSampler, compile};
    use forge_viewport::lights::{Light, Lighting, SceneLights};
    use forge_viewport::material_preview::PreviewMaterial;

    struct Chart;
    impl TextureSampler for Chart {
        fn sample(&self, name: &str, ctx: &ShadeCtx) -> [f64; 3] {
            if name == "normal" {
                return [1.0, 0.5, 1.0];
            }
            match (ctx.uvw[0] > 0.5, ctx.uvw[1] > 0.5) {
                (false, false) => [0.7, 0.1, 0.1],
                (true, false) => [0.1, 0.7, 0.1],
                (false, true) => [0.1, 0.1, 0.7],
                (true, true) => [0.7, 0.7, 0.1],
            }
        }
    }

    let Some((_lock, gpu)) = gpu() else { return };
    let cam = Camera::default();
    let quad = forge_geometry::PolyMesh {
        points: vec![
            DVec3::new(-1.0, -1.0, 0.0),
            DVec3::new(1.0, -1.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(-1.0, 1.0, 0.0),
        ],
        polys: vec![vec![0, 1, 2, 3]],
    };
    let uv = CornerUvs {
        corners: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]],
    };
    let mut graph = compile(&Material::new());
    let diffuse = graph.outputs.diffuse.as_mut().unwrap();
    diffuse.color = graph.nodes.len();
    diffuse.level = 1.0;
    diffuse.roughness = 0.2;
    graph.nodes.push(IrNode::Texture("chart".into()));
    let shot = |graph: &forge_material::ShadingGraph, settings| {
        let surface = PreviewMaterial::bake(&gpu.device, &gpu.queue, graph, &Chart, |_| true);
        let mesh = MeshBuffers::with_preview(
            &gpu.device,
            &quad,
            &Selection::default(),
            None,
            false,
            &uv,
            surface.into(),
        );
        render(
            gpu,
            &cam,
            &[DrawMesh {
                mesh: &mesh,
                color: Palette::default().object,
            }],
            settings,
        )
    };
    let chart = shot(&graph, settings(Shading::Constant));
    let baseline = [
        [[26, 26, 179], [179, 179, 26]],
        [[179, 26, 26], [26, 179, 26]],
    ];
    for (j, row) in baseline.iter().enumerate() {
        for (i, &expected) in row.iter().enumerate() {
            for dy in [-0.15, 0.0, 0.15] {
                for dx in [-0.15, 0.0, 0.15] {
                    let p = DVec3::new(i as f64 - 0.5 + dx, 0.5 - j as f64 + dy, 0.0);
                    assert!(
                        close(chart.under(&cam, p), expected),
                        "{p}: {:?}",
                        chart.under(&cam, p)
                    );
                }
            }
        }
    }
    use forge_viewport::instancing::InstanceInput;
    use forge_viewport::render::{DrawInstanced, InstanceSet};
    use glam::DMat4;
    let surface = PreviewMaterial::bake(&gpu.device, &gpu.queue, &graph, &Chart, |_| true);
    let set = InstanceSet::with_preview(
        &gpu.device,
        std::slice::from_ref(&quad),
        None,
        &InstanceInput {
            matrices: &[DMat4::IDENTITY],
            colors: &[[0.5, 1.0, 1.0]],
            visible: &[true],
            child: &[0],
        },
        &DMat4::IDENTITY,
        std::slice::from_ref(&uv),
        surface.into(),
    );
    let cloned = Image(render_to_rgba(
        gpu,
        W,
        H,
        &cam,
        &Frame {
            instanced: &[DrawInstanced {
                set: &set,
                color: Palette::default().object,
            }],
            meshes: &[],
            grid: None,
            lines: &[],
            gizmo: None,
            settings: settings(Shading::Constant),
        },
    ));
    assert!(close(
        cloned.under(&cam, DVec3::new(-0.5, -0.5, 0.0)),
        [90, 26, 26]
    ));
    let mut lit = settings(Shading::Flat);
    lit.lighting = Lighting::Scene(SceneLights::new(&[Light::directional(
        DVec3::new(-0.65, 0.0, -1.0),
        0.9,
    )]));
    let sample = |image: Image| image.under(&cam, DVec3::new(-0.5, -0.5, 0.0));
    let original = sample(shot(&graph, lit));
    graph.outputs.diffuse.as_mut().unwrap().roughness = 0.95;
    let rough = sample(shot(&graph, lit));
    assert!(
        !close(rough, original),
        "roughness must change the highlight"
    );
    graph.outputs.specular.as_mut().unwrap().metal = true;
    let metallic = sample(shot(&graph, lit));
    assert!(
        metallic[1] + 10 < rough[1],
        "metal must remove diffuse light"
    );
    graph.outputs.specular.as_mut().unwrap().metal = false;
    graph.outputs.normal = Some(forge_material::NormalMap {
        texture: graph.nodes.len(),
        strength: 1.0,
        method: "tangent".into(),
        flip: [false; 3],
        swap_yz: false,
    });
    graph.nodes.push(IrNode::Texture("normal".into()));
    let mapped = sample(shot(&graph, lit));
    assert!(
        mapped[0] > rough[0] + 8,
        "normal map should tilt toward the light: {mapped:?} vs {rough:?}"
    );
}

#[test]
fn wireframe_and_overlay() {
    let Some((_lock, gpu)) = gpu() else { return };
    let cam = Camera::default();
    let mesh = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    let items = [DrawMesh {
        mesh: &mesh,
        color: [0.7; 3],
    }];
    let pal = Palette::default();
    let wire = u8c(pal.wire, 1.0);
    let edge_mid = DVec3::new(0., 1., 1.);

    let w = render(gpu, &cam, &items, settings(Shading::Wire));
    assert!(close(
        w.under(&cam, DVec3::new(0.2, 0.1, 1.)),
        u8c(pal.background, 1.0)
    ));
    assert!(w.near(&cam, edge_mid, wire));

    let o = render(gpu, &cam, &items, settings(Shading::SmoothWire));
    assert!(
        o.near(&cam, edge_mid, wire),
        "overlay edges survive the depth test"
    );
    assert!(!close(
        o.under(&cam, DVec3::new(0.2, 0.1, 1.)),
        u8c(pal.background, 1.0)
    ));
}

#[test]
fn depth_test_and_selection_colour() {
    let Some((_lock, gpu)) = gpu() else { return };
    let cam = Camera::default();
    let back = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    let small = boxed(DVec3::new(-0.3, -0.3, 1.5), DVec3::new(0.3, 0.3, 2.5));
    let front = MeshBuffers::new(gpu, &small, &Selection::default());
    let red = [0.9, 0.1, 0.1];
    let grey = [0.7; 3];
    for order in [[0, 1], [1, 0]] {
        let all = [
            DrawMesh {
                mesh: &front,
                color: red,
            },
            DrawMesh {
                mesh: &back,
                color: grey,
            },
        ];
        let items = [all[order[0]], all[order[1]]];
        let img = render(gpu, &cam, &items, settings(Shading::Flat));
        assert!(
            close(img.under(&cam, DVec3::new(0., 0., 2.5)), u8c(red, 1.0)),
            "order {order:?}"
        );
    }

    let mut sel = Selection::default();
    sel.polys.insert(3); // the +Z face
    let picked = MeshBuffers::new(gpu, &unit_box(), &sel);
    let img = render(
        gpu,
        &cam,
        &[DrawMesh {
            mesh: &picked,
            color: grey,
        }],
        settings(Shading::Flat),
    );
    assert!(close(
        img.under(&cam, DVec3::new(0., 0., 1.)),
        u8c(Palette::default().selection, 1.0)
    ));
}

#[test]
fn grid_and_gizmo_overlays() {
    let Some((_lock, gpu)) = gpu() else { return };
    let mut cam = Camera::default();
    cam.orbit(0.5, 0.6);
    let bg = u8c(Palette::default().background, 1.0);
    let count_non_bg = |img: &Image| {
        (0..H)
            .flat_map(|y| (0..W).map(move |x| (x, y)))
            .filter(|&(x, y)| !close(img.at(x, y), bg))
            .count()
    };

    let none = render(gpu, &cam, &[], Settings::default());
    assert_eq!(count_non_bg(&none), 0);
    let grid = grid_lines(&cam, H as f64);
    let with_grid = Image(render_to_rgba(
        gpu,
        W,
        H,
        &cam,
        &Frame {
            instanced: &[],
            meshes: &[],
            grid: Some(&grid),
            lines: &[],
            gizmo: None,
            settings: Settings::default(),
        },
    ));
    assert!(count_non_bg(&with_grid) > 200);

    // gizmo inside an opaque box is still drawn on top
    let cam = Camera::default();
    let mesh = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    let gizmo = Gizmo {
        origin: DVec3::ZERO,
        mode: GizmoMode::Translate,
        size_px: 60.0,
        ..Gizmo::default()
    };
    let l = gizmo.world_length(&cam, H as f64);
    let img = Image(render_to_rgba(
        gpu,
        W,
        H,
        &cam,
        &Frame {
            instanced: &[],
            meshes: &[DrawMesh {
                mesh: &mesh,
                color: [0.7; 3],
            }],
            grid: None,
            lines: &[],
            gizmo: Some(&gizmo),
            settings: settings(Shading::Flat),
        },
    ));
    let s = cam.project(DVec3::X * 0.7 * l, W as f64, H as f64).unwrap();
    let reddest = (-2..=2)
        .map(|dy: i64| img.at(s.x as u32, (s.y as i64 + dy) as u32))
        .max_by_key(|c| c[0] as i32 - c[1] as i32)
        .unwrap();
    assert!(reddest[0] as i32 > reddest[1] as i32 + 80, "{reddest:?}");
}

#[test]
fn move_and_scale_render_solid_tips_beyond_the_axis_line() {
    let Some((_lock, gpu)) = gpu() else { return };
    let cam = Camera::default();
    for (mode, sample) in [
        (GizmoMode::Translate, DVec3::new(0.89, 0.028, 0.0)),
        (GizmoMode::Scale, DVec3::new(1.025, 0.035, 0.0)),
    ] {
        let gizmo = Gizmo {
            mode,
            size_px: 75.0,
            ..Gizmo::default()
        };
        let length = gizmo.world_length(&cam, H as f64);
        let img = Image(render_to_rgba(
            gpu,
            W,
            H,
            &cam,
            &Frame {
                instanced: &[],
                meshes: &[],
                grid: None,
                lines: &[],
                gizmo: Some(&gizmo),
                settings: Settings::default(),
            },
        ));
        let pixel = img.under(&cam, sample * length);
        assert!(
            i32::from(pixel[0]) > i32::from(pixel[1]) + 65,
            "{mode:?}: {pixel:?}"
        );
        let clear = img.under(&cam, DVec3::new(sample.x, 0.085, 0.0) * length);
        assert!(
            close(clear, u8c(Palette::default().background, 1.0)),
            "{mode:?}: {clear:?}"
        );
    }
}

#[test]
fn rotate_gizmo_draws_rings() {
    let Some((_lock, gpu)) = gpu() else { return };
    let cam = Camera::default(); // looking down -Z: the Z ring faces the camera
    let gizmo = Gizmo {
        mode: GizmoMode::Rotate,
        size_px: 50.0,
        ..Gizmo::default()
    };
    let l = gizmo.world_length(&cam, H as f64);
    let img = Image(render_to_rgba(
        gpu,
        W,
        H,
        &cam,
        &Frame {
            instanced: &[],
            meshes: &[],
            grid: None,
            lines: &[],
            gizmo: Some(&gizmo),
            settings: Settings::default(),
        },
    ));
    // a point on the Z ring at 45° is drawn in the Z colour (blue-dominant)
    let p = DVec3::new(l, l, 0.0) * std::f64::consts::FRAC_1_SQRT_2;
    let s = cam.project(p, W as f64, H as f64).unwrap();
    let bluest = (-2..=2)
        .flat_map(|dy: i64| (-2..=2).map(move |dx: i64| (dx, dy)))
        .map(|(dx, dy)| img.at((s.x as i64 + dx) as u32, (s.y as i64 + dy) as u32))
        .max_by_key(|c| c[2] as i32 - c[0] as i32)
        .unwrap();
    assert!(bluest[2] as i32 > bluest[0] as i32 + 80, "{bluest:?}");
}

#[test]
fn sized_points_are_squares_and_depth_tested() {
    let Some((_lock, gpu)) = gpu() else { return };
    let cam = Camera::default();
    let selected = |i: u32| Selection {
        points: [i].into_iter().collect(),
        ..Selection::default()
    };
    let sel = u8c(Palette::default().selection, 1.0);
    let bg = u8c(Palette::default().background, 1.0);
    let at = |img: &Image, p: DVec3, dx: f64, dy: f64| {
        let s = cam.project(p, W as f64, H as f64).unwrap();
        img.at((s.x + dx) as u32, (s.y + dy) as u32)
    };
    let pts = |shading, point_size| Settings {
        shading,
        show_points: true,
        point_size,
        ..Settings::default()
    };

    // front corner (1, 1, 1), index 7: a 9 px square reaches 3 px up and right of the
    // projected point, where there is only background; 1 px points do not
    let front = MeshBuffers::new(gpu, &unit_box(), &selected(7));
    let items = [DrawMesh {
        mesh: &front,
        color: [0.7; 3],
    }];
    let corner = DVec3::new(1.0, 1.0, 1.0);
    let sized = render(gpu, &cam, &items, pts(Shading::Wire, 9.0));
    assert!(close(at(&sized, corner, 3.0, -3.0), sel));
    assert!(close(at(&sized, corner, -3.0, 3.0), sel));
    assert!(
        close(at(&sized, corner, 7.0, -7.0), bg),
        "outside the square"
    );
    let thin = render(gpu, &cam, &items, pts(Shading::Wire, 0.0));
    assert!(close(at(&thin, corner, 3.0, -3.0), bg));
    // unselected points use the wire colour
    let plain = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    let items = [DrawMesh {
        mesh: &plain,
        color: [0.7; 3],
    }];
    let img = render(gpu, &cam, &items, pts(Shading::Wire, 9.0));
    assert!(close(
        at(&img, corner, 3.0, -3.0),
        u8c(Palette::default().wire, 1.0)
    ));

    // the far corner (-1, -1, -1), index 0, lies behind the front face: its square shows
    // in wireframe (no depth from faces) and not with filled faces
    let back = MeshBuffers::new(gpu, &unit_box(), &selected(0));
    let items = [DrawMesh {
        mesh: &back,
        color: [0.7; 3],
    }];
    let far = DVec3::new(-1.0, -1.0, -1.0);
    let wire = render(gpu, &cam, &items, pts(Shading::Wire, 9.0));
    assert!(close(at(&wire, far, 3.0, 0.0), sel));
    let filled = render(gpu, &cam, &items, pts(Shading::FlatWire, 9.0));
    assert!(
        !close(at(&filled, far, 3.0, 0.0), sel),
        "hidden by the face"
    );

    // no point buffers: nothing is drawn
    let none = MeshBuffers::with_device_no_points(&gpu.device, &unit_box(), &selected(7), None);
    let items = [DrawMesh {
        mesh: &none,
        color: [0.7; 3],
    }];
    let img = render(gpu, &cam, &items, pts(Shading::Wire, 9.0));
    assert!(close(at(&img, corner, 3.0, -3.0), bg));
}
