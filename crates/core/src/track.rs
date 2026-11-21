//! Keyframe tracks: data only (docs/specs/ANIM.md §1). Evaluation lives in forge-anim.

use serde::{Deserialize, Serialize};

use crate::{ParamDesc, ParamKind};

/// How the segment that starts at a key is interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Interp {
    /// Ease in and out with automatic tangents (the default of a new key).
    #[default]
    Spline,
    Linear,
    Step,
}

/// One key: time in seconds, value in parameter units (radians for angles).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub time: f64,
    pub value: f64,
    pub interp: Interp,
}

impl Key {
    pub fn new(time: f64, value: f64) -> Key {
        Key {
            time,
            value,
            interp: Interp::Spline,
        }
    }

    pub fn with_interp(mut self, interp: Interp) -> Key {
        self.interp = interp;
        self
    }
}

/// How a handle of a key is defined (docs/specs/ANIM.md section 1.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TangentMode {
    /// The automatic ease of section 1.5.
    #[default]
    Auto,
    /// A third of the span and of the chord to the neighbouring key: a straight line.
    Chord,
    /// The stored `dt` / `dv`.
    User,
}

/// One handle: its end relative to the key (`dt` in seconds, negative on the left side).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Handle {
    pub mode: TangentMode,
    pub dt: f64,
    pub dv: f64,
}

/// The tangents of one key. `broken` means the two sides are edited independently (editing
/// only); `weighted` means the time component of a user handle is free (editing only: the
/// stored handle end is what is evaluated either way).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct KeyTangent {
    pub left: Handle,
    pub right: Handle,
    pub weighted: bool,
    pub broken: bool,
}

impl KeyTangent {
    /// A user tangent with both handle ends given (left `dt` is made non-positive).
    pub fn user(left: (f64, f64), right: (f64, f64)) -> KeyTangent {
        let h = |(dt, dv): (f64, f64), sign: f64| Handle {
            mode: TangentMode::User,
            dt: sign * dt.abs(),
            dv,
        };
        KeyTangent {
            left: h(left, -1.0),
            right: h(right, 1.0),
            weighted: false,
            broken: false,
        }
    }

    /// Both sides on the straight line to the neighbours.
    pub fn chord() -> KeyTangent {
        let h = Handle {
            mode: TangentMode::Chord,
            ..Handle::default()
        };
        KeyTangent {
            left: h,
            right: h,
            ..KeyTangent::default()
        }
    }
}

/// What a track does outside its first and last key (ANIM.md section 1.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Behavior {
    /// Holds the end value.
    #[default]
    Constant,
    /// Carries on with the end tangent's slope.
    Continue,
    /// Repeats the key range.
    Repeat,
    /// Repeats the key range, adding the range's value change each cycle.
    OffsetRepeat,
    /// Plays the key range forwards and backwards alternately.
    Oscillate,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// Keys sorted by time, no two at the same time. The optional side data (`tangents`,
/// `points`) is either empty or holds one entry per key.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub keys: Vec<Key>,
    /// Per-key tangents; empty means every key is automatic.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tangents: Vec<KeyTangent>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub before: Behavior,
    #[serde(default, skip_serializing_if = "is_default")]
    pub after: Behavior,
    /// Point-level animation: the point array of each key (a mesh point track). The key's
    /// `value` is unused (0).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<Vec<[f64; 3]>>,
    /// Rotation tracks only: interpolate the three Euler components as one orientation by
    /// spherical interpolation (ANIM.md section 1.11).
    #[serde(default, skip_serializing_if = "is_default")]
    pub quaternion: bool,
}

/// Why a track or a track key is rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrackError {
    #[error("a track needs at least one key")]
    Empty,
    #[error("key time or value is not finite")]
    NotFinite,
    #[error("keys must be sorted by time with no two at the same time")]
    Unsorted,
    #[error("{0:?} is not an animatable parameter")]
    BadTarget(String),
    #[error("side data (tangents or points) does not match the keys")]
    SideData,
}

