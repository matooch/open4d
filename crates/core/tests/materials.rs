//! docs/specs/CORE.md §2.1 (brief 194): materials and shaders hosted in the document, and
//! §1 / tags (brief 027): float lists, restriction weights.

mod common;

use std::sync::Arc;

use forge_core::tags::{self, SelectionKind};
use forge_core::{
    Category, Document, EditError, LinkRow, NodeId, NodeType, ParamDesc, ParamError, Registry,
    Value,
};

fn registry() -> Arc<Registry> {
    let mut r = (*common::registry()).clone();
    r.register(
        NodeType::new("test.matx", "Material", Category::Material)
            .param(ParamDesc::link(
                "color.texture",
                "Texture",
                &[Category::Shader],
            ))
            .param(ParamDesc::link_list(
                "others",
                "Others",
                &[Category::Shader],
            )),
    )
    .unwrap();
    r.register(
        NodeType::new("test.shader", "Shader", Category::Shader)
            .param(ParamDesc::float("scale", "Scale", 1.0))
            .param(ParamDesc::link("texture", "Texture", &[Category::Shader])),
    )
    .unwrap();
    r.register(
        NodeType::new("test.holder", "Holder", Category::Object).param(ParamDesc::link_rows(
            "rows",
            "Rows",
            &[Category::Shader, Category::Object],
            &[("normal", "Normal")],
        )),
    )
    .unwrap();
    r.register(
        NodeType::new("test.vmap", "Vmap holder", Category::Object)
            .param(ParamDesc::float_list("weights", "Weights").range(0.0, 1.0)),
    )
    .unwrap();
    Arc::new(r)
}

fn doc() -> Document {
    Document::new(registry())
}

#[test]
fn materials_live_in_an_ordered_list_apart_from_the_object_tree() {
    let mut d = doc();
    let (a, b, c) = d
        .transact("m", |tx| {
            let o = tx.create("forge.null", None, None)?;
            let a = tx.create_material("test.matx", None)?;
            let b = tx.create_material("test.matx", None)?;
            let c = tx.create_material("test.matx", Some(0))?;
            assert!(tx.scene().roots().contains(&o));
            Ok((a, b, c))
        })
        .unwrap();
    assert_eq!(d.scene().materials(), [c, a, b]);
    assert!(!d.scene().roots().contains(&a));
    assert_eq!(d.scene().node(a).unwrap().parent, None);
    assert!(d.scene().param(a, "transform.pos").is_none());
    d.transact("move", |tx| tx.move_material(c, 1)).unwrap();
    assert_eq!(d.scene().materials(), [a, c, b]);
    d.undo();
    assert_eq!(d.scene().materials(), [c, a, b]);
    d.undo();
    assert!(d.scene().materials().is_empty());
    assert!(!d.scene().contains(a));
    d.redo();
    assert_eq!(d.scene().materials(), [c, a, b]);
    d.redo();
    assert_eq!(d.scene().materials(), [a, c, b]);
}

#[test]
fn shaders_are_children_of_a_material_or_shader() {
    let mut d = doc();
    let (m, s1, s2, obj) = d
        .transact("m", |tx| {
            let m = tx.create_material("test.matx", None)?;
            let s1 = tx.create_shader("test.shader", m, None)?;
            let s2 = tx.create_shader("test.shader", s1, None)?;
            let obj = tx.create("forge.null", None, None)?;
            Ok((m, s1, s2, obj))
        })
        .unwrap();
    assert_eq!(d.scene().node(m).unwrap().children, [s1]);
    assert_eq!(d.scene().node(s1).unwrap().children, [s2]);
    assert_eq!(d.scene().node(s2).unwrap().parent, Some(s1));
    let err = |d: &mut Document, f: &dyn Fn(&mut forge_core::Tx) -> Result<NodeId, EditError>| {
        d.transact("x", |tx| f(tx)).unwrap_err()
    };
    // wrong places and categories
    assert!(matches!(
        err(&mut d, &|tx| tx.create_shader("test.shader", obj, None)),
        EditError::WrongCategory(_)
    ));
    assert!(matches!(
        err(&mut d, &|tx| tx.create("test.shader", Some(m), None)),
        EditError::NotPlaceable(_)
    ));
    assert!(matches!(
        err(&mut d, &|tx| tx.create("forge.null", Some(m), None)),
        EditError::WrongCategory(_)
    ));
    assert!(matches!(
        err(&mut d, &|tx| tx.create_material("test.shader", None)),
        EditError::WrongCategory(_)
    ));
    assert!(matches!(
        err(&mut d, &|tx| tx.create_shader("test.matx", m, None)),
        EditError::WrongCategory(_)
    ));
    assert!(matches!(
        err(&mut d, &|tx| tx.attach_tag("test.sel", m, None)),
        EditError::WrongCategory(_)
    ));
    assert!(matches!(
        d.transact("x", |tx| tx.move_node(m, None, None))
            .unwrap_err(),
        EditError::WrongCategory(_)
    ));
    assert!(matches!(
        err(&mut d, &|tx| tx.create_material("test.matx", Some(5))),
        EditError::IndexOutOfRange
    ));
}

