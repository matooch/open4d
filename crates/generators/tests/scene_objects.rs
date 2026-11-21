//! Scene objects (docs/specs/SCENE-OBJECTS.md, brief 047): descriptor defaults against the
//! observation O29 tables, and the node readers.

use std::collections::BTreeMap;

use forge_core::{Document, ParamKind, Registry, Unit, Value};
use forge_generators::Primitives;
use forge_generators::scene_objects::*;

const OBSERVATION: &str = include_str!("../../../docs/parity/scene-defaults.md");

/// Rows of one O29 table: (our key, default text, options text).
fn table(start: &str, end: &str) -> Vec<(String, String, String)> {
    let from = OBSERVATION.find(start).expect("section start");
    let to = from + OBSERVATION[from..].find(end).expect("section end");
    OBSERVATION[from..to]
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| Tab") && !l.starts_with("|---"))
        .filter_map(|l| {
            let c: Vec<&str> = l
                .split(" | ")
                .map(|s| s.trim().trim_matches('|').trim())
                .collect();
            (c.len() >= 7).then(|| (c[2].to_string(), c[3].to_string(), c[6].to_string()))
        })
        .collect()
}

fn registry() -> std::sync::Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    std::sync::Arc::new(r)
}

/// O29 key -> our param id where they differ.
fn alias(key: &str) -> &str {
    match key {
        "sensor_size_film_gate" => "sensor_size",
        "field_of_view_horizontal" => "fov_horizontal",
        "f_stop_f" => "f_stop",
        "shutter_speed_s" => "shutter_speed",
        "angle" => "diaphragm_angle",
        "falloff_radius_decay" => "falloff_radius",
        "film_offset_x" | "film_offset_y" => key,
        other => other,
    }
}

fn check_defaults(type_id: &str, rows: &[(String, String, String)], skip: &[&str]) -> usize {
    let reg = registry();
    let ty = reg.get(type_id).unwrap().clone();
    let mut checked = 0;
    for (key, default, _) in rows {
        let id = alias(key);
        if skip.contains(&key.as_str()) {
            continue;
        }
        let Some(desc) = ty.find_param(id) else {
            continue;
        };
        let to_unit = |v: f64| match desc.ui.unit {
            Unit::Angle => v.to_radians(),
            Unit::Percent => v / 100.0,
            _ => v,
        };
        match (&desc.kind, &desc.default) {
            (ParamKind::Bool, Value::Bool(b)) => assert_eq!(default == "1", *b, "{type_id}.{id}"),
            (ParamKind::Float { .. }, Value::Float(f)) => {
                let want = to_unit(default.parse::<f64>().unwrap());
                assert!(
                    (want - f).abs() < 1e-9 * want.abs().max(1.0),
                    "{type_id}.{id}: {want} vs {f}"
                );
            }
            (ParamKind::Int { .. }, Value::Int(i)) => {
                assert_eq!(default.parse::<i64>().unwrap(), *i, "{type_id}.{id}");
            }
            (ParamKind::Enum { options }, Value::Enum(e)) => {
                let label = &options.iter().find(|o| &o.id == e).unwrap().label;
                assert_eq!(label, default, "{type_id}.{id}");
            }
            (ParamKind::Color, Value::Color(c)) => {
                let want: Vec<f64> = default
                    .trim_matches(|ch| ch == '[' || ch == ']')
                    .split(", ")
                    .map(|x| x.parse().unwrap())
                    .collect();
                for k in 0..3 {
                    assert!((want[k] - c[k]).abs() < 1e-9, "{type_id}.{id}");
                }
            }
            (ParamKind::Link { .. }, Value::Link(None)) => assert_eq!(default, "none"),
            _ => panic!("{type_id}.{id}: unexpected kind"),
        }
        checked += 1;
    }
    checked
}

#[test]
fn camera_defaults_equal_o29() {
    let rows = table("## 2. Camera object", "## 3. Light object");
    // FOV vertical is a readback; Zoom has no effect on a perspective camera (O57) but is a param.
    let n = check_defaults(CAMERA, &rows, &["field_of_view_vertical"]);
    assert!(n >= 36, "checked only {n}");
    // every Object, Physical and Details parameter is a row of the table
    let reg = registry();
    let ty = reg.get(CAMERA).unwrap().clone();
    for p in &ty.params {
        if p.ui.group.is_empty() || (p.id.starts_with("transform.") || p.id.starts_with("object."))
        {
            continue;
        }
        assert!(
            rows.iter().any(|(k, _, _)| alias(k) == p.id),
            "camera.{} has no O29 row",
            p.id
        );
    }
}

#[test]
fn light_defaults_equal_o29_for_every_type() {
    let reg = registry();
    let ty = reg.get(LIGHT).unwrap().clone();
    let opts: Vec<String> = match &ty.find_param("type").unwrap().kind {
        ParamKind::Enum { options } => options.iter().map(|o| o.label.clone()).collect(),
        _ => unreachable!(),
    };
    let names = [
        "Omni",
        "Spot",
        "Infinite",
        "Area",
        "Square Spot",
        "Parallel",
        "Parallel Spot",
        "Square Parallel Spot",
        "IES",
    ];
    assert_eq!(opts, names);
    for (i, name) in names.iter().enumerate() {
        let start = format!("### Type: {name}\n");
        let end = names
            .get(i + 1)
            .map_or("## 4. Standard material".to_string(), |n| {
                format!("### Type: {n}\n")
            });
        let rows = table(&start, &end);
        // the Type row shows the type of the table; every other default is the same
        let n = check_defaults(
            LIGHT,
            &rows,
            &["type", "shadow_2", "color_2", "intensity_2", "units"],
        );
        assert!(n >= 40, "{name}: checked only {n}");
    }
    let rows = table("### Type: Omni\n", "### Type: Spot\n");
    for p in &ty.params {
        if p.ui.group.is_empty() || (p.id.starts_with("transform.") || p.id.starts_with("object."))
        {
            continue;
        }
        assert!(
            rows.iter().any(|(k, _, _)| alias(k) == p.id),
            "light.{} has no O29 row",
            p.id
        );
    }
}

