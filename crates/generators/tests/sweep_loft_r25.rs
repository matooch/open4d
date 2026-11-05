//! docs/specs/GENERATORS2.md §9a / §10a: the R25 rules of brief 042 that the golden cases
//! cannot show through our spline primitives (the uniform helix deviates from the reference
//! by up to 3.5e-4, `helix-uniform`, which the sweep frames amplify). The paths here are the
//! ring centres of the reference meshes themselves, so only the rule under test is measured.

use std::collections::BTreeMap;
use std::path::Path;

use forge_core::Value;
use forge_generators::spline_sampling::{Interpolation, Sampling};
use forge_generators::splines::{Curve, build_curve};
use forge_generators::{loft, sweep};
use forge_parity_harness::{Mesh, Tolerance, compare_meshes, export_weld, parse_obj};
use glam::DVec3;

fn golden(case: &str) -> Mesh {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tests/golden/{case}/mesh.obj"));
    parse_obj(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn vec3(p: [f64; 3]) -> DVec3 {
    DVec3::from_array(p)
}

/// The 36-point rings of the 901-ring sweep cases: ring `k` is `positions[36k..36k+36]`.
const RING: usize = 36;
const RINGS: usize = 901;

fn ring(mesh: &Mesh, k: usize) -> &[[f64; 3]] {
    &mesh.positions[k * RING..(k + 1) * RING]
}

fn centre(mesh: &Mesh, k: usize) -> DVec3 {
    ring(mesh, k).iter().map(|p| vec3(*p)).sum::<DVec3>() / RING as f64
}

fn circle() -> Curve {
    build_curve("forge.circle_spline", &BTreeMap::new()).unwrap()
}

fn mesh_of(m: &forge_geometry::PolyMesh) -> Mesh {
    export_weld(&Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys.clone(),
    })
}

/// `sweep/banking`: with Banking off the profile's x axis is `Y × tangent` (bisector of the
/// unit chords). Swept along the reference's own ring centres every vertex and face matches.
#[test]
fn banking_off_follows_world_up() {
    let expected = golden("sweep/banking");
    let centres: Vec<DVec3> = (0..RINGS).map(|k| centre(&expected, k)).collect();
    let path = Curve::polyline(&centres, false, Sampling::new(Interpolation::None));
    let values = BTreeMap::from([("banking".to_string(), Value::Bool(false))]);
    let ours = sweep(&circle(), &path, None, &values).unwrap();
    let r = compare_meshes(
        &export_weld(&expected),
        &mesh_of(&ours),
        &Tolerance::default(),
    );
    assert!(
        r.passed(),
        "{:?} (max error {:?})",
        r.failures,
        r.max_position_error
    );
}

/// `sweep/rail`: ring `k` lies on the segment from path point `k` (profile x minimum) to rail
/// point `k` (maximum), the profile's y dropped. Path and rail are the reference's own ring
/// ends, so the mapping, the collapsed first ring and the N-gon caps are what is compared.
#[test]
fn two_rail_maps_the_profile_width_onto_the_rails() {
    let expected = golden("sweep/rail");
    let (path, rail): (Vec<DVec3>, Vec<DVec3>) = (0..RINGS)
        .map(|k| (vec3(ring(&expected, k)[18]), vec3(ring(&expected, k)[0])))
        .unzip();
    let none = || Sampling::new(Interpolation::None);
    let path = Curve::polyline(&path, false, none());
    let rail = Curve::polyline(&rail, false, none());
    let ours = sweep(&circle(), &path, Some(&rail), &BTreeMap::new()).unwrap();
    let r = compare_meshes(
        &export_weld(&expected),
        &mesh_of(&ours),
        &Tolerance::default(),
    );
    assert!(
        r.passed(),
        "{:?} (max error {:?})",
        r.failures,
        r.max_position_error
    );
}

/// `sweep/scale-end` with Banking on is not matched even along the exact route: the reference
/// frame twists 55.3 degrees over the helix, parallel transport 57.0 (GENERATORS2.md §10a).
#[test]
fn banking_on_frame_is_still_a_known_mismatch() {
    let expected = golden("sweep/scale-end");
    let centres: Vec<DVec3> = (0..RINGS).map(|k| centre(&expected, k)).collect();
    let path = Curve::polyline(&centres, false, Sampling::new(Interpolation::None));
    let values = BTreeMap::from([
        ("end_scale".to_string(), Value::Float(20.0)),
        (
            "end_rotation".to_string(),
            Value::Float(180f64.to_radians()),
        ),
    ]);
    let ours = sweep(&circle(), &path, None, &values).unwrap();
    let r = compare_meshes(
        &export_weld(&expected),
        &mesh_of(&ours),
        &Tolerance::default(),
    );
    assert!(
        !r.passed(),
        "sweep/scale-end matches along the exact route: tighten §10a"
    );
}

fn canonical(f: &[u32]) -> Vec<u32> {
    let mut best: Option<Vec<u32>> = None;
    for g in [f.to_vec(), f.iter().rev().copied().collect::<Vec<_>>()] {
        for r in 0..g.len() {
            let c: Vec<u32> = g[r..].iter().chain(&g[..r]).copied().collect();
            if best.as_ref().is_none_or(|b| c < *b) {
                best = Some(c);
            }
        }
    }
    best.unwrap()
}

/// `loft/three-circles`, `loft/linear`: the reference's R25 loft has `U` points per ring
/// where the R20 loft has `U - 1` and lists each quad `[r(j,i), r(j+1,i), r(j+1,i+1),
/// r(j,i+1)]` (ours: the R20 order, reversed and rotated). With `U + 1` asked of our loft the
/// points match in order and the faces match up to rotation and reversal. Documented
/// R20/R25 conflict (§9a); the production default stays R20, the cases stay parity-pending.
#[test]
fn r25_loft_is_ours_with_one_more_sample_and_the_other_quad_order() {
    for (case, linear) in [("loft/three-circles", false), ("loft/linear", true)] {
        let expected = golden(case);
        let sections: Vec<Curve> = [(0.0, 200.0), (200.0, 100.0), (400.0, 200.0)]
            .iter()
            .map(|(y, r)| {
                let values = BTreeMap::from([
                    ("plane".to_string(), Value::Enum("xz".into())),
                    ("radius".to_string(), Value::Float(*r)),
                ]);
                build_curve("forge.circle_spline", &values)
                    .unwrap()
                    .transformed(&glam::DMat4::from_translation(DVec3::new(0.0, *y, 0.0)))
            })
            .collect();
        let values = BTreeMap::from([
            ("mesh_subdivision_u".to_string(), Value::Int(17)),
            ("mesh_subdivision_v".to_string(), Value::Int(4)),
            ("linear_interpolation".to_string(), Value::Bool(linear)),
        ]);
        let ours = loft(&sections, &values).unwrap();
        assert_eq!(ours.points.len(), expected.positions.len(), "{case}");
        for (i, (a, b)) in ours.points.iter().zip(&expected.positions).enumerate() {
            let err = (*a - vec3(*b)).abs().max_element();
            assert!(err < 1e-4, "{case}: point {i} differs by {err}");
        }
        let mut want: Vec<Vec<u32>> = expected.faces.iter().map(|f| canonical(f)).collect();
        let mut got: Vec<Vec<u32>> = ours.polys.iter().map(|f| canonical(f)).collect();
        want.sort();
        got.sort();
        assert_eq!(got, want, "{case}: face set");
    }
}

/// Lathe Movement lifts the profile by `movement · k / subdivision` and Scaling scales it
/// about the axis towards `scaling` per cent at the last column (GENERATORS2.md §8; the
/// movement is `lathe/movement`, the scaling has no case and is **parity-pending**).
#[test]
fn lathe_moves_and_scales_along_the_turn() {
    let profile = build_curve("forge.rectangle_spline", &BTreeMap::new())
        .unwrap()
        .transformed(&glam::DMat4::from_translation(DVec3::new(300.0, 0.0, 0.0)));
    let values = BTreeMap::from([
        ("subdivision".to_string(), Value::Int(4)),
        ("movement".to_string(), Value::Float(100.0)),
        ("scaling".to_string(), Value::Float(50.0)),
    ]);
    let m = forge_generators::lathe(&profile, &values).unwrap();
    // five columns of four points: the turn no longer closes on itself
    assert_eq!(m.points.len(), 20);
    let (first, last) = (m.points[0], m.points[16]);
    assert!((last.y - first.y - 100.0).abs() < 1e-9);
    let radius = |p: DVec3| p.x.hypot(p.z);
    assert!((radius(last) - radius(first) * 0.5).abs() < 1e-9);
}
