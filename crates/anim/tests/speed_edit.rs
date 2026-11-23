use forge_anim::{SpeedSide, speed_at, speed_at_key, tangent};
use forge_core::{Interp, Key, Track};

#[test]
fn speed_edit_sets_both_one_sided_slopes_without_moving_keys() {
    let mut track = Track::new(vec![
        Key::new(0.0, 0.0),
        Key::new(1.25, 80.0),
        Key::new(4.0, 150.0),
    ]);
    let keys = track.keys.clone();
    let original = tangent::zero_length(track.tangents.get(1).copied().unwrap_or_default());
    track.set_tangent(1, original);
    let tangent = tangent::set_speed(&track, 1, SpeedSide::Right, 23.75).unwrap();
    assert_eq!(tangent.right.dt, (4.0 - 1.25) / 3.0);
    assert_eq!(tangent.left.dt, -1.25 / 3.0);
    track.set_tangent(1, tangent);
    assert_eq!(track.keys, keys);
    for side in [SpeedSide::Left, SpeedSide::Right] {
        assert!((speed_at_key(&track, 1, side).unwrap() - 23.75).abs() < 1e-6);
    }
    for t in [1.25 - 1e-10, 1.25 + 1e-10] {
        assert!((speed_at(&track, t) - 23.75).abs() < 1e-3);
    }
}

#[test]
fn broken_weighted_handle_keeps_its_length_and_leaves_the_other_side_alone() {
    let mut track = Track::new(vec![
        Key::new(0.0, 0.0),
        Key::new(1.0, 80.0),
        Key::new(3.0, 150.0),
    ]);
    let mut original = tangent::ease(&track.keys, 1);
    original.broken = true;
    original.weighted = true;
    original.right.dt = 0.125;
    track.set_tangent(1, original);
    let edited = tangent::set_speed(&track, 1, SpeedSide::Right, -7.0).unwrap();
    assert_eq!(edited.left, original.left);
    assert_eq!(edited.right.dt, 0.125);
    assert!(edited.weighted && edited.broken);
    track.set_tangent(1, edited);
    assert!((speed_at_key(&track, 1, SpeedSide::Right).unwrap() + 7.0).abs() < 1e-6);
    track.keys[1].interp = Interp::Linear;
    assert_eq!(speed_at_key(&track, 1, SpeedSide::Right), Some(35.0));
    assert!(tangent::set_speed(&track, 1, SpeedSide::Right, -7.0).is_none());
    assert!(tangent::set_speed(&track, 1, SpeedSide::Left, f64::INFINITY).is_none());
}
