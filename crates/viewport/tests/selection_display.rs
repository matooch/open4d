//! The selection display: outline, amber wire edges, instanced selection, the overlay for the
//! render engine's image, grid fade and grid occlusion (docs/specs/VIEWPORT.md §16). Needs a GPU
//! adapter (lavapipe in CI); skips without one unless FORGE_REQUIRE_GPU=1. One shared device
//! per test file behind a lock (implementer playbook §5a).

mod common;

use std::sync::{Mutex, MutexGuard, OnceLock};

use common::boxed;
use forge_geometry::selection::Selection;
use forge_viewport::camera::Camera;
use forge_viewport::grid::{LineKind, grid_lines};
use forge_viewport::instancing::InstanceInput;
use forge_viewport::picking::pick_poly;
use forge_viewport::render::{
    DrawInstanced, DrawMesh, Frame, Gpu, Highlight, InstanceSet, MeshBuffers, Msaa, Palette,
    SelectionDisplay, Settings, Shading, grid_fade_mix, grid_vertex_list, try_render_to_rgba_with,
};
use glam::{DMat4, DVec3};

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

struct Img(Vec<u8>);

impl Img {
    fn px(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * W + x) * 4) as usize;
        [self.0[i], self.0[i + 1], self.0[i + 2], self.0[i + 3]]
    }

    fn rgb(&self, x: u32, y: u32) -> [u8; 3] {
        let p = self.px(x, y);
        [p[0], p[1], p[2]]
    }
}

fn u8c(c: [f32; 3]) -> [u8; 3] {
    c.map(|v| (v * 255.0).round() as u8)
}

fn close(a: [u8; 3], b: [u8; 3], tol: i32) -> bool {
    (0..3).all(|i| (i32::from(a[i]) - i32::from(b[i])).abs() <= tol)
}

fn render(gpu: &Gpu, cam: &Camera, frame: &Frame, sel: &SelectionDisplay) -> Img {
    Img(try_render_to_rgba_with(gpu, W, H, cam, frame, sel)
        .unwrap_or_else(|e| panic!("offscreen readback failed: {e}")))
}

fn settings(shading: Shading) -> Settings {
    Settings {
        shading,
        ..Settings::default()
    }
}

fn cam() -> Camera {
    Camera {
        distance: 16.0,
        ..Camera::default()
    }
}

fn buffers(gpu: &Gpu, lo: DVec3, hi: DVec3) -> MeshBuffers {
    MeshBuffers::new(gpu, &boxed(lo, hi), &Selection::default())
}

fn marks(a: Highlight, b: Highlight) -> SelectionDisplay {
    SelectionDisplay {
        meshes: vec![a, b],
        ..SelectionDisplay::default()
    }
}

/// The pixels where two images differ.
fn diff(a: &Img, b: &Img) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for y in 0..H {
        for x in 0..W {
            if a.px(x, y) != b.px(x, y) {
                out.push((x, y));
            }
        }
    }
    out
}

/// Screen x of a world point (pixels).
fn screen(c: &Camera, p: DVec3) -> (f64, f64) {
    let s = c.project(p, f64::from(W), f64::from(H)).expect("in front");
    (s.x, s.y)
}

/// Two boxes side by side: the left one (index 0) is the one the tests mark.
struct Pair {
    left: MeshBuffers,
    right: MeshBuffers,
}

fn pair(gpu: &Gpu) -> Pair {
    Pair {
        left: buffers(
            gpu,
            DVec3::new(-5.0, -1.5, -1.5),
            DVec3::new(-2.0, 1.5, 1.5),
        ),
        right: buffers(gpu, DVec3::new(2.0, -1.5, -1.5), DVec3::new(5.0, 1.5, 1.5)),
    }
}

fn frame_of<'a>(p: &'a Pair, draws: &'a [DrawMesh<'a>], shading: Shading) -> Frame<'a> {
    let _ = p;
    Frame {
        meshes: draws,
        instanced: &[],
        grid: None,
        lines: &[],
        gizmo: None,
        settings: settings(shading),
    }
}

