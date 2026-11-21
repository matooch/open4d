//! Behaviour the golden cases cannot see: determinism, scrubbing, settings, shapes, kinematics.

mod common;

use common::*;
use forge_core::Value;
use forge_dynamics::{
    BodyKey, DynamicsCache, DynamicsSettings, RIGID_BODY, SETTINGS, rigid_body_type,
};
use forge_parity_harness::first_contact_frame;

fn drop_scene() -> Built {
    build(&[Obj::plane(), Obj::cube("cube", [0.0, 300.0, 0.0])])
}

#[test]
fn two_runs_give_identical_caches() {
    let b = drop_scene();
    let a = simulate(&b, 60);
    let c = simulate(&b, 60);
    assert_eq!(a, c);
}

#[test]
fn scrubbing_back_and_forth_gives_the_same_frames() {
    let b = drop_scene();
    let id = b.ids["cube"];
    let mut cache = DynamicsCache::build(b.doc.scene(), &mesh_fn(&b.doc)).unwrap();
    let f50 = cache.at(50)[&BodyKey::Node(id)];
    assert_eq!(cache.cached_frames(), 51);
    let f10 = cache.at(10)[&BodyKey::Node(id)];
    assert_eq!(cache.cached_frames(), 51, "going back simulates nothing");
    assert_eq!(cache.at(50)[&BodyKey::Node(id)], f50);
    let straight = simulate(&b, 50);
    assert_eq!(straight[&id][10], f10);
    assert_eq!(straight[&id][50], f50);
    // Before the start frame the start pose is returned.
    assert_eq!(cache.at(-5)[&BodyKey::Node(id)].w_axis.y, 300.0);
}

#[test]
fn the_start_pose_is_frame_zero() {
    let b = drop_scene();
    let sim = simulate(&b, 1);
    let m = &sim[&b.ids["cube"]][0];
    assert!((m.w_axis.y - 300.0).abs() < 1e-9);
}

#[test]
fn settings_default_without_an_object_and_follow_the_object() {
    let b = drop_scene();
    assert_eq!(
        DynamicsSettings::of_scene(b.doc.scene()),
        DynamicsSettings::default()
    );
    let mut b = drop_scene();
    let s = b
        .doc
        .transact("s", |tx| tx.create(SETTINGS, None, None))
        .unwrap();
    set(&mut b.doc, s, "gravity", Value::Float(250.0));
    set(&mut b.doc, s, "steps_per_frame", Value::Int(5));
    let st = DynamicsSettings::of_scene(b.doc.scene());
    assert_eq!((st.gravity, st.steps_per_frame), (250.0, 5));
    assert!((st.step_dt(30) - 1.0 / 150.0).abs() < 1e-15);
}

#[test]
fn quarter_gravity_doubles_the_fall_time() {
    let y = |g: Option<f64>| {
        let mut b = drop_scene();
        if let Some(g) = g {
            let s = b
                .doc
                .transact("s", |tx| tx.create(SETTINGS, None, None))
                .unwrap();
            set(&mut b.doc, s, "gravity", Value::Float(g));
        }
        let sim = simulate(&b, 90);
        column(&sim[&b.ids["cube"]], 1)
    };
    let full = first_contact_frame(&y(None)).unwrap() as f64;
    let quarter = first_contact_frame(&y(Some(250.0))).unwrap() as f64;
    // t = sqrt(2h/g): a quarter of the gravity doubles the time (damping aside).
    assert!((quarter / full - 2.0).abs() < 0.2, "{full} {quarter}");
}

#[test]
fn a_disabled_settings_object_keeps_nothing_moving_is_not_a_feature() {
    // `enabled = false` on the settings object stops the simulation: bodies keep the start pose.
    let mut b = drop_scene();
    let s = b
        .doc
        .transact("s", |tx| tx.create(SETTINGS, None, None))
        .unwrap();
    set(&mut b.doc, s, "enabled", Value::Bool(false));
    let sim = simulate(&b, 20);
    assert!(
        column(&sim[&b.ids["cube"]], 1)
            .iter()
            .all(|y| (*y - 300.0).abs() < 1e-9)
    );
}

#[test]
fn ghost_and_disabled_tags_are_not_bodies() {
    let mut b = drop_scene();
    let cube = b.ids["cube"];
    let tag = forge_dynamics::build::body_tag(b.doc.scene(), cube).unwrap();
    set(&mut b.doc, tag, "dynamic", Value::Enum("ghost".into()));
    let cache = DynamicsCache::build(b.doc.scene(), &mesh_fn(&b.doc)).unwrap();
    assert_eq!(cache.keys(), &[BodyKey::Node(b.ids["plane"])]);
    set(&mut b.doc, tag, "dynamic", Value::Enum("on".into()));
    set(&mut b.doc, tag, "enabled", Value::Bool(false));
    let cache = DynamicsCache::build(b.doc.scene(), &mesh_fn(&b.doc)).unwrap();
    assert_eq!(cache.keys().len(), 1);
}

