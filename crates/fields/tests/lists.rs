//! Field lists (docs/specs/FIELDS.md F7): rows with their own storage, blend modes, groups,
//! the Formula and Shader fields beyond the golden cases.

use std::sync::Arc;

use forge_core::{Document, EvalCtx, LinkRow, NodeId, Registry, Tx, Value, transform};
use forge_fields::{BLEND_OPTIONS, BlendMode, FieldError, FieldList, FieldToolset};
use glam::DVec3;

fn doc() -> Document {
    let mut reg = Registry::with_builtins();
    reg.install(&FieldToolset).unwrap();
    Document::new(Arc::new(reg))
}

fn sphere(tx: &mut Tx, x: f64, radius: f64) -> NodeId {
    let n = tx.create(forge_fields::SPHERICAL, None, None).unwrap();
    tx.set_param(n, "radius", Value::Float(radius)).unwrap();
    tx.set_param(n, transform::POS, Value::Vec3([x, 0.0, 0.0]))
        .unwrap();
    n
}

fn at(list: &FieldList, p: DVec3) -> f64 {
    list.sample(p, &EvalCtx::default()).value
}

fn row(t: NodeId, blend: &str, strength: f64) -> LinkRow {
    LinkRow {
        strength,
        ..LinkRow::new(t, blend)
    }
}

#[test]
fn every_blend_mode_has_an_option_and_round_trips() {
    assert_eq!(BLEND_OPTIONS.len(), 9);
    for (id, _) in BLEND_OPTIONS {
        assert_eq!(BlendMode::from_id(id).unwrap().id(), id);
    }
    assert!(BlendMode::from_id("dissolve").is_none());
}

#[test]
fn blend_formulas() {
    let (a, b) = (0.3, 0.8);
    assert_eq!(BlendMode::Normal.apply(a, b), b);
    assert_eq!(BlendMode::Max.apply(a, b), b);
    assert_eq!(BlendMode::Min.apply(a, b), a);
    assert_eq!(BlendMode::Clip.apply(a, b), a);
    assert_eq!(BlendMode::Add.apply(a, b), 1.0);
    assert_eq!(BlendMode::Subtract.apply(a, b), 0.0);
    assert!((BlendMode::Multiply.apply(a, b) - 0.24).abs() < 1e-12);
    assert!((BlendMode::Screen.apply(a, b) - 0.86).abs() < 1e-12);
    // Overlay depends on the value below: dark base multiplies, light base screens
    assert!((BlendMode::Overlay.apply(0.3, 0.8) - 0.48).abs() < 1e-12);
    assert!((BlendMode::Overlay.apply(0.8, 0.3) - 0.72).abs() < 1e-12);
    // strength mixes between the base and the blended value
    assert!((BlendMode::Add.mix(0.5, 0.25, 0.5) - 0.625).abs() < 1e-12);
}

#[test]
fn rows_keep_their_own_strength_for_the_same_field() {
    // One field in two lists: each list has its own strength (the row, not the node).
    let mut d = doc();
    let (f, g1, g2) = d
        .transact("scene", |tx| {
            let f = sphere(tx, 0.0, 100.0);
            let g1 = tx.create(forge_fields::GROUP, None, None)?;
            let g2 = tx.create(forge_fields::GROUP, None, None)?;
            tx.set_param(
                g1,
                forge_fields::FIELD_LIST,
                Value::LinkRows(vec![row(f, "normal", 0.5)]),
            )?;
            tx.set_param(
                g2,
                forge_fields::FIELD_LIST,
                Value::LinkRows(vec![row(f, "normal", 0.25)]),
            )?;
            Ok((f, g1, g2))
        })
        .unwrap();
    let p = DVec3::new(25.0, 0.0, 0.0); // sphere value 0.75 / 0.5 -> 1.0 (inner offset 50 %)
    let full = at(&FieldList::from_nodes(d.scene(), &[f]).unwrap(), p);
    for (g, k) in [(g1, 0.5), (g2, 0.25)] {
        let l = FieldList::from_param(d.scene(), g, forge_fields::FIELD_LIST).unwrap();
        assert!((at(&l, p) - full * k).abs() < 1e-12);
    }
}

#[test]
fn off_rows_and_non_field_rows_are_left_out() {
    let mut d = doc();
    let (holder, a, b) = d
        .transact("scene", |tx| {
            let a = sphere(tx, 0.0, 100.0);
            let b = sphere(tx, 0.0, 200.0);
            let holder = tx.create(forge_fields::GROUP, None, None)?;
            let off = LinkRow {
                enabled: false,
                ..row(b, "normal", 1.0)
            };
            tx.set_param(
                holder,
                forge_fields::FIELD_LIST,
                Value::LinkRows(vec![off, row(a, "normal", 1.0)]),
            )?;
            Ok((holder, a, b))
        })
        .unwrap();
    let l = FieldList::from_param(d.scene(), holder, forge_fields::FIELD_LIST).unwrap();
    assert_eq!(l.layers.len(), 1);
    let only_a = FieldList::from_nodes(d.scene(), &[a]).unwrap();
    let p = DVec3::new(40.0, 0.0, 0.0);
    assert_eq!(at(&l, p), at(&only_a, p));
    // the node itself is untouched by the row's enable box
    assert_eq!(
        d.scene().param(b, "object.enabled"),
        Some(&Value::Bool(true))
    );
    // a row of a node that is not a field is skipped
    let cube = d
        .transact("cube", |tx| tx.create("forge.null", None, None))
        .unwrap();
    let rows = [row(cube, "normal", 1.0)];
    assert!(FieldList::from_rows(d.scene(), &rows).unwrap().is_empty());
}

