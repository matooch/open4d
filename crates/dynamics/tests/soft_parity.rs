//! Soft bodies against O41 `soft-sphere-*` (docs/specs/DYNAMICS.md §8, §7).

mod common;

use common::*;
use forge_core::{Document, EvalCtx, NodeId, Value};
use forge_dynamics::{BodyKey, DynamicsCache, RIGID_BODY};
use forge_generators::evaluate;
use forge_parity_harness::parse_obj;
use glam::DVec3;

const POINTS: usize = 114; // a 16-segment sphere: 7 rings of 16 plus two poles
const PLANE_POINTS: usize = 121;

struct World {
    doc: Document,
    sphere: NodeId,
}

fn world(structural: f64, pressure: f64) -> World {
    let mut doc = Document::new(registry());
    let sphere = doc
        .transact("soft sphere", |tx| {
            add_object(tx, &Obj::plane());
            let s = tx.create("forge.sphere", None, None)?;
            tx.set_param(s, "radius", Value::Float(50.0))?;
            tx.set_param(s, "transform.pos", Value::Vec3([0.0, 200.0, 0.0]))?;
            let t = tx.attach_tag(RIGID_BODY, s, None)?;
            tx.set_param(t, "soft_body", Value::Enum("polygons".into()))?;
            tx.set_param(t, "structural", Value::Float(structural))?;
            tx.set_param(t, "pressure", Value::Float(pressure))?;
            Ok(s)
        })
        .unwrap();
    World { doc, sphere }
}

fn frames(w: &World, upto: i64) -> Vec<Vec<DVec3>> {
    let mesh = |id: NodeId| evaluate(w.doc.scene(), id, &EvalCtx::default()).unwrap();
    let mut c = DynamicsCache::build(w.doc.scene(), &mesh).unwrap();
    (0..=upto)
        .map(|f| c.soft_at(f)[&BodyKey::Node(w.sphere)].clone())
        .collect()
}

fn bounds(p: &[DVec3]) -> (DVec3, DVec3) {
    let lo = p.iter().fold(DVec3::splat(f64::MAX), |a, b| a.min(*b));
    let hi = p.iter().fold(DVec3::splat(f64::MIN), |a, b| a.max(*b));
    (lo, hi)
}

fn centre_y(p: &[DVec3]) -> f64 {
    p.iter().map(|q| q.y).sum::<f64>() / p.len() as f64
}

/// The reference sphere's points in the exported mesh of `file` (the plane comes last).
fn golden_sphere(case: &str, file: &str) -> Vec<DVec3> {
    let src = std::fs::read_to_string(golden("soft-bodies", case).join(file)).unwrap();
    let mesh = parse_obj(&src).unwrap();
    assert_eq!(mesh.positions.len(), POINTS + PLANE_POINTS);
    mesh.positions[..POINTS]
        .iter()
        .map(|p| DVec3::from_array(*p))
        .collect()
}

fn structural_of(case: &str) -> f64 {
    let t = case_table("soft-bodies", case);
    let body = t["params"].as_table().unwrap()["body"]
        .as_table()
        .unwrap()
        .clone();
    body["structural"].as_float().unwrap() / 100.0
}

/// Rest-shape check at one frame: the extents within 10 % of the undeformed 100 cm.
fn near_rest_shape(p: &[DVec3]) -> bool {
    let (lo, hi) = bounds(p);
    let e = hi - lo;
    [e.x, e.y, e.z].iter().all(|v| (v - 100.0).abs() <= 10.0)
}

#[test]
fn soft_sphere_drop() {
    let case = "soft-sphere-drop";
    let w = world(structural_of(case), 0.0);
    let f = frames(&w, 60);
    // Frame 15, the fall: the centre within 5 cm of the reference's 74.44 (the fall is 125.6 cm;
    // semi-implicit Euler at 2 steps per frame over-falls by 1/(2n), 2 cm here), undeformed.
    let g15 = golden_sphere(case, "mesh.obj");
    assert!(
        (centre_y(&f[15]) - centre_y(&g15)).abs() < 5.0,
        "{}",
        centre_y(&f[15])
    );
    let (lo, hi) = bounds(&f[15]);
    assert!(((hi - lo) - DVec3::splat(100.0)).abs().max_element() < 0.5);
    // The rest shape at frames 15, 30, 60: every extent within 10 % of 100 cm, as the
    // reference's are (99.3 to 104.4).
    for (n, file) in [(15, "mesh.obj"), (30, "mesh-f30.obj"), (60, "mesh-f60.obj")] {
        assert!(
            near_rest_shape(&golden_sphere(case, file)),
            "reference frame {n}"
        );
        assert!(near_rest_shape(&f[n]), "frame {n}: {:?}", bounds(&f[n]));
    }
    // Falling under gravity: strictly downward until it lands.
    assert!(centre_y(&f[10]) > centre_y(&f[15]));
    // Pinned (KNOWN_MISMATCH, spec §8.3): the reference sinks through the plane (its lowest point
    // is at y -21.2 at frame 60); ours rests on it, 0.5 cm above.
    let (rlo, _) = bounds(&golden_sphere(case, "mesh-f60.obj"));
    assert!(rlo.y < -10.0);
    let (olo, _) = bounds(&f[60]);
    assert!(olo.y > -1.0, "ours sinks now: remove the pin ({})", olo.y);
}

#[test]
fn soft_sphere_stiff_deforms_less_in_both() {
    let drop = frames(&world(structural_of("soft-sphere-drop"), 0.0), 90);
    let case = "soft-sphere-stiff";
    let stiff = frames(&world(structural_of(case), 0.0), 90);
    // The reference: at frames 30 and 60 the stiff body is closer to the undeformed height.
    let dev = |p: &[DVec3]| {
        let (lo, hi) = bounds(p);
        (hi.y - lo.y - 100.0).abs()
    };
    for (file, case_default) in [("mesh-f30.obj", 30), ("mesh-f60.obj", 60)] {
        let d = dev(&golden_sphere("soft-sphere-drop", file));
        let s = dev(&golden_sphere(case, file));
        assert!(s < d, "reference frame {case_default}: {s} vs {d}");
    }
    // Ours: the largest squash over the landing (frames 20..90) is smaller when stiffer.
    let worst = |fr: &[Vec<DVec3>]| fr[20..].iter().map(|p| dev(p)).fold(0.0, f64::max);
    assert!(
        worst(&stiff) < worst(&drop),
        "{} vs {}",
        worst(&stiff),
        worst(&drop)
    );
    // And at frame 60 both are within 10 % of the rest height (reference: 0.7 and 4.4 %).
    assert!(near_rest_shape(&stiff[60]));
    // Pinned: the reference sinks 15.9 cm (stiff) and 21.2 cm (default) through the plane.
    let (lo, _) = bounds(&golden_sphere(case, "mesh-f60.obj"));
    assert!(lo.y < -10.0 && bounds(&stiff[60]).0.y > -1.0);
}

#[test]
fn soft_sphere_pressure_changes_nothing_as_observed() {
    // The reference's frames are identical to the default case with Pressure 100 (observation:
    // the value is stored, nothing changes). Ours does not simulate pressure either.
    let case = "soft-sphere-pressure";
    let base = frames(&world(1.0, 0.0), 40);
    let with = frames(&world(1.0, 100.0), 40);
    assert_eq!(base, with);
    for file in ["mesh.obj", "mesh-f30.obj", "mesh-f60.obj"] {
        assert_eq!(
            golden_sphere(case, file),
            golden_sphere("soft-sphere-drop", file),
            "{file}"
        );
    }
}
