//! docs/specs/ANIM.md §1.1–1.3, §1.6–1.7 and CORE.md §4, §6: tracks as data.

mod common;

use forge_core::{EditError, Interp, Key, NodeId, Track, TrackError, Value, file, parse_target};

fn track(keys: &[(f64, f64)]) -> Track {
    Track::new(keys.iter().map(|(t, v)| Key::new(*t, *v)).collect())
}

fn doc_with_box() -> (forge_core::Document, NodeId) {
    let mut doc = common::doc();
    let id = doc
        .transact("box", |tx| tx.create("test.box", None, None))
        .unwrap();
    (doc, id)
}

#[test]
fn track_validation() {
    assert_eq!(Track::default().validate(), Err(TrackError::Empty));
    assert_eq!(track(&[(0.0, 1.0)]).validate(), Ok(()));
    assert_eq!(
        track(&[(1.0, 0.0), (0.0, 1.0)]).validate(),
        Err(TrackError::Unsorted)
    );
    assert_eq!(
        track(&[(1.0, 0.0), (1.0, 1.0)]).validate(),
        Err(TrackError::Unsorted)
    );
    assert_eq!(
        track(&[(0.0, f64::NAN)]).validate(),
        Err(TrackError::NotFinite)
    );
    assert_eq!(
        track(&[(f64::INFINITY, 0.0)]).validate(),
        Err(TrackError::NotFinite)
    );
}

#[test]
fn track_keys_resolve_against_descriptors() {
    let (doc, id) = doc_with_box();
    let ty = doc.scene().node_type(id).unwrap().clone();
    let find = |p: &str| ty.find_param(p);
    assert!(parse_target("size", find).is_ok());
    assert!(parse_target("offset.y", find).is_ok());
    assert!(
        parse_target("offset", find).is_err(),
        "a Vec3 needs a component"
    );
    assert!(parse_target("offset.w", find).is_err());
    assert!(
        parse_target("size.x", find).is_err(),
        "a Float has no components"
    );
    assert!(
        parse_target("segments", find).is_err(),
        "Int tracks are out of scope"
    );
    assert!(parse_target("enabled", find).is_err());
    assert!(parse_target("tint.x", find).is_err());
    assert!(parse_target("nope", find).is_err());
    // dotted param ids: the full id is tried before the axis suffix is split off
    let null = common::registry().get("forge.null").unwrap().clone();
    assert!(parse_target("transform.pos.x", |p| null.find_param(p)).is_ok());
    assert!(parse_target("transform.rot.x", |p| null.find_param(p)).is_ok());
}

#[test]
fn set_track_validates_and_rejects_bad_targets() {
    let (mut doc, id) = doc_with_box();
    let good = track(&[(0.0, 0.0), (1.0, 5.0)]);
    doc.transact("t", |tx| tx.set_track(id, "size", Some(good.clone())))
        .unwrap();
    assert_eq!(doc.scene().node(id).unwrap().tracks["size"], good);

    let before = file::save(doc.scene());
    for (key, t, expect) in [
        ("segments", good.clone(), "BadTarget"),
        ("offset", good.clone(), "BadTarget"),
        ("size", Track::default(), "Empty"),
        ("size", track(&[(1.0, 0.0), (0.5, 1.0)]), "Unsorted"),
    ] {
        let err = doc
            .transact("bad", |tx| tx.set_track(id, key, Some(t)))
            .unwrap_err();
        assert!(matches!(err, EditError::Track(_)), "{key}: {err:?}");
        assert!(format!("{err:?}").contains(expect), "{key}: {err:?}");
    }
    assert_eq!(
        doc.transact("missing", |tx| tx.set_track(NodeId(99), "size", None)),
        Err(EditError::UnknownNode(NodeId(99)))
    );
    assert_eq!(
        file::save(doc.scene()),
        before,
        "failed edits change nothing"
    );
    assert_eq!(doc.history().undo_labels(), ["box", "t"]);
}

#[test]
fn set_key_remove_key_and_undo() {
    let (mut doc, id) = doc_with_box();
    let at = |doc: &forge_core::Document| {
        doc.scene()
            .node(id)
            .unwrap()
            .tracks
            .get("offset.x")
            .cloned()
    };
    assert_eq!(at(&doc), None);
    let rev0 = doc.scene().node(id).unwrap().rev;

    doc.transact("k1", |tx| tx.set_key(id, "offset.x", Key::new(1.0, 2.0)))
        .unwrap();
    doc.transact("k0", |tx| tx.set_key(id, "offset.x", Key::new(0.0, 1.0)))
        .unwrap();
    doc.transact("k1 again", |tx| {
        tx.set_key(id, "offset.x", Key::new(1.0, 3.0).with_interp(Interp::Step))
    })
    .unwrap();
    let t = at(&doc).unwrap();
    assert_eq!(t.keys.len(), 2, "same time replaces");
    assert_eq!(t.keys[0], Key::new(0.0, 1.0), "sorted by time");
    assert_eq!(t.keys[1].value, 3.0);
    assert_eq!(t.keys[1].interp, Interp::Step);
    assert!(
        doc.scene().node(id).unwrap().rev > rev0,
        "edits stamp the node"
    );

    // undo of set_key restores the previous key exactly
    assert!(doc.undo());
    assert_eq!(at(&doc).unwrap().keys[1], Key::new(1.0, 2.0));
    assert!(doc.undo());
    assert_eq!(at(&doc).unwrap().keys.len(), 1);
    assert!(doc.undo());
    assert_eq!(at(&doc), None, "undoing the first key removes the track");
    assert!(doc.redo() && doc.redo() && doc.redo());
    assert_eq!(at(&doc).unwrap().keys[1].interp, Interp::Step);

    // remove_key: exact time only, last key removes the track, missing is a no-op
    let n = doc.history().undo_labels().len();
    doc.transact("none", |tx| tx.remove_key(id, "offset.x", 0.5))
        .unwrap();
    doc.transact("none", |tx| tx.remove_key(id, "size", 0.0))
        .unwrap();
    assert_eq!(
        doc.history().undo_labels().len(),
        n,
        "no-ops are not recorded"
    );
    doc.transact("rm", |tx| tx.remove_key(id, "offset.x", 0.0))
        .unwrap();
    assert_eq!(at(&doc).unwrap().keys.len(), 1);
    doc.transact("rm", |tx| tx.remove_key(id, "offset.x", 1.0))
        .unwrap();
    assert_eq!(at(&doc), None);
    assert!(doc.undo());
    assert_eq!(at(&doc).unwrap().keys.len(), 1);
}

