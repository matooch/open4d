//! Parity: docs/specs/UV.md section U2 against tests/golden/uv (Part A: ten primitives).
//! Corner UVs are matched per corner within 1e-5; the deduplicated table size must equal the
//! observed `vt` count.

use std::collections::BTreeSet;
use std::path::Path;

mod common;

use common::typed_params;
use forge_core::Registry;
use forge_generators::{Primitives, uv::build_with_uvs};
use forge_parity_harness::discover_cases;

/// Part A cases and their node type.
const CASES: &[(&str, &str)] = &[
    ("cube", "forge.cube"),
    ("cube-segments", "forge.cube"),
    ("sphere", "forge.sphere"),
    ("cylinder", "forge.cylinder"),
    ("cone", "forge.cone"),
    ("torus", "forge.torus"),
    ("plane", "forge.plane"),
    ("disc", "forge.disc"),
    ("capsule", "forge.capsule"),
    ("tube", "forge.tube"),
];

/// Cases built but differing (none: all ten match).
const KNOWN_MISMATCH: &[&str] = &[];

type ObjUvs = (
    Vec<[f64; 3]>,
    Vec<Vec<usize>>,
    Vec<[f64; 2]>,
    Vec<Vec<usize>>,
);

/// positions, faces (position indices), vt table, per-corner vt indices.
fn parse(src: &str) -> ObjUvs {
    let (mut v, mut vt, mut faces, mut fvt) = (vec![], vec![], vec![], vec![]);
    for l in src.lines() {
        let t: Vec<&str> = l.split_whitespace().collect();
        match t.first().copied() {
            Some("v") => v.push([
                t[1].parse().unwrap(),
                t[2].parse().unwrap(),
                t[3].parse().unwrap(),
            ]),
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
    (v, faces, vt, fvt)
}

/// The case's scalar params; the slice angles, the regular-grid width and the fillet rows only
/// matter with their switches, which the Part A cases leave off.
fn table(
    type_id: &str,
    m: &forge_parity_harness::CaseManifest,
) -> toml::map::Map<String, toml::Value> {
    m.params
        .iter()
        .filter(|(k, _)| {
            !matches!(k.as_str(), "from_deg" | "to_deg" | "regular_grid")
                && !(k.as_str() == "width" && type_id != "forge.plane")
                && !k.contains("fillet")
        })
        .map(|(k, v)| {
            let k = match (type_id, k.as_str()) {
                ("forge.plane", "width_segments") => "segments_w",
                ("forge.plane", "height_segments") => "segments_h",
                _ => k.as_str(),
            };
            (k.to_string(), v.clone())
        })
        .collect()
}

fn registry() -> Registry {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r
}

#[test]
fn primitive_uvs_match_the_reference_per_corner() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut seen = BTreeSet::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "uv" {
            continue;
        }
        let Some((_, type_id)) = CASES.iter().find(|(n, _)| *n == case.name) else {
            continue;
        };
        let manifest = case.manifest().unwrap();
        let values = typed_params(&reg, type_id, &table(type_id, &manifest));
        let (mesh, uvs) = build_with_uvs(type_id, &values).unwrap();
        let uvs = uvs.unwrap_or_else(|| panic!("{}: no layout", case.name));
        let src = std::fs::read_to_string(case.dir.join("mesh.obj")).unwrap();
        let (_, faces, vt, fvt) = parse(&src);
        assert_eq!(faces.len(), mesh.polys.len(), "{}", case.name);
        let mut worst: f64 = 0.0;
        for (f, (ours, theirs)) in uvs.corners.iter().zip(&fvt).enumerate() {
            assert_eq!(ours.len(), theirs.len(), "{} face {f}", case.name);
            for (c, (o, t)) in ours.iter().zip(theirs).enumerate() {
                let e = (o[0] - vt[*t][0]).abs().max((o[1] - vt[*t][1]).abs());
                worst = worst.max(e);
                let ok = e <= 1e-5;
                if KNOWN_MISMATCH.contains(&case.name.as_str()) {
                    continue;
                }
                assert!(
                    ok,
                    "{} face {f} corner {c}: ours {o:?} vs {:?}",
                    case.name, vt[*t]
                );
            }
        }
        let (table, _) = uvs.indexed(1e-6);
        // the reference table can hold entries no face uses (capsule: 220 listed, 219 used)
        let used: BTreeSet<usize> = fvt.iter().flatten().copied().collect();
        if case.name == "cube-segments" {
            // every corner matches; the reference keeps 32 table entries where equal values of
            // different faces are merged to 29 (UV.md U3): pinned so a change shows up
            assert_eq!((table.len(), used.len()), (29, 32), "{}", case.name);
        } else {
            assert_eq!(table.len(), used.len(), "{}: vt count", case.name);
        }
        println!("{}: max error {worst:.2e}, {} vt", case.name, vt.len());
        seen.insert(case.name.clone());
    }
    assert_eq!(seen.len(), CASES.len());
}

#[test]
fn unsupported_options_have_no_layout() {
    let reg = registry();
    let mut p = std::collections::BTreeMap::new();
    p.insert("fillet".to_string(), forge_core::Value::Bool(true));
    let values = typed_params(&reg, "forge.cube", &toml::map::Map::new());
    let mut v = values.clone();
    v.extend(p);
    let (_, uvs) = build_with_uvs("forge.cube", &v).unwrap();
    assert!(uvs.is_none());
    let (mesh, uvs) = build_with_uvs("forge.sphere", &values_for(&reg, "forge.sphere")).unwrap();
    assert!(uvs.unwrap().matches(&mesh));
}

fn values_for(reg: &Registry, id: &str) -> std::collections::BTreeMap<String, forge_core::Value> {
    typed_params(reg, id, &toml::map::Map::new())
}
