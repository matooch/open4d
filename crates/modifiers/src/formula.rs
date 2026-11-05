//! The Formula deformer: text formulas for the displacement `d` and the point
//! (docs/specs/MODIFIERS.md §17, language: docs/specs/EXPR.md).

use forge_expr::Expr;
use glam::DVec3;

/// The effect: how `u`, `v`, the displacement direction and its scale are taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Manual,
    Spherical,
    Cylindrical,
    XRadial,
    YRadial,
    ZRadial,
}

impl Effect {
    /// From the enum id of the param.
    pub fn from_id(id: &str) -> Effect {
        match id {
            "manual" => Effect::Manual,
            "spherical" => Effect::Spherical,
            "cylindrical" => Effect::Cylindrical,
            "x_radial" => Effect::XRadial,
            "z_radial" => Effect::ZRadial,
            _ => Effect::YRadial,
        }
    }
}

/// Formula parameters; the formulas are the texts of the node.
#[derive(Debug, Clone)]
pub struct FormulaParams {
    pub size: DVec3,
    pub effect: Effect,
    pub d: String,
    pub x: String,
    pub y: String,
    pub z: String,
}

/// `Some(expr)` for a formula that parses with the given variable names.
fn compile(src: &str, vars: &[&str]) -> Option<Expr> {
    Expr::parse(src, vars).ok()
}

/// `(u, v, dir, amp)` of the effect for the point `p` (spec §17).
fn frame(effect: Effect, p: DVec3, size: DVec3) -> (f64, f64, DVec3, f64) {
    let h = size * 0.5;
    // A zero half-size would divide by zero: the unit ellipse degenerates to 0.
    let norm = |a: f64, b: f64, ha: f64, hb: f64| {
        let f = |v: f64, h: f64| if h > 0.0 { v / h } else { 0.0 };
        f(a, ha).hypot(f(b, hb))
    };
    let along = |v: f64, s: f64| if s != 0.0 { v / s + 0.5 } else { 0.5 };
    match effect {
        Effect::YRadial => (norm(p.x, p.z, h.x, h.z), along(p.y, size.y), DVec3::Y, h.y),
        Effect::XRadial => (norm(p.y, p.z, h.y, h.z), along(p.x, size.x), DVec3::X, h.x),
        Effect::ZRadial => (norm(p.x, p.y, h.x, h.y), along(p.z, size.z), DVec3::Z, h.z),
        Effect::Cylindrical => {
            let u = (p.z.atan2(p.x) / std::f64::consts::TAU).rem_euclid(1.0);
            let out = DVec3::new(p.x, 0.0, p.z).normalize_or_zero();
            (u, along(p.y, size.y), out, h.x)
        }
        Effect::Spherical => {
            let r = p.length();
            let u = (p.z.atan2(p.x) / std::f64::consts::TAU).rem_euclid(1.0);
            let v = if r > 0.0 {
                (p.y / r).clamp(-1.0, 1.0).asin() / std::f64::consts::PI + 0.5
            } else {
                0.5
            };
            (u, v, p.normalize_or_zero(), h.x)
        }
        Effect::Manual => (along(p.x, size.x), along(p.z, size.z), DVec3::Y, h.y),
    }
}

/// Moves `points` (in the deformer's space) at `time` seconds.
pub fn formula(points: &mut [DVec3], p: &FormulaParams, time: f64) {
    let d = compile(&p.d, &["u", "v", "x", "y", "z", "t"]);
    let axes = [
        compile(&p.x, &["x", "y", "z", "t"]),
        compile(&p.y, &["x", "y", "z", "t"]),
        compile(&p.z, &["x", "y", "z", "t"]),
    ];
    for q in points.iter_mut() {
        let o = *q;
        let (u, v, dir, amp) = frame(p.effect, o, p.size);
        let disp = d
            .as_ref()
            .map(|e| e.eval(&[u, v, o.x, o.y, o.z, time]))
            .filter(|x| x.is_finite())
            .unwrap_or(0.0);
        let mut out = o.to_array();
        for (slot, e) in out.iter_mut().zip(&axes) {
            if let Some(e) = e {
                let val = e.eval(&[o.x, o.y, o.z, time]);
                if val.is_finite() {
                    *slot = val;
                }
            }
        }
        *q = DVec3::from_array(out) + dir * (disp * amp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(effect: Effect, d: &str) -> FormulaParams {
        FormulaParams {
            size: DVec3::splat(400.0),
            effect,
            d: d.to_string(),
            x: "x".to_string(),
            y: "y".to_string(),
            z: "z".to_string(),
        }
    }

    #[test]
    fn y_radial_displaces_along_y_by_d_times_half_height() {
        let mut pts = vec![DVec3::new(100.0, 7.0, 0.0)];
        formula(&mut pts, &params(Effect::YRadial, "u"), 0.0);
        assert!((pts[0].y - (7.0 + 0.5 * 200.0)).abs() < 1e-12);
        assert_eq!((pts[0].x, pts[0].z), (100.0, 0.0));
    }

    #[test]
    fn bad_formulas_do_nothing() {
        let mut pts = vec![DVec3::new(1.0, 2.0, 3.0)];
        let mut p = params(Effect::YRadial, "1 +");
        p.x = "nonsense(".to_string();
        p.y = "1/0".to_string();
        formula(&mut pts, &p, 0.0);
        assert_eq!(pts[0], DVec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn point_formulas_and_time() {
        let mut pts = vec![DVec3::new(1.0, 2.0, 3.0)];
        let mut p = params(Effect::YRadial, "0");
        p.x = "x*2".to_string();
        p.z = "z+t".to_string();
        formula(&mut pts, &p, 0.5);
        assert_eq!(pts[0], DVec3::new(2.0, 2.0, 3.5));
    }
}
