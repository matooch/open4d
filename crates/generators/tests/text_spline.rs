//! Behaviour of the Text spline that the golden cases cannot see (docs/specs/TEXT.md).

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_core::{Document, Registry, Value};
use forge_generators::editable_spline::{self, settings_of};
use forge_generators::splines::{Curve, build_curve};
use forge_generators::text_spline::TYPE;
use forge_generators::{BuildError, EditableError, Primitives, make_editable};

fn values(extra: &[(&str, Value)]) -> BTreeMap<String, Value> {
    extra
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn text(t: &str) -> Value {
    Value::String(t.into())
}

fn curve(extra: &[(&str, Value)]) -> Result<Curve, BuildError> {
    build_curve(TYPE, &values(extra))
}

#[test]
fn defaults_build_the_word_text() {
    let c = curve(&[]).unwrap();
    assert_eq!(c.extra.len(), 4, "T e x t = 5 loops");
    assert!(c.closed && c.extra.iter().all(|e| e.closed));
}

#[test]
fn unsupported_and_failing_inputs_are_errors_not_panics() {
    assert!(matches!(
        curve(&[("separate_letters", Value::Bool(true))]),
        Err(BuildError::Unsupported(_))
    ));
    assert!(matches!(
        curve(&[("text", text(" "))]),
        Err(BuildError::Failed(_))
    ));
    assert!(matches!(
        curve(&[("text", text(""))]),
        Err(BuildError::Failed(_))
    ));
    assert!(matches!(
        curve(&[("height", Value::Float(0.0))]),
        Err(BuildError::Failed(_))
    ));
}

#[test]
fn an_unknown_font_family_uses_the_bundled_font() {
    let a = curve(&[("text", text("Fg"))]).unwrap().loops();
    let b = curve(&[
        ("text", text("Fg")),
        ("font", Value::String("No Such Family 98765".into())),
    ])
    .unwrap()
    .loops();
    assert_eq!(a, b);
}

#[test]
fn reverse_runs_every_loop_backwards() {
    let fwd = curve(&[("text", text("O"))]).unwrap();
    let rev = curve(&[("text", text("O")), ("reverse", Value::Bool(true))]).unwrap();
    let signed = |l: &[glam::DVec3]| -> f64 {
        (0..l.len())
            .map(|i| l[i].x * l[(i + 1) % l.len()].y - l[(i + 1) % l.len()].x * l[i].y)
            .sum()
    };
    for (a, b) in fwd.loops().iter().zip(rev.loops()) {
        assert_eq!(a.len(), b.len());
        assert!(signed(a) * signed(&b) < 0.0);
    }
}

#[test]
fn planes_place_the_text() {
    let xy = curve(&[("text", text("T"))]).unwrap().points();
    let xz = curve(&[("text", text("T")), ("plane", Value::Enum("xz".into()))])
        .unwrap()
        .points();
    assert_eq!(xy.len(), xz.len());
    for (p, q) in xy.iter().zip(&xz) {
        assert!((q.x - p.x).abs() < 1e-12 && (q.y).abs() < 1e-12 && (q.z - p.y).abs() < 1e-12);
    }
}

#[test]
fn make_editable_keeps_every_loop_and_the_text_is_a_parameter() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    let mut doc = Document::new(Arc::new(reg));
    let id = doc
        .transact("text", |tx| {
            let id = tx.create(TYPE, None, None)?;
            tx.set_param(id, "text", text("AB"))?;
            Ok(id)
        })
        .unwrap();
    let before = curve(&[("text", text("AB"))]).unwrap().loops();
    // editing the text is a parameter change, and undoable
    doc.transact("edit", |tx| tx.set_param(id, "text", text("O")))
        .unwrap();
    assert!(doc.undo());
    let editable = doc
        .transact("make editable", |tx| {
            make_editable(tx, id).map_err(|e| match e {
                EditableError::Edit(e) => e,
                other => panic!("{other}"),
            })
        })
        .unwrap();
    assert_eq!(
        doc.scene().node(editable).unwrap().type_id,
        editable_spline::TYPE
    );
    let (data, ty, sampling) = settings_of(doc.scene(), editable).unwrap();
    assert_eq!(data.curve(ty, sampling).loops().len(), before.len());
    assert_eq!(before.len(), 5);
}
