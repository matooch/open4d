//! The Volume Builder and Volume Mesher nodes (docs/specs/VOLUME.md sections 6, 7).

use std::sync::Arc;

use forge_core::{Document, EvalCtx, LinkRow, NodeId, Registry, Value};
use forge_generators::Primitives;
use forge_volume::{VolumeBuildError, VolumeKind, VolumePlugin, build_volume, mesh_volume_node};
use glam::{DVec3, IVec3};

fn doc() -> Document {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&VolumePlugin).unwrap();
    Document::new(Arc::new(r))
}

struct Scene {
    d: Document,
    mesher: NodeId,
    builder: NodeId,
}

fn scene() -> Scene {
    let mut d = doc();
    let (mesher, builder) = d
        .transact("s", |tx| {
            let m = tx.create("forge.volume_mesher", None, None)?;
            let b = tx.create("forge.volume_builder", Some(m), None)?;
            Ok((m, b))
        })
        .unwrap();
    Scene { d, mesher, builder }
}

impl Scene {
    fn add(&mut self, ty: &str, parent: Option<NodeId>, pos: [f64; 3]) -> NodeId {
        let parent = parent.unwrap_or(self.builder);
        self.d
            .transact("add", |tx| {
                let id = tx.create(ty, Some(parent), None)?;
                tx.set_param(id, "transform.pos", Value::Vec3(pos))?;
                Ok(id)
            })
            .unwrap()
    }
    fn set(&mut self, id: NodeId, k: &str, v: Value) {
        self.d.transact("set", |tx| tx.set_param(id, k, v)).unwrap();
    }
    fn builder_child(&self) -> NodeId {
        self.d.scene().node(self.builder).unwrap().children[0]
    }
    fn volume(&self) -> forge_volume::Volume {
        build_volume(self.d.scene(), self.builder, &EvalCtx::default()).unwrap()
    }
    fn mesh(&self) -> Option<forge_geometry::PolyMesh> {
        mesh_volume_node(self.d.scene(), self.mesher, &EvalCtx::default()).unwrap()
    }
}

fn value(s: &Scene, i: [i32; 3]) -> f32 {
    s.volume().grid.get(IVec3::from_array(i))
}

#[test]
fn types_and_defaults_follow_o32() {
    let d = doc();
    let r = d.scene().registry();
    let b = r.get("forge.volume_builder").unwrap();
    let get = |id: &str| b.find_param(id).unwrap().default.clone();
    assert_eq!(get("voxel_size"), Value::Float(10.0));
    for k in [
        "interior_voxel_range",
        "exterior_voxel_range",
        "spline_voxel_range",
        "particle_voxel_range",
    ] {
        assert_eq!(get(k), Value::Int(2));
    }
    assert_eq!(get("volume_type"), Value::Enum("sdf".into()));
    assert_eq!(get("auto_update_settings"), Value::Bool(true));
    assert!(b.find_param("voxel_size").is_some());
    let m = r.get("forge.volume_mesher").unwrap();
    let get = |id: &str| m.find_param(id).unwrap().default.clone();
    assert_eq!(get("surface_threshold"), Value::Float(0.01));
    assert_eq!(get("voxel_range_threshold"), Value::Float(0.5));
    assert_eq!(get("adaptive"), Value::Float(0.0));
    assert_eq!(get("curvature_direction"), Value::Enum("both".into()));
}

#[test]
fn a_builder_without_entries_is_empty_and_has_no_mesh_of_its_own() {
    let s = scene();
    let v = s.volume();
    assert_eq!(v.grid.active_count(), 0);
    assert_eq!(v.kind, VolumeKind::Sdf);
    assert_eq!(s.mesh().unwrap().polys.len(), 0);
    let none = forge_generators::evaluate(s.d.scene(), s.builder, &EvalCtx::default()).unwrap();
    assert!(none.is_none());
    // a mesher without a builder child has no mesh
    let mut s2 = scene();
    s2.d.transact("rm", |tx| tx.delete(s2.builder)).unwrap();
    assert!(s2.mesh().is_none());
}

