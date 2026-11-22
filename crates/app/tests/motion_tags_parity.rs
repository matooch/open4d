//! Motion tags (brief 106): Target, Align to Spline, Vibrate, Look at Camera, Protection,
//! Display and Compositing against the O27/O54 golden cases.

use std::path::Path;

use forge_app::App;
use forge_core::{NodeId, Value};
use forge_io::export_scene;
use forge_io::obj::{WriteOptions, write};
use forge_parity_harness::{
    GoldenCase, Mesh, Samples, compare_meshes, compare_samples, discover_cases, parse_obj,
};
use serde_json::json;

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

fn animation2_cases() -> Vec<GoldenCase> {
    discover_cases(&root())
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "animation-2")
        .filter(|c| {
            matches!(
                c.name.as_str(),
                "target-tag" | "align-to-spline" | "align-to-spline-tangential" | "vibrate-tag"
            )
        })
        .collect()
}

fn tags2_cases() -> Vec<GoldenCase> {
    discover_cases(&root())
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "tags-2")
        .filter(|c| {
            matches!(
                c.name.as_str(),
                "look-at-camera" | "protection" | "display-tag" | "compositing"
            )
        })
        .collect()
}

fn value_at_frame(app: &App, frame: f64, id: NodeId, param: &str) -> f64 {
    let scene = app.doc.scene();
    let fps = f64::from(scene.settings().fps);
    let time = frame / fps;
    let (tracks, tags, enabled) = forge_app::schedule::animation_inputs(scene);
    let animated = forge_app::schedule::animate(scene, &tracks, &tags, enabled, time);
    let scene = animated.scene;
    let (full_param, comp) = scene_param(param);
    let v = scene
        .param(id, full_param)
        .unwrap_or_else(|| panic!("node {id:?} has no param {full_param:?}"));
    match v {
        Value::Float(f) => *f,
        Value::Vec3(a) => a[comp],
        _ => panic!("unexpected value for {param}"),
    }
}

fn set_transform(app: &mut App, id: NodeId, pos: [f64; 3]) {
    app.run(
        "transform.set",
        json!({"node": id.0, "pos": pos, "rot": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]}),
    )
    .unwrap();
}

fn key_float(app: &mut App, id: NodeId, track: &str, frame: i64, value: f64) {
    app.run(
        "key.set",
        json!({
            "node": id.0,
            "track": track,
            "frame": frame,
            "value": value,
        }),
    )
    .unwrap();
}

fn set_timeline(app: &mut App, fps: u32, start: i64, end: i64) {
    app.run(
        "timeline.settings",
        json!({"fps": fps, "start": start, "end": end}),
    )
    .unwrap();
}

fn scene_param(param: &str) -> (&str, usize) {
    match param {
        "rot_h" => ("transform.rot", 0),
        "rot_p" => ("transform.rot", 1),
        "rot_b" => ("transform.rot", 2),
        "pos_x" => ("transform.pos", 0),
        "pos_y" => ("transform.pos", 1),
        "pos_z" => ("transform.pos", 2),
        _ => (param, 0),
    }
}

fn normalize_degrees(v: f64) -> f64 {
    if v < 0.0 { v + 360.0 } else { v }
}

fn eval_track(app: &App, id: NodeId, param: &str, expected: &Samples) -> Samples {
    let Samples::Track {
        fps,
        tolerance,
        samples,
        ..
    } = expected
    else {
        panic!("track samples expected");
    };
    let mut out = Vec::new();
    for (frame, _) in samples {
        let v = value_at_frame(app, *frame, id, param);
        let v = if param == "rot_h" {
            normalize_degrees(v.to_degrees())
        } else if param.starts_with("rot_") {
            v.to_degrees()
        } else {
            v
        };
        out.push((*frame, v));
    }
    Samples::Track {
        param: param.to_string(),
        fps: *fps,
        tolerance: *tolerance,
        samples: out,
    }
}

fn track_param(expected: &Samples) -> &str {
    match expected {
        Samples::Track { param, .. } => param,
        _ => panic!("track samples expected"),
    }
}

fn golden_mesh(case: &GoldenCase) -> Mesh {
    let src = std::fs::read_to_string(case.dir.join("mesh.obj")).unwrap();
    parse_obj(&src).unwrap()
}

fn exported(app: &App) -> Mesh {
    let objects = export_scene(&app.evaluated(), None);
    let opts = WriteOptions {
        normals: false,
        weld: false,
    };
    parse_obj(&write(&objects, &opts)).expect("our own file parses")
}

