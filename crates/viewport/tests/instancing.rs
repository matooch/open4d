//! GPU instancing, culling, chunked upload and LOD on a real (software) adapter
//! (docs/specs/VIEWPORT.md §10). Skips without an adapter unless FORGE_REQUIRE_GPU=1. The
//! baseline images are our own: FORGE_UPDATE_BASELINES=1 rewrites them (§8.5).

mod common;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use common::{boxed, unit_box};
use forge_geometry::PolyMesh;
use forge_geometry::selection::Selection;
use forge_viewport::camera::Camera;
use forge_viewport::instancing::InstanceInput;
use forge_viewport::lights::Lighting;
use forge_viewport::lod::lod_proxy;
use forge_viewport::render::{
    DrawInstanced, DrawMesh, Frame, Gpu, InstanceSet, MeshBuffers, Msaa, Settings, Shading,
    UploadOptions, render_to_rgba,
};
use glam::{DMat4, DQuat, DVec3};

const W: u32 = 160;
const H: u32 = 120;
const GREY: [f32; 3] = [0.7, 0.7, 0.7];

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

struct Clones {
    matrices: Vec<DMat4>,
    colors: Vec<[f64; 3]>,
    visible: Vec<bool>,
    child: Vec<u32>,
}

impl Clones {
    fn input(&self) -> InstanceInput<'_> {
        InstanceInput {
            matrices: &self.matrices,
            colors: &self.colors,
            visible: &self.visible,
            child: &self.child,
        }
    }

    /// The geometry the cloner generator expands these clones to (clone-toolset spec C3).
    fn expand(&self, protos: &[PolyMesh]) -> PolyMesh {
        let mut out = PolyMesh::default();
        for i in 0..self.matrices.len() {
            if !self.visible[i] {
                continue;
            }
            let proto = &protos[self.child[i] as usize];
            let base = out.points.len() as u32;
            out.points.extend(
                proto
                    .points
                    .iter()
                    .map(|p| self.matrices[i].transform_point3(*p)),
            );
            out.polys.extend(
                proto
                    .polys
                    .iter()
                    .map(|f| f.iter().map(|v| v + base).collect::<Vec<u32>>()),
            );
        }
        out
    }
}

/// `n x n` clones spaced 3 apart, centred on the origin; `rotate` turns each by a different
/// angle, `scale` makes the size uniform-scale vary.
fn clones(n: usize, rotate: bool, scale: bool) -> Clones {
    let mut c = Clones {
        matrices: vec![],
        colors: vec![],
        visible: vec![],
        child: vec![],
    };
    let half = (n as f64 - 1.0) * 1.5;
    for i in 0..n {
        for j in 0..n {
            let k = i * n + j;
            let s = if scale {
                0.5 + (k % 4) as f64 * 0.2
            } else {
                0.6
            };
            let r = if rotate {
                DQuat::from_euler(glam::EulerRot::YXZ, k as f64 * 0.5, k as f64 * 0.3, 0.0)
            } else {
                DQuat::IDENTITY
            };
            c.matrices.push(DMat4::from_scale_rotation_translation(
                DVec3::splat(s),
                r,
                DVec3::new(i as f64 * 3.0 - half, 0.0, j as f64 * 3.0 - half),
            ));
            c.colors.push([1.0; 3]);
            c.visible.push(true);
            c.child.push(0);
        }
    }
    c
}

fn cam() -> Camera {
    Camera {
        yaw: 0.6,
        pitch: 0.7,
        distance: 20.0,
        ..Camera::default()
    }
}

fn settings(shading: Shading) -> Settings {
    Settings {
        shading,
        ..Settings::default()
    }
}

fn pixels(gpu: &Gpu, cam: &Camera, frame: &Frame) -> Vec<u8> {
    render_to_rgba(gpu, W, H, cam, frame)
}

fn render_mesh(gpu: &Gpu, cam: &Camera, mesh: &MeshBuffers, s: Settings) -> Vec<u8> {
    pixels(
        gpu,
        cam,
        &Frame {
            meshes: &[DrawMesh { mesh, color: GREY }],
            instanced: &[],
            grid: None,
            gizmo: None,
            settings: s,
        },
    )
}

