//! Culling maths, instance layout and draw lists, the revision cache, LOD and instance picking
//! (docs/specs/VIEWPORT.md §10). Pure: no GPU.

mod common;

use common::{boxed, close, unit_box};
use forge_geometry::PolyMesh;
use forge_viewport::cache::{RevisionCache, combine, mesh_revision};
use forge_viewport::camera::{Camera, Projection, Ray};
use forge_viewport::cull::{Frustum, distance_to_aabb, transform_aabb};
use forge_viewport::instancing::{InstanceInput, LodView, build_layout, visible_ranges};
use forge_viewport::lod::{lod_proxy, screen_diameter_px, use_proxy};
use forge_viewport::picking::{pick_instances, pick_poly};
use glam::{DMat4, DQuat, DVec3};

const W: f64 = 800.0;
const H: f64 = 600.0;

fn frustum(cam: &Camera) -> Frustum {
    Frustum::from_view_proj(&cam.view_proj(W / H))
}

// ---- frustum ---------------------------------------------------------------------------------

#[test]
fn frustum_contains_points_in_front_and_rejects_others() {
    // eye (0,0,10) looking at the origin, fov 45 degrees
    let f = frustum(&Camera::default());
    assert!(f.contains_point(DVec3::ZERO));
    assert!(f.contains_point(DVec3::new(2.0, 1.0, 5.0)));
    assert!(
        !f.contains_point(DVec3::new(0.0, 0.0, 11.0)),
        "behind the eye"
    );
    assert!(
        !f.contains_point(DVec3::new(100.0, 0.0, 0.0)),
        "right of the view"
    );
    assert!(
        !f.contains_point(DVec3::new(-100.0, 0.0, 0.0)),
        "left of the view"
    );
    assert!(!f.contains_point(DVec3::new(0.0, 100.0, 0.0)), "above");
    assert!(!f.contains_point(DVec3::new(0.0, -100.0, 0.0)), "below");
    assert!(
        !f.contains_point(DVec3::new(0.0, 0.0, -1.0e6)),
        "beyond the far plane"
    );
}