fn make_cube(app: &mut App, pos: [f64; 3]) -> NodeId {
    let r = app
        .run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let id = NodeId(r["node"].as_u64().unwrap());
    set_transform(app, id, pos);
    id
}

fn make_null(app: &mut App, pos: [f64; 3]) -> NodeId {
    let r = app
        .run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let id = NodeId(r["node"].as_u64().unwrap());
    set_transform(app, id, pos);
    id
}

fn make_camera(app: &mut App, pos: [f64; 3]) -> NodeId {
    let r = app
        .run("object.create", json!({"type": "forge.camera"}))
        .unwrap();
    let id = NodeId(r["node"].as_u64().unwrap());
    set_transform(app, id, pos);
    id
}

fn make_circle(app: &mut App) -> NodeId {
    let r = app
        .run("object.create", json!({"type": "forge.circle_spline"}))
        .unwrap();
    NodeId(r["node"].as_u64().unwrap())
}

fn make_polygon_cube(app: &mut App) -> NodeId {
    let cube = make_cube(app, [0.0; 3]);
    for a in ["segments_x", "segments_y", "segments_z"] {
        app.run("param.set", json!({"node": cube.0, "param": a, "value": 2}))
            .unwrap();
    }
    let r = app
        .run("mesh.make_editable", json!({"nodes": [cube.0]}))
        .unwrap();
    NodeId(r["made"].as_array().unwrap()[0].as_u64().unwrap())
}

fn attach_tag(app: &mut App, type_id: &str) -> NodeId {
    let r = app.run("object.create", json!({"type": type_id})).unwrap();
    NodeId(r["node"].as_u64().unwrap())
}

fn param_float(v: &toml::Value) -> f64 {
    v.as_float()
        .or_else(|| v.as_integer().map(|i| i as f64))
        .unwrap()
}

fn param_bool(v: &toml::Value) -> bool {
    v.as_bool()
        .unwrap_or_else(|| v.as_integer().map(|i| i != 0).unwrap())
}

#[test]
fn target_tag_matches_reference() {
    let case = animation2_cases()
        .into_iter()
        .find(|c| c.name == "target-tag")
        .unwrap();
    let m = case.manifest().unwrap();
    let fps = m.params["document_fps"].as_integer().unwrap() as u32;
    let mut app = App::new_default();
    set_timeline(&mut app, fps, 0, 90);

    let cube = make_cube(&mut app, [0.0; 3]);
    let tag = attach_tag(&mut app, "forge.target_tag");

    let target = make_null(&mut app, [0.0; 3]);
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "target", "value": target.0}),
    )
    .unwrap();

    let keys = m.params["target_keys_pos"].as_array().unwrap();
    for k in keys {
        let frame = k.as_array().unwrap()[0].as_integer().unwrap();
        let pos = k.as_array().unwrap()[1].as_array().unwrap();
        for (i, comp) in ["transform.pos.x", "transform.pos.y", "transform.pos.z"]
            .iter()
            .enumerate()
        {
            key_float(&mut app, target, comp, frame, param_float(&pos[i]));
        }
    }

    let mut mismatched = Vec::new();
    for (file, expected) in case.samples().unwrap() {
        let param = track_param(&expected);
        let actual = eval_track(&app, cube, param, &expected);
        let r = compare_samples(&expected, &actual);
        println!("target-tag/{file}: max error {:.3e}", r.max_error);
        if !r.passed() {
            mismatched.push((file, r));
        }
    }
    assert!(mismatched.is_empty(), "{mismatched:?}");
}

#[test]
fn align_to_spline_matches_reference() {
    check_align("align-to-spline");
}

#[test]
fn align_to_spline_tangential_matches_reference() {
    check_align("align-to-spline-tangential");
}

#[test]
fn align_to_spline_writes_parent_relative_position() {
    let mut app = App::new_default();
    let parent = make_null(&mut app, [10.0, 20.0, 30.0]);
    let cube = make_cube(&mut app, [0.0; 3]);
    let tag = attach_tag(&mut app, "forge.align_to_spline_tag");
    let circle = make_circle(&mut app);
    app.run(
        "object.reparent",
        json!({"node": cube.0, "parent": parent.0}),
    )
    .unwrap();
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "spline", "value": circle.0}),
    )
    .unwrap();
    let world = app.evaluated().world_matrix(cube).unwrap();
    assert_eq!(world.w_axis.truncate().to_array(), [200.0, 0.0, 0.0]);
}

