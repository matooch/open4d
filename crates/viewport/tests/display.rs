//! Lights, shadows, MSAA and display modes (docs/specs/VIEWPORT.md §8). Needs a GPU adapter
//! (lavapipe in CI); skips without one unless FORGE_REQUIRE_GPU=1. The baseline images are
//! our own: FORGE_UPDATE_BASELINES=1 rewrites them (§8.5).

mod common;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use common::{boxed, unit_box};
use forge_geometry::PolyMesh;
use forge_geometry::selection::Selection;
use forge_viewport::camera::Camera;
use forge_viewport::curves::{Controls, Loop, SplineDraw, draw_list};
use forge_viewport::lights::{Light, Lighting, SceneLights};
use forge_viewport::motion::{
    GhostDraw, MotionDraw, PATH_FUTURE, PATH_PAST, PathDraw, ghost_color, ghost_geometry,
    overlay_lines,
};
use forge_viewport::render::{
    DrawMesh, Frame, Gpu, LineSet, MeshBuffers, Msaa, Palette, Settings, Shading, render_to_rgba,
};
use glam::DVec3;

const W: u32 = 160;
const H: u32 = 120;

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
    fn at(&self, x: i64, y: i64) -> [u8; 3] {
        let (x, y) = (
            x.clamp(0, W as i64 - 1) as usize,
            y.clamp(0, H as i64 - 1) as usize,
        );
        let i = (y * W as usize + x) * 4;
        [self.0[i], self.0[i + 1], self.0[i + 2]]
    }

    /// Mean of the 3x3 block under a world point, per channel.
    fn mean_under(&self, cam: &Camera, p: DVec3) -> [f64; 3] {
        let s = cam.project(p, W as f64, H as f64).expect("in front");
        let (cx, cy) = (s.x as i64, s.y as i64);
        let mut sum = [0.0; 3];
        for dy in -1..=1 {
            for dx in -1..=1 {
                let px = self.at(cx + dx, cy + dy);
                for c in 0..3 {
                    sum[c] += px[c] as f64 / 9.0;
                }
            }
        }
        sum
    }

    fn under(&self, cam: &Camera, p: DVec3) -> [u8; 3] {
        let s = cam.project(p, W as f64, H as f64).expect("in front");
        self.at(s.x as i64, s.y as i64)
    }

    /// Whether a pixel within 2 px of the world point is within 1 of `c`.
    fn near(&self, cam: &Camera, p: DVec3, c: [u8; 3]) -> bool {
        let s = cam.project(p, W as f64, H as f64).expect("in front");
        (-2..=2).any(|dy| (-2..=2).any(|dx| close(self.at(s.x as i64 + dx, s.y as i64 + dy), c)))
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

fn brightness(c: [f64; 3]) -> f64 {
    (c[0] + c[1] + c[2]) / 3.0
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

const GREY: [f32; 3] = [0.7, 0.7, 0.7];

fn cam() -> Camera {
    Camera {
        yaw: 0.6,
        pitch: 0.7,
        distance: 14.0,
        ..Camera::default()
    }
}

/// A ground slab and a box above it: the scene of the baselines and the shadow test.
fn ground_and_box(gpu: &Gpu) -> (MeshBuffers, MeshBuffers) {
    let ground = boxed(DVec3::new(-6.0, -0.2, -6.0), DVec3::new(6.0, 0.0, 6.0));
    let cube = boxed(DVec3::new(-1.0, 0.5, -1.0), DVec3::new(1.0, 2.5, 1.0));
    (
        MeshBuffers::new(gpu, &ground, &Selection::default()),
        MeshBuffers::new(gpu, &cube, &Selection::default()),
    )
}

fn items<'a>(g: &'a MeshBuffers, c: &'a MeshBuffers) -> [DrawMesh<'a>; 2] {
    [
        DrawMesh {
            mesh: g,
            color: [0.55, 0.6, 0.55],
        },
        DrawMesh {
            mesh: c,
            color: [0.8, 0.5, 0.4],
        },
    ]
}

// ---- display modes ---------------------------------------------------------------------

#[test]
fn constant_ignores_lighting() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = Camera {
        yaw: 0.5,
        pitch: 0.4,
        ..Camera::default()
    };
    let mesh = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    let items = [DrawMesh {
        mesh: &mesh,
        color: GREY,
    }];
    let img = render(gpu, &c, &items, settings(Shading::Constant));
    let full = u8c(GREY, 1.0);
    // three visible faces, three different normals, one colour
    for p in [
        DVec3::new(0.0, 1.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
        DVec3::new(1.0, 0.0, 0.0),
    ] {
        assert!(
            close(img.under(&c, p), full),
            "{p}: {:?} vs {full:?}",
            img.under(&c, p)
        );
    }
    let smooth = render(gpu, &c, &items, settings(Shading::Smooth));
    assert_ne!(smooth.under(&c, DVec3::Y), smooth.under(&c, DVec3::Z));
}

#[test]
fn hidden_line_hides_the_far_edges() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = Camera {
        yaw: 0.5,
        pitch: 0.4,
        ..Camera::default()
    };
    let mesh = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    let items = [DrawMesh {
        mesh: &mesh,
        color: GREY,
    }];
    let wire = u8c(Palette::default().wire, 1.0);
    let bg = u8c(Palette::default().background, 1.0);
    let far_edge = DVec3::new(-1.0, -0.5, -1.0);
    let near_edge = DVec3::new(1.0, 0.0, 1.0);
    let w = render(gpu, &c, &items, settings(Shading::Wire));
    assert!(w.near(&c, far_edge, wire), "wire shows the far edge");
    let h = render(gpu, &c, &items, settings(Shading::HiddenLine));
    assert!(!h.near(&c, far_edge, wire), "hidden line hides it");
    assert!(h.near(&c, near_edge, wire), "and keeps the near edge");
    // faces are background-coloured
    assert!(
        h.near(&c, DVec3::new(0.0, 1.0, 0.0), bg),
        "{:?}",
        h.under(&c, DVec3::Y)
    );
}

