//! The mesher never emits non-manifold geometry (docs/specs/VOLUME.md section 7.2 rule 4,
//! brief 208): one vertex per surface component of a cell.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::Primitives;
use forge_geometry::PolyMesh;
use forge_volume::{Polarity, SparseGrid, VolumePlugin, cell_mesh, merge_blocks, mesh_volume_node};
use glam::{DVec3, IVec3};

/// Closed and manifold: every undirected edge in exactly two faces running in opposite
/// directions, every vertex's faces one disc, no repeated index, no zero-area face.
fn assert_closed_manifold(m: &PolyMesh, what: &str) {
    assert_eq!(check_manifold(m, what, false), 0);
}

/// [`assert_closed_manifold`]; with `allow_double` two cells glued along both surface segments
/// of an ambiguous face (a ring of inside samples around a hole smaller than a cell) may share
/// a vertex pair twice (VOLUME.md section 7.2): those multi-edges must be balanced (as many
/// quads each way) and the vertices on them skip the fan check. Returns the multi-edge count.
fn check_manifold(m: &PolyMesh, what: &str, allow_double: bool) -> usize {
    let mut directed: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    // vertex -> link edges (previous, next) of its faces
    let mut link: BTreeMap<u32, Vec<(u32, u32)>> = BTreeMap::new();
    for f in &m.polys {
        assert!(f.len() >= 3, "{what}: face with {} corners", f.len());
        let distinct: BTreeSet<u32> = f.iter().copied().collect();
        assert_eq!(distinct.len(), f.len(), "{what}: repeated index in {f:?}");
        let mut normal = DVec3::ZERO;
        for i in 0..f.len() {
            let (a, b) = (f[i], f[(i + 1) % f.len()]);
            *directed.entry((a, b)).or_default() += 1;
            link.entry(a)
                .or_default()
                .push((f[(i + f.len() - 1) % f.len()], b));
            let (pa, pb) = (m.points[a as usize], m.points[b as usize]);
            normal += pa.cross(pb);
        }
        assert!(normal.length() > 1e-9, "{what}: degenerate face {f:?}");
    }
    let mut doubled: BTreeSet<u32> = BTreeSet::new();
    let mut multi = 0;
    for (&(a, b), &n) in &directed {
        if allow_double {
            assert_eq!(
                directed.get(&(b, a)),
                Some(&n),
                "{what}: edge {a}->{b} used {n} times, unbalanced"
            );
            if n > 1 {
                multi += 1;
                doubled.extend([a, b]);
            }
        } else {
            assert_eq!(n, 1, "{what}: edge {a}->{b} used {n} times");
            assert_eq!(
                directed.get(&(b, a)),
                Some(&1),
                "{what}: edge {a}->{b} has no opposite"
            );
        }
    }
    // every vertex fan is one cycle: each link vertex has degree 2 and the link is connected
    for (v, edges) in &link {
        if doubled.contains(v) {
            continue;
        }
        let mut adj: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &(p, n) in edges {
            adj.entry(p).or_default().push(n);
            adj.entry(n).or_default().push(p);
        }
        assert!(
            adj.values().all(|a| a.len() == 2),
            "{what}: vertex {v} has a pinched or open fan"
        );
        let start = *adj.keys().next().unwrap();
        let mut seen = BTreeSet::from([start]);
        let mut stack = vec![start];
        while let Some(x) = stack.pop() {
            for &y in &adj[&x] {
                if seen.insert(y) {
                    stack.push(y);
                }
            }
        }
        assert_eq!(seen.len(), adj.len(), "{what}: vertex {v} has two fans");
    }
    multi / 2
}

fn grid_of(inside: &[[i32; 3]], span: i32) -> SparseGrid<f32> {
    let mut g = SparseGrid::new(1.0f32, 1.0, DVec3::ZERO);
    for x in -span..=span {
        for y in -span..=span {
            for z in -span..=span {
                let v = if inside.contains(&[x, y, z]) {
                    -1.0
                } else {
                    1.0
                };
                g.set(IVec3::new(x, y, z), v);
            }
        }
    }
    g
}