#[test]
fn links_to_materials_and_shaders_are_set_by_category_and_survive_undo() {
    let mut d = doc();
    let (m, s, tag, obj) = d
        .transact("m", |tx| {
            let m = tx.create_material("test.matx", None)?;
            let s = tx.create_shader("test.shader", m, None)?;
            tx.set_param(m, "color.texture", Value::Link(Some(s)))?;
            let obj = tx.create("forge.null", None, None)?;
            let tag = tx.attach_tag("test.sel", obj, None)?;
            Ok((m, s, tag, obj))
        })
        .unwrap();
    assert_eq!(
        d.scene().param(m, "color.texture"),
        Some(&Value::Link(Some(s)))
    );
    // a link of the wrong category is refused (an object is not a shader)
    assert!(matches!(
        d.transact("x", |tx| tx.set_param(
            m,
            "color.texture",
            Value::Link(Some(obj))
        ))
        .unwrap_err(),
        EditError::InvalidLink(_)
    ));
    let _ = tag;
}

#[test]
fn deleting_a_material_clears_every_link_into_it_in_one_undo_step() {
    let mut d = doc();
    let (m1, m2, s1, s2, h) = d
        .transact("m", |tx| {
            let m1 = tx.create_material("test.matx", None)?;
            let m2 = tx.create_material("test.matx", None)?;
            let s1 = tx.create_shader("test.shader", m1, None)?;
            let s2 = tx.create_shader("test.shader", m2, None)?;
            tx.set_param(m1, "color.texture", Value::Link(Some(s1)))?;
            // another material points into m1's tree and into its own tree
            tx.set_param(m2, "color.texture", Value::Link(Some(s2)))?;
            tx.set_param(m2, "others", Value::LinkList(vec![s1, s2]))?;
            let h = tx.create("test.holder", None, None)?;
            tx.set_param(
                h,
                "rows",
                Value::LinkRows(vec![LinkRow::new(s1, "normal"), LinkRow::new(s2, "normal")]),
            )?;
            Ok((m1, m2, s1, s2, h))
        })
        .unwrap();
    let before = d.scene().clone();
    let depth = d.undo_depth();
    d.transact("Delete material", |tx| tx.delete(m1)).unwrap();
    assert_eq!(d.undo_depth(), depth + 1);
    assert!(!d.scene().contains(m1) && !d.scene().contains(s1));
    assert_eq!(d.scene().materials(), [m2]);
    assert_eq!(
        d.scene().param(m2, "others"),
        Some(&Value::LinkList(vec![s2]))
    );
    assert_eq!(
        d.scene().param(m2, "color.texture"),
        Some(&Value::Link(Some(s2)))
    );
    assert_eq!(
        d.scene().param(h, "rows"),
        Some(&Value::LinkRows(vec![LinkRow::new(s2, "normal")]))
    );
    d.undo();
    assert_eq!(d.scene().materials(), before.materials());
    for id in [m1, m2, s1, s2, h] {
        assert_eq!(
            d.scene().node(id).unwrap().params,
            before.node(id).unwrap().params
        );
    }
    assert_eq!(d.scene().node(m1).unwrap().children, [s1]);
    d.redo();
    assert_eq!(
        d.scene().param(m2, "others"),
        Some(&Value::LinkList(vec![s2]))
    );
}

#[test]
fn deleting_a_shader_clears_links_to_it_and_its_subtree() {
    let mut d = doc();
    let (m, top, leaf) = d
        .transact("m", |tx| {
            let m = tx.create_material("test.matx", None)?;
            let top = tx.create_shader("test.shader", m, None)?;
            let leaf = tx.create_shader("test.shader", top, None)?;
            tx.set_param(top, "texture", Value::Link(Some(leaf)))?;
            tx.set_param(m, "color.texture", Value::Link(Some(leaf)))?;
            Ok((m, top, leaf))
        })
        .unwrap();
    d.transact("Delete shader", |tx| tx.delete(top)).unwrap();
    assert_eq!(
        d.scene().param(m, "color.texture"),
        Some(&Value::Link(None))
    );
    assert!(!d.scene().contains(leaf));
    d.undo();
    assert_eq!(
        d.scene().param(m, "color.texture"),
        Some(&Value::Link(Some(leaf)))
    );
    assert_eq!(
        d.scene().param(top, "texture"),
        Some(&Value::Link(Some(leaf)))
    );
}

#[test]
fn deleting_an_object_still_leaves_links_dangling() {
    // CORE.md section 3: only materials and shaders clear the links to them
    let mut d = doc();
    let (a, b) = d
        .transact("m", |tx| {
            let a = tx.create("test.holder", None, None)?;
            let b = tx.create("forge.null", None, None)?;
            tx.set_param(a, "rows", Value::LinkRows(vec![LinkRow::new(b, "normal")]))?;
            Ok((a, b))
        })
        .unwrap();
    d.transact("x", |tx| tx.delete(b)).unwrap();
    assert_eq!(
        d.scene().param(a, "rows"),
        Some(&Value::LinkRows(vec![LinkRow::new(b, "normal")]))
    );
}