fn tetra() -> PolyMesh {
    PolyMesh {
        points: vec![
            DVec3::ZERO,
            DVec3::new(2.0, 0.0, 0.0),
            DVec3::new(0.0, 2.0, 0.0),
            DVec3::new(0.0, 0.0, 2.0),
        ],
        polys: vec![vec![0, 2, 1], vec![0, 1, 3], vec![0, 3, 2], vec![1, 2, 3]],
    }
}

#[test]
fn bounding_box_draws_the_box_not_the_mesh() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = Camera {
        yaw: 0.5,
        pitch: 0.4,
        target: DVec3::splat(1.0),
        distance: 10.0,
        ..Camera::default()
    };
    let mesh = MeshBuffers::new(gpu, &tetra(), &Selection::default());
    assert_eq!(mesh.bounds(), Some((DVec3::ZERO, DVec3::splat(2.0))));
    let color = [0.9, 0.4, 0.2];
    let items = [DrawMesh { mesh: &mesh, color }];
    let bg = u8c(Palette::default().background, 1.0);
    let b = render(gpu, &c, &items, settings(Shading::BoundingBox));
    // the middle of the edge from (2,2,0) to (2,2,2) is not on the tetrahedron
    assert!(b.near(&c, DVec3::new(2.0, 2.0, 1.0), u8c(color, 1.0)));
    // the centre of the slanted face is not drawn (no fill)
    assert_eq!(b.under(&c, DVec3::new(2.0, 2.0, 2.0) / 3.0), bg);
    let s = render(gpu, &c, &items, settings(Shading::Smooth));
    assert!(!s.near(&c, DVec3::new(2.0, 2.0, 1.0), u8c(color, 1.0)));
}

