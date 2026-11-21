//! Camera parity at level C against tests/golden/camera (docs/specs/RENDER.md R15, O57).
//!
//! The scene of every case (a horizontal plane with a cube on it, read from the OBJ of the case)
//! is rendered through a `forge.camera` object that the case file's camera table fills, via the
//! render bridge. Only geometry is compared: the silhouette (any channel above 0.05 in the
//! reference, coverage >= 0.5 in ours, both with the Cubic filter of width 2) must agree within
//! 1 px, like the O30 render cases.

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use common::golden::{ObjGroup, parse_obj};
use forge_core::{Document, Registry, Value};
use forge_parity_harness::{CompareOptions, ImageBuf, compare_image};
use forge_render_api::*;
use forge_render_bridge::{apply_scene_objects, default_material};
use forge_render_cpu::CpuEngine;
use glam::{DMat4, DVec3};

const REFERENCE_THRESHOLD: f32 = 0.05;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/camera")
}

/// Cases whose picture cannot be compared (reason in RENDER.md R15).
const UNSUPPORTED: &[(&str, &str)] = &[];

/// Cases that build but differ; asserted to still differ.
const KNOWN_MISMATCH: &[(&str, &str)] = &[(
    "ortho-zoom-2",
    "the orthographic width at zoom 2 is about 510.8 units in the reference (plane 35.0..285.6 px, \
     cube 125.3 px of 200 units), not 1000 / zoom = 500 (plane 31..288) and not a halving of any \
     width consistent with ortho-front; a third zoom value is needed",
)];

/// Cases of the form that were observed as an empty frame: their folders must stay absent until
/// the observation is redone (docs/parity/camera.md notes).
/// Cases of the calibration scene (a texture tag is recorded): the plane carries a checker whose
/// black cells are zero in the reference picture, so they are indistinguishable from the
/// background and the picture's silhouette is not its geometry. They are compared one-sided
/// (RENDER.md R15): every lit reference pixel lies within 1 px of a pixel we touch (coverage
/// above the reference's own 0.05 threshold), and the lit bounding box agrees within 1 px.
fn is_calibration(name: &str) -> bool {
    let doc: toml::Table = std::fs::read_to_string(root().join(name).join("case.toml"))
        .unwrap()
        .parse()
        .unwrap();
    doc["params"]
        .as_table()
        .unwrap()
        .contains_key("texture_tag")
}

const NOT_OBSERVED: &[&str] = &["default", "focal-100", "aspect-square", "dof", "sensor"];

fn case_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(root())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("case.toml").exists())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn registry() -> Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&forge_generators::Primitives).unwrap();
    Arc::new(r)
}

fn f3(v: &toml::Value) -> [f64; 3] {
    let a = v.as_array().unwrap();
    [
        a[0].as_float().unwrap(),
        a[1].as_float().unwrap(),
        a[2].as_float().unwrap(),
    ]
}

/// A document with one camera object whose parameters come from the case's camera table.
fn camera_document(cam: &toml::Table) -> Document {
    let mut doc = Document::new(registry());
    let rad = f64::to_radians;
    let projection = match cam["projection"].as_str().unwrap() {
        "Perspective" => "perspective",
        "Front" => "front",
        "Top" => "top",
        other => panic!("projection {other} has no mapping"),
    };
    doc.transact("camera", |tx| {
        let n = tx.create("forge.camera", None, None)?;
        let r = f3(&cam["rotation_deg"]);
        tx.set_param(n, "transform.pos", Value::Vec3(f3(&cam["position"])))?;
        tx.set_param(
            n,
            "transform.rot",
            Value::Vec3([rad(r[0]), rad(r[1]), rad(r[2])]),
        )?;
        tx.set_param(n, "projection", Value::Enum(projection.into()))?;
        let fov = cam["field_of_view_horizontal_deg"].as_float().unwrap();
        tx.set_param(n, "fov_horizontal", Value::Float(rad(fov)))?;
        tx.set_param(
            n,
            "sensor_size",
            Value::Float(cam["sensor_size_film_gate"].as_float().unwrap()),
        )?;
        tx.set_param(n, "zoom", Value::Float(cam["zoom"].as_float().unwrap()))?;
        let pct = |k: &str| cam[k].as_float().unwrap() / 100.0;
        tx.set_param(
            n,
            "film_offset_x",
            Value::Float(pct("film_offset_x_percent")),
        )?;
        tx.set_param(
            n,
            "film_offset_y",
            Value::Float(pct("film_offset_y_percent")),
        )?;
        Ok(())
    })
    .unwrap();
    doc
}

