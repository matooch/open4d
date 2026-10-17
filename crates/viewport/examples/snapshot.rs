//! Renders a sample scene offscreen and writes a PNG.
//! `cargo run -p forge-viewport --example snapshot -- out.png`

use std::fs::File;
use std::io::BufWriter;

use forge_geometry::PolyMesh;
use forge_geometry::selection::Selection;
use forge_viewport::camera::Camera;
use forge_viewport::gizmo::Gizmo;
use forge_viewport::grid::grid_lines;
use forge_viewport::render::{
    DrawMesh, Frame, Gpu, MeshBuffers, Settings, Shading, render_to_rgba,
};
use glam::DVec3;

/// Box from `lo` to `hi` with outward counter-clockwise quads.
fn boxed(lo: DVec3, hi: DVec3) -> PolyMesh {
    let p = |i: usize| {
        DVec3::new(
            if i & 1 == 1 { hi.x } else { lo.x },
            if i & 2 == 2 { hi.y } else { lo.y },
            if i & 4 == 4 { hi.z } else { lo.z },
        )
    };
    PolyMesh {
        points: (0..8).map(p).collect(),
        polys: vec![
            vec![0, 1, 5, 4],
            vec![2, 6, 7, 3],
            vec![0, 2, 3, 1],
            vec![4, 5, 7, 6],
            vec![0, 4, 6, 2],
            vec![1, 3, 7, 5],
        ],
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "snapshot.png".into());
    let gpu = Gpu::new_headless().expect("a GPU adapter");
    let (w, h) = (960, 540);

    let mut cam = Camera::default();
    cam.orbit(0.6, 0.45);
    cam.frame(DVec3::new(-2.0, 0.0, -2.0), DVec3::new(2.5, 2.0, 2.0));

    let a = boxed(DVec3::new(-1.5, 0.0, -1.0), DVec3::new(0.0, 1.5, 0.5));
    let b = boxed(DVec3::new(0.5, 0.0, -0.5), DVec3::new(2.0, 0.75, 1.0));
    let mut sel = Selection::default();
    sel.polys.insert(1); // top face of the second box
    let (ma, mb) = (
        MeshBuffers::new(&gpu, &a, &Selection::default()),
        MeshBuffers::new(&gpu, &b, &sel),
    );
    let meshes = [
        DrawMesh {
            mesh: &ma,
            color: [0.70, 0.70, 0.70],
        },
        DrawMesh {
            mesh: &mb,
            color: [0.45, 0.60, 0.80],
        },
    ];
    let grid = grid_lines(&cam, h as f64);
    let gizmo = Gizmo {
        origin: DVec3::new(1.25, 0.75, 0.25),
        ..Gizmo::default()
    };
    let frame = Frame {
        meshes: &meshes,
        grid: Some(&grid),
        gizmo: Some(&gizmo),
        settings: Settings {
            shading: Shading::SmoothWire,
            ..Settings::default()
        },
    };
    let rgba = render_to_rgba(&gpu, w, h, &cam, &frame);

    let mut enc = png::Encoder::new(
        BufWriter::new(File::create(&path).expect("create png")),
        w,
        h,
    );
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(&rgba)
        .expect("png data");
    println!("wrote {path}");
}