fn draws<'a>(p: &'a Pair) -> [DrawMesh<'a>; 2] {
    [
        DrawMesh {
            mesh: &p.left,
            color: GREY,
        },
        DrawMesh {
            mesh: &p.right,
            color: GREY,
        },
    ]
}

#[test]
fn the_selected_box_gets_an_amber_ring_and_the_other_does_not() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (p, c) = (pair(gpu), cam());
    let pal = Palette::default();
    let d = draws(&p);
    let frame = frame_of(&p, &d, Shading::Smooth);
    let plain = render(gpu, &c, &frame, &SelectionDisplay::default());
    let marked = render(
        gpu,
        &c,
        &frame,
        &marks(Highlight::Selected, Highlight::None),
    );
    let changed = diff(&plain, &marked);
    assert!(
        changed.len() > 40,
        "a ring should be drawn, {} pixels changed",
        changed.len()
    );
    let amber = u8c(pal.outline);
    let (lx, _) = screen(&c, DVec3::new(-3.5, 0.0, 0.0));
    let (rx, _) = screen(&c, DVec3::new(3.5, 0.0, 0.0));
    let mid = (lx + rx) / 2.0;
    for (x, y) in &changed {
        // every changed pixel is outline amber, was background before, and lies on the left
        assert!(
            close(marked.rgb(*x, *y), amber, 1),
            "({x},{y}) is {:?}, not amber {amber:?}",
            marked.rgb(*x, *y)
        );
        assert!(
            close(plain.rgb(*x, *y), u8c(pal.background), 1),
            "({x},{y}) was not background"
        );
        assert!(
            f64::from(*x) < mid,
            "({x},{y}) is on the unselected box's side"
        );
    }
    // the unselected box's surroundings are untouched: its ring pixels are background
    let (rx0, ry) = screen(&c, DVec3::new(5.0, 0.0, 1.5));
    for dx in [2, 3] {
        let (x, y) = (rx0 as u32 + dx, ry as u32);
        assert!(
            close(marked.rgb(x, y), u8c(pal.background), 1),
            "right box ring"
        );
    }
    // the box interior keeps its shading
    let (cx, cy) = screen(&c, DVec3::new(-3.5, 0.0, 1.5));
    assert_eq!(
        plain.px(cx as u32, cy as u32),
        marked.px(cx as u32, cy as u32)
    );
}

#[test]
fn the_active_object_has_a_brighter_and_wider_outline() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (p, c) = (pair(gpu), cam());
    let pal = Palette::default();
    let d = draws(&p);
    let frame = frame_of(&p, &d, Shading::Smooth);
    let plain = render(gpu, &c, &frame, &SelectionDisplay::default());
    let selected = render(
        gpu,
        &c,
        &frame,
        &marks(Highlight::Selected, Highlight::None),
    );
    let active = render(gpu, &c, &frame, &marks(Highlight::Active, Highlight::None));
    let (ds, da) = (diff(&plain, &selected), diff(&plain, &active));
    assert!(
        da.len() as f64 > ds.len() as f64 * 1.2,
        "{} vs {}",
        da.len(),
        ds.len()
    );
    let bright = u8c(pal.outline_active);
    for (x, y) in &da {
        assert!(close(active.rgb(*x, *y), bright, 1), "({x},{y})");
    }
    let lum = |c: [u8; 3]| u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2]);
    assert!(
        lum(bright) > lum(u8c(pal.outline)),
        "active amber is brighter"
    );
    // two selected boxes: one active, one not, in the same frame
    let both = render(
        gpu,
        &c,
        &frame,
        &marks(Highlight::Selected, Highlight::Active),
    );
    let (rx, _) = screen(&c, DVec3::new(3.5, 0.0, 0.0));
    let right: Vec<_> = diff(&plain, &both)
        .into_iter()
        .filter(|(x, _)| f64::from(*x) > rx)
        .collect();
    assert!(!right.is_empty());
    assert!(
        right
            .iter()
            .all(|(x, y)| close(both.rgb(*x, *y), bright, 1))
    );
}

