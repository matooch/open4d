//! The Jiggle deformer (docs/specs/MODIFIERS.md §14.5): springs pulling the points to
//! their moving rest positions. The simulation is a pure function of the target's
//! world matrices over the replayed frames, so it needs no hidden cache state.

use glam::{DMat4, DVec3};

/// Jiggle parameters; `strength`, `stiffness`, `structural`, `drag` and `motion_scale`
/// are percent as the app shows them.
#[derive(Debug, Clone, Copy)]
pub struct JiggleParams {
    pub strength: f64,
    pub stiffness: f64,
    pub structural: f64,
    pub drag: f64,
    pub motion_scale: f64,
    pub springs: usize,
    pub iterations: usize,
    pub gravity: f64,
    pub local: bool,
    pub fps: f64,
}

/// Squared spring frequency at 100 % stiffness, per second squared.
const OMEGA2_AT_100: f64 = 2000.0;

/// Simulates the points `rest` (target space, the input of the frame) through the
/// target world matrices `worlds[0..=n]` (the replayed frames, the last one is the
/// frame asked for) and returns the points of the last frame in target space.
pub fn jiggle(
    rest: &[DVec3],
    neighbours: &[Vec<u32>],
    worlds: &[DMat4],
    p: &JiggleParams,
) -> Vec<DVec3> {
    let Some(&last) = worlds.last() else {
        return rest.to_vec();
    };
    let to_local = last.inverse();
    if rest.is_empty() || !to_local.is_finite() {
        return rest.to_vec();
    }
    // Local: the simulation runs in the target's space and ignores its motion.
    let space = |w: &DMat4| if p.local { DMat4::IDENTITY } else { *w };
    let start: Vec<DVec3> = rest
        .iter()
        .map(|r| space(&worlds[0]).transform_point3(*r))
        .collect();
    let m = p.motion_scale / 100.0;
    let n = p.iterations.max(1);
    let h = 1.0 / (p.fps.max(1e-9) * n as f64);
    let omega2 = OMEGA2_AT_100 * p.stiffness / 100.0;
    let damp = (1.0 - (p.drag / 100.0).clamp(0.0, 1.0)).powf(1.0 / n as f64);
    let coupling = 0.5 * (p.structural / 100.0).clamp(0.0, 1.0);
    let gravity = DVec3::new(0.0, -p.gravity, 0.0);

    let mut x = start.clone();
    let mut v = vec![DVec3::ZERO; rest.len()];
    let mut target = start.clone();
    for w in worlds.iter().skip(1) {
        let wm = space(w);
        for (t, (r, s)) in target.iter_mut().zip(rest.iter().zip(&start)) {
            *t = *s + (wm.transform_point3(*r) - *s) * m;
        }
        for _ in 0..n {
            for ((xi, vi), ti) in x.iter_mut().zip(v.iter_mut()).zip(&target) {
                let a = (*ti - *xi) * omega2 + gravity;
                *vi = (*vi + a * h) * damp;
                *xi += *vi * h;
            }
            for _ in 0..p.springs {
                let d: Vec<DVec3> = x.iter().zip(&target).map(|(x, t)| *x - *t).collect();
                for (i, xi) in x.iter_mut().enumerate() {
                    let nb = &neighbours[i];
                    if nb.is_empty() {
                        continue;
                    }
                    let mean: DVec3 =
                        nb.iter().map(|j| d[*j as usize]).sum::<DVec3>() / nb.len() as f64;
                    *xi += (mean - d[i]) * coupling;
                }
            }
        }
    }
    let s = p.strength / 100.0;
    x.iter()
        .zip(&target)
        .map(|(x, t)| {
            let w = *t + (*x - *t) * s;
            if p.local {
                w
            } else {
                to_local.transform_point3(w)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> JiggleParams {
        JiggleParams {
            strength: 100.0,
            stiffness: 40.0,
            structural: 100.0,
            drag: 10.0,
            motion_scale: 100.0,
            springs: 4,
            iterations: 4,
            gravity: 0.0,
            local: false,
            fps: 30.0,
        }
    }

    fn walk(frames: usize, speed: f64) -> Vec<DMat4> {
        (0..=frames)
            .map(|k| DMat4::from_translation(DVec3::new(speed * k as f64, 0.0, 0.0)))
            .collect()
    }

    #[test]
    fn a_static_object_does_not_jiggle() {
        let rest = vec![DVec3::new(1.0, 2.0, 3.0), DVec3::new(-5.0, 0.0, 4.0)];
        let nb = vec![vec![1], vec![0]];
        let out = jiggle(&rest, &nb, &walk(10, 0.0), &params());
        for (a, b) in out.iter().zip(&rest) {
            assert!((*a - *b).length() < 1e-9);
        }
        assert_eq!(jiggle(&rest, &nb, &[], &params()), rest);
    }

    #[test]
    fn a_moving_object_makes_the_points_lag_and_strength_zero_is_rigid() {
        let rest = vec![DVec3::new(1.0, 2.0, 3.0)];
        let nb = vec![vec![]];
        let w = walk(2, 30.0);
        let out = jiggle(&rest, &nb, &w, &params());
        // in the object's space the point sits behind its rest position (negative x)
        assert!(out[0].x < rest[0].x - 1.0, "{:?}", out[0]);
        let again = jiggle(&rest, &nb, &w, &params());
        assert_eq!(out, again);
        let rigid = jiggle(
            &rest,
            &nb,
            &w,
            &JiggleParams {
                strength: 0.0,
                ..params()
            },
        );
        assert!((rigid[0] - rest[0]).length() < 1e-9);
    }

    #[test]
    fn gravity_sags_the_points_in_local_mode() {
        let rest = vec![DVec3::ZERO];
        let nb = vec![vec![]];
        let p = JiggleParams {
            local: true,
            gravity: 980.0,
            ..params()
        };
        let out = jiggle(&rest, &nb, &walk(10, 0.0), &p);
        assert!(out[0].y < -1.0);
    }
}
