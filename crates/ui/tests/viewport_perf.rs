//! Viewport performance in the editor (docs/specs/VIEWPORT.md §10.8, §10.2, §10.7): cloners are
//! drawn as instances, the upload cache re-uploads only what changed, picking goes through the
//! instances and returns the cloner and the clone index. The cache test needs a wgpu device
//! (skips without one unless FORGE_REQUIRE_GPU=1).

use std::collections::BTreeSet;
use std::sync::Arc;

use forge_core::{EvalCtx, NodeId, Value};
use forge_generators::cloner_instances;
use forge_ui::manipulate::{PickHit, pick_node, pick_object};
use forge_ui::state::EditorState;
use forge_ui::viewport_panel::{
    DrawItem, GpuCache, GpuItem, UploadSettings, scene_draw_items_at, scene_meshes_at, upload_items,
};
use forge_viewport::camera::Camera;
use forge_viewport::render::{Gpu, Palette};
use glam::DVec3;

fn state() -> EditorState {
    EditorState::new(Arc::new(forge_app::default_registry().unwrap()))
}

/// A Linear cloner (20 clones, 500 cm apart along X) with a cube prototype, and a separate plain cube far away.
fn scene_with_cloner(st: &mut EditorState) -> (NodeId, NodeId, NodeId) {
    st.doc
        .transact("setup", |tx| {
            let c = tx.create("forge.cloner", None, None)?;
            tx.set_param(c, "mode", Value::Enum("linear".into()))?;
            tx.set_param(c, "linear_count", Value::Int(20))?;
            tx.set_param(c, "transform_position", Value::Vec3([500.0, 0.0, 0.0]))?;
            let proto = tx.create("forge.cube", Some(c), None)?;
            let lone = tx.create("forge.cube", None, None)?;
            tx.set_param(lone, "transform.pos", Value::Vec3([0.0, 0.0, -1500.0]))?;
            Ok((c, proto, lone))
        })
        .unwrap()
}

fn camera() -> Camera {
    Camera {
        target: DVec3::new(4750.0, 0.0, 0.0),
        distance: 12000.0,
        pitch: 0.3,
        ..EditorState::default_camera()
    }
}

#[test]
fn cloners_are_drawn_as_instances_and_everything_else_as_meshes() {
    let mut st = state();
    let (c, _, lone) = scene_with_cloner(&mut st);
    let items = scene_draw_items_at(st.doc.scene(), &BTreeSet::new(), &EvalCtx::default());
    assert_eq!(items.len(), 2);
    let DrawItem::Instanced { id, instances, .. } = &items[0] else {
        panic!("the cloner is instanced")
    };
    assert_eq!(*id, c);
    assert_eq!(instances.clones.len(), 20);
    assert_eq!(instances.prototypes.len(), 1);
    assert!(matches!(&items[1], DrawItem::Mesh { id, .. } if *id == lone));
    // the expanded view still has the clone geometry (export, bounds)
    let expanded = scene_meshes_at(st.doc.scene(), &BTreeSet::new(), &EvalCtx::default());
    let (_, mesh, _, _) = expanded.iter().find(|(id, ..)| *id == c).unwrap();
    assert_eq!(mesh.points.len(), 20 * 8);
}

#[test]
fn a_cloner_with_a_deformer_is_expanded_not_instanced() {
    let mut st = state();
    let (c, _, _) = scene_with_cloner(&mut st);
    st.doc
        .transact("bend", |tx| tx.create("forge.bend", Some(c), None))
        .unwrap();
    let items = scene_draw_items_at(st.doc.scene(), &BTreeSet::new(), &EvalCtx::default());
    assert!(items.iter().all(|i| matches!(i, DrawItem::Mesh { .. })));
    assert!(
        cloner_instances(st.doc.scene(), c, &EvalCtx::default())
            .unwrap()
            .is_none()
    );
}