#[test]
fn quick_is_the_headlight_whatever_the_lights() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (g, b) = ground_and_box(gpu);
    let it = items(&g, &b);
    let quick = render(
        gpu,
        &cam(),
        &it,
        Settings {
            shading: Shading::Quick,
            lighting: Lighting::Rig,
            shadows: true,
            ..Settings::default()
        },
    );
    let plain = render(gpu, &cam(), &it, settings(Shading::Smooth));
    assert!(quick.0 == plain.0, "quick = plain headlight");
    let rig = render(
        gpu,
        &cam(),
        &it,
        Settings {
            lighting: Lighting::Rig,
            ..settings(Shading::Smooth)
        },
    );
    assert!(rig.0 != plain.0, "the rig does change the smooth image");
}

// ---- lights ----------------------------------------------------------------------------

fn ground_only(gpu: &Gpu) -> MeshBuffers {
    let ground = boxed(DVec3::new(-8.0, -0.2, -8.0), DVec3::new(8.0, 0.0, 8.0));
    MeshBuffers::new(gpu, &ground, &Selection::default())
}

fn top_down() -> Camera {
    Camera {
        yaw: 0.0,
        pitch: 1.5,
        distance: 20.0,
        ..Camera::default()
    }
}

fn lit(lights: &[Light]) -> Settings {
    Settings {
        lighting: Lighting::Scene(SceneLights::new(lights)),
        ..settings(Shading::Smooth)
    }
}

#[test]
fn a_spot_light_lights_its_cone_only() {
    let Some((_lock, gpu)) = gpu() else { return };
    let ground = ground_only(gpu);
    let it = [DrawMesh {
        mesh: &ground,
        color: GREY,
    }];
    let spot = Light::spot(DVec3::new(0.0, 5.0, 0.0), DVec3::NEG_Y, 0.2, 0.5, 1.0);
    let img = render(gpu, &top_down(), &it, lit(&[spot]));
    let centre = brightness(img.mean_under(&top_down(), DVec3::ZERO));
    let outside = brightness(img.mean_under(&top_down(), DVec3::new(5.0, 0.0, 0.0)));
    let ambient = 0.7 * 0.25 * 255.0;
    assert!((outside - ambient).abs() < 2.0, "{outside} vs {ambient}");
    // facing the light: ambient + 1.0 * intensity, clamped by the target
    assert!(centre > ambient * 3.0, "{centre} vs {ambient}");
}

#[test]
fn a_point_light_falls_off_with_range() {
    let Some((_lock, gpu)) = gpu() else { return };
    let ground = ground_only(gpu);
    let it = [DrawMesh {
        mesh: &ground,
        color: GREY,
    }];
    let point = Light::point(DVec3::new(0.0, 2.0, 0.0), 1.0).with_range(7.0);
    let img = render(gpu, &top_down(), &it, lit(&[point]));
    let at = |x: f64| brightness(img.mean_under(&top_down(), DVec3::new(x, 0.0, 0.0)));
    let (b0, b2, b4, b7) = (at(0.0), at(2.0), at(4.0), at(7.5));
    assert!(b0 > b2 && b2 > b4 && b4 > b7, "{b0} {b2} {b4} {b7}");
    let ambient = 0.7 * 0.25 * 255.0;
    assert!(
        (b7 - ambient).abs() < 2.0,
        "beyond the range only ambient: {b7}"
    );
}