#[test]
fn a_nearer_object_hides_the_outline_behind_it() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = cam();
    let back = buffers(
        gpu,
        DVec3::new(-3.0, -2.0, -6.0),
        DVec3::new(3.0, 2.0, -3.0),
    );
    let front = buffers(gpu, DVec3::new(0.5, -3.0, 0.0), DVec3::new(7.0, 3.0, 3.0));
    let d = [
        DrawMesh {
            mesh: &back,
            color: GREY,
        },
        DrawMesh {
            mesh: &front,
            color: GREY,
        },
    ];
    let frame = Frame {
        meshes: &d,
        instanced: &[],
        grid: None,
        lines: &[],
        gizmo: None,
        settings: settings(Shading::Smooth),
    };
    let plain = render(gpu, &c, &frame, &SelectionDisplay::default());
    let marked = render(
        gpu,
        &c,
        &frame,
        &marks(Highlight::Selected, Highlight::None),
    );
    let changed = diff(&plain, &marked);
    assert!(changed.len() > 40);
    // pixels of the front box (shrunk by 2 px) never change, and the outline still shows on
    // the part of the back box's silhouette the front box does not cover
    let (x0, _) = screen(&c, DVec3::new(0.5, 0.0, 3.0));
    let (_, top) = screen(&c, DVec3::new(0.5, 3.0, 3.0));
    let (_, bottom) = screen(&c, DVec3::new(0.5, -3.0, 3.0));
    for (x, y) in &changed {
        let inside =
            f64::from(*x) > x0 + 2.0 && f64::from(*y) > top + 2.0 && f64::from(*y) < bottom - 2.0;
        assert!(!inside, "({x},{y}) is on the nearer box");
    }
    let (lx, _) = screen(&c, DVec3::new(-3.0, 0.0, -3.0));
    assert!(
        changed.iter().any(|(x, _)| f64::from(*x) < lx + 1.0),
        "left of the back box"
    );
}

#[test]
fn wire_mode_draws_the_selected_objects_edges_in_amber() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (p, c) = (pair(gpu), cam());
    let pal = Palette::default();
    let d = draws(&p);
    let frame = frame_of(&p, &d, Shading::Wire);
    let plain = render(gpu, &c, &frame, &SelectionDisplay::default());
    let marked = render(
        gpu,
        &c,
        &frame,
        &marks(Highlight::Selected, Highlight::None),
    );
    let changed = diff(&plain, &marked);
    assert!(changed.len() > 20, "{}", changed.len());
    let amber = u8c(pal.outline);
    assert!(
        changed
            .iter()
            .all(|(x, y)| close(marked.rgb(*x, *y), amber, 1))
    );
    // an edge of the unselected box keeps the wire colour
    let (ex, ey) = screen(&c, DVec3::new(3.5, 1.5, 1.5));
    let wire = u8c(pal.wire);
    let found = (-2..=2).any(|dy| {
        (-2..=2).any(|dx| {
            close(
                marked.rgb((ex as i32 + dx) as u32, (ey as i32 + dy) as u32),
                wire,
                2,
            )
        })
    });
    assert!(found, "the unselected box keeps its wire colour");
}