impl Track {
    pub fn new(keys: Vec<Key>) -> Track {
        Track {
            keys,
            ..Track::default()
        }
    }

    /// Non-empty, finite, strictly increasing times.
    pub fn validate(&self) -> Result<(), TrackError> {
        if self.keys.is_empty() {
            return Err(TrackError::Empty);
        }
        if self
            .keys
            .iter()
            .any(|k| !k.time.is_finite() || !k.value.is_finite())
        {
            return Err(TrackError::NotFinite);
        }
        if self.keys.windows(2).any(|w| w[0].time >= w[1].time) {
            return Err(TrackError::Unsorted);
        }
        let n = self.keys.len();
        if !self.tangents.is_empty() {
            let finite = |h: &Handle| h.dt.is_finite() && h.dv.is_finite();
            if self.tangents.len() != n
                || self
                    .tangents
                    .iter()
                    .any(|t| !finite(&t.left) || !finite(&t.right))
            {
                return Err(TrackError::SideData);
            }
        }
        if !self.points.is_empty() {
            let len = self.points[0].len();
            if self.points.len() != n || self.points.iter().any(|p| p.len() != len) {
                return Err(TrackError::SideData);
            }
            if self
                .points
                .iter()
                .flatten()
                .flatten()
                .any(|c| !c.is_finite())
            {
                return Err(TrackError::NotFinite);
            }
        }
        Ok(())
    }

    /// Inserts `key`, replacing a key at exactly the same time.
    pub fn set_key(&mut self, key: Key) {
        match self.keys.binary_search_by(|k| k.time.total_cmp(&key.time)) {
            Ok(i) => self.keys[i] = key,
            Err(i) => {
                self.keys.insert(i, key);
                if !self.tangents.is_empty() {
                    self.tangents.insert(i, KeyTangent::default());
                }
                if !self.points.is_empty() {
                    let fill = self.points[0].clone();
                    self.points.insert(i, fill);
                }
            }
        }
    }

    /// Sets the tangent of key `i` (no-op past the last key).
    pub fn set_tangent(&mut self, i: usize, tangent: KeyTangent) {
        if i >= self.keys.len() {
            return;
        }
        self.tangents.resize(self.keys.len(), KeyTangent::default());
        self.tangents[i] = tangent;
    }

    /// Inserts a point-level key (a whole point array) at `time`; all keys of a point track
    /// have the same point count. Returns false when the count differs from the track's.
    pub fn set_points_key(&mut self, time: f64, interp: Interp, points: Vec<[f64; 3]>) -> bool {
        if self.points.first().is_some_and(|p| p.len() != points.len()) {
            return false;
        }
        if self.points.is_empty() && !self.keys.is_empty() {
            // a track that had float keys becomes a point track: repeat the first array
            self.points = vec![points.clone(); self.keys.len()];
        }
        let key = Key::new(time, 0.0).with_interp(interp);
        match self.keys.binary_search_by(|k| k.time.total_cmp(&time)) {
            Ok(i) => {
                self.keys[i] = key;
                self.points[i] = points;
            }
            Err(i) => {
                self.keys.insert(i, key);
                self.points.insert(i, points);
                if !self.tangents.is_empty() {
                    self.tangents.insert(i, KeyTangent::default());
                }
            }
        }
        true
    }

    /// Removes the key at exactly `time`; returns whether there was one.
    pub fn remove_key(&mut self, time: f64) -> bool {
        match self.keys.binary_search_by(|k| k.time.total_cmp(&time)) {
            Ok(i) => {
                self.keys.remove(i);
                if !self.tangents.is_empty() {
                    self.tangents.remove(i);
                }
                if !self.points.is_empty() {
                    self.points.remove(i);
                }
                true
            }
            Err(_) => false,
        }
    }
}

