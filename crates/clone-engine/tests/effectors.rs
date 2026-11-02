//! Unit tests of the effector framework and the Plain effector beyond the golden cases
//! (clone-toolset spec E1-E8).

use forge_clone_engine::*;
use forge_core::EvalCtx;
use forge_fields::FieldList;
use glam::{DMat4, DVec3};

/// Three clones along x at 0, 100, 200, the first rotated 90 degrees about Y (heading).
fn clones() -> CloneData {
    let mut d = CloneData::default();
    for i in 0..3 {
        let rot = if i == 0 {
            DMat4::from_rotation_y(-std::f64::consts::FRAC_PI_2)
        } else {
            DMat4::IDENTITY
        };
        d.push(CloneItem::at(
            DMat4::from_translation(DVec3::new(100.0 * i as f64, 0.0, 0.0)) * rot,
            DVec3::ZERO,
        ));
    }
    d
}

fn apply(plain: &Plain, mut d: CloneData, cloner_world: DMat4) -> Result<CloneData, CloneError> {
    let eval = EvalCtx::default();
    let ctx = EffectorCtx {
        eval: &eval,
        cloner_world,
    };
    plain.apply(&mut d, &FieldList::new(), &ctx)?;
    Ok(d)
}

fn pos(d: &CloneData, i: usize) -> DVec3 {
    d.matrices[i].w_axis.truncate()
}

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).length() < 1e-9, "{a:?} != {b:?}");
}

#[test]
fn node_space_follows_the_clone_axes_and_object_space_does_not() {
    // Clone 0 is turned by heading 90 degrees: its x axis points along +z.
    let node = Plain {
        position: Some(DVec3::new(50.0, 0.0, 0.0)),
        space: Space::Node,
        ..Plain::default()
    };
    let d = apply(&node, clones(), DMat4::IDENTITY).unwrap();
    close(pos(&d, 0), DVec3::new(0.0, 0.0, 50.0));
    close(pos(&d, 1), DVec3::new(150.0, 0.0, 0.0));
    let object = Plain {
        space: Space::Object,
        ..node.clone()
    };
    let d = apply(&object, clones(), DMat4::IDENTITY).unwrap();
    close(pos(&d, 0), DVec3::new(50.0, 0.0, 0.0));
}

#[test]
fn effector_space_uses_the_effector_axes_relative_to_the_cloner() {
    // The effector is turned by heading 90 degrees: its x axis points along +z.
    let effector = DMat4::from_rotation_y(-std::f64::consts::FRAC_PI_2);
    let plain = Plain {
        position: Some(DVec3::new(50.0, 0.0, 0.0)),
        space: Space::Effector,
        world: effector,
        ..Plain::default()
    };
    let d = apply(&plain, clones(), DMat4::IDENTITY).unwrap();
    close(pos(&d, 1), DVec3::new(100.0, 0.0, 50.0));
    // A cloner turned the same way sees the effector unturned.
    let d = apply(&plain, clones(), effector).unwrap();
    close(pos(&d, 1), DVec3::new(150.0, 0.0, 0.0));
}

#[test]
fn a_scaled_clone_moves_by_the_unscaled_delta() {
    let mut d = CloneData::default();
    d.push(CloneItem::at(
        DMat4::from_scale(DVec3::splat(3.0)),
        DVec3::ZERO,
    ));
    let plain = Plain::default();
    let d = apply(&plain, d, DMat4::IDENTITY).unwrap();
    close(pos(&d, 0), DVec3::new(0.0, 100.0, 0.0));
    assert!((d.matrices[0].x_axis.length() - 3.0).abs() < 1e-9);
}

#[test]
fn absolute_scale_moves_towards_the_value_and_strength_scales_it() {
    let mut d = CloneData::default();
    d.push(CloneItem::at(
        DMat4::from_scale(DVec3::splat(3.0)),
        DVec3::ZERO,
    ));
    let plain = Plain {
        position: None,
        scale: Some(DVec3::splat(1.0)),
        absolute_scale: true,
        strength: 0.5,
        ..Plain::default()
    };
    let d = apply(&plain, d, DMat4::IDENTITY).unwrap();
    assert!((d.matrices[0].x_axis.length() - 2.0).abs() < 1e-9);
}

#[test]
fn remap_uses_minimum_and_maximum() {
    let plain = Plain {
        mode: TransformMode::Remap,
        minimum: 0.25,
        maximum: 0.75,
        ..Plain::default()
    };
    let d = apply(&plain, clones(), DMat4::IDENTITY).unwrap();
    // w = 1 maps to the Maximum: 75 of the 100.
    close(pos(&d, 1), DVec3::new(100.0, 75.0, 0.0));
}

#[test]
fn visibility_hides_clones_with_a_low_weight_and_weight_and_uv_add_up() {
    let plain = Plain {
        position: None,
        visibility: true,
        strength: 0.25,
        weight: 1.0,
        u: 2.0,
        v: -1.0,
        ..Plain::default()
    };
    let d = apply(&plain, clones(), DMat4::IDENTITY).unwrap();
    assert!(d.visible.iter().all(|v| !v));
    assert!((d.weights[1] - 0.25).abs() < 1e-12);
    assert!((d.uvw[1].x - 0.5).abs() < 1e-12 && (d.uvw[1].y + 0.25).abs() < 1e-12);
    // Full strength keeps them.
    let full = Plain {
        strength: 1.0,
        ..plain
    };
    assert!(apply(&full, clones(), DMat4::IDENTITY).unwrap().visible[0]);
}

#[test]
fn options_without_rules_are_unsupported_not_ignored() {
    let eval = EvalCtx::default();
    let ctx = EffectorCtx {
        eval: &eval,
        cloner_world: DMat4::IDENTITY,
    };
    let plain = Plain {
        unsupported: Some("effector deformation modes"),
        ..Plain::default()
    };
    assert_eq!(
        plain.apply(&mut clones(), &FieldList::new(), &ctx),
        Err(CloneError::Unsupported("effector deformation modes"))
    );
}

#[test]
fn the_plain_type_is_registered_with_descriptor_defaults() {
    let ty = plain_type();
    assert_eq!(ty.category, forge_core::Category::Effector);
    assert!(ty.params.iter().all(|p| !p.label.is_empty()));
    let mut reg = forge_core::Registry::with_builtins();
    reg.install(&CloneToolset).unwrap();
    assert!(reg.get(PLAIN).is_some());
    assert!(reg.get(CLONER).unwrap().find_param("effectors").is_some());
}
