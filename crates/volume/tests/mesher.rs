//! The volume mesher (docs/specs/VOLUME.md section 7).

mod common;

use std::collections::BTreeMap;

use common::{cube, sphere};
use forge_geometry::PolyMesh;
use forge_volume::{
    MeshOptions, Polarity, SdfParams, Volume, VolumeKind, cell_mesh, gradient_normals,
    merge_blocks, mesh_to_sdf, mesh_volume, volume_to_mesh,
};
use glam::{DMat4, DVec3};

fn volume(mesh: &PolyMesh, voxel: f64) -> Volume {
    Volume {
        grid: mesh_to_sdf(
            mesh,
            &SdfParams {
                voxel_size: voxel,
                ..SdfParams::default()
            },
        ),
        kind: VolumeKind::Sdf,
        to_parent: DMat4::IDENTITY,
    }
}

fn opts(iso: f64, adaptive: f64) -> MeshOptions {
    MeshOptions {
        iso,
        absolute: false,
        adaptive,
    }
}

fn signed_volume(m: &PolyMesh) -> f64 {
    let mut v = 0.0;
    for f in &m.polys {
        for i in 1..f.len() - 1 {
            v += m.points[f[0] as usize]
                .dot(m.points[f[i] as usize].cross(m.points[f[i + 1] as usize]))
                / 6.0;
        }
    }
    v
}

/// Every undirected edge has exactly two faces, running in opposite directions.
fn is_closed(m: &PolyMesh) -> bool {
    let mut dir: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for f in &m.polys {
        for i in 0..f.len() {
            *dir.entry((f[i], f[(i + 1) % f.len()])).or_default() += 1;
        }
    }
    dir.iter()
        .all(|(&(a, b), &n)| n == 1 && dir.get(&(b, a)) == Some(&1))
}

#[test]
fn the_surface_is_closed_and_wound_outward() {
    let v = volume(&sphere(100.0, 24, 32, DVec3::ZERO), 10.0);
    let m = mesh_volume(&v, &opts(0.01, 0.0));
    assert!(is_closed(&m));
    let vol = signed_volume(&m);
    assert!(
        vol > 0.9 * (4.0 / 3.0) * std::f64::consts::PI * 1e6,
        "{vol}"
    );
    assert!(m.polys.iter().all(|f| f.len() == 4));
    // a volume with a hole inside (a sphere minus a smaller one) has an inward inner surface
    let outer = sphere(100.0, 24, 32, DVec3::ZERO);
    let o = volume(&outer, 10.0);
    let inner = mesh_to_sdf(&sphere(40.0, 24, 32, DVec3::ZERO), &SdfParams::default());
    let shell = forge_volume::combine(&o.grid, &inner, forge_volume::CsgOp::Subtract).unwrap();
    let sv = Volume {
        grid: shell,
        kind: VolumeKind::Sdf,
        to_parent: DMat4::IDENTITY,
    };
    let m = mesh_volume(&sv, &opts(0.01, 0.0));
    assert!(is_closed(&m));
    let shell_volume = signed_volume(&m);
    assert!(
        shell_volume < vol && shell_volume > 0.8 * vol,
        "{shell_volume} {vol}"
    );
}

#[test]
fn a_cube_keeps_its_corner_samples_on_the_faces() {
    let v = volume(&cube(DVec3::splat(100.0), DVec3::ZERO), 10.0);
    let m = mesh_volume(&v, &opts(0.01, 0.0));
    assert_eq!((m.points.len(), m.polys.len()), (2168, 2166));
    let hi = m.points.iter().fold(DVec3::MIN, |a, p| a.max(*p));
    assert_eq!(hi, DVec3::splat(100.0));
    assert!(
        m.points
            .iter()
            .any(|p| (p.x + 93.33333333333333).abs() < 1e-9 && p.x == p.y && p.y == p.z)
    );
}

#[test]
fn threshold_dead_zone_and_shift() {
    let v = volume(&sphere(100.0, 24, 32, DVec3::ZERO), 10.0);
    let base = mesh_volume(&v, &opts(0.01, 0.0));
    // up to 0.05 voxel the surface does not move (O32 mesher-threshold)
    assert_eq!(base, mesh_volume(&v, &opts(0.5, 0.0)));
    assert_eq!(base, mesh_volume(&v, &opts(-0.5, 0.0)));
    // beyond that the iso value moves the surface: positive grows, negative shrinks
    let grown = mesh_volume(&v, &opts(6.0, 0.0));
    let shrunk = mesh_volume(&v, &opts(-6.0, 0.0));
    assert!(signed_volume(&grown) > signed_volume(&base));
    assert!(signed_volume(&shrunk) < signed_volume(&base));
}