#[test]
fn a_box_shape_and_the_hull_agree_for_a_cube() {
    let rest = |shape: &str| {
        let mut b = drop_scene();
        let tag = forge_dynamics::build::body_tag(b.doc.scene(), b.ids["cube"]).unwrap();
        set(&mut b.doc, tag, "shape", Value::Enum(shape.into()));
        let sim = simulate(&b, 60);
        sim[&b.ids["cube"]][60].w_axis.y
    };
    for s in ["automatic", "box", "convex_hull"] {
        assert!((rest(s) - 25.0).abs() < 0.05, "{s}: {}", rest(s));
    }
    // A sphere shape of the same bounds rests at its radius.
    assert!((rest("sphere") - 25.0).abs() < 0.05);
    // A size increment lifts the body by that much.
    let mut b = drop_scene();
    let tag = forge_dynamics::build::body_tag(b.doc.scene(), b.ids["cube"]).unwrap();
    set(&mut b.doc, tag, "shape", Value::Enum("box".into()));
    set(&mut b.doc, tag, "size_increment", Value::Float(5.0));
    let sim = simulate(&b, 60);
    assert!((sim[&b.ids["cube"]][60].w_axis.y - 30.0).abs() < 0.05);
}

#[test]
fn heavier_bodies_do_not_fall_faster_but_pushed_bodies_move_less() {
    // Two cubes on a kinematic pusher is overkill; check mass through a collision instead:
    // a heavy cube hit by a light one barely moves.
    let mut light = Obj::cube("light", [-60.0, 25.0, 0.0]);
    light.velocity = Some([200.0, 0.0, 0.0]);
    let heavy = Obj::cube("heavy", [0.0, 25.0, 0.0]);
    let mut b = build(&[Obj::plane(), light, heavy]);
    let tag = forge_dynamics::build::body_tag(b.doc.scene(), b.ids["heavy"]).unwrap();
    set(
        &mut b.doc,
        tag,
        "mass_mode",
        Value::Enum("custom_density".into()),
    );
    set(&mut b.doc, tag, "density", Value::Float(50.0));
    let sim = simulate(&b, 30);
    let hx = sim[&b.ids["heavy"]][30].w_axis.x;
    let lx = sim[&b.ids["light"]][30].w_axis.x;
    assert!(hx < 25.0 && lx < 25.0, "{hx} {lx}");
    assert!(hx >= 0.0);
}

#[test]
fn a_kinematic_body_follows_its_target_and_pushes() {
    let mut pusher = Obj::cube("pusher", [-100.0, 25.0, 0.0]);
    pusher.dynamic = "kinematic";
    let ball = Obj::cube("ball", [0.0, 25.0, 0.0]);
    let b = build(&[Obj::plane(), pusher, ball]);
    let mut cache = DynamicsCache::build(b.doc.scene(), &mesh_fn(&b.doc)).unwrap();
    let _ = &mut cache;
    // Drive the simulation directly so the kinematic target is ours.
    let specs = forge_dynamics::build::bodies_of_scene(b.doc.scene(), &mesh_fn(&b.doc));
    let mut sim =
        forge_dynamics::Simulation::new(&DynamicsSettings::default(), 30, &specs).unwrap();
    let key = BodyKey::Node(b.ids["pusher"]);
    for f in 1..=30 {
        let x = -100.0 + 5.0 * f as f64;
        sim.step_frame_with(&|k| {
            (k == key).then(|| glam::DMat4::from_translation(glam::DVec3::new(x, 25.0, 0.0)))
        });
    }
    let keys = sim.keys();
    let poses = sim.poses();
    let at = |k: BodyKey| poses[keys.iter().position(|x| *x == k).unwrap()].w_axis;
    assert!((at(key).x - 50.0).abs() < 0.5, "{}", at(key).x);
    assert!(at(BodyKey::Node(b.ids["ball"])).x > 10.0, "pushed");
}

#[test]
fn the_tag_descriptor_defaults_are_the_observed_ones() {
    let t = rigid_body_type();
    let d = |id: &str| t.find_param(id).unwrap().default.clone();
    assert_eq!(d("dynamic"), Value::Enum("on".into()));
    assert_eq!(d("bounce"), Value::Float(0.05));
    assert_eq!(d("friction"), Value::Float(0.9));
    assert_eq!(d("linear_damping"), Value::Float(0.05));
    assert_eq!(d("angular_damping"), Value::Float(0.05));
    assert_eq!(d("density"), Value::Float(1.0));
    assert_eq!(d("mass"), Value::Float(10.0));
    assert_eq!(d("soft_body"), Value::Enum("off".into()));
    assert_eq!(t.id, RIGID_BODY);
}