#[test]
fn every_display_mode_marks_the_selected_object_in_the_outline_colour_only() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (p, c) = (pair(gpu), cam());
    let pal = Palette::default();
    let amber = u8c(pal.outline);
    for shading in Shading::ALL {
        let d = draws(&p);
        let frame = frame_of(&p, &d, shading);
        let plain = render(gpu, &c, &frame, &SelectionDisplay::default());
        let marked = render(
            gpu,
            &c,
            &frame,
            &marks(Highlight::Selected, Highlight::None),
        );
        let changed = diff(&plain, &marked);
        assert!(
            !changed.is_empty(),
            "{shading:?}: nothing marks the selection"
        );
        for (x, y) in &changed {
            assert!(
                close(marked.rgb(*x, *y), amber, 1),
                "{shading:?}: ({x},{y}) is {:?}",
                marked.rgb(*x, *y)
            );
        }
        let (rx, _) = screen(&c, DVec3::new(0.0, 0.0, 0.0));
        assert!(
            changed.iter().all(|(x, _)| f64::from(*x) < rx),
            "{shading:?}: the unselected box changed"
        );
    }
}

#[test]
fn msaa_keeps_the_outline() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (p, c) = (pair(gpu), cam());
    let pal = Palette::default();
    let d = draws(&p);
    let frame = Frame {
        settings: Settings {
            msaa: Msaa::X4,
            ..settings(Shading::Smooth)
        },
        ..frame_of(&p, &d, Shading::Smooth)
    };
    let plain = render(gpu, &c, &frame, &SelectionDisplay::default());
    let marked = render(
        gpu,
        &c,
        &frame,
        &marks(Highlight::Selected, Highlight::None),
    );
    let changed = diff(&plain, &marked);
    let amber = u8c(pal.outline);
    let solid = changed
        .iter()
        .filter(|(x, y)| close(marked.rgb(*x, *y), amber, 3))
        .count();
    assert!(
        solid > 30,
        "{solid} solid amber pixels of {}",
        changed.len()
    );
}

/// A 3 x 3 grid of unit boxes as one instance set, plus the matching prototype list.
fn clones(gpu: &Gpu) -> InstanceSet {
    let mut matrices = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            matrices.push(DMat4::from_translation(DVec3::new(
                -3.0 + i as f64 * 3.0,
                0.0,
                -3.0 + j as f64 * 3.0,
            )));
        }
    }
    let n = matrices.len();
    let input = InstanceInput {
        matrices: &matrices,
        colors: &vec![[1.0; 3]; n],
        visible: &vec![true; n],
        child: &vec![0; n],
    };
    InstanceSet::new(
        &gpu.device,
        &[boxed(DVec3::splat(-0.8), DVec3::splat(0.8))],
        None,
        &input,
        &DMat4::IDENTITY,
    )
}

#[test]
fn a_selected_cloner_is_outlined_around_its_clones_and_an_unselected_one_is_not() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = Camera {
        distance: 20.0,
        ..Camera::default()
    };
    let pal = Palette::default();
    let set = clones(gpu);
    let d = [DrawInstanced {
        set: &set,
        color: GREY,
    }];
    let frame = Frame {
        meshes: &[],
        instanced: &d,
        grid: None,
        lines: &[],
        gizmo: None,
        settings: settings(Shading::Smooth),
    };
    let none = render(gpu, &c, &frame, &SelectionDisplay::default());
    let unmarked = render(
        gpu,
        &c,
        &frame,
        &SelectionDisplay {
            instanced: vec![Highlight::None],
            ..SelectionDisplay::default()
        },
    );
    assert!(diff(&none, &unmarked).is_empty(), "no highlight, no change");
    let marked = render(
        gpu,
        &c,
        &frame,
        &SelectionDisplay {
            instanced: vec![Highlight::Selected],
            ..SelectionDisplay::default()
        },
    );
    let changed = diff(&none, &marked);
    assert!(
        changed.len() > 100,
        "{} ring pixels around nine clones",
        changed.len()
    );
    let amber = u8c(pal.outline);
    for (x, y) in &changed {
        // the ring of a nearer clone may fall on a farther one, so only the colour is fixed
        assert!(
            close(marked.rgb(*x, *y), amber, 1),
            "({x},{y}): {:?} over {:?}",
            marked.rgb(*x, *y),
            none.rgb(*x, *y)
        );
    }
    // each clone has its own ring: the middle of the gap between two clones is background
    // again, so the rings are per clone and not one blob around the set
    let (gx, gy) = screen(&c, DVec3::new(-1.5, 0.0, 0.0));
    assert!(close(
        marked.rgb(gx as u32, gy as u32),
        u8c(pal.background),
        1
    ));
    // and the rings of the clones in the middle row exist as well as the outer ones
    let (mx, my) = screen(&c, DVec3::new(0.0, 0.0, 0.0));
    let ring_near_middle = changed
        .iter()
        .any(|(x, y)| (f64::from(*x) - mx).abs() < 30.0 && (f64::from(*y) - my).abs() < 30.0);
    assert!(ring_near_middle);
}