fn animated_doc() -> (forge_core::Document, NodeId) {
    let (mut doc, id) = doc_with_box();
    doc.transact("anim", |tx| {
        tx.set_track(id, "size", Some(track(&[(0.0, 1.0), (2.0, 5.0)])))?;
        tx.set_key(
            id,
            "offset.x",
            Key::new(0.0, -1.0).with_interp(Interp::Linear),
        )?;
        tx.set_key(id, "offset.x", Key::new(1.0, 4.5))?;
        Ok(())
    })
    .unwrap();
    (doc, id)
}

#[test]
fn v4_round_trip_is_lossless_and_deterministic() {
    let (doc, id) = animated_doc();
    let text = file::save(doc.scene());
    assert_eq!(text, file::save(animated_doc().0.scene()));
    assert!(text.contains("version: 5"), "{text}");
    assert!(text.contains("tracks"), "{text}");
    assert!(text.contains("Linear"), "{text}");
    let (scene, report) = file::load(common::registry(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(file::save(&scene), text);
    assert_eq!(
        scene.node(id).unwrap().tracks,
        doc.scene().node(id).unwrap().tracks
    );
}

#[test]
fn v3_files_still_load_without_tracks() {
    let text = r#"(format: "forge3d-scene", version: 3, roots: [
        (id: 1, type_id: "test.box", name: "b", tags: [], children: [], params: {"size": Float(3.0)}),
    ])"#;
    let (scene, report) = file::load(common::registry(), text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(scene.node(NodeId(1)).unwrap().tracks.is_empty());
    assert_eq!(scene.param(NodeId(1), "size"), Some(&Value::Float(3.0)));
    assert!(file::save(&scene).contains("version: 5"));
}

#[test]
fn invalid_stored_tracks_are_dropped_with_a_warning_and_unknown_types_keep_theirs() {
    let text = r#"(format: "forge3d-scene", version: 4, roots: [
        (id: 1, type_id: "test.box", name: "b", tags: [], children: [], params: {}, tracks: {
            "size": (keys: [(time: 0.0, value: 1.0, interp: Spline)]),
            "segments": (keys: [(time: 0.0, value: 1.0, interp: Spline)]),
            "offset.y": (keys: [(time: 1.0, value: 1.0, interp: Step), (time: 0.0, value: 2.0, interp: Spline)]),
        }),
        (id: 2, type_id: "ghost.type", name: "g", tags: [], children: [], params: {}, tracks: {
            "whatever": (keys: [(time: 0.0, value: 1.0, interp: Spline)]),
        }),
    ])"#;
    let (scene, report) = file::load(common::registry(), text).unwrap();
    let tracks: Vec<_> = scene
        .node(NodeId(1))
        .unwrap()
        .tracks
        .keys()
        .cloned()
        .collect();
    assert_eq!(tracks, ["size"]);
    assert_eq!(report.warnings.len(), 3, "{:?}", report.warnings);
    assert!(report.warnings.iter().any(|w| w.contains("segments")));
    assert!(report.warnings.iter().any(|w| w.contains("offset.y")));
    assert!(
        scene
            .node(NodeId(2))
            .unwrap()
            .tracks
            .contains_key("whatever")
    );
}

#[test]
fn with_params_validates_clamps_and_stamps_fresh_revisions() {
    let (doc, id) = doc_with_box();
    let scene = doc.scene();
    let overrides = vec![
        (id, "size".to_string(), Value::Float(7.0)),
        (id, "offset".to_string(), Value::Vec3([100.0, 0.0, 0.0])), // clamped to 10
        (id, "mode".to_string(), Value::Float(1.0)),                // wrong type
        (id, "nope".to_string(), Value::Float(1.0)),
        (NodeId(99), "size".to_string(), Value::Float(1.0)),
    ];
    let (copy, skipped) = scene.with_params(overrides);
    assert_eq!(copy.param(id, "size"), Some(&Value::Float(7.0)));
    assert_eq!(
        copy.param(id, "offset"),
        Some(&Value::Vec3([10.0, 0.0, 0.0]))
    );
    assert_eq!(skipped.len(), 3);
    assert_eq!(
        scene.param(id, "size"),
        Some(&Value::Float(100.0)),
        "original untouched"
    );

    // two copies made for different values never share a revision
    let (a, _) = scene.with_params([(id, "size".to_string(), Value::Float(1.0))]);
    let (b, _) = scene.with_params([(id, "size".to_string(), Value::Float(2.0))]);
    assert_ne!(a.node(id).unwrap().rev, b.node(id).unwrap().rev);
    assert!(a.node(id).unwrap().rev > scene.node(id).unwrap().rev);
    // an unchanged value keeps the revision
    let (same, _) = scene.with_params([(id, "size".to_string(), Value::Float(100.0))]);
    assert_eq!(same.node(id).unwrap().rev, scene.node(id).unwrap().rev);
}