#[test]
fn a_shadow_darkens_the_ground_under_the_box() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (g, b) = ground_and_box(gpu);
    let it = items(&g, &b);
    let sun = Light::directional(DVec3::new(-0.8, -1.0, -0.2), 0.75).with_shadow(true);
    // flat shading: the ground has one normal, so lit pixels are all alike
    let on = Settings {
        shadows: true,
        shading: Shading::Flat,
        ..lit(&[sun])
    };
    let off = Settings {
        shadows: false,
        ..on
    };
    // seen from above, nothing hides the ground left of the box
    let c = top_down();
    let in_shadow = DVec3::new(-2.0, 0.0, -0.5);
    let in_light = DVec3::new(3.0, 0.0, 3.0);
    let with = render(gpu, &c, &it, on);
    let without = render(gpu, &c, &it, off);
    let (s_on, l_on) = (
        brightness(with.mean_under(&c, in_shadow)),
        brightness(with.mean_under(&c, in_light)),
    );
    let (s_off, l_off) = (
        brightness(without.mean_under(&c, in_shadow)),
        brightness(without.mean_under(&c, in_light)),
    );
    assert!(s_on < 0.45 * l_on, "shadow {s_on} vs lit {l_on}");
    assert!(
        (s_off - l_off).abs() < 2.0,
        "no shadows: {s_off} vs {l_off}"
    );
    assert!(
        (l_on - l_off).abs() < 2.0,
        "lit ground is unchanged by shadows"
    );
    // the shadow keeps the ambient light: ground colour * 0.25
    let ambient = 0.55 * 0.25 * 255.0;
    assert!(s_on >= ambient * 0.8, "{s_on} vs ambient {ambient}");
    // a light without cast_shadow leaves the image alone
    let no_cast = render(
        gpu,
        &c,
        &it,
        Settings {
            shadows: true,
            shading: Shading::Flat,
            ..lit(&[sun.with_shadow(false)])
        },
    );
    assert!(no_cast.0 == without.0, "no cast_shadow, no shadow");
    // a spot light casts one too
    let spot = Light::spot(
        DVec3::new(6.0, 8.0, 2.0),
        DVec3::new(-1.0, -1.0, -0.3),
        0.3,
        0.6,
        1.0,
    )
    .with_shadow(true);
    let sp = render(
        gpu,
        &top_down(),
        &it,
        Settings {
            shadows: true,
            ..lit(&[spot])
        },
    );
    let sp_off = render(gpu, &top_down(), &it, lit(&[spot]));
    assert!(sp.0 != sp_off.0, "the spot's shadow changes the image");
}

#[test]
fn lights_and_shadow_layers_are_capped_at_eight() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (g, b) = ground_and_box(gpu);
    let it = items(&g, &b);
    let all: Vec<Light> = (0..10)
        .map(|i| {
            let a = i as f64 * 0.6;
            Light::directional(DVec3::new(a.cos(), -1.5, a.sin()), 0.1).with_shadow(true)
        })
        .collect();
    let ten = render(
        gpu,
        &cam(),
        &it,
        Settings {
            shadows: true,
            ..lit(&all)
        },
    );
    let eight = render(
        gpu,
        &cam(),
        &it,
        Settings {
            shadows: true,
            ..lit(&all[..8])
        },
    );
    assert!(ten.0 == eight.0, "lights 9 and 10 are ignored");
}

#[test]
fn bounds_come_from_the_mesh() {
    let Some((_lock, gpu)) = gpu() else { return };
    let m = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    assert_eq!(m.bounds(), Some((DVec3::splat(-1.0), DVec3::splat(1.0))));
    let empty = MeshBuffers::new(gpu, &PolyMesh::default(), &Selection::default());
    assert_eq!(empty.bounds(), None);
}

// ---- MSAA ------------------------------------------------------------------------------

