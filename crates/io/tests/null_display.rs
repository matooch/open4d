//! SCENE-OBJECTS.md S9: a Null's display shape never reaches an export.

use std::sync::Arc;

use forge_core::{Document, Registry, Value};
use forge_generators::Primitives;
use forge_io::export_scene;

#[test]
fn a_null_with_a_display_shape_exports_nothing() {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    let mut d = Document::new(Arc::new(r));
    d.transact("null", |tx| {
        let n = tx.create("forge.null", None, None)?;
        tx.set_param(n, "shape", Value::Enum("sphere".into()))?;
        tx.set_param(n, "radius", Value::Float(100.0))
    })
    .unwrap();
    assert!(export_scene(d.scene(), None).is_empty());
}