#[test]
fn entries_are_unions_by_default_and_the_mesh_covers_both() {
    let mut s = scene();
    s.add("forge.sphere", None, [-60.0, 0.0, 0.0]);
    s.add("forge.sphere", None, [60.0, 0.0, 0.0]);
    let m = s.mesh().unwrap();
    let (lo, hi) = m
        .points
        .iter()
        .fold((DVec3::MAX, DVec3::MIN), |(l, h), p| (l.min(*p), h.max(*p)));
    assert!(lo.x < -150.0 && hi.x > 150.0, "{lo:?} {hi:?}");
    // the overlap is one surface: the middle of the overlap is inside
    assert!(value(&s, [0, 0, 0]) < 0.0);
}

#[test]
fn the_objects_rows_set_the_blend_and_enabled_per_child() {
    // children are the list top to bottom, evaluated bottom to top: `b` first, then `a` by its blend
    let mut s = scene();
    let a = s.add("forge.cube", None, [0.0, 0.0, 0.0]);
    let b = s.add("forge.cube", None, [100.0, 0.0, 0.0]);
    // cubes are 200 wide: a spans -100..100, b 0..200
    let blend = |s: &mut Scene, rows: Vec<LinkRow>| {
        let builder = s.builder;
        s.set(builder, "objects", Value::LinkRows(rows));
    };
    assert!(
        value(&s, [-8, 0, 0]) < 0.0 && value(&s, [18, 0, 0]) < 0.0,
        "union"
    );
    blend(&mut s, vec![LinkRow::new(a, "subtract")]);
    assert!(value(&s, [18, 0, 0]) < 0.0, "b only");
    assert!(value(&s, [5, 0, 0]) > 0.0, "the overlap is cut away");
    assert!(value(&s, [-8, 0, 0]) > 0.0, "a alone does not appear");
    blend(&mut s, vec![LinkRow::new(a, "intersect")]);
    assert!(value(&s, [5, 0, 0]) < 0.0);
    assert!(value(&s, [-8, 0, 0]) > 0.0 && value(&s, [18, 0, 0]) > 0.0);
    let mut off = LinkRow::new(a, "union");
    off.enabled = false;
    blend(&mut s, vec![off]);
    assert!(value(&s, [-8, 0, 0]) > 0.0, "a disabled row is skipped");
    // a disabled object is skipped too
    blend(&mut s, vec![]);
    s.set(b, "object.enabled", Value::Bool(false));
    assert!(value(&s, [18, 0, 0]) > 0.0 && value(&s, [-8, 0, 0]) < 0.0);
}

#[test]
fn the_bottom_entry_starts_the_accumulator_whatever_its_blend() {
    let mut s = scene();
    let a = s.add("forge.cube", None, [0.0; 3]);
    let builder = s.builder;
    s.set(
        builder,
        "objects",
        Value::LinkRows(vec![LinkRow::new(a, "subtract")]),
    );
    assert!(value(&s, [0, 0, 0]) < 0.0);
}

#[test]
fn a_null_is_traversed_with_the_full_transform() {
    let mut s = scene();
    let null = s.add("forge.null", None, [0.0, 100.0, 0.0]);
    s.add("forge.cube", Some(null), [0.0, 0.0, 0.0]);
    s.add("forge.cube", Some(null), [300.0, 0.0, 0.0]);
    // the cubes sit at y = 100: samples at y = 10 are their centres
    assert!(value(&s, [0, 10, 0]) < 0.0);
    assert!(value(&s, [30, 10, 0]) < 0.0);
    assert_eq!(value(&s, [0, 0, 0]), 0.0, "the cube face is at y = 0");
    assert!(value(&s, [0, -1, 0]) > 0.0);
}