fn check_align(name: &str) {
    let case = animation2_cases()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap();
    let m = case.manifest().unwrap();
    let fps = m.params["document_fps"].as_integer().unwrap() as u32;
    let mut app = App::new_default();
    set_timeline(&mut app, fps, 0, 90);

    let cube = make_cube(&mut app, [0.0; 3]);
    let tag = attach_tag(&mut app, "forge.align_to_spline_tag");
    let circle = make_circle(&mut app);
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "spline", "value": circle.0}),
    )
    .unwrap();
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "tangential", "value": param_bool(&m.params["tangential"])}),
    )
    .unwrap();

    for key in m.params["position_keys"].as_array().unwrap() {
        let key = key.as_array().unwrap();
        let frame = key[0].as_integer().unwrap();
        key_float(&mut app, tag, "position", frame, param_float(&key[1]));
        app.run(
            "key.set_tangent",
            json!({"node": tag.0, "track": "position", "frame": frame, "preset": "zero_length"}),
        )
        .unwrap();
    }

    let mut mismatched = Vec::new();
    for (file, expected) in case.samples().unwrap() {
        let param = track_param(&expected);
        let actual = eval_track(&app, cube, param, &expected);
        let r = compare_samples(&expected, &actual);
        println!("{name}/{file}: max error {:.3e}", r.max_error);
        if !r.passed() {
            mismatched.push((file, r));
        }
    }
    assert!(mismatched.is_empty(), "{mismatched:?}");
}

#[test]
fn vibrate_tag_is_statistical() {
    let case = animation2_cases()
        .into_iter()
        .find(|c| c.name == "vibrate-tag")
        .unwrap();
    let m = case.manifest().unwrap();
    let fps = m.params["document_fps"].as_integer().unwrap() as u32;
    let mut app = App::new_default();
    set_timeline(&mut app, fps, 0, 90);

    let cube = make_cube(&mut app, [0.0; 3]);
    let tag = attach_tag(&mut app, "forge.vibrate_tag");
    let p = &m.params;
    app.run(
        "param.set",
        json!({
            "node": tag.0,
            "param": "enable_position",
            "value": param_bool(&p["enable_position"]),
        }),
    )
    .unwrap();
    app.run(
        "param.set",
        json!({
            "node": tag.0,
            "param": "position_amplitude",
            "value": p["position_amplitude"].as_array().unwrap().iter().map(param_float).collect::<Vec<_>>(),
        }),
    )
    .unwrap();
    app.run(
        "param.set",
        json!({
            "node": tag.0,
            "param": "position_frequency",
            "value": param_float(&p["position_frequency"]),
        }),
    )
    .unwrap();
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "seed", "value": param_float(&p["seed"])}),
    )
    .unwrap();
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "regular_pulse", "value": param_bool(&p["regular_pulse"])}),
    )
    .unwrap();
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "relative", "value": param_bool(&p["relative"])}),
    )
    .unwrap();

    let amp = param_float(&p["position_amplitude"].as_array().unwrap()[0]);
    for (_file, expected) in case.samples().unwrap() {
        let Samples::Track {
            ref param,
            ref samples,
            ..
        } = expected
        else {
            continue;
        };
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for (frame, _) in samples {
            let v = value_at_frame(&app, *frame, cube, param);
            assert_eq!(v, value_at_frame(&app, *frame, cube, param));
            assert!(
                v.abs() <= amp,
                "vibrate offset {v} exceeds amplitude {amp} at frame {frame}"
            );
            lo = lo.min(v);
            hi = hi.max(v);
        }
        let observed_lo = samples
            .iter()
            .map(|(_, v)| *v)
            .fold(f64::INFINITY, f64::min);
        let observed_hi = samples
            .iter()
            .map(|(_, v)| *v)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            hi - lo >= 0.1 * (observed_hi - observed_lo),
            "{param} has too little motion: {lo}..{hi}"
        );
    }
}

#[test]
fn vibrate_pulse_can_offset_existing_position() {
    let mut app = App::new_default();
    let cube = make_cube(&mut app, [200.0, 0.0, 0.0]);
    let tag = attach_tag(&mut app, "forge.vibrate_tag");
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "regular_pulse", "value": true}),
    )
    .unwrap();
    // The default amplitude is 50 and the 2 Hz pulse peaks at 1/8 second.
    let frame = f64::from(app.doc.scene().settings().fps) / 8.0;
    assert_eq!(value_at_frame(&app, frame, cube, "pos_x"), 250.0);
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "relative", "value": false}),
    )
    .unwrap();
    assert_eq!(value_at_frame(&app, frame, cube, "pos_x"), 50.0);
}

