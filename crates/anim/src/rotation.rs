//! Rotation tracks: the Euler filter and the quaternion interpolation option
//! (docs/specs/ANIM.md section 1.11).

use forge_core::transform::{Transform, hpb_from_rotation};
use forge_core::{Key, Track};
use glam::{DMat3, DQuat, DVec3};

use crate::value_at_exact;

/// Makes the key values of an angle track continuous: each key is shifted by a whole
/// number of turns so it lies within half a turn of the previous key (the Euler filter).
/// Values only; times, interpolation and tangents are kept. Returns whether a key moved.
pub fn euler_filter(track: &mut Track) -> bool {
    let tau = std::f64::consts::TAU;
    let mut changed = false;
    for i in 1..track.keys.len() {
        let prev = track.keys[i - 1].value;
        let v = track.keys[i].value;
        let shifted = v - tau * ((v - prev) / tau).round();
        if shifted != v {
            track.keys[i].value = shifted;
            changed = true;
        }
    }
    changed
}

/// The rotation `(h, p, b)` in radians at `time` with the three components interpolated as
/// one orientation: between two key times (the union of the keys of the three tracks) the
/// orientations at those times are blended by the shortest-arc spherical interpolation, the
/// blend fraction following the segment's own curve (the first track that has a key there,
/// as a 0 to 1 curve with that key's interpolation and tangents). Outside the keys and for
/// components without a track the plain component values hold (`base`).
pub fn quaternion_rotation_at(tracks: [Option<&Track>; 3], base: DVec3, time: f64) -> DVec3 {
    let comp = |c: usize, t: f64| tracks[c].map_or(base[c], |tr| value_at_exact(tr, &[], t));
    let euler = |t: f64| DVec3::new(comp(0, t), comp(1, t), comp(2, t));
    let mut times: Vec<f64> = tracks
        .iter()
        .flatten()
        .flat_map(|t| t.keys.iter().map(|k| k.time))
        .collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    let j = times.partition_point(|t| *t <= time);
    if j == 0 || j == times.len() {
        return euler(time);
    }
    let (ta, tb) = (times[j - 1], times[j]);
    let quat = |t: f64| {
        let m = Transform {
            rot: euler(t),
            ..Transform::IDENTITY
        }
        .rotation();
        DQuat::from_mat3(&m)
    };
    // the blend fraction: the segment as a 0 -> 1 curve
    let f = tracks
        .iter()
        .flatten()
        .find_map(|tr| {
            let i = tr.keys.iter().position(|k| k.time == ta)?;
            let mut unit = Track::new(vec![
                Key::new(ta, 0.0).with_interp(tr.keys[i].interp),
                Key::new(tb, 1.0),
            ]);
            if let Some(tg) = tr.tangents.get(i) {
                unit.set_tangent(0, *tg);
            }
            Some(value_at_exact(&unit, &[], time))
        })
        .unwrap_or((time - ta) / (tb - ta));
    let q = quat(ta).slerp(quat(tb), f);
    hpb_from_rotation(DMat3::from_quat(q.normalize()))
}
