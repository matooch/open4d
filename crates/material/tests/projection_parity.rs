//! Parity: docs/specs/UV.md section U4 against tests/golden/uv (Part B, `proj-*`).

use std::collections::BTreeMap;
use std::path::Path;

use forge_core::Registry;
use forge_generators::{Primitives, build};
use forge_material::{Camera, Projection, TagSpace, generate_uvw};
use forge_parity_harness::discover_cases;
use glam::{DMat4, DVec3};

const EXACT: &[&str] = &[
    "proj-flat",
    "proj-cubic",
    "proj-cubic-scaled",
    "proj-spherical",
    "proj-cylindrical",
    "proj-cylindrical-offset",
];
/// Built but different: a horizontal-only term the recorded camera does not explain (UV.md U4).
const KNOWN_MISMATCH: &[&str] = &["proj-frontal", "proj-frontal-2"];

type Parsed = (Vec<Vec<usize>>, Vec<[f64; 2]>, Vec<Vec<usize>>);

fn parse(src: &str) -> Parsed {
    let (mut vt, mut faces, mut fvt) = (vec![], vec![], vec![]);
    for l in src.lines() {
        let t: Vec<&str> = l.split_whitespace().collect();
        match t.first().copied() {
            Some("vt") => vt.push([t[1].parse().unwrap(), t[2].parse().unwrap()]),
            Some("f") => {
                let c: Vec<Vec<&str>> = t[1..].iter().map(|s| s.split('/').collect()).collect();
                faces.push(
                    c.iter()
                        .map(|x| x[0].parse::<usize>().unwrap() - 1)
                        .collect(),
                );
                fvt.push(
                    c.iter()
                        .map(|x| x[1].parse::<usize>().unwrap() - 1)
                        .collect(),
                );
            }
            _ => {}
        }
    }
    (faces, vt, fvt)
}

fn f3(v: &toml::Value) -> [f64; 3] {
    let a = v.as_array().unwrap();
    std::array::from_fn(|i| {
        a[i].as_float()
            .or_else(|| a[i].as_integer().map(|x| x as f64))
            .unwrap()
    })
}

fn max_error(name: &str) -> f64 {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    let ty = r.get("forge.sphere").unwrap();
    let values: BTreeMap<_, _> = ty
        .params
        .iter()
        .map(|p| (p.id.clone(), p.default.clone()))
        .collect();
    let mesh = build("forge.sphere", &values).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let case = discover_cases(&root)
        .unwrap()
        .into_iter()
        .find(|c| c.feature == "uv" && c.name == name)
        .unwrap();
    let m = case.manifest().unwrap();
    let tag = m.params["texture_tag"].as_table().unwrap();
    let space = TagSpace {
        position: DVec3::from(f3(&tag["position"])),
        rotation: f3(&tag["rotation_deg"]).map(f64::to_radians),
        size: DVec3::from(f3(&tag["size"])),
    };
    let camera = Camera {
        position: DVec3::from(f3(&tag["camera_position"])),
        hpb: f3(&tag["camera_rotation_deg"]).map(f64::to_radians),
        aspect: 16.0 / 9.0,
    };
    let id = match tag["projection_set"].as_str().unwrap() {
        "Flat" => "flat",
        "Cubic" => "cubic",
        "Spherical" => "spherical",
        "Cylindrical" => "cylindrical",
        "Frontal" => "frontal",
        other => panic!("{other}"),
    };
    let proj = Projection::from_id(id, Some(camera)).unwrap();
    let uvs = generate_uvw(&mesh, &space, proj, DMat4::IDENTITY).unwrap();
    let (faces, vt, fvt) = parse(&std::fs::read_to_string(case.dir.join("mesh.obj")).unwrap());
    assert_eq!(faces.len(), mesh.polys.len());
    let mut worst: f64 = 0.0;
    for (o, t) in uvs.corners.iter().flatten().zip(fvt.iter().flatten()) {
        worst = worst
            .max((o[0] - vt[*t][0]).abs())
            .max((o[1] - vt[*t][1]).abs());
    }
    worst
}

#[test]
fn exact_projections_match_per_corner() {
    for name in EXACT {
        let e = max_error(name);
        println!("{name}: max corner error {e:.2e}");
        assert!(e <= 1e-5, "{name}: {e}");
    }
}

#[test]
fn known_mismatch_projections_still_differ() {
    for name in KNOWN_MISMATCH {
        let e = max_error(name);
        println!("{name}: max corner error {e:.2e}");
        assert!(e > 1e-5 && e < 1e-3, "{name}: {e}");
    }
}

#[test]
fn unsupported_projections_are_reported() {
    for id in ["spatial", "uvw", "shrink_wrap", "camera"] {
        assert!(Projection::from_id(id, None).is_err(), "{id}");
    }
    assert!(Projection::from_id("frontal", None).is_err());
}