#[test]
fn rows_validate_and_bare_links_become_default_rows() {
    let mut d = doc();
    let (holder, f) = d
        .transact("scene", |tx| {
            let f = sphere(tx, 0.0, 100.0);
            Ok((tx.create(forge_fields::GROUP, None, None)?, f))
        })
        .unwrap();
    // an unknown blend mode, a repeated node and a non-field target are refused
    for bad in [
        vec![row(f, "dissolve", 1.0)],
        vec![row(f, "add", 1.0), row(f, "add", 1.0)],
    ] {
        let r = d.transact("bad", |tx| {
            tx.set_param(
                holder,
                forge_fields::FIELD_LIST,
                Value::LinkRows(bad.clone()),
            )
        });
        assert!(r.is_err());
    }
    let cube = d
        .transact("cube", |tx| tx.create("forge.null", None, None))
        .unwrap();
    assert!(
        d.transact("cube row", |tx| tx.set_param(
            holder,
            forge_fields::FIELD_LIST,
            Value::LinkRows(vec![row(cube, "normal", 1.0)])
        ))
        .is_err()
    );
    // a bare list of links (older callers) turns into rows at 100 %, Normal
    d.transact("links", |tx| {
        tx.set_param(holder, forge_fields::FIELD_LIST, Value::LinkList(vec![f]))
    })
    .unwrap();
    assert_eq!(
        d.scene().param(holder, forge_fields::FIELD_LIST),
        Some(&Value::LinkRows(vec![LinkRow::new(f, "normal")]))
    );
}

#[test]
fn a_group_inside_a_group_works_and_a_cycle_is_refused() {
    let mut d = doc();
    let (inner, outer, a) = d
        .transact("scene", |tx| {
            let a = sphere(tx, 0.0, 100.0);
            let inner = tx.create(forge_fields::GROUP, None, None)?;
            let outer = tx.create(forge_fields::GROUP, None, None)?;
            tx.set_param(
                inner,
                forge_fields::FIELD_LIST,
                Value::LinkRows(vec![row(a, "normal", 1.0)]),
            )?;
            tx.set_param(
                outer,
                forge_fields::FIELD_LIST,
                Value::LinkRows(vec![row(inner, "normal", 1.0)]),
            )?;
            Ok((inner, outer, a))
        })
        .unwrap();
    let p = DVec3::new(30.0, 0.0, 0.0);
    let direct = at(&FieldList::from_nodes(d.scene(), &[a]).unwrap(), p);
    assert_eq!(
        at(&FieldList::from_nodes(d.scene(), &[outer]).unwrap(), p),
        direct
    );
    // inner -> outer -> inner
    d.transact("cycle", |tx| {
        tx.set_param(
            inner,
            forge_fields::FIELD_LIST,
            Value::LinkRows(vec![row(outer, "normal", 1.0)]),
        )
    })
    .unwrap();
    let r = FieldList::from_nodes(d.scene(), &[outer]);
    assert!(
        matches!(r, Err(FieldError::Unsupported(_))),
        "{:?}",
        r.err()
    );
}

#[test]
fn a_group_remaps_its_list_when_remapping_is_on() {
    let mut d = doc();
    let (g, a) = d
        .transact("scene", |tx| {
            let a = sphere(tx, 0.0, 100.0);
            let g = tx.create(forge_fields::GROUP, None, None)?;
            tx.set_param(
                g,
                forge_fields::FIELD_LIST,
                Value::LinkRows(vec![row(a, "normal", 1.0)]),
            )?;
            Ok((g, a))
        })
        .unwrap();
    let p = DVec3::new(75.0, 0.0, 0.0);
    let plain = at(&FieldList::from_nodes(d.scene(), &[a]).unwrap(), p);
    assert_eq!(
        at(&FieldList::from_nodes(d.scene(), &[g]).unwrap(), p),
        plain
    );
    d.transact("remap", |tx| {
        tx.set_param(g, "enable_remapping", Value::Bool(true))?;
        tx.set_param(g, "invert", Value::Bool(true))
    })
    .unwrap();
    let inverted = at(&FieldList::from_nodes(d.scene(), &[g]).unwrap(), p);
    assert!((inverted - (1.0 - plain)).abs() < 1e-12);
    // a group without layers has no value
    d.transact("empty", |tx| {
        tx.set_param(g, forge_fields::FIELD_LIST, Value::LinkRows(vec![]))
    })
    .unwrap();
    assert_eq!(at(&FieldList::from_nodes(d.scene(), &[g]).unwrap(), p), 0.0);
}

