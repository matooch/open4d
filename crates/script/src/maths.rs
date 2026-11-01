//! `vec3` and the seeded generator (docs/specs/SCRIPTING.md §3).

use rhai::{Dynamic, Engine};

use crate::host::error;

type RhaiResult<T> = Result<T, Box<rhai::EvalAltResult>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { x, y, z }
    }

    pub fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    fn map(self, f: impl Fn(f64) -> f64) -> Vec3 {
        Vec3::new(f(self.x), f(self.y), f(self.z))
    }

    fn zip(self, o: Vec3, f: impl Fn(f64, f64) -> f64) -> Vec3 {
        Vec3::new(f(self.x, o.x), f(self.y, o.y), f(self.z, o.z))
    }

    fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
}

/// An integer or float argument as `f64`.
pub fn number(d: &Dynamic) -> RhaiResult<f64> {
    if let Ok(i) = d.as_int() {
        Ok(i as f64)
    } else if let Ok(f) = d.as_float() {
        Ok(f)
    } else {
        Err(error(format!("expected a number, got {}", d.type_name())))
    }
}

/// SplitMix64: the same seed gives the same sequence everywhere.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: i64) -> Rng {
        Rng { state: seed as u64 }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// In `[0, 1)`: the top 53 bits over 2^53.
    pub fn float(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// In `[a, b]`.
    pub fn int(&mut self, a: i64, b: i64) -> i64 {
        let span = (i128::from(b) - i128::from(a) + 1) as u128;
        let offset = u128::from(self.next_u64()) % span;
        (i128::from(a) + offset as i128) as i64
    }
}

pub fn register(engine: &mut Engine) {
    engine.register_type_with_name::<Vec3>("vec3");
    engine.register_fn(
        "vec3",
        |x: Dynamic, y: Dynamic, z: Dynamic| -> RhaiResult<Vec3> {
            Ok(Vec3::new(number(&x)?, number(&y)?, number(&z)?))
        },
    );
    engine.register_get_set("x", |v: &mut Vec3| v.x, |v: &mut Vec3, n: f64| v.x = n);
    engine.register_get_set("y", |v: &mut Vec3| v.y, |v: &mut Vec3, n: f64| v.y = n);
    engine.register_get_set("z", |v: &mut Vec3| v.z, |v: &mut Vec3, n: f64| v.z = n);
    engine.register_fn("==", |a: Vec3, b: Vec3| a == b);
    engine.register_fn("!=", |a: Vec3, b: Vec3| a != b);
    engine.register_fn("+", |a: Vec3, b: Vec3| a.zip(b, |p, q| p + q));
    engine.register_fn("-", |a: Vec3, b: Vec3| a.zip(b, |p, q| p - q));
    engine.register_fn("-", |a: Vec3| a.map(|p| -p));
    engine.register_fn("*", |a: Vec3, s: Dynamic| -> RhaiResult<Vec3> {
        let s = number(&s)?;
        Ok(a.map(|p| p * s))
    });
    engine.register_fn("*", |s: Dynamic, a: Vec3| -> RhaiResult<Vec3> {
        let s = number(&s)?;
        Ok(a.map(|p| p * s))
    });
    engine.register_fn("/", |a: Vec3, s: Dynamic| -> RhaiResult<Vec3> {
        let s = number(&s)?;
        Ok(a.map(|p| p / s))
    });
    engine.register_fn("len", |a: &mut Vec3| a.dot(*a).sqrt());
    engine.register_fn("dot", |a: &mut Vec3, b: Vec3| a.dot(b));
    engine.register_fn("cross", |a: &mut Vec3, b: Vec3| {
        Vec3::new(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x,
        )
    });
    engine.register_fn("normalized", |a: &mut Vec3| {
        let n = a.dot(*a).sqrt();
        if n == 0.0 { *a } else { a.map(|p| p / n) }
    });
    engine.register_fn("to_array", |a: &mut Vec3| -> rhai::Array {
        a.to_array().into_iter().map(Dynamic::from).collect()
    });
    engine.register_fn("to_string", |a: &mut Vec3| {
        format!("vec3({}, {}, {})", a.x, a.y, a.z)
    });
    engine.register_fn("to_debug", |a: &mut Vec3| {
        format!("vec3({}, {}, {})", a.x, a.y, a.z)
    });

    engine.register_fn("deg", |x: Dynamic| -> RhaiResult<f64> {
        Ok(number(&x)?.to_radians())
    });
    engine.register_fn("to_deg", |x: Dynamic| -> RhaiResult<f64> {
        Ok(number(&x)?.to_degrees())
    });

    engine.register_type_with_name::<Rng>("rng");
    engine.register_fn("rand", Rng::new);
    engine.register_fn("float", |r: &mut Rng| r.float());
    engine.register_fn(
        "range",
        |r: &mut Rng, a: Dynamic, b: Dynamic| -> RhaiResult<f64> {
            let (a, b) = (number(&a)?, number(&b)?);
            Ok(a + (b - a) * r.float())
        },
    );
    engine.register_fn("int", |r: &mut Rng, a: i64, b: i64| -> RhaiResult<i64> {
        if a > b {
            return Err(error(format!("int({a}, {b}): a must not exceed b")));
        }
        Ok(r.int(a, b))
    });
}