#[test]
fn msaa_smooths_silhouette_edges() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = Camera {
        yaw: 0.5,
        pitch: 0.4,
        ..Camera::default()
    };
    let mesh = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    let it = [DrawMesh {
        mesh: &mesh,
        color: GREY,
    }];
    let bg = u8c(Palette::default().background, 1.0);
    // pixels that are neither background nor one of the three face shades
    let blended = |img: &Image| {
        let mut faces: Vec<[u8; 3]> = Vec::new();
        for y in 0..H as i64 {
            for x in 0..W as i64 {
                let p = img.at(x, y);
                if p != bg && !faces.contains(&p) {
                    faces.push(p);
                }
            }
        }
        faces.len()
    };
    let off = render(gpu, &c, &it, settings(Shading::Smooth));
    let on = render(
        gpu,
        &c,
        &it,
        Settings {
            msaa: Msaa::X4,
            ..settings(Shading::Smooth)
        },
    );
    // smooth shading has a gradient inside faces, so compare against the same mode: the
    // difference is the silhouette and crease pixels
    assert!(
        blended(&on) > blended(&off) + 10,
        "{} vs {}",
        blended(&on),
        blended(&off)
    );
    // the interior of a face is the same
    let centre = DVec3::new(0.0, 1.0, 0.0);
    assert_eq!(on.under(&c, centre), off.under(&c, centre));
    // constant colour: only the face colour, the background and blends
    let flat_off = render(gpu, &c, &it, settings(Shading::Constant));
    let flat_on = render(
        gpu,
        &c,
        &it,
        Settings {
            msaa: Msaa::X4,
            ..settings(Shading::Constant)
        },
    );
    assert_eq!(blended(&flat_off), 1, "aliased: background and one colour");
    assert!(blended(&flat_on) > 3, "anti-aliased: {}", blended(&flat_on));
}

// ---- baseline images -------------------------------------------------------------------

fn baseline_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/baselines")
        .join(format!("{name}.png"))
}

fn write_png(path: &PathBuf, rgba: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), W, H);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
}

fn read_png(path: &PathBuf) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (W, H), "baseline size");
    buf.truncate(info.buffer_size());
    Some(buf)
}

/// (mean absolute channel error, fraction of pixels with a channel off by more than 4).
fn diff(a: &[u8], b: &[u8]) -> (f64, f64) {
    assert_eq!(a.len(), b.len());
    let (mut sum, mut bad) = (0u64, 0usize);
    for (pa, pb) in a.chunks(4).zip(b.chunks(4)) {
        let mut worst = 0;
        for c in 0..3 {
            let d = (pa[c] as i32 - pb[c] as i32).unsigned_abs();
            sum += d as u64;
            worst = worst.max(d);
        }
        if worst > 4 {
            bad += 1;
        }
    }
    let n = (a.len() / 4) as f64;
    (sum as f64 / (n * 3.0), bad as f64 / n)
}

fn check_baseline(gpu: &Gpu, name: &str, img: &Image) {
    let path = baseline_path(name);
    if std::env::var("FORGE_UPDATE_BASELINES").as_deref() == Ok("1") {
        write_png(&path, &img.0);
        return;
    }
    let Some(expected) = read_png(&path) else {
        panic!(
            "missing baseline {}: run FORGE_UPDATE_BASELINES=1 cargo test -p forge-viewport --test display",
            path.display()
        );
    };
    let (mean, frac) = diff(&expected, &img.0);
    // The baselines come from lavapipe (Linux CI, adapter name "llvmpipe"); only that
    // rasterizer is held to the tight bound. Other adapters, including Windows' WARP software
    // renderer, rasterize MSAA and line edges slightly differently, so they get the loose one.
    let lavapipe = gpu.is_software() && gpu.adapter_info.name.to_lowercase().contains("llvmpipe");
    let (max_mean, max_frac) = if lavapipe { (1.0, 0.003) } else { (4.0, 0.05) };
    assert!(
        mean <= max_mean && frac <= max_frac,
        "{name}: mean error {mean:.3} (max {max_mean}), differing pixels {:.3}% (max {:.1}%)",
        frac * 100.0,
        max_frac * 100.0
    );
}

#[test]
fn baseline_images_per_mode() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (g, b) = ground_and_box(gpu);
    let it = items(&g, &b);
    let c = cam();
    let modes = [
        ("smooth", Shading::Smooth),
        ("flat", Shading::Flat),
        ("wire", Shading::Wire),
        ("smooth_wire", Shading::SmoothWire),
        ("flat_wire", Shading::FlatWire),
        ("quick", Shading::Quick),
        ("constant", Shading::Constant),
        ("hidden_line", Shading::HiddenLine),
        ("bounding_box", Shading::BoundingBox),
    ];
    for (name, shading) in modes {
        let img = render(gpu, &c, &it, settings(shading));
        check_baseline(gpu, &format!("mode_{name}"), &img);
    }
}