#[test]
fn adaptivity_reduces_the_mesh_and_keeps_it_closed() {
    let v = volume(&sphere(100.0, 24, 32, DVec3::ZERO), 10.0);
    let counts: Vec<usize> = [0.0, 0.1, 0.3, 0.5, 1.0]
        .iter()
        .map(|a| mesh_volume(&v, &opts(0.01, *a)).polys.len())
        .collect();
    assert!(counts.windows(2).all(|w| w[0] >= w[1]), "{counts:?}");
    assert!(counts[3] < counts[0] / 10, "{counts:?}");
    let m = mesh_volume(&v, &opts(0.01, 0.5));
    assert!(m.polys.iter().any(|f| f.len() == 3));
    assert!(m.polys.iter().all(|f| (3..=4).contains(&f.len())));
    assert!(m.points.iter().all(|p| p.length() < 100.0));
}

#[test]
fn adaptivity_merges_flat_faces_but_not_edges() {
    let v = volume(&cube(DVec3::splat(100.0), DVec3::ZERO), 10.0);
    let base = mesh_volume(&v, &opts(0.01, 0.0)).polys.len();
    let m = mesh_volume(&v, &opts(0.2, 0.0));
    assert_eq!(m.polys.len(), base);
    // tolerance 0.4 voxel: face interiors merge, the bevelled edge strips stay
    let coarse = mesh_volume(&v, &opts(0.01, 0.2));
    assert!(
        coarse.polys.len() < base / 3,
        "{} of {base}",
        coarse.polys.len()
    );
    let extent = coarse.points.iter().fold(DVec3::MIN, |a, p| a.max(p.abs()));
    assert!(extent.x > 99.0, "the faces stay on the cube: {extent:?}");
    // tolerance 0 still merges exactly planar blocks, and nothing else
    let cm = cell_mesh(&v.grid, 0.0, Polarity::LowInside);
    let flat = merge_blocks(&cm, 0.0);
    assert!(flat.polys.len() < cm.mesh.polys.len());
    assert!(flat.polys.len() > coarse.polys.len());
}

#[test]
fn gradient_normals_point_outward() {
    let v = volume(&sphere(100.0, 48, 64, DVec3::ZERO), 10.0);
    let m = mesh_volume(&v, &opts(0.01, 0.0));
    let n = gradient_normals(&v, &m.points);
    let worst = m
        .points
        .iter()
        .zip(&n)
        .map(|(p, n)| n.dot(p.normalize()))
        .fold(1.0, f64::min);
    assert!(worst > 0.95, "{worst}");
    // fog: the density grows inward, so the normals still point out
    let fog = Volume {
        grid: forge_volume::sdf_to_fog(&v.grid, 20.0),
        kind: VolumeKind::Fog,
        to_parent: DMat4::IDENTITY,
    };
    let fm = mesh_volume(&fog, &opts(0.01, 0.0));
    let fnormals = gradient_normals(&fog, &fm.points);
    let worst = fm
        .points
        .iter()
        .zip(&fnormals)
        .map(|(p, n)| n.dot(p.normalize()))
        .fold(1.0, f64::min);
    assert!(worst > 0.9, "{worst}");
}

#[test]
fn absolute_value_meshes_a_shell() {
    let v = volume(&sphere(100.0, 24, 32, DVec3::ZERO), 10.0);
    let shell = mesh_volume(
        &v,
        &MeshOptions {
            iso: 8.0,
            absolute: true,
            adaptive: 0.0,
        },
    );
    // two surfaces: outside at about +8 and inside at about -8
    let radii: Vec<f64> = shell.points.iter().map(|p| p.length()).collect();
    assert!(radii.iter().any(|r| *r > 104.0));
    assert!(radii.iter().any(|r| *r < 94.0));
    assert!(is_closed(&shell));
    let plain = volume_to_mesh(&v.grid, 0.0, Polarity::LowInside);
    assert!(shell.polys.len() > plain.polys.len());
}
