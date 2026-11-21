//! Parity: docs/specs/ANIM.md §1.8 and §1.11 against tests/golden/animation-3 (level B
//! samples, tolerance 1e-4; the generator-param case also compares the mesh).

use std::path::Path;
use std::sync::Arc;

use forge_anim::{int_at, scene_at, value_at_exact};
use forge_core::{Document, Key, KeyTangent, Registry, Track, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_parity_harness::{
    GoldenCase, Mesh, Samples, compare_meshes, compare_samples, discover_cases, export_weld,
};

/// Sample files that build but miss the 1e-4 tolerance (ANIM.md §1.8, "solver residual").
const KNOWN_MISMATCH: &[&str] = &[
    "euler-cross/samples-rot_h.json",
    "quaternion/samples-rot_h.json",
    "generator-param/samples-strength.json",
];

fn number(v: &toml::Value) -> f64 {
    v.as_float()
        .or_else(|| v.as_integer().map(|i| i as f64))
        .expect("number")
}

/// Which entry of a key's value list a sampled parameter reads (rotation `[X, Y, Z]` is
/// `[P, H, B]`; colour is `[r, g, b]`).
fn component(param: &str) -> Option<usize> {
    match param {
        "color_r" | "rot_p" => Some(0),
        "color_g" | "rot_h" => Some(1),
        "color_b" | "rot_b" => Some(2),
        _ => None,
    }
}

/// The track of one sampled parameter: the O53 keys evaluate as straight chords (§1.8).
fn track_of(case: &GoldenCase, fps: f64, param: &str) -> Track {
    let params = case.manifest().unwrap().params;
    let angle = param.starts_with("rot_") || param == "strength";
    let mut t = Track::new(
        params["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| {
                let k = k.as_array().unwrap();
                let v = match (&k[1], component(param)) {
                    (toml::Value::Array(a), Some(c)) => number(&a[c]),
                    (v, _) => number(v),
                };
                Key::new(number(&k[0]) / fps, if angle { v.to_radians() } else { v })
            })
            .collect(),
    );
    for i in 0..t.keys.len() {
        t.set_tangent(i, KeyTangent::chord());
    }
    t
}

fn evaluate_samples(case: &GoldenCase, expected: &Samples) -> Samples {
    let Samples::Track {
        param,
        fps,
        tolerance,
        samples,
    } = expected
    else {
        panic!("track samples expected");
    };
    let track = track_of(case, *fps, param);
    let angle = param.starts_with("rot_") || param == "strength";
    Samples::Track {
        param: param.clone(),
        fps: *fps,
        tolerance: *tolerance,
        samples: samples
            .iter()
            .map(|(frame, _)| {
                let t = frame / fps;
                let v = if param == "segments_x" {
                    int_at(&track, t, true) as f64
                } else {
                    value_at_exact(&track, &[], t)
                };
                (*frame, if angle { v.to_degrees() } else { v })
            })
            .collect(),
    }
}

/// A cube with 8 segments per axis and a Bend child, optionally with a keyed strength;
/// the evaluated mesh at `time` seconds, welded as the exporter writes it.
fn bent_mesh(strength: Result<Track, f64>, time: f64) -> Mesh {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    let mut doc = Document::new(Arc::new(reg));
    let cube = doc
        .transact("case", |tx| {
            let cube = tx.create("forge.cube", None, None)?;
            for a in ["segments_x", "segments_y", "segments_z"] {
                tx.set_param(cube, a, Value::Int(8))?;
            }
            let bend = tx.create("forge.bend", Some(cube), None)?;
            match strength {
                Ok(t) => tx.set_track(bend, "strength", Some(t))?,
                Err(rad) => tx.set_param(bend, "strength", Value::Float(rad))?,
            }
            Ok(cube)
        })
        .unwrap();
    let scene = scene_at(doc.scene(), time);
    let mesh = evaluate(&scene, cube, &forge_core::EvalCtx::default())
        .unwrap()
        .expect("geometry");
    export_weld(&Mesh {
        positions: mesh.points.iter().map(|p| p.to_array()).collect(),
        faces: mesh.polys.clone(),
    })
}

#[test]
fn animation3_matches_the_reference() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut mismatched) = (Vec::new(), Vec::new());
    let mut meshes = 0;
    for case in discover_cases(&root).unwrap() {
        if case.feature != "animation-3" {
            continue;
        }
        for (file, expected) in case.samples().unwrap() {
            let key = format!("{}/{file}", case.name);
            let report = compare_samples(&expected, &evaluate_samples(&case, &expected));
            println!("{key}: max error {:.3e}", report.max_error);
            if report.passed() {
                matched.push(key);
            } else {
                mismatched.push(key);
            }
        }
        if case.name == "generator-param" {
            let m = case.manifest().unwrap();
            let fps = number(&m.params["document_fps"]);
            let time = number(&m.params["mesh_frame"]) / fps;
            let keyed = bent_mesh(Ok(track_of(&case, fps, "strength")), time);
            // the keyed strength drives the deformer: same mesh as the stored value it reads
            let strength = value_at_exact(&track_of(&case, fps, "strength"), &[], time);
            let stored = bent_mesh(Err(strength), time);
            let r = compare_meshes(&stored, &keyed, &m.tolerance);
            assert!(
                r.passed(),
                "keyed and stored strength differ: {:?}",
                r.failures
            );
            let rest = bent_mesh(Err(0.0), time);
            assert!(
                !compare_meshes(&rest, &keyed, &m.tolerance).passed(),
                "the bend acts"
            );
            // the reference mesh was exported from a setup that the case does not record
            // (cube size, the Bend's size, mode and angle): known mismatch, ANIM.md §1.8
            let src = std::fs::read_to_string(case.dir.join("mesh.obj")).unwrap();
            let expected = export_weld(&forge_parity_harness::parse_obj(&src).unwrap());
            let r = compare_meshes(&expected, &keyed, &m.tolerance);
            assert!(
                !r.passed(),
                "generator-param/mesh.obj matches now: update the spec"
            );
            meshes += 1;
        }
    }
    let want: Vec<String> = KNOWN_MISMATCH.iter().map(|s| s.to_string()).collect();
    mismatched.sort();
    let mut want_sorted = want.clone();
    want_sorted.sort();
    assert_eq!(mismatched, want_sorted, "mismatching sample files");
    // 11 sample files: three colour, three quaternion, two tie-span, one each otherwise
    assert_eq!(matched.len() + mismatched.len(), 11);
    assert_eq!(meshes, 1);
}