#[test]
fn frustum_agrees_with_projection_for_perspective_and_orthographic() {
    for projection in [
        Projection::Perspective { fov_y: 0.8 },
        Projection::Orthographic { height: 6.0 },
    ] {
        let cam = Camera {
            yaw: 0.4,
            pitch: 0.3,
            distance: 9.0,
            projection,
            ..Camera::default()
        };
        let f = frustum(&cam);
        let mut checked = 0;
        for i in -6..=6 {
            for j in -6..=6 {
                for k in -3..=3 {
                    let p = DVec3::new(i as f64, j as f64, k as f64) * 1.7;
                    let Some(s) = cam.project(p, W, H) else {
                        assert!(!f.contains_point(p), "behind the camera: {p}");
                        continue;
                    };
                    // stay away from the borders: exact agreement is not required on them
                    let (nx, ny) = (s.x / W, s.y / H);
                    let well_in = (0.02..0.98).contains(&nx)
                        && (0.02..0.98).contains(&ny)
                        && (0.0..1.0).contains(&s.z);
                    let well_out = !(-0.02..1.02).contains(&nx) || !(-0.02..1.02).contains(&ny);
                    if well_in {
                        assert!(f.contains_point(p), "{p} projects inside");
                        checked += 1;
                    } else if well_out {
                        assert!(!f.contains_point(p), "{p} projects outside");
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 100, "{checked}");
    }
}

#[test]
fn aabb_test_keeps_straddling_and_touching_boxes_and_rejects_outside_ones() {
    let f = frustum(&Camera::default());
    let b = |c: DVec3, h: f64| (c - DVec3::splat(h), c + DVec3::splat(h));
    let inside = b(DVec3::ZERO, 1.0);
    assert!(f.intersects_aabb(inside.0, inside.1));
    // a big box that contains the whole frustum has no plane that excludes it
    let huge = b(DVec3::ZERO, 1.0e6);
    assert!(f.intersects_aabb(huge.0, huge.1));
    // straddling the right plane: at z = 0 the half width is 10*tan(22.5deg)*aspect = 5.52
    let half_w = 10.0 * (std::f64::consts::FRAC_PI_8).tan() * W / H;
    let straddle = b(DVec3::new(half_w, 0.0, 0.0), 1.0);
    assert!(f.intersects_aabb(straddle.0, straddle.1));
    let outside_r = b(DVec3::new(half_w + 3.0, 0.0, 0.0), 1.0);
    assert!(!f.intersects_aabb(outside_r.0, outside_r.1));
    let outside_l = b(DVec3::new(-half_w - 3.0, 0.0, 0.0), 1.0);
    assert!(!f.intersects_aabb(outside_l.0, outside_l.1));
    let above = b(DVec3::new(0.0, 20.0, 0.0), 1.0);
    assert!(!f.intersects_aabb(above.0, above.1));
    let below = b(DVec3::new(0.0, -20.0, 0.0), 1.0);
    assert!(!f.intersects_aabb(below.0, below.1));
    let behind = b(DVec3::new(0.0, 0.0, 30.0), 1.0);
    assert!(!f.intersects_aabb(behind.0, behind.1), "behind the eye");
    let far = b(DVec3::new(0.0, 0.0, -2.0e5), 1.0);
    assert!(
        !f.intersects_aabb(far.0, far.1),
        "beyond far (1e4 * distance = 1e5)"
    );
    // a box that touches the near/right plane exactly is kept (padding)
    let touching = b(DVec3::new(half_w + 1.0, 0.0, 0.0), 1.0);
    assert!(f.intersects_aabb(touching.0, touching.1));
}

#[test]
fn aabb_test_never_rejects_a_box_with_a_point_inside() {
    let cam = Camera {
        yaw: -0.7,
        pitch: 0.5,
        distance: 12.0,
        ..Camera::default()
    };
    let f = frustum(&cam);
    // deterministic pseudo-random boxes
    let mut state = 12345u64;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
    };
    let mut seen_inside = 0;
    for _ in 0..2000 {
        let c = DVec3::new(next(), next(), next()) * 15.0;
        let h = DVec3::new(next().abs(), next().abs(), next().abs()) * 2.0;
        let (lo, hi) = (c - h, c + h);
        if f.contains_point(c) {
            seen_inside += 1;
            assert!(f.intersects_aabb(lo, hi), "box around an inside point: {c}");
        }
    }
    assert!(seen_inside > 100);
}

#[test]
fn sphere_test_matches_the_plane_distances() {
    let f = frustum(&Camera::default());
    assert!(f.intersects_sphere(DVec3::ZERO, 0.5));
    assert!(!f.intersects_sphere(DVec3::new(0.0, 0.0, 30.0), 1.0));
    assert!(
        f.intersects_sphere(DVec3::new(0.0, 0.0, 10.5), 1.0),
        "reaches the eye plane"
    );
}

#[test]
fn transform_aabb_is_the_box_of_the_transformed_corners() {
    let (lo, hi) = (DVec3::new(-1.0, -2.0, -0.5), DVec3::new(3.0, 1.0, 2.0));
    let m = DMat4::from_scale_rotation_translation(
        DVec3::new(2.0, 0.5, -1.5),
        DQuat::from_euler(glam::EulerRot::YXZ, 0.7, -0.3, 1.1),
        DVec3::new(4.0, -2.0, 7.0),
    );
    let (tlo, thi) = transform_aabb(&m, lo, hi);
    let mut elo = DVec3::splat(f64::INFINITY);
    let mut ehi = DVec3::splat(f64::NEG_INFINITY);
    for i in 0..8 {
        let c = DVec3::new(
            if i & 1 == 1 { hi.x } else { lo.x },
            if i & 2 == 2 { hi.y } else { lo.y },
            if i & 4 == 4 { hi.z } else { lo.z },
        );
        let t = m.transform_point3(c);
        elo = elo.min(t);
        ehi = ehi.max(t);
    }
    close(tlo, elo, 1e-12);
    close(thi, ehi, 1e-12);
}

#[test]
fn distance_to_a_box() {
    let (lo, hi) = (DVec3::splat(-1.0), DVec3::splat(1.0));
    assert_eq!(distance_to_aabb(DVec3::ZERO, lo, hi), 0.0);
    assert!((distance_to_aabb(DVec3::new(4.0, 0.0, 0.0), lo, hi) - 3.0).abs() < 1e-12);
    assert!((distance_to_aabb(DVec3::new(4.0, 5.0, 0.0), lo, hi) - 5.0).abs() < 1e-12);
}

// ---- instance layout and draw lists ------------------------------------------------------------

struct Clones {
    matrices: Vec<DMat4>,
    colors: Vec<[f64; 3]>,
    visible: Vec<bool>,
    child: Vec<u32>,
}

impl Clones {
    fn input(&self) -> InstanceInput<'_> {
        InstanceInput {
            matrices: &self.matrices,
            colors: &self.colors,
            visible: &self.visible,
            child: &self.child,
        }
    }
}

/// An `n x n` grid of clones in the XZ plane, spacing 3.
fn grid(n: usize) -> Clones {
    let mut c = Clones {
        matrices: vec![],
        colors: vec![],
        visible: vec![],
        child: vec![],
    };
    for i in 0..n {
        for j in 0..n {
            c.matrices.push(DMat4::from_translation(DVec3::new(
                i as f64 * 3.0,
                0.0,
                j as f64 * 3.0,
            )));
            c.colors.push([1.0, i as f64 / n as f64, 0.0]);
            c.visible.push(true);
            c.child.push(0);
        }
    }
    c
}

const UNIT: Option<(DVec3, DVec3)> =
    Some((DVec3::new(-1.0, -1.0, -1.0), DVec3::new(1.0, 1.0, 1.0)));

#[test]
fn layout_has_one_slot_per_visible_clone_and_valid_chunks() {
    let mut c = grid(30); // 900 clones
    c.visible[5] = false;
    c.visible[100] = false;
    c.child[7] = 1;
    c.child[8] = 9; // no such prototype: skipped
    let l = build_layout(&[UNIT, UNIT], &c.input(), &DMat4::IDENTITY, 128);
    assert_eq!(l.raw.len(), 900 - 2 - 1);
    assert_eq!(l.clone_of_slot.len(), l.raw.len());
    // every visible clone with a prototype appears once
    let mut seen = l.clone_of_slot.clone();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), l.raw.len());
    assert!(!l.clone_of_slot.contains(&5) && !l.clone_of_slot.contains(&100));
    assert!(!l.clone_of_slot.contains(&8));
    // chunks tile the slots in order, never mix prototypes, never exceed the size
    let mut next = 0;
    for ch in &l.chunks {
        assert_eq!(ch.first, next);
        assert!(ch.count >= 1 && ch.count <= 128);
        next += ch.count;
        for slot in ch.first..ch.first + ch.count {
            let clone = l.clone_of_slot[slot as usize] as usize;
            assert_eq!(c.child[clone], ch.proto);
            // the member's world box is inside the chunk box
            let m = c.matrices[clone];
            let (lo, hi) = transform_aabb(&m, DVec3::splat(-1.0), DVec3::splat(1.0));
            assert!(ch.min.cmple(lo + 1e-9).all() && ch.max.cmpge(hi - 1e-9).all());
            assert!(ch.max_radius >= (hi - lo).length() / 2.0 - 1e-9);
        }
    }
    assert_eq!(next as usize, l.raw.len());
    // prototype groups ascend
    assert!(l.chunks.windows(2).all(|w| w[0].proto <= w[1].proto));
    let (lo, hi) = l.bounds.unwrap();
    assert!(
        l.chunks
            .iter()
            .all(|ch| lo.cmple(ch.min).all() && hi.cmpge(ch.max).all())
    );
}