fn render_set(gpu: &Gpu, cam: &Camera, set: &InstanceSet, s: Settings) -> Vec<u8> {
    pixels(
        gpu,
        cam,
        &Frame {
            meshes: &[],
            instanced: &[DrawInstanced { set, color: GREY }],
            grid: None,
            gizmo: None,
            settings: s,
        },
    )
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

/// The tolerance of the baseline images (§8.5): tight on lavapipe, loose elsewhere.
fn tolerance(gpu: &Gpu) -> (f64, f64) {
    let lavapipe = gpu.is_software() && gpu.adapter_info.name.to_lowercase().contains("llvmpipe");
    if lavapipe { (1.0, 0.003) } else { (4.0, 0.05) }
}

fn assert_same_image(gpu: &Gpu, what: &str, a: &[u8], b: &[u8]) {
    let (mean, frac) = diff(a, b);
    let (max_mean, max_frac) = tolerance(gpu);
    assert!(
        mean <= max_mean && frac <= max_frac,
        "{what}: mean error {mean:.3} (max {max_mean}), differing pixels {:.3}% (max {:.1}%)",
        frac * 100.0,
        max_frac * 100.0
    );
}

fn background_only(gpu: &Gpu, s: Settings) -> Vec<u8> {
    pixels(
        gpu,
        &cam(),
        &Frame {
            meshes: &[],
            instanced: &[],
            grid: None,
            gizmo: None,
            settings: s,
        },
    )
}

fn nonbackground(img: &[u8], bg: &[u8]) -> usize {
    img.chunks(4)
        .zip(bg.chunks(4))
        .filter(|(a, b)| a != b)
        .count()
}

// ---- instanced == expanded -----------------------------------------------------------------------

#[test]
fn instanced_render_equals_the_expanded_geometry_per_display_mode() {
    let Some((_g, gpu)) = gpu() else { return };
    let protos = vec![unit_box()];
    // rotation and uniform scale: smooth normals transform exactly
    let c = clones(5, true, true);
    let set = InstanceSet::new(
        &gpu.device,
        &protos,
        Some(std::f64::consts::PI),
        &c.input(),
        &DMat4::IDENTITY,
    );
    assert_eq!(set.len(), 25);
    let expanded = MeshBuffers::new(gpu, &c.expand(&protos), &Selection::default());
    for shading in [
        Shading::Smooth,
        Shading::Flat,
        Shading::SmoothWire,
        Shading::FlatWire,
        Shading::Wire,
        Shading::Quick,
        Shading::Constant,
        Shading::HiddenLine,
    ] {
        let s = settings(shading);
        let a = render_set(gpu, &cam(), &set, s);
        let b = render_mesh(gpu, &cam(), &expanded, s);
        assert!(
            nonbackground(&a, &background_only(gpu, s)) > 1000,
            "{shading:?} draws something"
        );
        assert_same_image(gpu, &format!("{shading:?}"), &a, &b);
    }
}

#[test]
fn non_uniform_scale_shades_like_the_expanded_mesh_with_face_normals() {
    let Some((_g, gpu)) = gpu() else { return };
    let protos = vec![unit_box()];
    let mut c = clones(4, true, false);
    for (k, m) in c.matrices.iter_mut().enumerate() {
        let squash = DMat4::from_scale(DVec3::new(1.8, 0.4 + (k % 3) as f64 * 0.3, 0.7));
        *m *= squash;
    }
    let set = InstanceSet::new(
        &gpu.device,
        &protos,
        Some(std::f64::consts::PI),
        &c.input(),
        &DMat4::IDENTITY,
    );
    let expanded = MeshBuffers::new(gpu, &c.expand(&protos), &Selection::default());
    let s = settings(Shading::Flat);
    assert_same_image(
        gpu,
        "flat, non-uniform scale",
        &render_set(gpu, &cam(), &set, s),
        &render_mesh(gpu, &cam(), &expanded, s),
    );
}

#[test]
fn several_prototypes_and_a_world_matrix() {
    let Some((_g, gpu)) = gpu() else { return };
    let protos = vec![
        unit_box(),
        boxed(DVec3::new(-0.4, 0.0, -0.4), DVec3::new(0.4, 2.2, 0.4)),
    ];
    let mut c = clones(4, true, true);
    for (i, ch) in c.child.iter_mut().enumerate() {
        *ch = (i % 2) as u32;
    }
    let world = DMat4::from_translation(DVec3::new(0.5, 0.3, -0.5)) * DMat4::from_rotation_y(0.4);
    let set = InstanceSet::new(
        &gpu.device,
        &protos,
        Some(std::f64::consts::PI),
        &c.input(),
        &world,
    );
    let mut placed = c.expand(&protos);
    for p in &mut placed.points {
        *p = world.transform_point3(*p);
    }
    let expanded = MeshBuffers::new(gpu, &placed, &Selection::default());
    let s = settings(Shading::SmoothWire);
    assert_same_image(
        gpu,
        "two prototypes under a world matrix",
        &render_set(gpu, &cam(), &set, s),
        &render_mesh(gpu, &cam(), &expanded, s),
    );
}

fn rig_shadow_settings() -> Settings {
    Settings {
        // flat: the ground slab is scaled non-uniformly, where only face normals are exact
        shading: Shading::Flat,
        lighting: Lighting::Rig,
        shadows: true,
        shadow_size: 512,
        ..Settings::default()
    }
}

#[test]
fn instanced_shadows_equal_the_expanded_geometry() {
    let Some((_g, gpu)) = gpu() else { return };
    let protos = vec![unit_box()];
    // translation only: the instance boxes are exact, so the shadow fit equals the mesh bounds
    let mut c = clones(4, false, false);
    c.matrices.push(DMat4::from_scale_rotation_translation(
        DVec3::new(9.0, 0.1, 9.0),
        DQuat::IDENTITY,
        DVec3::new(0.0, -1.0, 0.0),
    ));
    c.colors.push([1.0; 3]);
    c.visible.push(true);
    c.child.push(0);
    let set = InstanceSet::new(
        &gpu.device,
        &protos,
        Some(std::f64::consts::PI),
        &c.input(),
        &DMat4::IDENTITY,
    );
    let expanded = MeshBuffers::new(gpu, &c.expand(&protos), &Selection::default());
    let s = rig_shadow_settings();
    let a = render_set(gpu, &cam(), &set, s);
    let b = render_mesh(gpu, &cam(), &expanded, s);
    assert_same_image(gpu, "rig with shadows", &a, &b);
    // and the shadows are really there: the same set without shadows differs
    let no_shadow = render_set(
        gpu,
        &cam(),
        &set,
        Settings {
            shadows: false,
            ..s
        },
    );
    assert!(diff(&a, &no_shadow).0 > 0.3, "shadows change the image");
    check_baseline(gpu, "instanced_rig_shadows", &a);
}

#[test]
fn instanced_msaa_equals_the_expanded_geometry() {
    let Some((_g, gpu)) = gpu() else { return };
    let protos = vec![unit_box()];
    let c = clones(3, true, true);
    let set = InstanceSet::new(
        &gpu.device,
        &protos,
        Some(std::f64::consts::PI),
        &c.input(),
        &DMat4::IDENTITY,
    );
    let expanded = MeshBuffers::new(gpu, &c.expand(&protos), &Selection::default());
    let s = Settings {
        msaa: Msaa::X4,
        ..settings(Shading::SmoothWire)
    };
    assert_same_image(
        gpu,
        "4x MSAA",
        &render_set(gpu, &cam(), &set, s),
        &render_mesh(gpu, &cam(), &expanded, s),
    );
}

// ---- colour, visibility ------------------------------------------------------------------------------

#[test]
fn clone_colours_tint_the_base_colour_and_white_leaves_it() {
    let Some((_g, gpu)) = gpu() else { return };
    let protos = vec![unit_box()];
    let mut c = clones(1, false, false);
    c.matrices[0] = DMat4::IDENTITY;
    let s = settings(Shading::Constant);
    let centre = |img: &[u8]| {
        let i = ((H / 2) * W + W / 2) as usize * 4;
        [img[i], img[i + 1], img[i + 2]]
    };
    let near = Camera {
        distance: 6.0,
        ..cam()
    };
    let white = InstanceSet::new(&gpu.device, &protos, None, &c.input(), &DMat4::IDENTITY);
    let img = render_set(gpu, &near, &white, s);
    assert_eq!(centre(&img), [178, 178, 178], "0.7 * 255 rounded");
    c.colors[0] = [1.0, 0.5, 0.0];
    let tinted = InstanceSet::new(&gpu.device, &protos, None, &c.input(), &DMat4::IDENTITY);
    let img = render_set(gpu, &near, &tinted, s);
    let p = centre(&img);
    assert_eq!(p[0], 178);
    assert!((p[1] as i32 - 89).abs() <= 1, "{p:?}");
    assert_eq!(p[2], 0);
    // Hidden Line ignores clone colours: tinted and white clones look the same
    let hl = settings(Shading::HiddenLine);
    assert_eq!(
        render_set(gpu, &near, &tinted, hl),
        render_set(gpu, &near, &white, hl)
    );
}

#[test]
fn hidden_clones_are_not_drawn() {
    let Some((_g, gpu)) = gpu() else { return };
    let protos = vec![unit_box()];
    let mut c = clones(3, false, false);
    c.visible.iter_mut().for_each(|v| *v = false);
    let set = InstanceSet::new(&gpu.device, &protos, None, &c.input(), &DMat4::IDENTITY);
    assert!(set.is_empty());
    let s = settings(Shading::Smooth);
    let img = render_set(gpu, &cam(), &set, s);
    assert_eq!(nonbackground(&img, &background_only(gpu, s)), 0);
    // one visible clone draws, and matches the expanded single box
    c.visible[4] = true;
    let one = InstanceSet::new(
        &gpu.device,
        &protos,
        Some(std::f64::consts::PI),
        &c.input(),
        &DMat4::IDENTITY,
    );
    assert_eq!(one.len(), 1);
    let expanded = MeshBuffers::new(gpu, &c.expand(&protos), &Selection::default());
    assert_same_image(
        gpu,
        "one visible clone",
        &render_set(gpu, &cam(), &one, s),
        &render_mesh(gpu, &cam(), &expanded, s),
    );
}

// ---- culling -------------------------------------------------------------------------------------------

#[test]
fn frustum_culling_skips_off_screen_chunks_without_changing_the_image() {
    let Some((_g, gpu)) = gpu() else { return };
    let protos = vec![unit_box()];
    let c = clones(20, false, false); // 400 clones over 57 x 57 units
    // small chunks so that culling has something to skip
    let set = InstanceSet::with_chunk(
        &gpu.device,
        &protos,
        Some(std::f64::consts::PI),
        &c.input(),
        &DMat4::IDENTITY,
        16,
    );
    assert_eq!(set.len(), 400);
    let expanded = MeshBuffers::new(gpu, &c.expand(&protos), &Selection::default());
    let s = settings(Shading::Smooth);
    // everything in view
    let wide = Camera {
        distance: 120.0,
        pitch: 1.2,
        ..Camera::default()
    };
    let img = render_set(gpu, &wide, &set, s);
    let st = gpu.last_stats(Msaa::Off);
    assert_eq!(st.instances_drawn, 400);
    assert_eq!(st.culled, 0);
    assert_same_image(
        gpu,
        "wide view",
        &img,
        &render_mesh(gpu, &wide, &expanded, s),
    );
    // zoomed in on one corner: most chunks are outside the frustum
    let close = Camera {
        target: DVec3::new(-28.0, 0.0, -28.0),
        distance: 9.0,
        pitch: 1.0,
        ..Camera::default()
    };
    let img = render_set(gpu, &close, &set, s);
    let st = gpu.last_stats(Msaa::Off);
    assert!(st.instances_drawn > 0 && st.instances_drawn < 100, "{st:?}");
    assert!(st.culled > 10, "{st:?}");
    assert_same_image(
        gpu,
        "zoomed view",
        &img,
        &render_mesh(gpu, &close, &expanded, s),
    );
    // looking at empty space: nothing is drawn, nothing but the background shows
    let away = Camera {
        target: DVec3::new(500.0, 0.0, 500.0),
        distance: 10.0,
        ..Camera::default()
    };
    let img = render_set(gpu, &away, &set, s);
    let st = gpu.last_stats(Msaa::Off);
    assert_eq!((st.instances_drawn, st.draw_calls), (0, 0), "{st:?}");
    assert_eq!(img, background_only_with(gpu, &away, s));
}

fn background_only_with(gpu: &Gpu, cam: &Camera, s: Settings) -> Vec<u8> {
    pixels(
        gpu,
        cam,
        &Frame {
            meshes: &[],
            instanced: &[],
            grid: None,
            gizmo: None,
            settings: s,
        },
    )
}

#[test]
fn a_mesh_outside_the_view_is_culled() {
    let Some((_g, gpu)) = gpu() else { return };
    let far = MeshBuffers::new(
        gpu,
        &boxed(DVec3::new(900.0, 0.0, 900.0), DVec3::new(902.0, 2.0, 902.0)),
        &Selection::default(),
    );
    let s = settings(Shading::Smooth);
    let img = render_mesh(gpu, &cam(), &far, s);
    assert_eq!(gpu.last_stats(Msaa::Off).culled, 1);
    assert_eq!(gpu.last_stats(Msaa::Off).draw_calls, 0);
    assert_eq!(img, background_only(gpu, s));
    let near = MeshBuffers::new(gpu, &unit_box(), &Selection::default());
    render_mesh(gpu, &cam(), &near, s);
    assert_eq!(gpu.last_stats(Msaa::Off).culled, 0);
    assert_eq!(gpu.last_stats(Msaa::Off).draw_calls, 1);
}

// ---- chunked upload ------------------------------------------------------------------------------------

/// A bumpy `n x n` quad sheet.
fn sheet(n: u32) -> PolyMesh {
    let mut points = vec![];
    for i in 0..=n {
        for j in 0..=n {
            let (x, z) = (
                i as f64 / n as f64 * 6.0 - 3.0,
                j as f64 / n as f64 * 6.0 - 3.0,
            );
            points.push(DVec3::new(x, (x * 1.3).sin() * (z * 1.1).cos() * 0.5, z));
        }
    }
    let mut polys = vec![];
    for i in 0..n {
        for j in 0..n {
            let a = i * (n + 1) + j;
            polys.push(vec![a, a + 1, a + n + 2, a + n + 1]);
        }
    }
    PolyMesh { points, polys }
}

#[test]
fn chunked_upload_renders_exactly_like_one_buffer() {
    let Some((_g, gpu)) = gpu() else { return };
    let mesh = sheet(24); // 1152 triangles
    let one = MeshBuffers::new(gpu, &mesh, &Selection::default());
    assert_eq!(one.triangle_count(), 1152);
    assert_eq!(one.triangle_chunks(), 1);
    let many = MeshBuffers::with_options(
        &gpu.device,
        &mesh,
        &Selection::default(),
        Some(std::f64::consts::PI),
        UploadOptions { chunk_tris: 100 },
    );
    assert_eq!(many.triangle_count(), 1152);
    assert_eq!(many.triangle_chunks(), 12);
    for shading in [Shading::Smooth, Shading::SmoothWire, Shading::Wire] {
        let s = settings(shading);
        assert_eq!(
            render_mesh(gpu, &cam(), &one, s),
            render_mesh(gpu, &cam(), &many, s),
            "{shading:?}: same vertices in the same order, byte-identical"
        );
    }
    // a camera that sees one corner skips chunks of the sheet without changing what is seen
    let corner = Camera {
        target: DVec3::new(-3.0, 0.0, -3.0),
        distance: 3.0,
        pitch: 1.0,
        ..Camera::default()
    };
    let s = settings(Shading::Smooth);
    let a = render_mesh(gpu, &corner, &one, s);
    let b = render_mesh(gpu, &corner, &many, s);
    assert_eq!(a, b);
    assert!(
        gpu.last_stats(Msaa::Off).culled > 0,
        "chunks outside the view are skipped"
    );
}

#[test]
fn an_empty_mesh_uploads_nothing_and_draws_nothing() {
    let Some((_g, gpu)) = gpu() else { return };
    let empty = MeshBuffers::new(gpu, &PolyMesh::default(), &Selection::default());
    assert_eq!((empty.triangle_count(), empty.triangle_chunks()), (0, 0));
    let s = settings(Shading::SmoothWire);
    assert_eq!(render_mesh(gpu, &cam(), &empty, s), background_only(gpu, s));
}

// ---- LOD --------------------------------------------------------------------------------------------------------

#[test]
fn lod_proxy_is_drawn_only_when_the_object_is_small_on_screen() {
    let Some((_g, gpu)) = gpu() else { return };
    let mesh = sheet(32);
    let mut full = MeshBuffers::new(gpu, &mesh, &Selection::default());
    let proxy = lod_proxy(&mesh, 3);
    assert!(proxy.polys.len() < mesh.polys.len() / 10);
    full.set_lod(MeshBuffers::new(gpu, &proxy, &Selection::default()));
    assert!(full.has_lod());
    let plain = MeshBuffers::new(gpu, &mesh, &Selection::default());
    let s = settings(Shading::Smooth);
    // lod_px = 0: off, identical to a mesh without a proxy
    assert_eq!(
        render_mesh(gpu, &cam(), &full, s),
        render_mesh(gpu, &cam(), &plain, s)
    );
    assert_eq!(gpu.last_stats(Msaa::Off).lod_draws, 0);
    // the sheet spans about 7 px... at distance 20 it is ~100 px wide: a 40 px threshold keeps the
    // full mesh, a 1000 px threshold swaps in the proxy
    let keep = render_mesh(gpu, &cam(), &full, Settings { lod_px: 40.0, ..s });
    assert_eq!(gpu.last_stats(Msaa::Off).lod_draws, 0);
    assert_eq!(keep, render_mesh(gpu, &cam(), &plain, s));
    let swapped = render_mesh(
        gpu,
        &cam(),
        &full,
        Settings {
            lod_px: 1000.0,
            ..s
        },
    );
    assert_eq!(gpu.last_stats(Msaa::Off).lod_draws, 1);
    assert_ne!(swapped, keep, "the proxy looks different");
    assert!(
        nonbackground(&swapped, &background_only(gpu, s)) > 500,
        "and still shows the object"
    );
}

#[test]
fn instanced_lod_swaps_prototype_proxies_per_chunk() {
    let Some((_g, gpu)) = gpu() else { return };
    let protos = vec![sheet(16)];
    let mut c = clones(2, false, false);
    c.matrices
        .iter_mut()
        .for_each(|m| *m *= DMat4::from_scale(DVec3::splat(0.4)));
    let mut set = InstanceSet::new(&gpu.device, &protos, None, &c.input(), &DMat4::IDENTITY);
    set.set_proto_lod(&gpu.device, 0, &lod_proxy(&protos[0], 2), None);
    let s = settings(Shading::Smooth);
    let full = render_set(gpu, &cam(), &set, s);
    let swapped = render_set(
        gpu,
        &cam(),
        &set,
        Settings {
            lod_px: 10_000.0,
            ..s
        },
    );
    assert_ne!(full, swapped);
    assert_eq!(
        gpu.last_stats(Msaa::Off).lod_draws,
        0,
        "counted for meshes; instance ranges draw the proxy"
    );
    assert_eq!(gpu.last_stats(Msaa::Off).instances_drawn, 4);
}

// ---- baseline image --------------------------------------------------------------------------------------------

fn baseline_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/baselines")
        .join(format!("{name}.png"))
}

fn check_baseline(gpu: &Gpu, name: &str, rgba: &[u8]) {
    let path = baseline_path(name);
    if std::env::var("FORGE_UPDATE_BASELINES").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = std::fs::File::create(&path).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), W, H);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(rgba).unwrap();
        return;
    }
    let Ok(file) = std::fs::File::open(&path) else {
        panic!(
            "missing baseline {}: run FORGE_UPDATE_BASELINES=1 cargo test -p forge-viewport --test instancing",
            path.display()
        );
    };
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (W, H), "baseline size");
    buf.truncate(info.buffer_size());
    assert_same_image(gpu, name, &buf, rgba);
}