#[test]
fn the_builder_transform_moves_the_mesh_not_the_grid() {
    let mut s = scene();
    s.add("forge.cube", None, [0.0; 3]);
    let base = s.mesh().unwrap();
    // a move that is not a multiple of the voxel size: the grid stays in builder space
    let builder = s.builder;
    s.set(builder, "transform.pos", Value::Vec3([37.0, -5.0, 12.5]));
    let moved = s.mesh().unwrap();
    assert_eq!(base.points.len(), moved.points.len());
    for (a, b) in base.points.iter().zip(&moved.points) {
        assert!((*b - *a - DVec3::new(37.0, -5.0, 12.5)).length() < 1e-9);
    }
}

#[test]
fn voxel_size_and_ranges_drive_the_grid() {
    let mut s = scene();
    s.add("forge.cube", None, [0.0; 3]);
    let builder = s.builder;
    s.set(builder, "voxel_size", Value::Float(20.0));
    let v = s.volume();
    assert_eq!(v.grid.voxel_size(), 20.0);
    assert_eq!(v.grid.background(), 40.0);
    s.set(builder, "exterior_voxel_range", Value::Int(3));
    assert_eq!(s.volume().grid.background(), 60.0);
}

#[test]
fn fog_volumes_hold_density() {
    let mut s = scene();
    s.add("forge.cube", None, [0.0; 3]);
    let builder = s.builder;
    s.set(builder, "volume_type", Value::Enum("fog".into()));
    let v = s.volume();
    assert_eq!(v.kind, VolumeKind::Fog);
    assert_eq!(v.grid.background(), 0.0);
    assert!(
        v.grid
            .active_voxels()
            .iter()
            .all(|(_, d)| (0.0..=1.0).contains(d))
    );
    // deeper than the interior band: full density; on the surface samples: none
    assert_eq!(v.grid.get(IVec3::new(0, 0, 0)), 1.0);
    assert_eq!(v.grid.get(IVec3::new(-10, 0, 0)), 0.0);
    assert!(v.grid.get(IVec3::new(-9, 0, 0)) > 0.0);
    // meshing a fog gives a closed surface inside the cube
    let m = s.mesh().unwrap();
    assert!(!m.polys.is_empty());
    assert!(m.points.iter().all(|p| p.x.abs() < 100.0));
}

#[test]
fn nested_builders_are_entries() {
    let mut s = scene();
    let inner = s.add("forge.volume_builder", None, [0.0; 3]);
    s.add("forge.sphere", Some(inner), [0.0; 3]);
    s.add("forge.cube", None, [300.0, 0.0, 0.0]);
    assert!(value(&s, [0, 0, 0]) < 0.0, "inner sphere");
    assert!(value(&s, [30, 0, 0]) < 0.0, "outer cube");
    // an inner builder on another lattice is resampled
    s.set(inner, "voxel_size", Value::Float(5.0));
    assert!(value(&s, [0, 0, 0]) < 0.0);
    assert!(value(&s, [15, 0, 0]) > 0.0);
}

#[test]
fn unsupported_options_error() {
    let mut s = scene();
    s.add("forge.cube", None, [0.0; 3]);
    let (m, b) = (s.mesher, s.builder);
    s.set(b, "override_grid_matrix", Value::Bool(true));
    assert!(matches!(
        build_volume(s.d.scene(), b, &EvalCtx::default()),
        Err(VolumeBuildError::Unsupported(_))
    ));
    s.set(b, "override_grid_matrix", Value::Bool(false));
    s.set(b, "volume_type", Value::Enum("vector".into()));
    assert!(matches!(
        build_volume(s.d.scene(), b, &EvalCtx::default()),
        Err(VolumeBuildError::Unsupported(_))
    ));
    s.set(b, "volume_type", Value::Enum("sdf".into()));
    s.set(m, "create_curvature_map", Value::Bool(true));
    assert!(matches!(
        mesh_volume_node(s.d.scene(), m, &EvalCtx::default()),
        Err(VolumeBuildError::Unsupported(_))
    ));
    // through the generator evaluation the error is the node's build error
    assert!(forge_generators::evaluate(s.d.scene(), m, &EvalCtx::default()).is_err());
}