/// What a track key addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A Float parameter.
    Float(String),
    /// Component `0..3` of a Vec3 parameter.
    Component(String, usize),
}

/// `"<param>"` for a float, `"<param>.<x|y|z>"` for a Vec3 component, checked against
/// the type's descriptors (`find`). Anything else, or a non-animatable parameter, is an
/// error.
pub fn parse_target<'a>(
    key: &str,
    find: impl Fn(&str) -> Option<&'a ParamDesc>,
) -> Result<Target, TrackError> {
    let bad = || TrackError::BadTarget(key.to_string());
    if let Some(d) = find(key) {
        return match d.kind {
            ParamKind::Float { .. } if d.animatable => Ok(Target::Float(key.to_string())),
            _ => Err(bad()),
        };
    }
    let (param, comp) = key.rsplit_once('.').ok_or_else(bad)?;
    let axis = match comp {
        "x" => 0,
        "y" => 1,
        "z" => 2,
        _ => return Err(bad()),
    };
    match find(param) {
        Some(d) if d.animatable && matches!(d.kind, ParamKind::Vec3 { .. }) => {
            Ok(Target::Component(param.to_string(), axis))
        }
        _ => Err(bad()),
    }
}

/// What a track key addresses, for every animatable parameter type (ANIM.md section 1.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackTarget {
    Float(String),
    Component(String, usize),
    /// An Int parameter (evaluated as a linear track, floored).
    Int(String),
    /// A Bool parameter (key value 0 or 1, always a step).
    Bool(String),
    /// An Enum parameter (key value = option index, always a step).
    Enum(String),
    /// A Colour parameter's channel `0..3` (`"<param>.r|g|b"`).
    Channel(String, usize),
    /// A Link parameter (key value = node id, 0 for none; always a step).
    Link(String),
    /// The points of a Mesh parameter (`"<param>.points"`, a point-level track).
    Points(String),
}

/// Like [`parse_target`], for every animatable type. Bool, Int, Float, Vec3 and Colour
/// parameters follow their descriptor's `animatable` flag; Enum and Link parameters are
/// always keyable; Mesh parameters are animatable through `"<param>.points"` only.
pub fn parse_track_target<'a>(
    key: &str,
    find: impl Fn(&str) -> Option<&'a ParamDesc>,
) -> Result<TrackTarget, TrackError> {
    let bad = || TrackError::BadTarget(key.to_string());
    if let Some(d) = find(key) {
        let k = key.to_string();
        // Enum and Link parameters are keyable whatever their `animatable` flag (their
        // descriptors default it to false); the other kinds follow the flag.
        return match d.kind {
            ParamKind::Float { .. } if d.animatable => Ok(TrackTarget::Float(k)),
            ParamKind::Int { .. } if d.animatable => Ok(TrackTarget::Int(k)),
            ParamKind::Bool if d.animatable => Ok(TrackTarget::Bool(k)),
            ParamKind::Enum { .. } => Ok(TrackTarget::Enum(k)),
            ParamKind::Link { .. } => Ok(TrackTarget::Link(k)),
            _ => Err(bad()),
        };
    }
    let (param, comp) = key.rsplit_once('.').ok_or_else(bad)?;
    let d = find(param).ok_or_else(bad)?;
    let axis = |names: [&str; 3]| names.iter().position(|n| *n == comp);
    match d.kind {
        ParamKind::Vec3 { .. } if d.animatable => axis(["x", "y", "z"])
            .map(|a| TrackTarget::Component(param.to_string(), a))
            .ok_or_else(bad),
        ParamKind::Color if d.animatable => axis(["r", "g", "b"])
            .map(|a| TrackTarget::Channel(param.to_string(), a))
            .ok_or_else(bad),
        ParamKind::Mesh if comp == "points" => Ok(TrackTarget::Points(param.to_string())),
        _ => Err(bad()),
    }
}