#[test]
fn the_overlay_target_is_transparent_except_the_visible_ring() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (p, c) = (pair(gpu), cam());
    let pal = Palette::default();
    let d = draws(&p);
    let frame = frame_of(&p, &d, Shading::Smooth);
    let plain = render(gpu, &c, &frame, &SelectionDisplay::default());
    let shaded = render(
        gpu,
        &c,
        &frame,
        &marks(Highlight::Selected, Highlight::None),
    );
    let ring = diff(&plain, &shaded);
    let overlay = render(
        gpu,
        &c,
        &frame,
        &SelectionDisplay {
            overlay_only: true,
            ..marks(Highlight::Selected, Highlight::None)
        },
    );
    let amber = u8c(pal.outline);
    let mut opaque = Vec::new();
    for y in 0..H {
        for x in 0..W {
            let p = overlay.px(x, y);
            if p[3] != 0 {
                opaque.push((x, y));
                assert_eq!(p[3], 255, "({x},{y})");
                assert!(close([p[0], p[1], p[2]], amber, 1), "({x},{y}) {p:?}");
            } else {
                assert_eq!([p[0], p[1], p[2]], [0, 0, 0], "({x},{y}) is not clear");
            }
        }
    }
    assert_eq!(
        opaque, ring,
        "the overlay has exactly the shaded frame's ring"
    );
}

#[test]
fn the_overlay_is_hidden_by_a_nearer_unselected_mesh() {
    let Some((_lock, gpu)) = gpu() else { return };
    let c = cam();
    let back = buffers(
        gpu,
        DVec3::new(-3.0, -2.0, -6.0),
        DVec3::new(3.0, 2.0, -3.0),
    );
    let front = buffers(gpu, DVec3::new(0.5, -3.0, 0.0), DVec3::new(7.0, 3.0, 3.0));
    let d = [
        DrawMesh {
            mesh: &back,
            color: GREY,
        },
        DrawMesh {
            mesh: &front,
            color: GREY,
        },
    ];
    let frame = Frame {
        meshes: &d,
        instanced: &[],
        grid: None,
        lines: &[],
        gizmo: None,
        settings: settings(Shading::Smooth),
    };
    let overlay = render(
        gpu,
        &c,
        &frame,
        &SelectionDisplay {
            overlay_only: true,
            ..marks(Highlight::Selected, Highlight::None)
        },
    );
    let (x0, _) = screen(&c, DVec3::new(0.5, 0.0, 3.0));
    let (_, top) = screen(&c, DVec3::new(0.5, 3.0, 3.0));
    let (_, bottom) = screen(&c, DVec3::new(0.5, -3.0, 3.0));
    let mut visible = 0;
    for y in 0..H {
        for x in 0..W {
            if overlay.px(x, y)[3] == 0 {
                continue;
            }
            visible += 1;
            let inside =
                f64::from(x) > x0 + 2.0 && f64::from(y) > top + 2.0 && f64::from(y) < bottom - 2.0;
            assert!(!inside, "({x},{y}) shows through the nearer mesh");
        }
    }
    assert!(visible > 40);
}

