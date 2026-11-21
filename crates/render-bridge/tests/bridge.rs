//! Scene adapter: normals, framing (docs/specs/RENDER.md R12).

use forge_geometry::PolyMesh;
use forge_render_bridge::*;
use glam::DVec3;

fn cube(h: f64) -> PolyMesh {
    let p = [
        [-h, -h, -h],
        [h, -h, -h],
        [h, h, -h],
        [-h, h, -h],
        [-h, -h, h],
        [h, -h, h],
        [h, h, h],
        [-h, h, h],
    ];
    PolyMesh {
        points: p.iter().map(|v| DVec3::from(*v)).collect(),
        polys: vec![
            vec![0, 3, 2, 1],
            vec![4, 5, 6, 7],
            vec![0, 4, 7, 3],
            vec![1, 2, 6, 5],
            vec![3, 7, 6, 2],
            vec![0, 1, 5, 4],
        ],
    }
}

#[test]
fn cube_corners_keep_their_face_normals() {
    let m = mesh_from_poly(&cube(1.0));
    assert_eq!(m.triangles.len(), 12);
    assert_eq!(m.normals.len(), 36);
    // The 90 degree edges are above the crease angle: each corner normal is an axis.
    for n in &m.normals {
        let v = DVec3::from(*n);
        assert!((v.length() - 1.0).abs() < 1e-9);
        assert!((v.abs().max_element() - 1.0).abs() < 1e-9, "{n:?}");
    }
}

#[test]
fn camera_frames_the_box_and_looks_at_it() {
    let (lo, hi) = (DVec3::splat(-100.0), DVec3::splat(100.0));
    let cam = frame_camera(lo, hi, DVec3::new(0.45, 0.4, -1.0), 4.0 / 3.0);
    let (o, d) = cam.ray(160.0, 120.0, 320, 240);
    // The centre ray passes through the box centre.
    let t = (-o).dot(d);
    assert!((o + d * t).length() < 1e-6, "{:?}", o + d * t);
    // All eight corners project inside the image.
    for x in [lo.x, hi.x] {
        for y in [lo.y, hi.y] {
            for z in [lo.z, hi.z] {
                let m = cam.matrix().inverse();
                let c = m.transform_point3(DVec3::new(x, y, z));
                assert!(c.z > 0.0);
                let (f, pp) = cam.pixel_frame(320, 240).unwrap();
                let (px, py) = (pp[0] + f * c.x / c.z, pp[1] - f * c.y / c.z);
                assert!(
                    (0.0..320.0).contains(&px) && (0.0..240.0).contains(&py),
                    "{px} {py}"
                );
            }
        }
    }
}

#[test]
fn snapshot_skips_empty_meshes_and_validates() {
    let s = snapshot_from_polys(&[cube(50.0), PolyMesh::default()], 4.0 / 3.0);
    assert_eq!(s.instances.len(), 1);
    assert!(s.validate().is_ok());
    let empty = snapshot_from_polys(&[], 1.0);
    assert!(empty.instances.is_empty() && empty.validate().is_ok());
}
