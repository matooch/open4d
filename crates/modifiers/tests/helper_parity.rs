//! Parity: docs/specs/MODIFIERS.md §14–15 against
//! tests/golden/{mesh-deformer,surface,camera-deformer,collision,jiggle,delta-mush,correction}.
//!
//! `correction/defaults` and `delta-mush/defaults` are compared exactly. In the other six
//! the reference export did not run the deformer (O45 Findings), so the exact comparison is
//! pinned in `KNOWN_MISMATCH`; for those the test checks that (a) the helper objects leaked
//! into the export equal ours, (b) the golden cube is the undeformed (jiggle: rigidly moved)
//! cube, and (c) our deformed cube satisfies the invariants of the spec.

use std::path::Path;
use std::sync::Arc;

use forge_core::{
    Document, EvalCtx, Interp, Key, MeshData, NodeId, Registry, Scene, Track, Tx, Value,
};
use forge_generators::{Primitives, evaluate, from_poly_mesh, make_editable};
use forge_geometry::PolyMesh;
use forge_modifiers::Deformers;
use forge_parity_harness::{GoldenCase, Mesh, compare_meshes, discover_cases, export_weld};
use glam::DVec3;

const FEATURES: &[&str] = &[
    "mesh-deformer",
    "surface",
    "camera-deformer",
    "collision",
    "jiggle",
    "delta-mush",
    "correction",
];

/// Cases that differ from the golden because the reference export did not deform.
const KNOWN_MISMATCH: &[&str] = &[
    "mesh-deformer/cage",
    "surface/defaults",
    "collision/sphere",
    "jiggle/frame-10",
    "jiggle/frame-20",
];

/// Observed cases we do not build.
const UNSUPPORTED: &[&str] = &[];

type Table = toml::map::Map<String, toml::Value>;

fn num(v: &toml::Value) -> f64 {
    v.as_float()
        .or(v.as_integer().map(|i| i as f64))
        .expect("number")
}

fn vec3(v: &toml::Value) -> [f64; 3] {
    let a = v.as_array().expect("vec3");
    [num(&a[0]), num(&a[1]), num(&a[2])]
}

fn rad(v: [f64; 3]) -> Value {
    Value::Vec3(v.map(f64::to_radians))
}