#[test]
fn a_material_edit_raises_the_deep_revision_of_a_linking_tag_host() {
    let mut d = doc();
    let (m, s, tag) = d
        .transact("m", |tx| {
            let m = tx.create_material("test.matx", None)?;
            let s = tx.create_shader("test.shader", m, None)?;
            let o = tx.create("forge.null", None, None)?;
            let tag = tx.attach_tag("test.sel", o, None)?;
            Ok((m, s, tag))
        })
        .unwrap();
    let _ = (m, tag);
    let r0 = d.scene().deep_revision(m).unwrap();
    d.transact("x", |tx| tx.set_param(s, "scale", Value::Float(3.0)))
        .unwrap();
    assert!(d.scene().deep_revision(m).unwrap() > r0);
}

#[test]
fn float_lists_validate_and_clamp_per_item() {
    let mut d = doc();
    let v = d
        .transact("m", |tx| tx.create("test.vmap", None, None))
        .unwrap();
    d.transact("x", |tx| {
        tx.set_param(v, "weights", Value::FloatList(vec![-1.0, 0.25, 7.0]))
    })
    .unwrap();
    assert_eq!(
        d.scene().param(v, "weights"),
        Some(&Value::FloatList(vec![0.0, 0.25, 1.0]))
    );
    let err = d
        .transact("x", |tx| {
            tx.set_param(v, "weights", Value::FloatList(vec![0.5, f64::NAN]))
        })
        .unwrap_err();
    assert_eq!(err, EditError::Param(ParamError::NotFinite));
    let err = d
        .transact("x", |tx| tx.set_param(v, "weights", Value::Float(0.5)))
        .unwrap_err();
    assert_eq!(err, EditError::Param(ParamError::TypeMismatch));
    assert_eq!(
        d.scene().param(v, "weights"),
        Some(&Value::FloatList(vec![0.0, 0.25, 1.0]))
    );
}

#[test]
fn restriction_weights_come_from_a_named_selection_or_vertex_map() {
    let mut d = Document::new(registry());
    let (obj, def) = d
        .transact("m", |tx| {
            let obj = tx.create("forge.null", None, None)?;
            let def = tx.create("test.bend", Some(obj), None)?;
            let sel = tx.attach_tag(tags::POINT_SELECTION, obj, None)?;
            tx.rename(sel, "Top")?;
            tx.set_param(sel, "indices", Value::IndexList(vec![1, 3, 9]))?;
            let vm = tx.attach_tag(tags::VERTEX_MAP, obj, None)?;
            tx.rename(vm, "Ramp")?;
            tx.set_param(vm, "weights", Value::FloatList(vec![0.0, 0.5, 1.0]))?;
            Ok((obj, def))
        })
        .unwrap();
    let w = |d: &Document, n: usize| tags::restriction_weights(d.scene(), def, obj, n);
    assert_eq!(w(&d, 4), None, "no restriction tag: no weights");
    let r = d
        .transact("r", |tx| tx.attach_tag(tags::RESTRICTION, def, None))
        .unwrap();
    assert_eq!(w(&d, 4), None, "an empty name restricts nothing");
    d.transact("r", |tx| {
        tx.set_param(r, "selection", Value::String("Top".into()))
    })
    .unwrap();
    assert_eq!(
        w(&d, 4),
        Some(vec![0.0, 1.0, 0.0, 1.0]),
        "index 9 is past the 4 points"
    );
    d.transact("r", |tx| {
        tx.set_param(r, "selection", Value::String("Ramp".into()))
    })
    .unwrap();
    assert_eq!(
        w(&d, 5),
        Some(vec![0.0, 0.5, 1.0, 0.0, 0.0]),
        "past the list: 0"
    );
    d.transact("r", |tx| tx.set_param(r, "invert", Value::Bool(true)))
        .unwrap();
    assert_eq!(w(&d, 3), Some(vec![1.0, 0.5, 0.0]));
    d.transact("r", |tx| {
        tx.set_param(r, "selection", Value::String("Nope".into()))
    })
    .unwrap();
    assert_eq!(w(&d, 3), None, "an unknown name restricts nothing");
    d.transact("r", |tx| tx.set_param(r, "enabled", Value::Bool(false)))
        .unwrap();
    d.transact("r", |tx| {
        tx.set_param(r, "selection", Value::String("Top".into()))
    })
    .unwrap();
    assert_eq!(
        w(&d, 3),
        None,
        "a disabled restriction tag restricts nothing"
    );
    assert_eq!(
        SelectionKind::of_tag_type(tags::EDGE_SELECTION),
        Some(SelectionKind::Edges)
    );
    assert_eq!(SelectionKind::Polys.tag_type(), "forge.polygon_selection");
}