#[test]
fn picking_returns_the_cloner_and_the_clone_index() {
    let mut st = state();
    let (c, _, lone) = scene_with_cloner(&mut st);
    let cam = camera();
    let (w, h) = (800.0, 600.0);
    let ctx = EvalCtx::default();
    let inst = cloner_instances(st.doc.scene(), c, &ctx).unwrap().unwrap();
    let world = st.doc.scene().world_matrix(c).unwrap();
    // aim at the centre of several clones: each pick names that clone
    let mut picked = 0;
    for k in [0usize, 7, 12, 19] {
        let centre = (world * inst.clones.matrices[k]).transform_point3(DVec3::ZERO);
        let Some(s) = cam.project(centre, w, h) else {
            continue;
        };
        let hit = pick_object(st.doc.scene(), &cam, s.x, s.y, w, h);
        assert_eq!(
            hit,
            Some(PickHit {
                node: c,
                clone: Some(k as u32)
            }),
            "clone {k} at {centre}"
        );
        assert_eq!(pick_node(st.doc.scene(), &cam, s.x, s.y, w, h), Some(c));
        picked += 1;
    }
    assert!(picked >= 3, "the camera sees the clones");
    // the plain cube still picks as before, with no clone index
    let s = cam.project(DVec3::new(0.0, 0.0, -1500.0), w, h).unwrap();
    assert_eq!(
        pick_object(st.doc.scene(), &cam, s.x, s.y, w, h),
        Some(PickHit {
            node: lone,
            clone: None
        })
    );
    // empty space picks nothing
    assert_eq!(pick_object(st.doc.scene(), &cam, 2.0, 2.0, w, h), None);
}

fn headless() -> Option<Gpu> {
    match Gpu::new_headless() {
        Ok(g) => Some(g),
        Err(e) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {e}")
        }
        Err(e) => {
            eprintln!("skipping GPU test: {e}");
            None
        }
    }
}

fn upload(
    st: &EditorState,
    gpu: &Gpu,
    cache: &mut GpuCache,
    selected: &BTreeSet<NodeId>,
) -> Vec<GpuItem> {
    let scene = st.doc.scene();
    upload_items(
        cache,
        &gpu.device,
        scene,
        scene_draw_items_at(scene, selected, &EvalCtx::default()),
        None,
        &Palette::default(),
        UploadSettings::default(),
    )
}

fn counts(cache: &GpuCache) -> (u64, u64, u64, u64) {
    let (m, i) = (cache.meshes.stats(), cache.instances.stats());
    (m.uploads, m.hits, i.uploads, i.hits)
}

#[test]
fn the_upload_cache_reuploads_only_what_changed() {
    let Some(gpu) = headless() else { return };
    let mut st = state();
    let (c, proto, lone) = scene_with_cloner(&mut st);
    let mut cache = GpuCache::default();
    let none = BTreeSet::new();

    // first frame: everything uploads (one mesh, one instance set)
    let first = upload(&st, &gpu, &mut cache, &none);
    assert_eq!(counts(&cache), (1, 0, 1, 0));
    // the same scene again: nothing uploads, and the very same GPU objects come back
    let again = upload(&st, &gpu, &mut cache, &none);
    assert_eq!(counts(&cache), (1, 1, 1, 1));
    match (&first[0], &again[0]) {
        (GpuItem::Instanced(a, _), GpuItem::Instanced(b, _)) => assert!(Arc::ptr_eq(a, b)),
        _ => panic!("the cloner is first"),
    }

    // selecting an object changes only its colour
    let selected: BTreeSet<NodeId> = [lone].into();
    let sel = upload(&st, &gpu, &mut cache, &selected);
    assert_eq!(counts(&cache), (1, 2, 1, 2), "selection uploads nothing");
    let (GpuItem::Mesh(_, plain), GpuItem::Mesh(_, tint)) = (&again[1], &sel[1]) else {
        panic!("the plain cube is a mesh")
    };
    assert_ne!(plain, tint);

    // editing the plain cube re-uploads that cube only
    st.doc
        .transact("move lone", |tx| {
            tx.set_param(lone, "transform.pos", Value::Vec3([50.0, 0.0, -1500.0]))
        })
        .unwrap();
    upload(&st, &gpu, &mut cache, &none);
    assert_eq!(counts(&cache), (2, 2, 1, 3));

    // editing the prototype or a cloner parameter re-uploads the instance set only
    st.doc
        .transact("count", |tx| {
            tx.set_param(c, "linear_count", Value::Int(25))
        })
        .unwrap();
    upload(&st, &gpu, &mut cache, &none);
    assert_eq!(counts(&cache), (2, 3, 2, 3));
    st.doc
        .transact("proto size", |tx| {
            tx.set_param(proto, "size_x", Value::Float(30.0))
        })
        .unwrap();
    upload(&st, &gpu, &mut cache, &none);
    assert_eq!(counts(&cache), (2, 4, 3, 3));

    // an unrelated edit (a new null) re-uploads nothing
    st.doc
        .transact("null", |tx| tx.create("forge.null", None, None))
        .unwrap();
    upload(&st, &gpu, &mut cache, &none);
    assert_eq!(counts(&cache), (2, 5, 3, 4));

    // removing the lone cube drops its entry
    st.doc.transact("delete", |tx| tx.delete(lone)).unwrap();
    upload(&st, &gpu, &mut cache, &none);
    assert_eq!(cache.meshes.len(), 0);
    assert_eq!(cache.instances.len(), 1);
}