/// Sets the generator keys of a `[params.*]` table that are parameters of `ty_id`.
fn set_generator(tx: &mut Tx, reg: &Registry, id: NodeId, ty_id: &str, table: &Table) {
    let ty = reg.get(ty_id).unwrap();
    for (k, v) in table {
        let key = match k.as_str() {
            "position" => {
                tx.set_param(id, "transform.pos", Value::Vec3(vec3(v)))
                    .unwrap();
                continue;
            }
            "rotation_deg" => {
                tx.set_param(id, "transform.rot", rad(vec3(v))).unwrap();
                continue;
            }
            "width_segments" => "segments_w",
            "height_segments" => "segments_h",
            k => k,
        };
        let Some(desc) = ty.find_param(key) else {
            continue; // helper keys that are not parameters (type, list, orientation label...)
        };
        let value = match (&desc.kind, v) {
            (forge_core::ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
            (forge_core::ParamKind::Float { .. }, v) if v.is_float() || v.is_integer() => {
                Value::Float(num(v))
            }
            (forge_core::ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
            _ => continue,
        };
        tx.set_param(id, key, value).unwrap();
    }
}

fn pts(m: &PolyMesh) -> Vec<DVec3> {
    m.points.clone()
}

/// Concatenates meshes (points and faces re-indexed) in order.
fn concat(parts: &[(Vec<DVec3>, Vec<Vec<u32>>)]) -> Mesh {
    let mut positions = Vec::new();
    let mut faces = Vec::new();
    for (p, f) in parts {
        let base = positions.len() as u32;
        positions.extend(p.iter().map(|q| q.to_array()));
        faces.extend(
            f.iter()
                .map(|f| f.iter().map(|i| i + base).collect::<Vec<_>>()),
        );
    }
    Mesh { positions, faces }
}

/// The scene of a case: the helper objects (exported before the cube), the cube.
struct Built {
    doc: Document,
    cube: NodeId,
    /// Helper objects in export order.
    helpers: Vec<NodeId>,
}

fn world_mesh(scene: &Scene, id: NodeId, ctx: &EvalCtx) -> (Vec<DVec3>, Vec<Vec<u32>>) {
    let m = evaluate(scene, id, ctx).unwrap().expect("geometry");
    let w = scene.world_matrix(id).unwrap();
    (
        m.points.iter().map(|p| w.transform_point3(*p)).collect(),
        m.polys,
    )
}

/// Builds the case; `deformed` adds the deformer(s), `bind` is the Initialize data.
fn build(reg: &Arc<Registry>, case: &GoldenCase, params: &Table, deformed: bool) -> Built {
    let feature = case.feature.as_str();
    let mut doc = Document::new(reg.clone());
    let input = params["input"].as_table().unwrap();
    let mut helper_ids: Vec<NodeId> = Vec::new();
    let cube = doc
        .transact("case", |tx| {
            let mut cube = tx.create("forge.cube", None, None)?;
            set_generator(tx, reg, cube, "forge.cube", input);
            let dty = match feature {
                "surface" => "forge.surface_deformer".to_string(),
                f => format!("forge.{}", f.replace('-', "_")),
            };
            match feature {
                "mesh-deformer" => {
                    let cage_t = params["cage"].as_table().unwrap();
                    let cage = tx.create("forge.cube", None, None)?;
                    for (k, n) in [("size_x", 300.0), ("size_y", 500.0), ("size_z", 300.0)] {
                        tx.set_param(cage, k, Value::Float(n))?;
                    }
                    for (k, n) in [("segments_x", 1), ("segments_y", 1), ("segments_z", 1)] {
                        tx.set_param(cage, k, Value::Int(n))?;
                    }
                    let cage = make_editable(tx, cage).unwrap();
                    set_generator(tx, reg, cage, "forge.polygon", cage_t);
                    let mesh = forge_generators::to_poly_mesh(&mesh_of(tx, cage));
                    let before = mesh.points.clone();
                    // Initialize on the unmoved cage, then move cage point 0.
                    let mut moved = before.clone();
                    let b = vec3(&cage_t["cage_point0_before"]);
                    let a = vec3(&cage_t_after(cage_t));
                    assert_eq!(before[0].to_array(), b);
                    moved[0] = DVec3::from_array(a);
                    let data = Arc::new(from_poly_mesh(&PolyMesh {
                        points: moved,
                        polys: mesh.polys.clone(),
                    }));
                    tx.set_param(cage, "mesh", Value::Mesh(data))?;
                    helper_ids.push(cage);
                    if deformed {
                        let d = tx.create(&dty, Some(cube), None)?;
                        tx.set_param(d, "cages", Value::LinkList(vec![cage]))?;
                        tx.set_param(d, "bind", list(&before))?;
                        tx.set_param(
                            d,
                            "strength",
                            Value::Float(num(&params["strength_percent"])),
                        )?;
                    }
                }
                "surface" => {
                    let plane_t = params["surface_plane"].as_table().unwrap();
                    let plane = tx.create("forge.plane", None, None)?;
                    set_generator(tx, reg, plane, "forge.plane", plane_t);
                    helper_ids.push(plane);
                    let bend_t = params["surface_bend"].as_table().unwrap();
                    let flat = world_mesh(tx.scene(), plane, &EvalCtx::default()).0;
                    let bend = tx.create("forge.bend", Some(plane), None)?;
                    tx.set_param(
                        bend,
                        "strength",
                        Value::Float(num(&bend_t["strength_deg"]).to_radians()),
                    )?;
                    if deformed {
                        let d = tx.create(&dty, Some(cube), None)?;
                        tx.set_param(d, "surface", Value::Link(Some(plane)))?;
                        tx.set_param(d, "bind", list(&flat))?;
                        tx.set_param(
                            d,
                            "strength",
                            Value::Float(num(&params["strength_percent"])),
                        )?;
                        tx.set_param(
                            d,
                            "proportional",
                            Value::Bool(params["proportional"].as_bool().unwrap()),
                        )?;
                    }
                }
                "camera-deformer" => {
                    let cam_t = params["camera"].as_table().unwrap();
                    let cam = tx.create("forge.null", None, None)?;
                    tx.set_param(cam, "transform.pos", Value::Vec3(vec3(&cam_t["position"])))?;
                    tx.set_param(cam, "transform.rot", rad(vec3(&cam_t["rotation_deg"])))?;
                    helper_ids.push(cam);
                    if deformed {
                        let d = tx.create(&dty, Some(cube), None)?;
                        tx.set_param(d, "camera", Value::Link(Some(cam)))?;
                        let (gx, gy) = (
                            params["grid_x"].as_integer().unwrap(),
                            params["grid_y"].as_integer().unwrap(),
                        );
                        tx.set_param(d, "grid_x", Value::Int(gx))?;
                        tx.set_param(d, "grid_y", Value::Int(gy))?;
                        // The grid point nearest to the recorded position (spec §14.3).
                        let before = vec3(&cam_t["grid_point34_before"]);
                        let after = vec3(&cam_t["grid_point34_after"]);
                        let at = |c: f64, g: i64| {
                            ((c / 1000.0 + 0.75) / 1.5 * g as f64).round() as usize
                        };
                        let (i, j) = (at(before[0], gx), at(before[1], gy));
                        let mut offsets = vec![[0.0; 3]; ((gx + 1) * (gy + 1)) as usize];
                        offsets[j * (gx as usize + 1) + i] =
                            [after[0] - before[0], after[1] - before[1], 0.0];
                        tx.set_param(d, "offsets", Value::Vec3List(offsets))?;
                        tx.set_param(
                            d,
                            "strength",
                            Value::Float(num(&params["strength_percent"])),
                        )?;
                    }
                }
                "collision" => {
                    let col_t = params["collider"].as_table().unwrap();
                    let sphere = tx.create("forge.sphere", None, None)?;
                    set_generator(tx, reg, sphere, "forge.sphere", col_t);
                    helper_ids.push(sphere);
                    if deformed {
                        let d = tx.create(&dty, Some(cube), None)?;
                        tx.set_param(d, "colliders", Value::LinkList(vec![sphere]))?;
                        tx.set_param(
                            d,
                            "strength",
                            Value::Float(num(&params["strength_percent"])),
                        )?;
                    }
                }
                "jiggle" => {
                    let end = num(&params["pos_x_end"]);
                    let fps = num(&params["fps"]);
                    let track = Track::new(vec![
                        Key {
                            time: 0.0,
                            value: 0.0,
                            interp: Interp::Linear,
                        },
                        Key {
                            time: 10.0 / fps,
                            value: end,
                            interp: Interp::Linear,
                        },
                    ]);
                    tx.set_track(cube, "transform.pos.x", Some(track))?;
                    if deformed {
                        let d = tx.create(&dty, Some(cube), None)?;
                        for (k, v) in params {
                            let Some(key) = k.strip_suffix("_percent") else {
                                continue;
                            };
                            if matches!(
                                key,
                                "strength" | "stiffness" | "structural" | "drag" | "motion_scale"
                            ) {
                                tx.set_param(d, key, Value::Float(num(v)))?;
                            }
                        }
                        for k in ["springs", "iterations"] {
                            tx.set_param(d, k, Value::Int(params[k].as_integer().unwrap()))?;
                        }
                        tx.set_param(d, "gravity", Value::Float(num(&params["gravity"])))?;
                        tx.set_param(d, "local", Value::Bool(params["local"].as_bool().unwrap()))?;
                        tx.set_param(
                            d,
                            "cache",
                            Value::Int(params["cache"].as_integer().unwrap()),
                        )?;
                    }
                }
                "delta-mush" => {
                    cube = make_editable(tx, cube).unwrap();
                    let mut mesh = forge_generators::to_poly_mesh(&mesh_of(tx, cube));
                    mesh.points[0] += DVec3::from_array(vec3(&params["input_point0_moved"]));
                    tx.set_param(cube, "mesh", Value::Mesh(Arc::new(from_poly_mesh(&mesh))))?;
                    if deformed {
                        let d = tx.create(&dty, Some(cube), None)?;
                        // Initialize on the edited cube.
                        tx.set_param(d, "bind", list(&mesh.points))?;
                        tx.set_param(
                            d,
                            "strength",
                            Value::Float(num(&params["strength_percent"])),
                        )?;
                        tx.set_param(
                            d,
                            "iterations",
                            Value::Int(params["iterations"].as_integer().unwrap()),
                        )?;
                        tx.set_param(
                            d,
                            "pin_border_points",
                            Value::Bool(params["pin_border_points"].as_bool().unwrap()),
                        )?;
                    }
                }
                "correction" => {
                    if deformed {
                        let above = tx.create("forge.bend", Some(cube), None)?;
                        tx.set_param(above, "strength", Value::Float(90f64.to_radians()))?;
                        // Update: store the bent cube; then edit corrected point 0.
                        let bent = evaluate(tx.scene(), cube, &EvalCtx::default())
                            .unwrap()
                            .unwrap();
                        let mut offsets = vec![[0.0; 3]; bent.points.len()];
                        assert_eq!(
                            bent.points.len() as i64,
                            params["correction_points"].as_integer().unwrap()
                        );
                        assert_eq!(
                            bent.points[0].to_array(),
                            vec3(&params["correction_point0_before"])
                        );
                        offsets[0] = vec3(&params["moved_by"]);
                        let d = tx.create(&dty, Some(cube), None)?;
                        tx.set_param(d, "reference", list(&bent.points))?;
                        tx.set_param(d, "offsets", Value::Vec3List(offsets))?;
                        tx.set_param(
                            d,
                            "strength",
                            Value::Float(num(&params["strength_percent"])),
                        )?;
                        let mapping = params["mapping"].as_str().unwrap().to_lowercase();
                        tx.set_param(d, "mapping", Value::Enum(mapping))?;
                    }
                }
                other => panic!("unknown feature {other}"),
            }
            Ok(cube)
        })
        .unwrap();
    Built {
        doc,
        cube,
        helpers: helper_ids,
    }
}

fn cage_t_after(t: &Table) -> toml::Value {
    t["cage_point0_after"].clone()
}

fn list(p: &[DVec3]) -> Value {
    Value::Vec3List(p.iter().map(|q| q.to_array()).collect())
}

fn mesh_of(tx: &Tx, id: NodeId) -> Arc<MeshData> {
    match tx.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => m.clone(),
        other => panic!("no mesh: {other:?}"),
    }
}

/// The frame of a case (0 unless it has `frame`) and its context.
fn ctx_of(params: &Table) -> (i64, EvalCtx) {
    let frame = params.get("frame").map_or(0, |v| num(v) as i64);
    let fps = params.get("fps").map_or(30.0, num);
    (frame, EvalCtx::at_frame(frame, fps))
}

/// Everything the scene exports: helper objects that are polygons (not nulls), then the cube.
fn export(b: &Built, ctx: &EvalCtx, scene: &Scene) -> (Mesh, usize, usize) {
    let mut parts = Vec::new();
    for h in &b.helpers {
        if scene.node(*h).unwrap().type_id == "forge.null" {
            continue;
        }
        parts.push(world_mesh(scene, *h, ctx));
    }
    let hp: usize = parts.iter().map(|p| p.0.len()).sum();
    let hf: usize = parts.iter().map(|p| p.1.len()).sum();
    parts.push(world_mesh(scene, b.cube, ctx));
    (concat(&parts), hp, hf)
}

fn finite(m: &Mesh) -> bool {
    m.positions.iter().all(|p| p.iter().all(|c| c.is_finite()))
}

fn max_delta(a: &Mesh, b: &Mesh) -> f64 {
    a.positions
        .iter()
        .zip(&b.positions)
        .map(|(p, q)| (DVec3::from_array(*p) - DVec3::from_array(*q)).length())
        .fold(0.0, f64::max)
}

/// The invariants of spec §14 on the pinned cases. `orig`/`got` are the cube parts of the
/// undeformed and the deformed scene, `gold` the golden cube part.
fn check_invariants(key: &str, orig: &Mesh, got: &Mesh, gold: &Mesh) {
    assert_eq!(orig.faces, got.faces, "{key}: topology");
    assert!(finite(got), "{key}: finite");
    let d = max_delta(orig, got);
    match key {
        "mesh-deformer/cage" => {
            // Cage point 0 moved by (-100, 0, 0): no point moves along +x or beyond 100.
            assert!(d > 1.0 && d <= 100.0 + 1e-6, "{key}: max move {d}");
            for (a, b) in orig.positions.iter().zip(&got.positions) {
                assert!(b[0] - a[0] <= 1e-6, "{key}: moved towards +x");
                assert!((b[1] - a[1]).abs() < 1e-6 || a[2].abs() <= 100.0, "{key}");
            }
        }
        "surface/defaults" => {
            assert!(d > 1.0, "{key}: nothing moved");
        }
        "camera-deformer/grid" => {
            // Offsets are at most 50 at depth 1000, the cube is at depth 700-900.
            assert!(d <= 50.0 + 1e-6, "{key}: max move {d}");
        }
        "collision/sphere" => {
            let c = DVec3::new(150.0, 0.0, 0.0);
            // The golden has points inside the sphere (the reference did not run).
            let inside_gold = gold
                .positions
                .iter()
                .filter(|p| (DVec3::from_array(**p) - c).length() < 99.0)
                .count();
            assert!(inside_gold > 0, "{key}: golden has no point inside");
            for (a, b) in orig.positions.iter().zip(&got.positions) {
                let (a, b) = (DVec3::from_array(*a), DVec3::from_array(*b));
                // 16 segments: the polyhedron is inscribed: its faces are at 100 cos^2(pi/16) = 96.2 or more
                assert!((b - c).length() >= 95.0, "{key}: {b:?} inside");
                if (a - c).length() >= 100.0 {
                    assert_eq!(a, b, "{key}: an outside point moved");
                }
            }
            assert!(d > 1.0, "{key}: nothing moved");
        }
        k if k.starts_with("jiggle/") => {
            // The object moves in +x: the points lag behind (or overshoot) but stay near.
            assert!(d < 400.0, "{key}: max move {d}");
        }
        other => panic!("no invariants for {other}"),
    }
}

#[test]
fn helper_deformers_match_the_reference() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    let reg = Arc::new(reg);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut exact, mut pinned) = (Vec::new(), Vec::new());
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        assert!(!UNSUPPORTED.contains(&key.as_str()));
        let m = case.manifest().unwrap();
        let params: Table = m.params.clone().into_iter().collect();
        let (frame, ctx) = ctx_of(&params);
        let expected = case.expected_mesh().unwrap().unwrap();

        let eval = |deformed: bool| {
            let b = build(&reg, &case, &params, deformed);
            // Animated parameters are read at the frame (the callers' job: CORE/ANIM).
            let scene = forge_anim::scene_at(b.doc.scene(), ctx.time);
            let e = export(&b, &ctx, &scene);
            (b, e)
        };
        let (_, (ours, hp, hf)) = eval(true);
        let (_, (orig, ohp, ohf)) = eval(false);
        assert_eq!((hp, hf), (ohp, ohf));

        let r = compare_meshes(&export_weld(&expected), &export_weld(&ours), &m.tolerance);
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            assert!(
                !r.passed(),
                "{key} matches exactly now: remove it from KNOWN_MISMATCH"
            );
            // (a) the leaked helper objects equal ours
            let split = |m: &Mesh| {
                (
                    Mesh {
                        positions: m.positions[..hp].to_vec(),
                        faces: m.faces[..hf].to_vec(),
                    },
                    Mesh {
                        positions: m.positions[hp..].to_vec(),
                        faces: m.faces[hf..]
                            .iter()
                            .map(|f| f.iter().map(|i| i - hp as u32).collect())
                            .collect(),
                    },
                )
            };
            let (gold_h, gold_cube) = split(&expected);
            let (ours_h, ours_cube) = split(&ours);
            let (_, orig_cube) = split(&orig);
            if hp > 0 {
                let r = compare_meshes(&export_weld(&gold_h), &export_weld(&ours_h), &m.tolerance);
                assert!(r.passed(), "{key}: helper objects: {:?}", r.failures);
            }
            // (b) the golden cube is the undeformed one (jiggle: the rigidly moved one)
            let r = compare_meshes(
                &export_weld(&gold_cube),
                &export_weld(&orig_cube),
                &m.tolerance,
            );
            assert!(
                r.passed(),
                "{key}: golden cube is not the undeformed cube: {:?}",
                r.failures
            );
            // (c) invariants of ours
            check_invariants(&key, &orig_cube, &ours_cube, &gold_cube);
            if key.starts_with("jiggle/") {
                jiggle_invariants(&reg, &case, &params, frame);
            }
            pinned.push(key);
            continue;
        }
        assert!(
            r.passed(),
            "{key}: {:?} (max error {:?})",
            r.failures,
            r.max_position_error
        );
        if key == "camera-deformer/grid" {
            camera_aimed_at_the_cube(&reg);
        }
        exact.push(key);
    }
    println!("exact: {exact:?}; pinned: {pinned:?}");
    assert_eq!(exact.len(), 3, "{exact:?}");
    assert_eq!(pinned.len(), 5, "{pinned:?}");
}

