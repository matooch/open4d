//! Weight painting and smoothing (docs/specs/RIG.md section 9). Pure math on weight maps.

use std::collections::BTreeMap;

use glam::DVec3;

use crate::weights::WeightMap;

/// What a stroke does to the active joint's weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BrushMode {
    #[default]
    Add,
    Subtract,
    Set,
    Smooth,
}

/// How the effect fades with the distance from the stroke centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Falloff {
    Constant,
    Linear,
    #[default]
    Smooth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Brush {
    pub mode: BrushMode,
    pub radius: f64,
    pub strength: f64,
    pub falloff: Falloff,
    /// The value `Set` moves towards.
    pub target: f64,
}

impl Default for Brush {
    fn default() -> Brush {
        Brush {
            mode: BrushMode::Add,
            radius: 50.0,
            strength: 1.0,
            falloff: Falloff::Smooth,
            target: 1.0,
        }
    }
}

impl Brush {
    /// The factor `strength * falloff(d)` for a point at distance `d`; 0 outside the radius.
    pub fn factor(&self, d: f64) -> f64 {
        if self.radius <= 0.0 || self.radius.is_nan() || d >= self.radius || d < 0.0 {
            return 0.0;
        }
        let t = d / self.radius;
        let fall = match self.falloff {
            Falloff::Constant => 1.0,
            Falloff::Linear => 1.0 - t,
            Falloff::Smooth => 1.0 - (3.0 * t * t - 2.0 * t * t * t),
        };
        self.strength.clamp(0.0, 1.0) * fall
    }
}

/// For every point the points that share an edge with it, sorted, without duplicates.
pub fn point_adjacency(polys: &[Vec<u32>], point_count: usize) -> Vec<Vec<u32>> {
    let mut adj = vec![Vec::new(); point_count];
    for poly in polys {
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            if a == b || a as usize >= point_count || b as usize >= point_count {
                continue;
            }
            adj[a as usize].push(b);
            adj[b as usize].push(a);
        }
    }
    for n in &mut adj {
        n.sort_unstable();
        n.dedup();
    }
    adj
}

/// Applies one brush dab for joint `slot` centred at `centre`; returns the number of points
/// inside the radius.
pub fn stroke(
    weights: &mut WeightMap,
    points: &[DVec3],
    adjacency: &[Vec<u32>],
    slot: u32,
    centre: DVec3,
    brush: &Brush,
) -> usize {
    let old = weights.clone();
    let mut inside = 0;
    for (i, p) in points.iter().enumerate() {
        let d = (*p - centre).length();
        if d >= brush.radius || d.is_nan() {
            continue;
        }
        inside += 1;
        let f = brush.factor(d);
        let i = i as u32;
        let row = old.get(i);
        let w = row
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or(0.0, |(_, w)| *w);
        let new = match brush.mode {
            BrushMode::Add => (w + f).min(1.0),
            BrushMode::Subtract => (w - f).max(0.0),
            BrushMode::Set => w + (brush.target.clamp(0.0, 1.0) - w) * f,
            BrushMode::Smooth => {
                let Some(nb) = adjacency.get(i as usize).filter(|n| !n.is_empty()) else {
                    continue;
                };
                let avg = nb
                    .iter()
                    .map(|n| {
                        old.get(*n)
                            .iter()
                            .find(|(s, _)| *s == slot)
                            .map_or(0.0, |(_, w)| *w)
                    })
                    .sum::<f64>()
                    / nb.len() as f64;
                w + (avg - w) * f
            }
        };
        let others: f64 = row
            .iter()
            .filter(|(s, _)| *s != slot)
            .map(|(_, w)| *w)
            .sum();
        let scale = if others > 0.0 {
            (1.0 - new) / others
        } else {
            1.0
        };
        let mut out: Vec<(u32, f64)> = row
            .iter()
            .filter(|(s, _)| *s != slot)
            .map(|(s, w)| (*s, *w * scale))
            .collect();
        out.push((slot, new));
        weights.set(i, out);
    }
    inside
}

/// Relaxes all weights towards their neighbours: `iterations` times every row becomes
/// `(1 - factor) row + factor mean(neighbour rows)`.
pub fn smooth_weights(
    weights: &mut WeightMap,
    adjacency: &[Vec<u32>],
    iterations: usize,
    factor: f64,
) {
    let factor = factor.clamp(0.0, 1.0);
    for _ in 0..iterations {
        let old = weights.clone();
        for (i, nb) in adjacency.iter().enumerate() {
            if nb.is_empty() {
                continue;
            }
            let i = i as u32;
            let mut acc: BTreeMap<u32, f64> = BTreeMap::new();
            for (s, w) in old.get(i) {
                *acc.entry(*s).or_default() += (1.0 - factor) * w;
            }
            let k = factor / nb.len() as f64;
            for n in nb {
                for (s, w) in old.get(*n) {
                    *acc.entry(*s).or_default() += k * w;
                }
            }
            weights.set(i, acc.into_iter().collect());
        }
    }
}
