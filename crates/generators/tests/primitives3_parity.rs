//! Parity: docs/specs/PRIMITIVES3.md against tests/golden/{platonic, oil-tank,
//! polygon-prim, figure}. Figure is level C (own mannequin): only its bounding box is
//! compared.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod common;

use common::{Table, resolved_params, typed_params};
use forge_core::Registry;
use forge_generators::{BuildError, Primitives, build};
use forge_parity_harness::{Mesh, compare_meshes, discover_cases};

/// Observed options not derived (none in these features).
const UNSUPPORTED: &[&str] = &["oil-tank/orient-z-grid"];

/// Built but differing (PRIMITIVES3.md P4): the exported Bucky is irregular (radii 97.6 to
/// 98.5, coordinate noise 0.05 to 0.2 against a 1e-4 tolerance) and its listing order is
/// not derived.
///
/// The Bucky cases at Radius 10 and 1000 show the same irregularity scaled with the radius
/// (sizes 19.268 / 192.675 / 1926.748): a fixed shape, not noise, but still not derived.
const KNOWN_MISMATCH: &[&str] = &[
    "platonic/type-bucky",
    "platonic/bucky-r10",
    "platonic/bucky-r1000",
    "platonic/bucky-seg2",
];

const STRICT: &[&str] = &["platonic", "oil-tank", "polygon-prim"];

fn registry() -> Registry {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r
}

/// Case spellings to ours: `plus_y` is `pos_y`, `minus_z` is `neg_z`; `grid_width` is
/// `regular_grid_width` (PRIMITIVES3.md header).
fn ours(raw: &Table) -> Table {
    raw.iter()
        .map(|(k, v)| {
            let k = if k == "grid_width" {
                "regular_grid_width".to_string()
            } else {
                k.clone()
            };
            let v = match (k.as_str(), v) {
                ("orientation", toml::Value::String(s)) => {
                    toml::Value::String(s.replace("plus_", "pos_").replace("minus_", "neg_"))
                }
                _ => v.clone(),
            };
            (k, v)
        })
        .collect()
}

fn as_mesh(m: &forge_geometry::PolyMesh) -> Mesh {
    Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys.clone(),
    }
}

fn size(m: &Mesh) -> [f64; 3] {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in &m.positions {
        for a in 0..3 {
            lo[a] = lo[a].min(p[a]);
            hi[a] = hi[a].max(p[a]);
        }
    }
    [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]
}

#[test]
fn primitives3_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    let mut unsupported = BTreeSet::new();
    let mut per_feature: BTreeMap<String, usize> = BTreeMap::new();
    for case in discover_cases(&root).unwrap() {
        let feature = case.feature.as_str();
        if !STRICT.contains(&feature) && feature != "figure" {
            continue;
        }
        let key = format!("{feature}/{}", case.name);
        let manifest = case.manifest().unwrap();
        let type_id = format!("forge.{}", feature.replace('-', "_"));
        let p = typed_params(&reg, &type_id, &ours(&resolved_params(&case, &manifest)));
        let mesh = match build(&type_id, &p) {
            Err(BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
                continue;
            }
            other => other.unwrap_or_else(|e| panic!("{key}: {e}")),
        };
        mesh.validate().unwrap();
        let (ours, expected) = (as_mesh(&mesh), case.expected_mesh().unwrap().unwrap());
        if feature == "figure" {
            // level C: bounding box within 5 % per axis, nothing else of the export is used
            let (a, b) = (size(&ours), size(&expected));
            for axis in 0..3 {
                assert!(
                    (a[axis] - b[axis]).abs() <= 0.05 * b[axis],
                    "{key}: axis {axis}: ours {} observed {}",
                    a[axis],
                    b[axis]
                );
            }
            matched.push(key);
            continue;
        }
        let report = compare_meshes(&expected, &ours, &manifest.tolerance);
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            assert!(
                !report.passed(),
                "{key} matches now: remove it from KNOWN_MISMATCH"
            );
            println!("{key}: known mismatch");
            continue;
        }
        assert!(
            report.passed(),
            "{key}: {:?} (max error {:?})",
            report.failures,
            report.max_position_error
        );
        *per_feature.entry(feature.to_string()).or_default() += 1;
        matched.push(key);
    }
    println!("matched: {}", matched.join(", "));
    let want: BTreeSet<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    assert_eq!(per_feature.get("platonic"), Some(&12), "{per_feature:?}");
    assert_eq!(per_feature.get("oil-tank"), Some(&6), "{per_feature:?}");
    assert_eq!(per_feature.get("polygon-prim"), Some(&6), "{per_feature:?}");
    assert_eq!(matched.len(), 12 + 6 + 6 + 2, "{matched:?}");
}

/// P4: our Bucky has the exported counts (60 points, 12 triangles and 52 quads) with every
/// point on the circumsphere.
#[test]
fn bucky_has_the_exported_counts() {
    let reg = registry();
    let p = typed_params(
        &reg,
        "forge.platonic",
        &[("type".to_string(), toml::Value::String("bucky".into()))]
            .into_iter()
            .collect(),
    );
    let m = build("forge.platonic", &p).unwrap();
    m.validate().unwrap();
    assert_eq!((m.points.len(), m.polys.len()), (60, 64));
    assert_eq!(m.polys.iter().filter(|f| f.len() == 3).count(), 12);
    assert_eq!(m.polys.iter().filter(|f| f.len() == 4).count(), 52);
    for q in &m.points {
        assert!((q.length() - 100.0).abs() < 1e-9);
    }
}

/// P2: dodeca and bucky cut flat at Segments 2 have the exported counts
/// (`platonic/dodeca-seg2` 74 points, 48 + 48 faces; `bucky-seg2` 234 points, 48 + 208).
#[test]
fn dodeca_and_bucky_segments_have_the_exported_counts() {
    let reg = registry();
    for (kind, points, tris, quads) in [("dodeca", 74, 48, 48), ("bucky", 234, 48, 208)] {
        let p = typed_params(
            &reg,
            "forge.platonic",
            &[
                ("type".to_string(), toml::Value::String(kind.into())),
                ("segments".to_string(), toml::Value::Integer(2)),
            ]
            .into_iter()
            .collect(),
        );
        let m = build("forge.platonic", &p).unwrap();
        m.validate().unwrap();
        assert_eq!(m.points.len(), points, "{kind}");
        assert_eq!(
            m.polys.iter().filter(|f| f.len() == 3).count(),
            tris,
            "{kind}"
        );
        assert_eq!(
            m.polys.iter().filter(|f| f.len() == 4).count(),
            quads,
            "{kind}"
        );
    }
}