#[test]
fn fov_readbacks_match_o29() {
    // O29: horizontal 53.13010235415599 deg -> vertical 31.41727565803149 at the 16:9 film aspect
    let h = 53.130_102_354_155_99_f64.to_radians();
    let v = vertical_fov(h, DEFAULT_FILM_ASPECT).to_degrees();
    assert!((v - 31.417_275_658_031_49).abs() < 1e-9, "{v}");
    assert!((focal_from_fov(h, 36.0) - 36.0).abs() < 1e-9);
    // O57: 1.570796 rad reads 18 mm, 0.356186 rad reads 100 mm on the 36 mm sensor
    assert!((focal_from_fov(std::f64::consts::FRAC_PI_2, 36.0) - 18.0).abs() < 1e-6);
    assert!((focal_from_fov(0.356_185_6, 36.0) - 100.0).abs() < 1e-3);
}

fn scene_with(types: &[&str]) -> (Document, Vec<forge_core::NodeId>) {
    let mut doc = Document::new(registry());
    let mut ids = Vec::new();
    doc.transact("create", |tx| {
        for t in types {
            ids.push(tx.create(t, None, None)?);
        }
        Ok(())
    })
    .unwrap();
    (doc, ids)
}

#[test]
fn new_nodes_read_back_with_defaults_and_scene_objects_lists_them() {
    let (doc, ids) = scene_with(&TYPES);
    let objs = scene_objects(doc.scene());
    assert_eq!(objs.cameras.len(), 1);
    assert_eq!(objs.lights.len(), 1);
    assert_eq!(objs.environments.len(), 1);
    assert_eq!(objs.floors.len(), 1);
    assert_eq!(objs.skies.len(), 1);
    let c = &objs.cameras[0];
    assert_eq!(c.node, ids[0]);
    let Lens::Perspective {
        focal_mm,
        sensor_mm,
        film_offset,
    } = c.lens
    else {
        panic!()
    };
    assert!((focal_mm - 36.0).abs() < 1e-9 && sensor_mm == 36.0 && film_offset == [0.0, 0.0]);
    assert_eq!(c.near_clip, Some(0.0));
    assert_eq!(c.far_clip, None);
    let l = &objs.lights[0];
    assert_eq!(
        (l.shape, l.intensity, l.falloff, l.shadow),
        (LightShape::Omni, 1.0, FalloffKind::None, ShadowKind::None)
    );
    assert!(l.notes.is_empty());
    assert_eq!(objs.floors[0].color, [0.8; 3]);
}

#[test]
fn light_types_map_to_shapes_and_notes() {
    let (mut doc, ids) = scene_with(&[LIGHT]);
    let cases = [
        ("spot", LightShape::Spot, 0),
        ("square_spot", LightShape::Spot, 1),
        ("infinite", LightShape::Infinite, 0),
        ("parallel_spot", LightShape::Infinite, 1),
        ("area", LightShape::Area, 0),
        ("ies", LightShape::Omni, 1),
    ];
    for (t, shape, notes) in cases {
        doc.transact("type", |tx| {
            tx.set_param(ids[0], "type", Value::Enum(t.into()))
        })
        .unwrap();
        let l = light_data(doc.scene(), ids[0]).unwrap();
        assert_eq!((l.shape, l.notes.len()), (shape, notes), "{t}");
    }
}

#[test]
fn camera_world_matrix_is_rigid_and_follows_heading_sign() {
    let (mut doc, ids) = scene_with(&[CAMERA]);
    // heading -15 deg looks to the right (docs/specs/RENDER.md R10): +Z turns toward +X
    doc.transact("rot", |tx| {
        tx.set_param(
            ids[0],
            "transform.rot",
            Value::Vec3([(-15f64).to_radians(), 0.0, 0.0]),
        )?;
        tx.set_param(ids[0], "transform.pos", Value::Vec3([0.0, 0.0, -600.0]))?;
        tx.set_param(ids[0], "transform.scale", Value::Vec3([2.0, 2.0, 2.0]))
    })
    .unwrap();
    let c = camera_data(doc.scene(), ids[0]).unwrap();
    let fwd = c.world.transform_vector3(glam::DVec3::Z);
    assert!(fwd.x > 0.0 && (fwd.length() - 1.0).abs() < 1e-12, "{fwd:?}");
    assert_eq!(
        c.world.w_axis.truncate(),
        glam::DVec3::new(0.0, 0.0, -600.0)
    );
}

#[test]
fn ranges_clamp_like_o29() {
    let reg = registry();
    let ty = reg.get(CAMERA).unwrap().clone();
    let fov = ty.find_param("fov_horizontal").unwrap();
    let Value::Float(hi) = fov.validate(Value::Float(10.0)).unwrap() else {
        panic!()
    };
    assert!((hi.to_degrees() - 174.0).abs() < 1e-9);
    let _: BTreeMap<String, Value> = BTreeMap::new();
}