/// `camera-deformer/grid` matches because the recorded camera pose (rotated 45 deg) has the
/// cube outside the grid, so it does not exercise the grid. The same grid point with the
/// camera aimed at the cube (no rotation) moves the cube's points by at most the offset.
fn camera_aimed_at_the_cube(reg: &Arc<Registry>) {
    let mut doc = Document::new(reg.clone());
    let (cube, d) = doc
        .transact("camera", |tx| {
            let cube = tx.create("forge.cube", None, None)?;
            for (k, n) in [("segments_x", 4), ("segments_y", 8), ("segments_z", 4)] {
                tx.set_param(cube, k, Value::Int(n))?;
            }
            let cam = tx.create("forge.null", None, None)?;
            tx.set_param(cam, "transform.pos", Value::Vec3([0.0, 0.0, -800.0]))?;
            let d = tx.create("forge.camera_deformer", Some(cube), None)?;
            tx.set_param(d, "camera", Value::Link(Some(cam)))?;
            let mut offsets = vec![[0.0; 3]; 33 * 33];
            offsets[15 * 33 + 15] = [50.0, 50.0, 0.0];
            tx.set_param(d, "offsets", Value::Vec3List(offsets))?;
            Ok((cube, d))
        })
        .unwrap();
    let on = evaluate(doc.scene(), cube, &EvalCtx::default())
        .unwrap()
        .unwrap();
    doc.transact("off", |tx| tx.set_param(d, "enabled", Value::Bool(false)))
        .unwrap();
    let off = evaluate(doc.scene(), cube, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let moved = on
        .points
        .iter()
        .zip(&off.points)
        .map(|(a, b)| (*a - *b).length())
        .fold(0.0, f64::max);
    assert!(
        moved > 1.0 && moved <= 50.0 + 1e-6,
        "aimed camera: max move {moved}"
    );
}

/// The jiggle cases: frame 0 is rigid, the result is repeatable, and the cube lags at 10.
fn jiggle_invariants(reg: &Arc<Registry>, case: &GoldenCase, params: &Table, frame: i64) {
    let fps = num(&params["fps"]);
    let at = |f: i64| {
        let b = build(reg, case, params, true);
        let ctx = EvalCtx::at_frame(f, fps);
        let scene = forge_anim::scene_at(b.doc.scene(), ctx.time);
        let m = evaluate(&scene, b.cube, &ctx).unwrap().unwrap();
        let rigid = {
            let b = build(reg, case, params, false);
            let scene = forge_anim::scene_at(b.doc.scene(), ctx.time);
            evaluate(&scene, b.cube, &ctx).unwrap().unwrap()
        };
        (pts(&m), pts(&rigid))
    };
    let (zero, zero_rigid) = at(0);
    assert_eq!(zero, zero_rigid, "frame 0 is rigid");
    let (a, rigid) = at(frame);
    let (b, _) = at(frame);
    assert_eq!(a, b, "repeatable");
    let lag = a
        .iter()
        .zip(&rigid)
        .map(|(p, r)| (*p - *r).length())
        .fold(0.0, f64::max);
    assert!(
        lag > 1.0,
        "jiggle {frame}: the cube lags the rigid motion ({lag})"
    );
}