#[test]
fn splines_become_tubes() {
    let mut s = scene();
    s.add("forge.circle_spline", None, [0.0; 3]);
    let v = s.volume();
    assert!(v.grid.active_count() > 100);
    let m = s.mesh().unwrap();
    assert!(!m.polys.is_empty());
    // the tube radius is 0.735 * range * voxel = 14.7 around the curve: the circle passes
    // through its first point, whose sample (radius / 10, 0, 0) is inside the tube
    let line = forge_generators::splines::polyline_mesh(s.d.scene(), s.builder_child()).unwrap();
    let p0 = line.points[0];
    let idx = IVec3::new((p0.x / 10.0).round() as i32, 0, 0);
    assert!(v.grid.get(idx) < 0.0, "{p0:?}");
    assert!(v.grid.get(idx + IVec3::X * 3) > 0.0);
    assert!((f64::from(v.grid.get(idx)) - (0.0 - 14.7)).abs() < 1e-3);
}

#[test]
fn a_filter_layer_acts_on_the_entries_below_it() {
    let mut s = scene();
    // list: the filter on top, the cube below it
    let filter = s.add("forge.volume_filter", None, [0.0; 3]);
    let cube = s.add("forge.cube", None, [0.0; 3]);
    s.set(filter, "kind", Value::Enum("reshape".into()));
    s.set(filter, "offset", Value::Float(-10.0));
    // opening with radius 10 rounds the edges: the mesh loses volume and keeps its faces
    let opened = s.mesh().unwrap();
    let plain = {
        let b = s.builder;
        s.set(
            b,
            "objects",
            Value::LinkRows(vec![LinkRow {
                enabled: false,
                ..LinkRow::new(filter, "union")
            }]),
        );
        let m = s.mesh().unwrap();
        s.set(b, "objects", Value::LinkRows(vec![]));
        m
    };
    let vol = |m: &forge_geometry::PolyMesh| {
        m.polys
            .iter()
            .map(|f| {
                (1..f.len() - 1)
                    .map(|i| {
                        m.points[f[0] as usize]
                            .dot(m.points[f[i] as usize].cross(m.points[f[i + 1] as usize]))
                            / 6.0
                    })
                    .sum::<f64>()
            })
            .sum::<f64>()
    };
    assert!(
        vol(&opened) < vol(&plain) - 1000.0,
        "{} {}",
        vol(&opened),
        vol(&plain)
    );
    assert_eq!(opened.polys.len(), plain.polys.len());
    assert!(value(&s, [0, 0, 0]) < 0.0);
    // the same filter below the cube (last child) has nothing to act on
    let builder = s.builder;
    s.d.transact("move", |tx| tx.move_node(filter, Some(builder), Some(1)))
        .unwrap();
    assert_eq!(
        s.d.scene().node(builder).unwrap().children,
        vec![cube, filter]
    );
    assert!(value(&s, [-9, -9, -9]) < 0.0);
}

#[test]
fn filter_parameters_and_offset() {
    let mut s = scene();
    let filter = s.add("forge.volume_filter", None, [0.0; 3]);
    s.add("forge.cube", None, [0.0; 3]);
    s.set(filter, "kind", Value::Enum("offset".into()));
    s.set(filter, "offset", Value::Float(10.0));
    // the surface moves out by one voxel: the sample on the cube face is now inside
    assert!(value(&s, [-10, 0, 0]) < 0.0);
    assert!(value(&s, [-11, 0, 0]) >= 0.0);
    s.set(filter, "kind", Value::Enum("smooth".into()));
    s.set(filter, "strength", Value::Float(0.0));
    // strength 0 blends nothing in: the cube is unchanged
    assert_eq!(value(&s, [-10, 0, 0]), 0.0);
    let ty =
        s.d.scene()
            .registry()
            .get("forge.volume_filter")
            .unwrap()
            .clone();
    assert_eq!(ty.find_param("iterations").unwrap().default, Value::Int(1));
    assert_eq!(
        ty.find_param("voxel_distance").unwrap().default,
        Value::Int(2)
    );
    assert_eq!(ty.find_param("voxel_range").unwrap().default, Value::Int(3));
    assert_eq!(ty.find_param("offset").unwrap().default, Value::Float(5.0));
}