#[test]
fn layout_is_deterministic_and_applies_the_world_matrix() {
    let c = grid(12);
    let world = DMat4::from_translation(DVec3::new(0.0, 5.0, 0.0));
    let a = build_layout(&[UNIT], &c.input(), &world, 32);
    let b = build_layout(&[UNIT], &c.input(), &world, 32);
    assert_eq!(a.clone_of_slot, b.clone_of_slot);
    assert_eq!(a.raw, b.raw);
    // clone 0 sits at (0, 5, 0): row i = (linear part, translation)
    let slot = a.clone_of_slot.iter().position(|c| *c == 0).unwrap();
    let r = a.raw[slot];
    assert_eq!(r.row1, [0.0, 1.0, 0.0, 5.0]);
    assert_eq!(r.row0, [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(r.color, [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn spatial_order_keeps_chunks_compact() {
    // a 60 x 60 grid in chunks of 100: Morton order gives chunk boxes far smaller than the
    // whole field (a row-major order would give full-width slabs)
    let c = grid(60);
    let l = build_layout(&[UNIT], &c.input(), &DMat4::IDENTITY, 100);
    let whole = l.bounds.unwrap();
    let whole_area = (whole.1.x - whole.0.x) * (whole.1.z - whole.0.z);
    let mean_area: f64 = l
        .chunks
        .iter()
        .map(|ch| (ch.max.x - ch.min.x) * (ch.max.z - ch.min.z))
        .sum::<f64>()
        / l.chunks.len() as f64;
    assert!(mean_area < whole_area / 10.0, "{mean_area} vs {whole_area}");
}

#[test]
fn draw_list_culls_far_chunks_and_merges_neighbours() {
    let c = grid(60);
    let l = build_layout(&[UNIT], &c.input(), &DMat4::IDENTITY, 100);
    // a camera over the field's corner looking at a 10-unit patch
    let cam = Camera {
        target: DVec3::new(5.0, 0.0, 5.0),
        distance: 15.0,
        pitch: 1.2,
        ..Camera::default()
    };
    let f = frustum(&cam);
    let r = visible_ranges(&l.chunks, &f, None);
    let drawn: u32 = r.iter().map(|r| r.count).sum();
    assert!(drawn > 0 && (drawn as usize) < l.raw.len() / 2, "{drawn}");
    // the target clone's slot is drawn; a clone at the far corner is not
    let slot_of = |clone: u32| l.clone_of_slot.iter().position(|c| *c == clone).unwrap() as u32;
    let covered = |slot: u32| {
        r.iter()
            .any(|r| slot >= r.first && slot < r.first + r.count)
    };
    assert!(covered(slot_of(2 * 60 + 2)));
    assert!(!covered(slot_of(59 * 60 + 59)));
    // fewer ranges than visible chunks when neighbours merge, and ranges never overlap
    assert!(r.windows(2).all(|w| w[0].first + w[0].count <= w[1].first));
    // a camera that sees everything draws everything in one range
    let wide = Camera {
        target: DVec3::new(90.0, 0.0, 90.0),
        distance: 600.0,
        pitch: 1.5,
        ..Camera::default()
    };
    let all = visible_ranges(&l.chunks, &frustum(&wide), None);
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].first, all[0].count as usize), (0, l.raw.len()));
    // looking at a point far from the field draws nothing
    let elsewhere = Camera {
        target: DVec3::new(-5000.0, 0.0, -5000.0),
        distance: 15.0,
        ..Camera::default()
    };
    assert!(visible_ranges(&l.chunks, &frustum(&elsewhere), None).is_empty());
}

#[test]
fn lod_flag_splits_ranges_by_distance() {
    let c = grid(60);
    let l = build_layout(&[UNIT], &c.input(), &DMat4::IDENTITY, 100);
    let cam = Camera {
        target: DVec3::new(90.0, 0.0, 90.0),
        distance: 400.0,
        pitch: 1.5,
        ..Camera::default()
    };
    let f = frustum(&cam);
    let near = |min_px| {
        visible_ranges(
            &l.chunks,
            &f,
            Some(LodView {
                camera: &cam,
                viewport_h: H,
                min_px,
            }),
        )
    };
    // a 2-unit cube is about 2 * 600 / (2 * 400 * tan(22.5deg)) = 3.6 px at 400 units
    assert!(near(0.0).iter().all(|r| !r.lod));
    assert!(near(100.0).iter().all(|r| r.lod));
    assert!(near(100.0).iter().map(|r| r.count).sum::<u32>() as usize == l.raw.len());
}

// ---- revision cache ---------------------------------------------------------------------------

#[test]
fn cache_reuses_the_same_revision_and_rebuilds_a_changed_one() {
    let mut cache: RevisionCache<(u64, u8), String> = RevisionCache::new();
    let builds = std::cell::Cell::new(0);
    let get = |cache: &mut RevisionCache<(u64, u8), String>, key, rev| {
        cache.get_or_insert_with(key, rev, || {
            builds.set(builds.get() + 1);
            format!("v{rev}")
        })
    };
    assert_eq!(get(&mut cache, (1, 0), 10), "v10");
    assert_eq!(get(&mut cache, (1, 0), 10), "v10");
    assert_eq!(
        get(&mut cache, (2, 0), 10),
        "v10",
        "another key builds on its own"
    );
    assert_eq!(get(&mut cache, (1, 0), 11), "v11", "new revision rebuilds");
    assert_eq!(get(&mut cache, (1, 0), 11), "v11");
    assert_eq!(
        get(&mut cache, (1, 1), 11),
        "v11",
        "the kind is part of the key"
    );
    let stats = cache.stats();
    assert_eq!((stats.uploads, stats.hits), (4, 2));
    assert_eq!(builds.get(), 4);
    // an older revision is not resurrected
    assert_eq!(get(&mut cache, (1, 0), 10), "v10");
    assert_eq!(cache.stats().uploads, 5);
}

#[test]
fn cache_retain_drops_gone_keys_and_clear_empties() {
    let mut cache: RevisionCache<u64, u32> = RevisionCache::new();
    for k in 0..5 {
        cache.get_or_insert_with(k, 1, || k as u32);
    }
    assert_eq!(cache.len(), 5);
    cache.retain(|k| *k % 2 == 0);
    assert_eq!(cache.len(), 3);
    assert!(cache.contains(&2) && !cache.contains(&3));
    cache.reset_stats();
    cache.get_or_insert_with(3, 1, || 3);
    assert_eq!(cache.stats().uploads, 1, "a dropped key uploads again");
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn mesh_revision_follows_the_content_only() {
    let a = unit_box();
    let b = unit_box();
    assert_eq!(mesh_revision(&a), mesh_revision(&b));
    let mut moved = unit_box();
    moved.points[3].x += 1e-9;
    assert_ne!(mesh_revision(&a), mesh_revision(&moved));
    let mut rewired = unit_box();
    rewired.polys[0].swap(0, 1);
    assert_ne!(mesh_revision(&a), mesh_revision(&rewired));
    let mut extra = unit_box();
    extra.polys.push(vec![0, 1, 2]);
    assert_ne!(mesh_revision(&a), mesh_revision(&extra));
    assert_ne!(combine(1, 2), combine(2, 1));
    assert_eq!(combine(7, 9), combine(7, 9));
}

// ---- LOD --------------------------------------------------------------------------------------

/// An n x n quad grid in the XZ plane, 1 unit wide.
fn quad_grid(n: u32) -> PolyMesh {
    let mut points = vec![];
    for i in 0..=n {
        for j in 0..=n {
            points.push(DVec3::new(i as f64 / n as f64, 0.0, j as f64 / n as f64));
        }
    }
    let mut polys = vec![];
    for i in 0..n {
        for j in 0..n {
            let a = i * (n + 1) + j;
            polys.push(vec![a, a + 1, a + n + 2, a + n + 1]);
        }
    }
    PolyMesh { points, polys }
}

#[test]
fn lod_proxy_reduces_polygons_and_stays_close() {
    let m = quad_grid(64);
    let p = lod_proxy(&m, 8);
    assert!(!p.polys.is_empty());
    assert!(p.polys.len() <= 64, "{}", p.polys.len());
    assert!(p.polys.len() < m.polys.len() / 32);
    assert!(p.polys.iter().all(|f| f.len() >= 3));
    assert!(
        p.polys
            .iter()
            .flatten()
            .all(|v| (*v as usize) < p.points.len())
    );
    // every proxy point lies within the original box
    assert!(
        p.points
            .iter()
            .all(|q| q.x >= -1e-9 && q.x <= 1.0 + 1e-9 && q.y.abs() < 1e-9)
    );
    // deterministic
    assert_eq!(p, lod_proxy(&m, 8));
    // a single cell collapses everything
    assert!(lod_proxy(&m, 1).polys.is_empty());
    // fine cells keep the mesh
    let same = lod_proxy(&quad_grid(4), 64);
    assert_eq!(same.polys.len(), 16);
    assert_eq!(lod_proxy(&PolyMesh::default(), 8), PolyMesh::default());
}

#[test]
fn screen_size_follows_distance_and_projection() {
    let cam = Camera::default(); // fov 45 degrees, viewport 600
    let near = screen_diameter_px(1.0, 10.0, &cam, 600.0);
    let far = screen_diameter_px(1.0, 20.0, &cam, 600.0);
    assert!((near / far - 2.0).abs() < 1e-9);
    // 2 * 1 * 600 / (2 * 10 * tan(22.5 deg)) = 144.84
    assert!((near - 144.84).abs() < 0.1, "{near}");
    let ortho = Camera {
        projection: Projection::Orthographic { height: 12.0 },
        ..Camera::default()
    };
    assert!((screen_diameter_px(1.0, 5.0, &ortho, 600.0) - 100.0).abs() < 1e-9);
    assert!(
        (screen_diameter_px(1.0, 500.0, &ortho, 600.0) - 100.0).abs() < 1e-9,
        "independent of distance"
    );
    let (lo, hi) = (DVec3::splat(-0.5), DVec3::splat(0.5));
    assert!(!use_proxy(lo, hi, &cam, 600.0, 0.0), "0 = off");
    assert!(use_proxy(lo, hi, &cam, 600.0, 500.0));
    assert!(!use_proxy(lo, hi, &cam, 600.0, 5.0));
}

// ---- picking instances ------------------------------------------------------------------------

fn expanded(protos: &[PolyMesh], c: &Clones, world: &DMat4) -> PolyMesh {
    let mut out = PolyMesh::default();
    for i in 0..c.matrices.len() {
        if !c.visible[i] {
            continue;
        }
        let proto = &protos[c.child[i] as usize];
        let base = out.points.len() as u32;
        let m = *world * c.matrices[i];
        out.points
            .extend(proto.points.iter().map(|p| m.transform_point3(*p)));
        out.polys.extend(
            proto
                .polys
                .iter()
                .map(|f| f.iter().map(|v| v + base).collect()),
        );
    }
    out
}

fn ray_down(x: f64, z: f64) -> Ray {
    Ray {
        origin: DVec3::new(x, 20.0, z),
        dir: DVec3::NEG_Y,
    }
}

#[test]
fn pick_instances_finds_the_clone_under_the_ray() {
    let protos = vec![unit_box()];
    let c = grid(5); // spacing 3, clone (i, j) at (3i, 0, 3j)
    let hit = pick_instances(&protos, &c.input(), &DMat4::IDENTITY, &ray_down(6.1, 9.2)).unwrap();
    assert_eq!(hit.clone, 2 * 5 + 3);
    assert_eq!(hit.proto, 0);
    assert!((hit.t - 19.0).abs() < 1e-9, "top face at y = 1: {}", hit.t);
    assert!(
        pick_instances(&protos, &c.input(), &DMat4::IDENTITY, &ray_down(1.5, 1.5)).is_none(),
        "between clones"
    );
}

#[test]
fn pick_instances_skips_hidden_clones_and_unknown_prototypes() {
    let protos = vec![unit_box()];
    let mut c = grid(3);
    c.visible[4] = false; // the middle clone (3, 0, 3)
    assert!(pick_instances(&protos, &c.input(), &DMat4::IDENTITY, &ray_down(3.0, 3.0)).is_none());
    c.visible[4] = true;
    c.child[4] = 7;
    assert!(pick_instances(&protos, &c.input(), &DMat4::IDENTITY, &ray_down(3.0, 3.0)).is_none());
}

#[test]
fn pick_instances_nearest_wins_and_ties_go_to_the_lower_index() {
    let protos = vec![unit_box()];
    let mut c = grid(1);
    // two clones stacked: index 1 is higher (nearer to the ray origin)
    c.matrices
        .push(DMat4::from_translation(DVec3::new(0.0, 5.0, 0.0)));
    c.colors.push([1.0; 3]);
    c.visible.push(true);
    c.child.push(0);
    let hit = pick_instances(&protos, &c.input(), &DMat4::IDENTITY, &ray_down(0.0, 0.0)).unwrap();
    assert_eq!(hit.clone, 1);
    assert!((hit.t - 14.0).abs() < 1e-9);
    // identical clones: the lower index wins
    c.matrices[1] = c.matrices[0];
    let tie = pick_instances(&protos, &c.input(), &DMat4::IDENTITY, &ray_down(0.0, 0.0)).unwrap();
    assert_eq!(tie.clone, 0);
}

#[test]
fn pick_instances_matches_picking_the_expanded_mesh() {
    // scaled, rotated and two prototypes, under a world transform
    let protos = vec![
        unit_box(),
        boxed(DVec3::new(-0.5, 0.0, -0.5), DVec3::new(0.5, 3.0, 0.5)),
    ];
    let mut c = grid(6);
    for (i, m) in c.matrices.iter_mut().enumerate() {
        let rot = DQuat::from_rotation_y(i as f64 * 0.37);
        let scale = DVec3::new(1.0 + (i % 3) as f64 * 0.4, 0.5 + (i % 4) as f64 * 0.3, 1.2);
        *m *= DMat4::from_scale_rotation_translation(scale, rot, DVec3::ZERO);
        c.child[i] = (i % 2) as u32;
        c.visible[i] = i % 7 != 3;
    }
    let world = DMat4::from_translation(DVec3::new(-4.0, 1.0, 2.0)) * DMat4::from_rotation_z(0.2);
    let flat = expanded(&protos, &c, &world);
    let cam = Camera {
        target: DVec3::new(6.0, 0.0, 6.0),
        distance: 30.0,
        yaw: 0.5,
        pitch: 0.9,
        ..Camera::default()
    };
    let (mut hits, mut misses) = (0, 0);
    for ix in 0..24 {
        for iy in 0..18 {
            let ray = cam.ray(ix as f64 * 33.0 + 7.0, iy as f64 * 33.0 + 5.0, W, H);
            let a = pick_instances(&protos, &c.input(), &world, &ray);
            let b = pick_poly(&flat, &ray);
            match (a, b) {
                (Some(a), Some(b)) => {
                    assert!((a.t - b.t).abs() < 1e-7, "{} vs {}", a.t, b.t);
                    hits += 1;
                }
                (None, None) => misses += 1,
                other => panic!("instance pick and expanded pick disagree: {other:?}"),
            }
        }
    }
    assert!(hits > 20 && misses > 20, "{hits} hits, {misses} misses");
}

#[test]
fn instances_revision_ignores_hidden_clones_and_sees_everything_else() {
    use forge_viewport::cache::instances_revision;
    let mut c = grid(4);
    c.visible[3] = false;
    let base = instances_revision(&[1, 2], &c.input(), &DMat4::IDENTITY);
    assert_eq!(
        base,
        instances_revision(&[1, 2], &c.input(), &DMat4::IDENTITY)
    );
    // a hidden clone's matrix or colour is irrelevant
    let mut hidden_moved = grid(4);
    hidden_moved.visible[3] = false;
    hidden_moved.matrices[3] = DMat4::from_translation(DVec3::splat(77.0));
    hidden_moved.colors[3] = [0.1, 0.2, 0.3];
    assert_eq!(
        base,
        instances_revision(&[1, 2], &hidden_moved.input(), &DMat4::IDENTITY)
    );
    // anything else changes it
    let mut moved = grid(4);
    moved.visible[3] = false;
    moved.matrices[5] = DMat4::from_translation(DVec3::splat(77.0));
    assert_ne!(
        base,
        instances_revision(&[1, 2], &moved.input(), &DMat4::IDENTITY)
    );
    let mut recoloured = grid(4);
    recoloured.visible[3] = false;
    recoloured.colors[5] = [0.0; 3];
    assert_ne!(
        base,
        instances_revision(&[1, 2], &recoloured.input(), &DMat4::IDENTITY)
    );
    let mut reassigned = grid(4);
    reassigned.visible[3] = false;
    reassigned.child[5] = 1;
    assert_ne!(
        base,
        instances_revision(&[1, 2], &reassigned.input(), &DMat4::IDENTITY)
    );
    let mut shown = grid(4);
    shown.visible[3] = true;
    assert_ne!(
        base,
        instances_revision(&[1, 2], &shown.input(), &DMat4::IDENTITY)
    );
    assert_ne!(
        base,
        instances_revision(&[1, 3], &c.input(), &DMat4::IDENTITY),
        "prototype changed"
    );
    assert_ne!(
        base,
        instances_revision(&[1, 2], &c.input(), &DMat4::from_translation(DVec3::X)),
        "world matrix"
    );
}