#[test]
fn look_at_camera_matches_reference() {
    let case = tags2_cases()
        .into_iter()
        .find(|c| c.name == "look-at-camera")
        .unwrap();
    let m = case.manifest().unwrap();
    let mut app = App::new_default();

    let p = &m.params["object"];
    let _cube = make_cube(
        &mut app,
        [
            param_float(&p["position"].as_array().unwrap()[0]),
            param_float(&p["position"].as_array().unwrap()[1]),
            param_float(&p["position"].as_array().unwrap()[2]),
        ],
    );
    attach_tag(&mut app, "forge.look_at_camera_tag");

    let p2 = &m.params["object_2"];
    make_camera(
        &mut app,
        [
            param_float(&p2["position"].as_array().unwrap()[0]),
            param_float(&p2["position"].as_array().unwrap()[1]),
            param_float(&p2["position"].as_array().unwrap()[2]),
        ],
    );

    let golden = golden_mesh(&case);
    let ours = exported(&app);
    let r = compare_meshes(&golden, &ours, &m.tolerance);
    assert!(r.passed(), "{:?}", r.failures);
}

#[test]
fn protection_leaves_unlocked_axes_editable() {
    let mut app = App::new_default();
    let cube = make_cube(&mut app, [1.0, 2.0, 3.0]);
    let tag = attach_tag(&mut app, "forge.protection_tag");
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "pos_y", "value": false}),
    )
    .unwrap();
    app.run(
        "transform.set",
        json!({"node": cube.0, "pos": [4.0, 5.0, 6.0]}),
    )
    .unwrap();
    assert_eq!(
        app.doc.scene().param(cube, "transform.pos"),
        Some(&Value::Vec3([1.0, 5.0, 3.0]))
    );
}

#[test]
fn protection_tag_blocks_position_writes() {
    let case = tags2_cases()
        .into_iter()
        .find(|c| c.name == "protection")
        .unwrap();
    let m = case.manifest().unwrap();
    let mut app = App::new_default();

    let poly = make_polygon_cube(&mut app);
    attach_tag(&mut app, "forge.protection_tag");

    app.run(
        "transform.set",
        json!({"node": poly.0, "pos": [100.0, 0.0, 0.0]}),
    )
    .unwrap();

    let scene = app.evaluated();
    let pos = match scene.param(poly, "transform.pos").unwrap() {
        Value::Vec3(v) => *v,
        _ => panic!("transform.pos is not a vec3"),
    };
    assert_eq!(pos, [0.0; 3]);

    let golden = golden_mesh(&case);
    let ours = exported(&app);
    let r = compare_meshes(&golden, &ours, &m.tolerance);
    assert!(r.passed(), "{:?}", r.failures);
}

#[test]
fn display_tag_does_not_change_export() {
    let case = tags2_cases()
        .into_iter()
        .find(|c| c.name == "display-tag")
        .unwrap();
    let m = case.manifest().unwrap();
    let mut app = App::new_default();

    let _poly = make_polygon_cube(&mut app);
    let tag = attach_tag(&mut app, "forge.display_tag");
    let v = m.params["display"]["visibility"].as_float().unwrap();
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "visibility", "value": v}),
    )
    .unwrap();
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "use_visibility", "value": true}),
    )
    .unwrap();

    let golden = golden_mesh(&case);
    let ours = exported(&app);
    let r = compare_meshes(&golden, &ours, &m.tolerance);
    assert!(r.passed(), "{:?}", r.failures);
}

#[test]
fn compositing_tag_does_not_change_export() {
    let case = tags2_cases()
        .into_iter()
        .find(|c| c.name == "compositing")
        .unwrap();
    let m = case.manifest().unwrap();
    let mut app = App::new_default();

    let _poly = make_polygon_cube(&mut app);
    let tag = attach_tag(&mut app, "forge.compositing_tag");
    let seen = param_bool(&m.params["compositing"]["seen_by_camera"]);
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "seen_by_camera", "value": seen}),
    )
    .unwrap();

    let golden = golden_mesh(&case);
    let ours = exported(&app);
    let r = compare_meshes(&golden, &ours, &m.tolerance);
    assert!(r.passed(), "{:?}", r.failures);
}