#[test]
fn baseline_images_per_lighting_setup() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (g, b) = ground_and_box(gpu);
    let it = items(&g, &b);
    let c = cam();
    let sun = Light::directional(DVec3::new(-0.8, -1.0, -0.2), 0.75).with_shadow(true);
    let spot = Light::spot(
        DVec3::new(5.0, 7.0, 4.0),
        DVec3::new(-5.0, -7.0, -4.0),
        0.25,
        0.6,
        1.0,
    )
    .with_shadow(true);
    let point = Light::point(DVec3::new(-3.0, 4.0, 3.0), 1.0).with_range(12.0);
    let setups = [
        ("headlight", settings(Shading::Smooth)),
        (
            "rig",
            Settings {
                lighting: Lighting::Rig,
                ..settings(Shading::Smooth)
            },
        ),
        (
            "rig_shadows",
            Settings {
                lighting: Lighting::Rig,
                shadows: true,
                ..settings(Shading::Smooth)
            },
        ),
        (
            "sun_shadow",
            Settings {
                shadows: true,
                ..lit(&[sun])
            },
        ),
        (
            "spot_shadow",
            Settings {
                shadows: true,
                ..lit(&[spot])
            },
        ),
        ("point_and_spot", lit(&[point, spot.with_shadow(false)])),
        (
            "msaa4",
            Settings {
                msaa: Msaa::X4,
                ..settings(Shading::SmoothWire)
            },
        ),
    ];
    for (name, s) in setups {
        let img = render(gpu, &c, &it, s);
        check_baseline(gpu, &format!("light_{name}"), &img);
    }
}

// ---- splines (VIEWPORT.md section 11) ----------------------------------------------------

/// A Bezier-like arc with its control points and handles, selected point in the middle.
fn arc_draw() -> SplineDraw {
    let pts: Vec<DVec3> = (0..=16)
        .map(|i| {
            let t = i as f64 / 16.0 * std::f64::consts::PI;
            DVec3::new(-3.0 * t.cos(), 3.0 * t.sin() - 1.0, 0.0)
        })
        .collect();
    let points = vec![
        DVec3::new(-3.0, -1.0, 0.0),
        DVec3::new(0.0, 2.0, 0.0),
        DVec3::new(3.0, -1.0, 0.0),
    ];
    let off = DVec3::new(0.0, 1.5, 0.0);
    let hs = [off, DVec3::new(1.5, 0.0, 0.0), -off];
    SplineDraw {
        loops: vec![Loop {
            points: pts,
            closed: false,
        }],
        selected: true,
        controls: Some(Controls {
            left: points.iter().zip(hs).map(|(p, h)| *p - h).collect(),
            right: points.iter().zip(hs).map(|(p, h)| *p + h).collect(),
            points,
            selected: vec![false, true, false],
            handles: true,
        }),
    }
}

fn render_splines(gpu: &Gpu, c: &Camera, d: &SplineDraw, shading: Shading) -> Image {
    let pal = Palette::default();
    let list = draw_list(d, shading, &pal, c, [W as f64, H as f64]);
    let set = LineSet::new(&gpu.device, &list);
    let frame = Frame {
        instanced: &[],
        meshes: &[],
        grid: None,
        lines: &[&set],
        gizmo: None,
        settings: settings(shading),
    };
    Image(render_to_rgba(gpu, W, H, c, &frame))
}

