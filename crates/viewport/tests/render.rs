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
        meshes,
        grid: None,
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
            meshes: &[],
            grid: Some(&grid),
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
            meshes: &[DrawMesh {
                mesh: &mesh,
                color: [0.7; 3],
            }],
            grid: None,
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
            meshes: &[],
            grid: None,
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
