//! Remapping rules that the golden cases do not see (docs/specs/FIELDS.md F4).

use forge_fields::{Contour, Remap};

fn remap() -> Remap {
    Remap::with_inner_offset(0.0)
}

#[test]
fn defaults_are_the_identity_on_the_ramp() {
    let r = remap();
    for t in [0.0, 0.1, 0.5, 0.99, 1.0] {
        assert_eq!(r.apply(t), t);
    }
}

#[test]
fn disabled_remapping_passes_the_ramp_through() {
    let r = Remap {
        enabled: false,
        invert: true,
        inner_offset: 0.5,
        ..remap()
    };
    assert_eq!(r.apply(0.25), 0.25);
}

#[test]
fn quadratic_with_a_curve_is_monotonic_and_fixes_the_ends() {
    for curve in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let r = Remap {
            contour: Contour::Quadratic,
            curve,
            ..remap()
        };
        let mut last = r.apply(0.0);
        assert!(last.abs() < 1e-12);
        for i in 1..=100 {
            let v = r.apply(f64::from(i) / 100.0);
            assert!(v >= last - 1e-12, "curve {curve} at {i}");
            last = v;
        }
        assert!((last - 1.0).abs() < 1e-12);
        assert!((r.apply(0.5) - 0.5).abs() < 1e-12);
    }
}

#[test]
fn clamps_limit_the_result_and_can_be_switched_off() {
    let r = Remap {
        multiplier: 3.0,
        ..remap()
    };
    assert_eq!(r.apply(0.9), 1.0);
    let r = Remap {
        clamp_max: false,
        ..r
    };
    assert!((r.apply(0.9) - 2.7).abs() < 1e-12);
}

#[test]
fn inner_offset_one_makes_a_hard_edge() {
    let r = Remap::with_inner_offset(1.0);
    assert_eq!(r.apply(0.0), 1.0);
    assert_eq!(r.apply(0.3), 1.0);
}