#[test]
fn picking_ignores_the_outline() {
    // picking is CPU side: the ring around a box is not a hit, the box is, and neither
    // depends on whether the object is highlighted (VIEWPORT.md §16.2)
    let c = cam();
    let mesh = boxed(DVec3::new(-5.0, -1.5, -1.5), DVec3::new(-2.0, 1.5, 1.5));
    let (cx, cy) = screen(&c, DVec3::new(-3.5, 0.0, 0.0));
    let ray = |x: f64, y: f64| c.ray(x, y, f64::from(W), f64::from(H));
    assert!(pick_poly(&mesh, &ray(cx, cy)).is_some());
    let (edge, _) = screen(&c, DVec3::new(-5.0, 0.0, 1.5));
    assert!(pick_poly(&mesh, &ray(edge - 2.5, cy)).is_none());
}

#[test]
fn minor_grid_lines_fade_with_the_distance_from_the_centre() {
    let c = Camera {
        distance: 20.0,
        pitch: 0.6,
        ..Camera::default()
    };
    let grid = grid_lines(&c, f64::from(H));
    let pal = Palette {
        grid_fade: 1.0,
        ..Palette::default()
    };
    // off: the two endpoints of each line, as before
    let off = grid_vertex_list(&grid, &Palette::default());
    assert_eq!(off.len(), grid.lines.len() * 2);
    assert!(off.iter().all(|(_, col)| *col != pal.background));
    let on = grid_vertex_list(&grid, &pal);
    assert!(on.len() > off.len(), "minor lines are cut into segments");
    // the strength is the smoothstep of the distance, 0 at the centre and `fade` at the edge
    assert_eq!(grid_fade_mix(0.0, 10.0, 1.0), 0.0);
    assert_eq!(grid_fade_mix(3.5, 10.0, 1.0), 0.0);
    assert!((grid_fade_mix(10.0, 10.0, 1.0) - 1.0).abs() < 1e-6);
    assert!((grid_fade_mix(10.0, 10.0, 0.5) - 0.5).abs() < 1e-6);
    assert!(grid_fade_mix(6.0, 10.0, 1.0) > grid_fade_mix(5.0, 10.0, 1.0));
    // colour distance from the background shrinks monotonically along a minor line from the
    // middle to the end, and the end is the background itself
    let dist = |col: [f32; 3]| {
        (0..3)
            .map(|i| (col[i] - pal.background[i]).abs())
            .sum::<f32>()
    };
    // lines before the first minor one are majors (two vertices each), then 16 segments
    let minor = grid
        .lines
        .iter()
        .position(|l| l.kind == LineKind::Minor)
        .expect("a minor line");
    assert!(
        grid.lines[..minor]
            .iter()
            .all(|l| l.kind != LineKind::Minor)
    );
    let first: Vec<_> = on[2 * minor..2 * minor + 32].iter().collect();
    let end = first.last().unwrap().1;
    assert!(
        dist(end) < dist(pal.grid_minor) * 0.2,
        "the line ends faded"
    );
    let mid = first[15].1;
    assert!(dist(mid) >= dist(end));
    // major lines and the axes do not fade
    let majors = on.iter().filter(|(_, col)| *col == pal.grid_major).count();
    assert_eq!(
        majors,
        grid.lines
            .iter()
            .filter(|l| l.kind == LineKind::Major)
            .count()
            * 2
    );
}