fn formula(d: &mut Document, text: &str) -> NodeId {
    d.transact("formula", |tx| {
        let f = tx.create(forge_fields::FORMULA, None, None)?;
        tx.set_param(f, "formula", Value::String(text.to_string()))?;
        Ok(f)
    })
    .unwrap()
}

#[test]
fn a_formula_sees_the_position_in_field_space_and_the_sample_index() {
    let mut d = doc();
    let f = formula(&mut d, "px / 100");
    d.transact("move", |tx| {
        tx.set_param(f, transform::POS, Value::Vec3([100.0, 0.0, 0.0]))
    })
    .unwrap();
    let l = FieldList::from_nodes(d.scene(), &[f]).unwrap();
    // field space: world x 150 is px 50; the result is limited to 0..1
    assert!((at(&l, DVec3::new(150.0, 0.0, 0.0)) - 0.5).abs() < 1e-12);
    assert_eq!(at(&l, DVec3::new(-500.0, 0.0, 0.0)), 0.0);
    assert_eq!(at(&l, DVec3::new(900.0, 0.0, 0.0)), 1.0);

    let g = formula(&mut d, "id / count");
    let l = FieldList::from_nodes(d.scene(), &[g]).unwrap();
    let pts = [DVec3::ZERO; 4];
    let v: Vec<f64> = l
        .sample_many(&pts, &EvalCtx::default())
        .iter()
        .map(|s| s.value)
        .collect();
    assert_eq!(v, vec![0.0, 0.25, 0.5, 0.75]);
}

#[test]
fn formula_angles_are_degrees_and_t_and_f_are_the_parameters() {
    let mut d = doc();
    let f = formula(&mut d, "sin(90 * f) * t");
    d.transact("params", |tx| {
        tx.set_param(f, "frequency", Value::Float(1.0))?;
        tx.set_param(f, "time_delta", Value::Float(0.5))
    })
    .unwrap();
    let l = FieldList::from_nodes(d.scene(), &[f]).unwrap();
    let ctx = EvalCtx {
        time: 1.5,
        ..EvalCtx::default()
    };
    assert!((l.sample(DVec3::ZERO, &ctx).value - 0.75).abs() < 1e-12);
    // non-finite results give 0
    let bad = formula(&mut d, "1 / 0 - 1 / 0");
    let l = FieldList::from_nodes(d.scene(), &[bad]).unwrap();
    assert_eq!(at(&l, DVec3::ZERO), 0.0);
}

#[test]
fn a_formula_that_does_not_parse_is_an_error_and_time_is_detected() {
    let mut d = doc();
    let bad = formula(&mut d, "1 +");
    assert!(matches!(
        FieldList::from_nodes(d.scene(), &[bad]).err(),
        Some(FieldError::Formula(_))
    ));
    let still = formula(&mut d, "px / 100");
    let moving = formula(&mut d, "sin(t * 90)");
    let group = d
        .transact("g", |tx| {
            let g = tx.create(forge_fields::GROUP, None, None)?;
            tx.set_param(
                g,
                forge_fields::FIELD_LIST,
                Value::LinkRows(vec![row(moving, "normal", 1.0)]),
            )?;
            Ok(g)
        })
        .unwrap();
    assert!(!forge_fields::reads_time(d.scene(), still));
    assert!(forge_fields::reads_time(d.scene(), moving));
    assert!(forge_fields::reads_time(d.scene(), group));
}

#[test]
fn the_shader_field_is_in_lists_but_unsupported_until_shaders_exist() {
    let mut d = doc();
    let s = d
        .transact("shader", |tx| tx.create(forge_fields::SHADER, None, None))
        .unwrap();
    assert!(matches!(
        FieldList::from_nodes(d.scene(), &[s]).err(),
        Some(FieldError::Unsupported(_))
    ));
}

#[test]
fn a_disabled_node_is_still_left_out_of_a_list() {
    // F-UI: the object state `object.enabled` of the field node also removes the layer.
    let mut d = doc();
    let (holder, f) = d
        .transact("scene", |tx| {
            let f = sphere(tx, 0.0, 100.0);
            let h = tx.create(forge_fields::GROUP, None, None)?;
            tx.set_param(
                h,
                forge_fields::FIELD_LIST,
                Value::LinkRows(vec![row(f, "normal", 1.0)]),
            )?;
            tx.set_param(f, "object.enabled", Value::Bool(false))?;
            Ok((h, f))
        })
        .unwrap();
    let _ = f;
    let l = FieldList::from_param(d.scene(), holder, forge_fields::FIELD_LIST).unwrap();
    assert!(l.is_empty());
}