fn mesh_of(g: &SparseGrid<f32>) -> PolyMesh {
    cell_mesh(g, 0.0, Polarity::LowInside).mesh
}

#[test]
fn two_inside_corners_of_a_cell_get_two_vertices() {
    // samples (0,0,0) and (1,1,1) are opposite corners of one cell: two sheets pass through it
    let m = mesh_of(&grid_of(&[[0, 0, 0], [1, 1, 1]], 3));
    // each isolated sample is a closed blob of 6 quads around 8 cells; the shared cell is split
    assert_eq!(m.polys.len(), 12);
    assert_eq!(m.points.len(), 16);
    assert_closed_manifold(&m, "opposite corners");
}

#[test]
fn diagonal_inside_corners_of_a_face_stay_apart() {
    // (0,0,0) and (1,1,0) touch only along a face diagonal: the ambiguous face is cut so that
    // each inside corner has its own sheet
    let m = mesh_of(&grid_of(&[[0, 0, 0], [1, 1, 0]], 3));
    assert_eq!(m.polys.len(), 12);
    assert_eq!(m.points.len(), 16);
    assert_closed_manifold(&m, "face diagonal");
}

#[test]
fn a_one_sample_thick_zigzag_slab_is_manifold() {
    // inside samples form a slab one sample thick whose height alternates in a checkerboard:
    // every cell between two heights sees two sheets
    let mut inside = Vec::new();
    for x in -3i32..=3 {
        for y in -3i32..=3 {
            inside.push([x, y, 4 + (x + y).rem_euclid(2)]);
        }
    }
    let m = mesh_of(&grid_of(&inside, 8));
    assert_closed_manifold(&m, "zigzag slab");
}

#[test]
fn every_corner_pattern_of_a_cell_is_manifold() {
    // all 256 sign patterns of the corners of one cell inside an outside border
    for pattern in 0u32..256 {
        let inside: Vec<[i32; 3]> = (0..8)
            .filter(|c| pattern >> c & 1 == 1)
            .map(|c| [c & 1, (c >> 1) & 1, (c >> 2) & 1])
            .collect();
        let m = mesh_of(&grid_of(&inside, 3));
        assert_closed_manifold(&m, &format!("pattern {pattern}"));
    }
}

#[test]
fn random_fields_have_balanced_edges_and_single_fans() {
    // deterministic noise (a 64-bit LCG), several densities; dense noise can hold rings of
    // inside samples around a hole smaller than a cell, which give coincident (balanced) edges
    for (seed, density) in [(1u64, 0.35), (2, 0.5), (3, 0.5), (4, 0.65), (5, 0.8)] {
        let mut s = seed;
        let mut rnd = move || {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (s >> 33) as f64 / (1u64 << 31) as f64
        };
        let mut inside = Vec::new();
        for x in -4..=4 {
            for y in -4..=4 {
                for z in -4..=4 {
                    if rnd() < density {
                        inside.push([x, y, z]);
                    }
                }
            }
        }
        let g = grid_of(&inside, 5);
        let cm = cell_mesh(&g, 0.0, Polarity::LowInside);
        let doubles = check_manifold(&cm.mesh, &format!("noise {seed}"), true);
        if seed == 1 {
            assert_eq!(doubles, 0, "a sparse field has no ring around a hole");
        }
        // the cell of each vertex is recorded; two vertices of one cell are different components
        assert_eq!(cm.cells.len(), cm.mesh.points.len());
    }
}

#[test]
fn adaptivity_keeps_the_components_of_a_cell_apart() {
    // opposite samples (1,1,1) and (2,2,2) share cell (1,1,1): two components, two vertices
    let g = grid_of(&[[1, 1, 1], [2, 2, 2]], 4);
    let cm = cell_mesh(&g, 0.0, Polarity::LowInside);
    let shared: Vec<usize> = (0..cm.cells.len())
        .filter(|&i| cm.cells[i] == IVec3::new(1, 1, 1))
        .collect();
    assert_eq!(shared.len(), 2);
    // a tolerance far above the feature size merges every block that may be merged
    let merged = merge_blocks(&cm, 1e6);
    // the shared cell's two vertices are single-vertex leaves: both are still in the output
    for i in shared {
        let p = cm.mesh.points[i];
        assert!(
            merged.points.iter().any(|q| (*q - p).length() < 1e-9),
            "vertex {i} at {p:?} was merged away"
        );
    }
    for f in &merged.polys {
        let d: BTreeSet<u32> = f.iter().copied().collect();
        assert_eq!(d.len(), f.len());
    }
}