#[test]
fn a_box_hides_the_grid_behind_and_under_it_and_the_grid_shows_beside_it() {
    let Some((_lock, gpu)) = gpu() else { return };
    let pal = Palette::default();
    let c = Camera {
        distance: 14.0,
        pitch: 0.7,
        ..Camera::default()
    };
    let grid = grid_lines(&c, f64::from(H));
    let boxed_mesh = buffers(gpu, DVec3::new(-2.0, 0.0, -2.0), DVec3::new(2.0, 3.0, 2.0));
    let with_box = [DrawMesh {
        mesh: &boxed_mesh,
        color: GREY,
    }];
    let scene = |draws: &[DrawMesh]| {
        render(
            gpu,
            &c,
            &Frame {
                meshes: draws,
                instanced: &[],
                grid: Some(&grid),
                lines: &[],
                gizmo: None,
                settings: settings(Shading::Constant),
            },
            &SelectionDisplay::default(),
        )
    };
    let bare = scene(&[]);
    let blocked = scene(&with_box);
    // a point of the X axis line under the box: the axis colour without the box, the box with it
    let (ux, uy) = screen(&c, DVec3::new(1.0, 0.0, 0.0));
    let hit = |img: &Img, col: [f32; 3]| {
        (-1..=1).any(|dy: i32| {
            (-1..=1).any(|dx: i32| {
                close(
                    img.rgb((ux as i32 + dx) as u32, (uy as i32 + dy) as u32),
                    u8c(col),
                    1,
                )
            })
        })
    };
    assert!(
        hit(&bare, pal.axis_x),
        "the axis line is drawn without the box"
    );
    assert!(
        !hit(&blocked, pal.axis_x),
        "the box hides the axis line under it"
    );
    // the line behind the box (further along -z) is hidden as well: the box top covers it
    let (bx, by) = screen(&c, DVec3::new(0.0, 0.0, -1.0));
    assert_eq!(
        blocked.rgb(bx as u32, by as u32),
        blocked.rgb(ux as u32, uy as u32 - 1),
        "behind the box is the box"
    );
    // beside the box the grid is still drawn (a major or minor line, not the background)
    let (sx, sy) = screen(&c, DVec3::new(6.0, 0.0, 0.0));
    let beside = (-1..=1).any(|dy: i32| {
        (-1..=1).any(|dx: i32| {
            close(
                blocked.rgb((sx as i32 + dx) as u32, (sy as i32 + dy) as u32),
                u8c(pal.axis_x),
                1,
            )
        })
    });
    assert!(beside, "the grid shows beside the box");
}

#[test]
fn an_open_surface_is_outlined_too_and_its_interior_is_untouched() {
    let Some((_lock, gpu)) = gpu() else { return };
    let (c, pal) = (cam(), Palette::default());
    let plane = forge_geometry::PolyMesh {
        points: vec![
            DVec3::new(-3.0, -2.0, 0.0),
            DVec3::new(3.0, -2.0, 0.0),
            DVec3::new(3.0, 2.0, 0.0),
            DVec3::new(-3.0, 2.0, 0.0),
        ],
        polys: vec![vec![0, 1, 2, 3]],
    };
    let buf = MeshBuffers::new(gpu, &plane, &Selection::default());
    for flip in [false, true] {
        // seen from the front and from the back (both faces are drawn for an open surface)
        let cam = if flip {
            Camera {
                yaw: std::f64::consts::PI,
                ..c
            }
        } else {
            c
        };
        let d = [DrawMesh {
            mesh: &buf,
            color: GREY,
        }];
        let frame = Frame {
            meshes: &d,
            instanced: &[],
            grid: None,
            lines: &[],
            gizmo: None,
            settings: settings(Shading::Smooth),
        };
        let plain = render(gpu, &cam, &frame, &SelectionDisplay::default());
        let marked = render(
            gpu,
            &cam,
            &frame,
            &marks(Highlight::Selected, Highlight::None),
        );
        let changed = diff(&plain, &marked);
        assert!(
            changed.len() > 40,
            "flip {flip}: {} ring pixels",
            changed.len()
        );
        for (x, y) in &changed {
            assert!(
                close(marked.rgb(*x, *y), u8c(pal.outline), 1),
                "flip {flip} ({x},{y})"
            );
            assert!(
                close(plain.rgb(*x, *y), u8c(pal.background), 1),
                "flip {flip} ({x},{y})"
            );
        }
    }
}