struct Case {
    snapshot: Snapshot,
    expected: ImageBuf,
    size: (u32, u32),
}

fn load(name: &str) -> Case {
    let dir = root().join(name);
    let doc: toml::Table = std::fs::read_to_string(dir.join("case.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let params = doc["params"].as_table().unwrap();
    let size = (
        params["render_width"].as_integer().unwrap() as u32,
        params["render_height"].as_integer().unwrap() as u32,
    );
    let mut snap = Snapshot {
        materials: vec![default_material()],
        ..Snapshot::default()
    };
    let obj = std::fs::read_to_string(dir.join("mesh.obj")).unwrap();
    for (i, ObjGroup { mesh, .. }) in parse_obj(&obj).into_iter().enumerate() {
        snap.meshes.push(mesh);
        snap.instances
            .push(Instance::new(i, DMat4::IDENTITY, 0, i as u32));
    }
    let scene_doc = camera_document(params["camera"].as_table().unwrap());
    let applied = apply_scene_objects(&mut snap, scene_doc.scene());
    assert!(
        applied.camera && applied.warnings.is_empty(),
        "{:?}",
        applied.warnings
    );
    let png = std::fs::read(dir.join("render.png")).unwrap();
    let img = forge_image::decode(&png, &forge_image::Limits::default())
        .unwrap()
        .image;
    let expected = ImageBuf::from_fn(img.width, img.height, |x, y| {
        let p = img.pixel(x, y);
        [p[0], p[1], p[2]]
    });
    Case {
        snapshot: snap,
        expected,
        size,
    }
}

fn settings(size: (u32, u32)) -> RenderSettings {
    RenderSettings {
        filter: PixelFilter::Cubic,
        filter_width: 2.0,
        ..RenderSettings::new(size.0, size.1, 16)
    }
}

/// Silhouette pixels beyond 1 px between the reference and our coverage.
fn mismatch(name: &str) -> usize {
    let c = load(name);
    let out = CpuEngine::new()
        .render(&c.snapshot, &settings(c.size), &Control::new())
        .unwrap();
    let mut e = c.expected.clone();
    e.alpha = Some(
        e.mask(REFERENCE_THRESHOLD)
            .iter()
            .map(|b| if *b { 1.0 } else { 0.0 })
            .collect(),
    );
    let act = ImageBuf::from_rgba(c.size.0, c.size.1, &out.beauty.data);
    compare_image(
        &e,
        &act,
        &CompareOptions {
            silhouette_threshold: 0.5,
            ..CompareOptions::default()
        },
    )
    .unwrap()
    .silhouette_mismatch_px
}

/// Lit reference pixels with no covered pixel of ours within 1 px, and the bounding boxes of
/// the two masks `(x0, y0, x1, y1)`.
fn one_sided(name: &str) -> (usize, [u32; 4], [u32; 4]) {
    let c = load(name);
    let out = CpuEngine::new()
        .render(&c.snapshot, &settings(c.size), &Control::new())
        .unwrap();
    let (w, h) = c.size;
    let lit = c.expected.mask(REFERENCE_THRESHOLD);
    let ours: Vec<bool> = (0..(w * h) as usize)
        .map(|i| out.beauty.data[i * 4 + 3] > REFERENCE_THRESHOLD)
        .collect();
    let near = |x: u32, y: u32| {
        (y.saturating_sub(1)..=(y + 1).min(h - 1)).any(|yy| {
            (x.saturating_sub(1)..=(x + 1).min(w - 1)).any(|xx| ours[(yy * w + xx) as usize])
        })
    };
    let bbox = |m: &[bool]| {
        let mut b = [w, h, 0, 0];
        for y in 0..h {
            for x in 0..w {
                if m[(y * w + x) as usize] {
                    b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
                }
            }
        }
        b
    };
    let far: Vec<(u32, u32)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|(x, y)| lit[(y * w + x) as usize] && !near(*x, *y))
        .collect();
    (far.len(), bbox(&lit), bbox(&ours))
}

#[test]
fn calibration_scene_silhouettes_cover_the_reference_within_one_pixel() {
    let mut checked = 0;
    for name in case_names().into_iter().filter(|n| is_calibration(n)) {
        let (far, r, o) = one_sided(&name);
        println!("{name:16} far reference px {far}, bbox ref {r:?} ours {o:?}");
        if KNOWN_MISMATCH.iter().any(|(n, _)| *n == name) {
            let off = (0..4).any(|k| r[k].abs_diff(o[k]) > 1);
            assert!(off, "{name} now matches: remove it from KNOWN_MISMATCH");
            continue;
        }
        assert_eq!(
            far, 0,
            "{name}: reference pixels beyond 1 px of our coverage"
        );
        for k in 0..4 {
            assert!(
                r[k].abs_diff(o[k]) <= 1,
                "{name}: bounding box {r:?} vs {o:?}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 3);
}

#[test]
fn silhouettes_match_within_one_pixel() {
    let mut checked = 0;
    for name in case_names() {
        if UNSUPPORTED.iter().any(|(n, _)| *n == name) || is_calibration(&name) {
            continue;
        }
        let m = mismatch(&name);
        println!("{name:12} mismatch {m}");
        if KNOWN_MISMATCH.iter().any(|(n, _)| *n == name) {
            assert!(m > 0, "{name} now matches: remove it from KNOWN_MISMATCH");
        } else {
            assert_eq!(m, 0, "{name}: silhouette differs by more than 1 px");
            checked += 1;
        }
    }
    assert!(checked >= 4, "only {checked} cases were compared");
}

#[test]
fn unobserved_cases_stay_absent() {
    for n in NOT_OBSERVED {
        assert!(
            !root().join(n).exists(),
            "{n} now exists: write its test (the observation needed redoing)"
        );
    }
}

#[test]
fn every_golden_case_is_listed_or_checked() {
    let names = case_names();
    assert_eq!(
        names,
        [
            "aspect-square-2",
            "default-2",
            "film-offset",
            "film-offset-y",
            "focal-18",
            "ortho-front",
            "ortho-top",
            "ortho-zoom-2"
        ]
    );
    for (n, why) in UNSUPPORTED.iter().chain(KNOWN_MISMATCH) {
        assert!(names.iter().any(|c| c == n) && !why.is_empty());
    }
}

#[test]
fn wide_lens_widens_the_view_and_film_offset_shifts_it() {
    // Independent of the pictures: a 90 degree lens sees twice the width of a 53 degree one at
    // equal distance, and +20 % film offset moves the picture left: the principal point 64 px left at 320 px.
    let mut doc = Document::new(registry());
    let id = doc
        .transact("c", |tx| tx.create("forge.camera", None, None))
        .unwrap();
    let frame = |doc: &Document| {
        let mut s = Snapshot::default();
        apply_scene_objects(&mut s, doc.scene());
        s.camera.pixel_frame(320, 240).unwrap()
    };
    let (f0, c0) = frame(&doc);
    assert!((f0 - 320.0).abs() < 1e-9 && (c0[0] - 160.0).abs() < 1e-9);
    doc.transact("fov", |tx| {
        tx.set_param(
            id,
            "fov_horizontal",
            Value::Float(std::f64::consts::FRAC_PI_2),
        )?;
        tx.set_param(id, "film_offset_x", Value::Float(0.2))
    })
    .unwrap();
    let (f1, c1) = frame(&doc);
    assert!((f1 - 160.0).abs() < 1e-9 && (c1[0] - 96.0).abs() < 1e-9);
    let _ = DVec3::ZERO;
}
