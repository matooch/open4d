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

/// Keys sorted by time, no two at the same time.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub keys: Vec<Key>,
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
    #[error("{0:?} is not an animatable float or vector component")]
    BadTarget(String),
}

impl Track {
    pub fn new(keys: Vec<Key>) -> Track {
        Track { keys }
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
        Ok(())
    }

    /// Inserts `key`, replacing a key at exactly the same time.
    pub fn set_key(&mut self, key: Key) {
        match self.keys.binary_search_by(|k| k.time.total_cmp(&key.time)) {
            Ok(i) => self.keys[i] = key,
            Err(i) => self.keys.insert(i, key),
        }
    }

    /// Removes the key at exactly `time`; returns whether there was one.
    pub fn remove_key(&mut self, time: f64) -> bool {
        match self.keys.binary_search_by(|k| k.time.total_cmp(&time)) {
            Ok(i) => {
                self.keys.remove(i);
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