// ---------------------------------------------------------------------------------------------
// The S5 scene of brief 206 with the default smooth layer
// ---------------------------------------------------------------------------------------------

fn s5_scene(smooth: Option<(i64, f64)>) -> (Document, NodeId) {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&forge_fields::FieldToolset).unwrap();
    r.install(&VolumePlugin).unwrap();
    let mut d = Document::new(Arc::new(r));
    let mesher = d
        .transact("s5", |tx| {
            let mesher = tx.create("forge.volume_mesher", None, None)?;
            let builder = tx.create("forge.volume_builder", Some(mesher), None)?;
            tx.set_param(builder, "voxel_size", Value::Float(10.0))?;
            tx.set_param(builder, "spline_voxel_range", Value::Int(4))?;
            // the smooth layer is the first list entry and acts on every entry below it; its
            // defaults (Gaussian, Voxel Distance 2, Strength 1) are the ones 206 found fins with
            let filter = tx.create("forge.volume_filter", Some(builder), None)?;
            tx.set_param(filter, "kind", Value::Enum("smooth".into()))?;
            tx.set_param(filter, "smooth_operator", Value::Enum("gaussian".into()))?;
            if let Some((distance, strength)) = smooth {
                tx.set_param(filter, "voxel_distance", Value::Int(distance))?;
                tx.set_param(filter, "strength", Value::Float(strength))?;
            }
            for x in [-120.0, 0.0, 120.0] {
                let s = tx.create("forge.sphere", Some(builder), None)?;
                tx.set_param(s, "radius", Value::Float(100.0))?;
                tx.set_param(s, "segments", Value::Int(64))?;
                tx.set_param(s, "transform.pos", Value::Vec3([x, 0.0, 0.0]))?;
            }
            let ring = tx.create("forge.circle_spline", Some(builder), None)?;
            tx.set_param(ring, "plane", Value::Enum("xz".into()))?;
            tx.set_param(ring, "radius", Value::Float(300.0))?;
            tx.set_param(ring, "radius_y", Value::Float(300.0))?;
            Ok(mesher)
        })
        .unwrap();
    (d, mesher)
}

#[test]
fn s5_scene_with_the_default_smooth_layer_is_closed_and_manifold() {
    let (d, mesher) = s5_scene(None);
    let m = mesh_volume_node(d.scene(), mesher, &EvalCtx::default())
        .unwrap()
        .unwrap();
    assert!(m.polys.len() > 5000, "{} faces", m.polys.len());
    assert_closed_manifold(&m, "S5, default smooth layer");
}

#[test]
fn s5_scene_with_a_wider_smooth_layer_is_manifold() {
    // a Voxel Distance of 3 (wider than the default 2) fins the tube on the pre-208 mesher
    let (d, mesher) = s5_scene(Some((3, 1.0)));
    let m = mesh_volume_node(d.scene(), mesher, &EvalCtx::default())
        .unwrap()
        .unwrap();
    assert_closed_manifold(&m, "S5, voxel distance 3");
}

#[test]
fn single_component_cells_keep_the_one_vertex_per_cell_numbering() {
    // a ball: every mixed cell has one component, so vertices and cells correspond one to one
    let mut inside = Vec::new();
    for x in -3..=3 {
        for y in -3..=3 {
            for z in -3..=3 {
                if x * x + y * y + z * z <= 9 {
                    inside.push([x, y, z]);
                }
            }
        }
    }
    let cm = cell_mesh(&grid_of(&inside, 5), 0.0, Polarity::LowInside);
    let cells: BTreeSet<[i32; 3]> = cm.cells.iter().map(|c| c.to_array()).collect();
    assert_eq!(cells.len(), cm.cells.len());
    assert_closed_manifold(&cm.mesh, "ball");
}