#[test]
fn splines_render_in_every_display_mode() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = Camera {
        distance: 12.0,
        ..Camera::default()
    };
    let d = arc_draw();
    let sel = u8c(Palette::default().selection, 1.0);
    let top = DVec3::new(0.0, 2.0, 0.0);
    for mode in Shading::ALL {
        let img = render_splines(gpu, &c, &d, mode);
        if mode == Shading::BoundingBox {
            // the box edge passes through the corners of the curve's box, not through the arc
            assert!(img.near(&c, DVec3::new(-3.0, 2.0, 0.0), sel), "{mode:?}");
        } else {
            // a curve sample away from every control marker
            assert!(
                img.near(&c, DVec3::new(-2.121_320_3, 1.121_320_3, 0.0), sel),
                "{mode:?}"
            );
        }
        // the selected control point is a 7 px square: its right edge is 3.5 px from the point
        let edge = top + DVec3::new(3.5 * c.world_per_pixel(H as f64), 0.0, 0.0);
        assert!(img.near(&c, edge, sel), "{mode:?}");
    }
}

#[test]
fn baseline_image_spline_points() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = Camera {
        distance: 12.0,
        ..Camera::default()
    };
    let img = render_splines(gpu, &c, &arc_draw(), Shading::Smooth);
    check_baseline(gpu, "spline_points", &img);
}

// ---- motion paths and ghosts (VIEWPORT.md section 12) -------------------------------------

fn render_motion(gpu: &Gpu, c: &Camera, d: &MotionDraw) -> Image {
    let list = overlay_lines(d, &Palette::default(), c, [W as f64, H as f64]);
    let set = LineSet::new(&gpu.device, &list);
    let frame = Frame {
        instanced: &[],
        meshes: &[],
        grid: None,
        lines: &[&set],
        gizmo: None,
        settings: settings(Shading::Smooth),
    };
    Image(render_to_rgba(gpu, W, H, c, &frame))
}

fn motion_draw() -> MotionDraw {
    let samples: Vec<(f64, DVec3)> = (0..=8)
        .map(|i| {
            let x = -3.0 + 0.75 * i as f64;
            (i as f64 * 5.0, DVec3::new(x, 1.5 * (x * 0.5).sin(), 0.0))
        })
        .collect();
    let key = samples[4].1;
    let ghost = |forward, rank, dx: f64| GhostDraw {
        forward,
        rank,
        of: 1,
        lines: ghost_geometry(&boxed(
            DVec3::new(dx - 0.4, -1.8, -0.4),
            DVec3::new(dx + 0.4, -1.0, 0.4),
        )),
    };
    MotionDraw {
        paths: vec![PathDraw {
            current: Some((20.0, key)),
            keys: vec![samples[0].1, key, samples[8].1],
            samples,
        }],
        ghosts: vec![ghost(false, 1, -1.5), ghost(true, 1, 1.5)],
    }
}

#[test]
fn motion_path_and_ghosts_are_visible() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = Camera {
        distance: 12.0,
        ..Camera::default()
    };
    let d = motion_draw();
    let img = render_motion(gpu, &c, &d);
    let pal = Palette::default();
    // the middle of the first (past) segment and of the last (future) one
    let mid = |i: usize| (d.paths[0].samples[i].1 + d.paths[0].samples[i + 1].1) * 0.5;
    assert!(img.near(&c, mid(1), u8c(PATH_PAST, 1.0)));
    assert!(img.near(&c, mid(6), u8c(PATH_FUTURE, 1.0)));
    // the current square's right edge, 4.5 px from the key point
    let key = d.paths[0].keys[1];
    let edge = key + DVec3::new(4.5 * c.world_per_pixel(H as f64), 0.0, 0.0);
    assert!(img.near(&c, edge, u8c(pal.selection, 1.0)));
    // a ghost edge of each side
    let gb = ghost_color(false, 1, 1, &pal);
    let ga = ghost_color(true, 1, 1, &pal);
    assert!(img.near(&c, DVec3::new(-1.5, -1.8, 0.4), u8c(gb, 1.0)));
    assert!(img.near(&c, DVec3::new(1.5, -1.8, 0.4), u8c(ga, 1.0)));
}

#[test]
fn baseline_image_motion_path() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = Camera {
        distance: 12.0,
        ..Camera::default()
    };
    let img = render_motion(gpu, &c, &motion_draw());
    check_baseline(gpu, "motion_path", &img);
}
