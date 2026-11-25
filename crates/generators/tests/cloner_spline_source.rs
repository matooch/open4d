//! A Cloner in Object mode on a spline source (clone-toolset spec C2, S4 acceptance):
//! a spline has no mesh of its own, so its sampled polyline is the clone source. The Vertex
//! distribution clones on the sampled points, Edge and Polygon Center on the chord midpoints;
//! Surface and Volume still need a mesh with area or a closed hull.

use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{BuildError, Primitives, clone_data, splines};
use glam::DVec3;

fn setup(plane: &str, distribution: &str) -> (Document, NodeId, NodeId) {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    let mut doc = Document::new(Arc::new(reg));
    let (cloner, circle) = doc
        .transact("spline source", |tx| {
            let circle = tx.create("forge.circle_spline", None, None)?;
            tx.set_param(circle, "plane", Value::Enum(plane.into()))?;
            tx.set_param(circle, "radius", Value::Float(200.0))?;
            let cloner = tx.create("forge.cloner", None, None)?;
            tx.create("forge.cube", Some(cloner), None)?;
            tx.set_param(cloner, "mode", Value::Enum("object".into()))?;
            tx.set_param(cloner, "object", Value::Link(Some(circle)))?;
            tx.set_param(cloner, "distribution", Value::Enum(distribution.into()))?;
            tx.set_param(cloner, "align_clone", Value::Bool(false))?;
            Ok((cloner, circle))
        })
        .unwrap();
    (doc, cloner, circle)
}

/// The sampled points of the circle (already in the spline's own plane).
fn sampled(doc: &Document, circle: NodeId) -> Vec<DVec3> {
    let loops = splines::sampled_loops(doc.scene(), circle).unwrap();
    assert_eq!(loops.len(), 1, "a plain circle has one loop");
    loops[0].0.clone()
}

fn positions(doc: &Document, cloner: NodeId) -> Result<Vec<DVec3>, BuildError> {
    let data = clone_data(doc.scene(), cloner, &EvalCtx::default())?.unwrap();
    Ok(data.matrices.iter().map(|m| m.w_axis.truncate()).collect())
}

#[test]
fn a_circle_spline_source_clones_on_its_sampled_points() {
    let (doc, cloner, circle) = setup("xz", "vertex");
    // The Circle's default Intermediate Points (Uniform, Number 8) put 4 * (8 + 1) points on it.
    let want = sampled(&doc, circle);
    assert_eq!(want.len(), 36);
    let got = positions(&doc, cloner).unwrap();
    assert_eq!(got.len(), want.len());
    for (i, (g, w)) in got.iter().zip(&want).enumerate() {
        assert!((*g - *w).length() < 1e-9, "clone {i}: {g:?} != {w:?}");
    }
    // All of them lie on the circle: 200 within the cubic approximation.
    for g in &got {
        let r = g.x.hypot(g.z);
        assert!((r - 200.0).abs() < 0.2, "clone off the circle: {g:?}");
    }
}

#[test]
fn spline_edge_and_polygon_center_use_the_chord_midpoints() {
    let (doc, cloner, _) = setup("xy", "edge");
    let edge = positions(&doc, cloner).unwrap();
    let (doc2, cloner2, _) = setup("xy", "polygon_center");
    let center = positions(&doc2, cloner2).unwrap();
    assert_eq!(edge.len(), 36);
    assert_eq!(edge, center);
    // Each midpoint is halfway between two neighbouring sampled points.
    let (doc, _, circle) = setup("xy", "vertex");
    let points = sampled(&doc, circle);
    for (i, m) in edge.iter().enumerate() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let want = (a + b) * 0.5;
        assert!((*m - want).length() < 1e-9, "edge {i}: {m:?} != {want:?}");
    }
}

#[test]
fn a_spline_source_needs_points_for_surface_and_volume() {
    // The sampled polyline has no area: Surface draws nothing, Volume is not a closed hull.
    let (doc, cloner, _) = setup("xy", "surface");
    assert!(positions(&doc, cloner).unwrap().is_empty());
    let (doc, cloner, _) = setup("xy", "volume");
    assert!(matches!(
        positions(&doc, cloner),
        Err(BuildError::Unsupported(_))
    ));
}
